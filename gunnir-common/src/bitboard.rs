use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct Bitboard(pub u64);

impl Bitboard {
    /// A bitboard representing the empty board, with no bits set.
    pub const EMPTY: Self = Self(0);

    /// A bitboard representing the full board, with all bits set.
    pub const UNIVERSE: Self = Self(u64::MAX);

    /// The diagonal a1-h8.
    pub const MAIN_DIAGONAL: Self = Self(0x8040201008040201);

    /// The diagonal a8-h1.
    pub const ANTI_DIAGONAL: Self = Self(0x0102040810204080);

    /// A bitboard representing the edges, with all squares on
    /// ranks 1 and 8 and on files A and H set.
    pub const EDGES: Self = File::A
        .bitboard()
        .union(File::H.bitboard())
        .union(Rank::R1.bitboard())
        .union(Rank::R8.bitboard());

    /// A bitboard representing all non-edge squares, with
    /// all squares set that are both on ranks 2-7 and
    /// on files B-G.
    pub const INNER: Self = Self::EDGES.invert();

    #[inline]
    pub const fn invert(self) -> Self {
        Self(!self.0)
    }

    #[inline]
    pub const fn subtract(self, rhs: Self) -> Self {
        self.intersect(rhs.invert())
    }

    #[inline]
    pub const fn contains(self, sq: Square) -> bool {
        self.intersect(sq.bitboard()).is_non_empty()
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn is_non_empty(self) -> bool {
        !self.is_empty()
    }

    #[inline]
    pub const fn try_next(self) -> Option<Square> {
        Square::try_from_idx(self.0.trailing_zeros() as u8)
    }

    #[inline]
    pub const fn next(self) -> Square {
        self.try_next().expect("Called next() on an empty bitboard")
    }

    #[inline]
    pub const fn popcnt(self) -> u8 {
        self.0.count_ones() as u8
    }

    #[inline]
    pub const fn shift_const<D: Direction, const STEPS: u8>(self) -> Self {
        shift_bb::<D>(self, STEPS as i8)
    }

    #[inline]
    pub const fn shift<D: Direction>(self, steps: i8) -> Self {
        shift_bb::<D>(self, steps)
    }

    #[inline]
    pub const fn main_diag_for(sq: Square) -> Self {
        let shift = sq.rank().idx() as i8 - sq.file().idx() as i8;

        Self::MAIN_DIAGONAL.shift::<Up>(shift)
    }

    #[inline]
    pub const fn anti_diag_for(sq: Square) -> Self {
        let shift = sq.rank().idx() as i8 + sq.file().idx() as i8 - 7;
        Self::ANTI_DIAGONAL.shift::<Up>(shift)
    }
}

impl Not for Bitboard {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        self.invert()
    }
}

macro_rules! impl_bitwise_ops {
    ($(($trait:ident, $fn:ident, $trait_assign:ident, $fn_assign:ident, $punct:tt, $method_name:ident),)*) => {
        impl Bitboard {
            $(
                #[inline]
                pub const fn $method_name(self, rhs: Self) -> Self {
                    Self(self.0 $punct rhs.0)
                }
            )*
        }

        $(
            impl $trait for Bitboard {
                type Output = Self;

                #[inline]
                fn $fn(self, rhs: Self) -> Self {
                    Self(self.0 $punct rhs.0)
                }
            }

            impl $trait<Square> for Bitboard {
                type Output = Self;

                #[inline]
                fn $fn(self, rhs: Square) -> Self {
                    Self(self.0 $punct rhs.bitboard().0)
                }
            }
        )*

        $(
            impl $trait_assign for Bitboard {
                #[inline]
                fn $fn_assign(&mut self, rhs: Self) {
                    self.0 = self.0 $punct rhs.0
                }
            }

            impl $trait_assign<Square> for Bitboard {
                #[inline]
                fn $fn_assign(&mut self, rhs: Square) {
                    self.0 = self.0 $punct rhs.bitboard().0
                }
            }
        )*
    };
}

impl_bitwise_ops!(
    (BitAnd, bitand, BitAndAssign, bitand_assign, &, intersect),
    (BitOr, bitor, BitOrAssign, bitor_assign, |, union),
    (BitXor, bitxor, BitXorAssign, bitxor_assign, ^, symmetric_difference),
);

pub struct BitboardIter(Bitboard);

impl Iterator for BitboardIter {
    type Item = Square;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let sq = self.0.try_next()?;

        self.0 ^= sq;
        Some(sq)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len();
        (n, Some(n))
    }
}

impl ExactSizeIterator for BitboardIter {
    #[inline]
    fn len(&self) -> usize {
        self.0.popcnt() as usize
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;

    type IntoIter = BitboardIter;

    fn into_iter(self) -> Self::IntoIter {
        BitboardIter(self)
    }
}

impl FromIterator<Square> for Bitboard {
    fn from_iter<T: IntoIterator<Item = Square>>(iter: T) -> Self {
        let mut bb = Self::EMPTY;
        bb.extend(iter);
        bb
    }
}

impl Extend<Square> for Bitboard {
    fn extend<T: IntoIterator<Item = Square>>(&mut self, iter: T) {
        *self = iter.into_iter().fold(*self, |bb, sq| bb | sq)
    }
}
