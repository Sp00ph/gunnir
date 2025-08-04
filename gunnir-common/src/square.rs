use std::fmt;

use crate::*;

define_enum!(
    #[derive(PartialOrd, Ord)]
    #[rustfmt::skip]
    pub enum File {
        A, B, C, D, E, F, G, H
    }
);

impl File {
    #[inline]
    pub const fn bitboard(self) -> Bitboard {
        Bitboard(0x0101010101010101 << self.idx())
    }

    #[inline]
    pub const fn try_offset(self, df: i8) -> Option<Self> {
        Self::try_from_idx(self.idx().wrapping_add(df as u8))
    }

    #[inline]
    pub const fn offset(self, df: i8) -> Self {
        self.try_offset(df).expect("Invalid file offset")
    }

    #[inline]
    pub const fn to_char(self) -> char {
        (b'A' + self.idx()) as char
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ch = match self {
            Self::A => b'A',
            Self::B => b'B',
            Self::C => b'C',
            Self::D => b'D',
            Self::E => b'E',
            Self::F => b'F',
            Self::G => b'G',
            Self::H => b'H',
        };

        if f.alternate() {
            ch = ch.to_ascii_lowercase();
        }

        fmt::Write::write_char(f, ch as char)
    }
}

define_enum!(
    #[derive(PartialOrd, Ord)]
    #[rustfmt::skip]
    pub enum Rank {
        R1, R2, R3, R4, R5, R6, R7, R8
    }
);

impl Rank {
    #[inline]
    pub const fn bitboard(self) -> Bitboard {
        Bitboard(0xff << (8 * self.idx()))
    }

    #[inline]
    pub const fn try_offset(self, dr: i8) -> Option<Self> {
        Self::try_from_idx(self.idx().wrapping_add(dr as u8))
    }

    #[inline]
    pub const fn offset(self, dr: i8) -> Self {
        self.try_offset(dr).expect("Invalid rank offset")
    }

    #[inline]
    pub const fn relative_to(self, color: Color) -> Self {
        Self::from_idx((color.idx() * 7) ^ self.idx())
    }
}

impl fmt::Debug for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.idx() + 1)
    }
}

define_enum!(
    #[derive(PartialOrd, Ord)]
    #[rustfmt::skip]
    pub enum Square {
        A1, B1, C1, D1, E1, F1, G1, H1,
        A2, B2, C2, D2, E2, F2, G2, H2,
        A3, B3, C3, D3, E3, F3, G3, H3,
        A4, B4, C4, D4, E4, F4, G4, H4,
        A5, B5, C5, D5, E5, F5, G5, H5,
        A6, B6, C6, D6, E6, F6, G6, H6,
        A7, B7, C7, D7, E7, F7, G7, H7,
        A8, B8, C8, D8, E8, F8, G8, H8,
    }
);

impl Square {
    #[inline]
    pub const fn file(self) -> File {
        File::from_idx(self.idx() % 8)
    }

    #[inline]
    pub const fn rank(self) -> Rank {
        Rank::from_idx(self.idx() / 8)
    }

    #[inline]
    pub const fn from_file_rank(file: File, rank: Rank) -> Self {
        Self::from_idx(rank.idx() * 8 + file.idx())
    }

    #[inline]
    pub const fn bitboard(self) -> Bitboard {
        Bitboard(1 << self.idx())
    }

    #[inline]
    pub const fn try_offset(self, df: i8, dr: i8) -> Option<Self> {
        let file = self.file().try_offset(df);
        let rank = self.rank().try_offset(dr);

        match (file, rank) {
            (Some(file), Some(rank)) => Some(Self::from_file_rank(file, rank)),
            _ => None,
        }
    }

    #[inline]
    pub const fn offset(self, df: i8, dr: i8) -> Self {
        self.try_offset(df, dr).expect("Invalid square offset")
    }
}

impl fmt::Debug for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.file(), f)?;
        fmt::Debug::fmt(&self.rank(), f)
    }
}
