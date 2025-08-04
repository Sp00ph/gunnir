use core::fmt;
use std::ops::Not;

use crate::*;

define_enum!(
    pub enum PieceType {
        Knight,
        Bishop,
        Rook,
        Queen,

        // We intentionally place these last so that 0-3 correspond to pieces that can be promoted to.
        Pawn,
        King,
    }
);

impl PieceType {
    pub fn to_char(self, color: Color) -> char {
        let mut ch = match self {
            PieceType::Knight => 'N',
            PieceType::Bishop => 'B',
            PieceType::Rook => 'R',
            PieceType::Queen => 'Q',
            PieceType::Pawn => 'P',
            PieceType::King => 'K',
        };

        if color == Color::Black {
            ch = ch.to_ascii_lowercase();
        }

        ch
    }
}

impl fmt::Debug for PieceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Write::write_char(f, self.to_char(Color::from_idx(f.alternate() as u8)))
    }
}

define_enum!(
    pub enum Color {
        White,
        Black,
    }
);

impl Color {
    #[inline]
    pub const fn invert(self) -> Self {
        Self::from_idx(1 - self.idx())
    }

    #[inline]
    /// Returns 1 for white, -1 for black
    pub const fn signum(self) -> i8 {
        1 - 2 * self.idx() as i8
    }

    #[inline]
    pub const fn to_char(self) -> char {
        match self {
            Color::White => 'w',
            Color::Black => 'b',
        }
    }
}

impl fmt::Debug for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Write::write_char(f, self.to_char())
    }
}

impl Not for Color {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        self.invert()
    }
}
