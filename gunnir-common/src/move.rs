use std::num::NonZeroU16;

use crate::*;

/// We pack all necessary information about a move into one sixteen bit
/// integer like so:
///
/// Bits 0-5:   Starting square index
/// Bits 6-11:  Target square index
/// Bits 12-13: Special move flag (none, castles, en passant, promote)
/// Bits 14-15: Promotion piece type (knight, bishop, rook, queen)
///
/// Additionally, since no move may have the same start and target square,
/// (except castles, which always lands on a nonzero file), at least one of
/// the square indices must be nonzero, so we can use a NonZeroU16 as storage
/// to introduce a niche value.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move(NonZeroU16);

define_enum!(
    #[derive(Debug)]
    pub enum MoveFlag {
        None,
        Castle,
        EnPassant,
        Promotion,
    }
);

impl Move {
    #[inline]
    pub const fn new(from: Square, to: Square, flag: MoveFlag) -> Self {
        debug_assert!(!matches!(flag, MoveFlag::Promotion));
        Self(
            NonZeroU16::new(
                (from.idx() as u16) | ((to.idx() as u16) << 6) | ((flag.idx() as u16) << 12),
            )
            .expect("Invalid move squares"),
        )
    }

    #[inline]
    pub const fn new_promotion(from: Square, to: Square, promote_to: PieceType) -> Self {
        debug_assert!(promote_to.idx() < 4);
        Self(
            NonZeroU16::new(
                (from.idx() as u16)
                    | ((to.idx() as u16) << 6)
                    | ((MoveFlag::Promotion.idx() as u16) << 12)
                    | ((promote_to.idx() as u16) << 14),
            )
            .unwrap(),
        )
    }

    #[inline]
    pub const fn from(self) -> Square {
        Square::from_idx(self.0.get() as u8 & 0x3f)
    }

    #[inline]
    pub const fn to(self) -> Square {
        Square::from_idx((self.0.get() >> 6) as u8 & 0x3f)
    }

    #[inline]
    pub const fn move_flag(self) -> MoveFlag {
        MoveFlag::from_idx((self.0.get() >> 12) as u8 & 0x3)
    }

    #[inline]
    pub const fn promotes_to(self) -> Option<PieceType> {
        if !matches!(self.move_flag(), MoveFlag::Promotion) {
            None
        } else {
            Some(self.promotes_to_unchecked())
        }
    }

    #[inline]
    /// Returns the piece that this move promotes to. If the move
    /// doesn't promote, the return value is arbitrary.
    pub const fn promotes_to_unchecked(self) -> PieceType {
        PieceType::from_idx((self.0.get() >> 14) as u8 & 0x3)
    }
}

#[derive(Clone)]
pub struct PieceMoves {
    move_flag: MoveFlag,
    piece: PieceType,
    from: Square,
    to: Bitboard,
}

impl PieceMoves {
    #[inline]
    pub const fn new(move_flag: MoveFlag, piece: PieceType, from: Square, to: Bitboard) -> Self {
        Self {
            move_flag,
            piece,
            from,
            to,
        }
    }

    #[inline]
    pub const fn piece_type(&self) -> PieceType {
        self.piece
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.to.popcnt() as usize
            * if self.move_flag.idx() == MoveFlag::Promotion.idx() {
                4
            } else {
                1
            }
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.to.is_empty()
    }
}

#[derive(Clone)]
pub struct PieceMovesIter {
    moves: PieceMoves,
    promote_idx: u8,
}

impl Iterator for PieceMovesIter {
    type Item = Move;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let from = self.moves.from;
        let to = self.moves.to.try_next()?;

        if self.moves.move_flag == MoveFlag::Promotion {
            let mov = Move::new_promotion(
                from,
                to,
                // Promote to queen first
                PieceType::from_idx(3 - self.promote_idx),
            );
            self.promote_idx += 1;
            if self.promote_idx >= 4 {
                self.promote_idx = 0;
                self.moves.to ^= to;
            }

            Some(mov)
        } else {
            self.moves.to ^= to;
            Some(Move::new(from, to, self.moves.move_flag))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.moves.len() - self.promote_idx as usize;
        (n, Some(n))
    }
}

impl ExactSizeIterator for PieceMovesIter {}

impl IntoIterator for PieceMoves {
    type Item = Move;

    type IntoIter = PieceMovesIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        PieceMovesIter {
            moves: self,
            promote_idx: 0,
        }
    }
}
