use std::mem;

use crate::*;
use enum_map::EnumMap;

/// To support Chess960, we can't just assume the rooks start the game on the A and H files, all we know is
/// there will be one rook on either side of the king. Thus, we store, we store their initial files alongside
/// their abilities to castle. The king may only castle with a rook on its left if `short` contains that rook's
/// file. `long` works analogously for a rook on the king's right.
#[derive(Clone, Copy, Default, Debug)]
pub struct CastlingRights {
    pub short: Option<File>,
    pub long: Option<File>,
}

#[derive(Clone, Copy)]
pub struct Board {
    /// For each piece type we store a bitboard of all pieces of that type. Note that these bitboards
    /// aren't aware of a piece's color.
    pub pieces: EnumMap<PieceType, Bitboard>,
    /// In addition to the piece bitboards, we store an array of piece types, to be able to efficiently
    /// query the piece type on a given square. We don't store colors in the mailbox, as they can
    /// efficiently be extracted from the `occupied` bitset, given the information that there is
    /// definitely a piece on the square.
    pub mailbox: EnumMap<Square, Option<PieceType>>,
    /// We store bitboards containing all pieces of each color. By intersecting these with a piece
    /// type's bitboard, we can compute a bitboard containing all pieces of one type belonging to one color.
    pub occupied: EnumMap<Color, Bitboard>,
    /// Each side needs to know their castling rights. Note that in Double Fisher Random, different colors
    /// may have their rooks on different files.
    pub castles: EnumMap<Color, CastlingRights>,
    /// If the most recent ply was a double pawn move, we store its file in here, so we know that
    /// it can be taken by En Passant in the next ply.
    pub en_passant: Option<File>,
    /// Stores all squares that contain pinned pieces for either side.
    pub pinned: Bitboard,
    /// Stores all squares (at most 2) that contain a piece currently giving check.
    pub checkers: Bitboard,
    /// Counts the number of plies since the last pawn move or capture. If this reaches 100,
    /// the game draws by the 50-move rule.
    pub halfmove_clock: u8,
    /// Number of full moves since the beginning of the game. We need to store this to be
    /// able to serialize to FEN notation.
    pub fullmove_count: u32,
    /// The side to make the next move.
    pub stm: Color,
    /// Zobrist hash of the board
    pub hash: u64,
}

impl Board {
    #[inline]
    pub fn occupied(&self) -> Bitboard {
        self.occupied[Color::White] | self.occupied[Color::Black]
    }

    #[inline]
    pub fn colored_pieces(&self, pt: PieceType, color: Color) -> Bitboard {
        self.pieces[pt] & self.occupied[color]
    }

    #[inline]
    pub fn piece_on(&self, sq: Square) -> Option<PieceType> {
        self.mailbox[sq]
    }

    #[inline]
    pub fn colored_piece_on(&self, sq: Square, color: Color) -> Option<PieceType> {
        self.piece_on(sq)
            .filter(|_| self.occupied[color].contains(sq))
    }

    #[inline]
    pub fn king(&self, color: Color) -> Square {
        self.colored_pieces(PieceType::King, color).next()
    }

    #[inline]
    pub fn colored_orth_sliders(&self, color: Color) -> Bitboard {
        self.colored_pieces(PieceType::Rook, color) | self.colored_pieces(PieceType::Queen, color)
    }

    #[inline]
    pub fn colored_diag_sliders(&self, color: Color) -> Bitboard {
        self.colored_pieces(PieceType::Bishop, color) | self.colored_pieces(PieceType::Queen, color)
    }

    #[inline]
    fn toggle_square(&mut self, sq: Square, color: Color, pt: PieceType) {
        self.pieces[pt] ^= sq;
        self.occupied[color] ^= sq;

        self.hash ^= ZOBRIST.piece(sq, pt, color);
    }

    #[inline]
    fn set_en_passant(&mut self, f: Option<File>) {
        if let Some(old) = mem::replace(&mut self.en_passant, f) {
            self.hash ^= ZOBRIST.en_passant(old);
        }
        if let Some(new) = f {
            self.hash ^= ZOBRIST.en_passant(new);
        }
    }

    #[inline]
    fn set_castles(&mut self, color: Color, castles: Option<File>, short: bool) {
        let rights = if short {
            &mut self.castles[color].short
        } else {
            &mut self.castles[color].long
        };

        if let Some(old) = mem::replace(rights, castles) {
            self.hash ^= ZOBRIST.castles(old, color);
        }

        if let Some(new) = castles {
            self.hash ^= ZOBRIST.castles(new, color);
        }
    }

    /// Makes a move on the current board. Assumes the move is legal for the current position.
    pub fn make_move(&mut self, mov: Move) {
        let (from, to, flag, promotion) = (
            mov.from(),
            mov.to(),
            mov.move_flag(),
            mov.promotes_to_unchecked(),
        );
        debug_assert!(
            from != to || flag == MoveFlag::Castle,
            "Move to same square"
        );
        debug_assert!(self.halfmove_clock < 100);

        self.halfmove_clock += 1;
        self.fullmove_count += (self.stm == Color::Black) as u32;

        self.set_en_passant(None);

        let piece = self.piece_on(from).expect("Move from empty square");
        let victim = self.piece_on(to);

        if piece == PieceType::Pawn || victim.is_some() {
            self.halfmove_clock = 0;
        }

        // If we land on a square with a piece on it, we must always take it. Since we assume legal
        // moves only, this piece will be one of the opponent.
        // We must ensure that the move is not a castle, because in chess960 the king sometimes
        // moves onto its own square (if it starts on the C or the G file), which is not a
        // capture.
        if let Some(victim) = victim
            && flag != MoveFlag::Castle
        {
            // We must update the victim's bitboard. The mailbox itself will be updated automatically
            // once the new piece moves to the square.
            self.toggle_square(to, !self.stm, victim);

            // If we take a rook, we must check if it still has its castling rights, and remove them.
            let their_back_rank = Rank::R8.relative_to(self.stm);
            if victim == PieceType::Rook && to.rank() == their_back_rank {
                if self.castles[!self.stm].short == Some(to.file()) {
                    self.set_castles(!self.stm, None, true);
                } else if self.castles[!self.stm].long == Some(to.file()) {
                    self.set_castles(!self.stm, None, false);
                }
            }
        }

        debug_assert!(self.occupied[self.stm].contains(from));
        debug_assert!(!self.occupied[self.stm].contains(to) || flag == MoveFlag::Castle);

        match flag {
            MoveFlag::None => {
                self.toggle_square(from, self.stm, piece);
                self.toggle_square(to, self.stm, piece);

                self.mailbox[from] = None;
                self.mailbox[to] = Some(piece);

                match piece {
                    PieceType::King => {
                        // Moving the king removes all castling rights
                        self.set_castles(self.stm, None, true);
                        self.set_castles(self.stm, None, false);
                    }
                    PieceType::Rook => {
                        // If we move a rook, it loses any castling rights it may have had.
                        let back_rank = Rank::R1.relative_to(self.stm);
                        if from.rank() == back_rank {
                            if self.castles[self.stm].short == Some(from.file()) {
                                self.set_castles(self.stm, None, true);
                            } else if self.castles[self.stm].long == Some(from.file()) {
                                self.set_castles(self.stm, None, false);
                            }
                        }
                    }
                    PieceType::Pawn => {
                        // If we double-push, we must update the en passant file.
                        if to.rank().idx().abs_diff(from.rank().idx()) == 2 {
                            debug_assert_eq!(to.file(), from.file());
                            debug_assert_eq!(from.rank(), Rank::R2.relative_to(self.stm));

                            // We actually only care about the en passant file if the opponent
                            // has a pawn that may take our pushed pawn on their next turn.
                            // Otherwise, just omitting the en-passant file cannot have an
                            // effect on the game state. This way, we only include the en-passant
                            // information in the zobrist hash when it is relevant, hopefully
                            // increasing the accuracy of any kinds of transposition tables
                            // using the hash.
                            // Note that this may result in a technically incorrect FEN string
                            // being printed, but it can't affect the next legal moves so we don't
                            // care.
                            if self
                                .colored_pieces(PieceType::Pawn, !self.stm)
                                .intersect(pawn_attacks(
                                    from.offset(0, self.stm.signum()),
                                    self.stm,
                                ))
                                .is_non_empty()
                            {
                                self.set_en_passant(Some(to.file()));
                            }
                        }
                    }
                    _ => {}
                }
            }
            MoveFlag::Castle => {
                // Sanity checks to detect illegal castles
                debug_assert!(to.file() == File::G || to.file() == File::C);
                debug_assert_eq!(piece, PieceType::King);

                debug_assert_eq!(from.rank(), to.rank());
                debug_assert_eq!(from.rank(), Rank::R1.relative_to(self.stm));

                // We must make sure to update both the king squares and the rook squares for castles
                let (rook_from, rook_to) = if to.file() == File::G {
                    // short castle
                    (self.castles[self.stm].short, File::F)
                } else {
                    (self.castles[self.stm].long, File::D)
                };
                let (rook_from, rook_to) = (
                    Square::from_file_rank(rook_from.expect("Illegal castle"), to.rank()),
                    Square::from_file_rank(rook_to, to.rank()),
                );

                // Update bitboards
                self.toggle_square(from, self.stm, PieceType::King);
                self.toggle_square(to, self.stm, PieceType::King);
                self.toggle_square(rook_from, self.stm, PieceType::Rook);
                self.toggle_square(rook_to, self.stm, PieceType::Rook);

                // Update the mailbox. We first clear both, then set both. This is to ensure
                // that even if one piece starts on the square that the other moves to, it
                // doesn't accidentally leave a mailbox entry empty.
                self.mailbox[from] = None;
                self.mailbox[rook_from] = None;
                self.mailbox[to] = Some(PieceType::King);
                self.mailbox[rook_to] = Some(PieceType::Rook);

                self.set_castles(self.stm, None, true);
                self.set_castles(self.stm, None, false);
            }
            MoveFlag::EnPassant => {
                let target_square = Square::from_file_rank(to.file(), from.rank());

                debug_assert_eq!(from.rank(), Rank::R5.relative_to(self.stm));
                debug_assert_eq!(
                    self.colored_piece_on(target_square, !self.stm),
                    Some(PieceType::Pawn)
                );

                self.toggle_square(from, self.stm, PieceType::Pawn);
                self.toggle_square(to, self.stm, PieceType::Pawn);

                self.mailbox[from] = None;
                self.mailbox[to] = Some(PieceType::Pawn);

                self.toggle_square(target_square, !self.stm, PieceType::Pawn);
                self.mailbox[target_square] = None;
            }
            MoveFlag::Promotion => {
                debug_assert_eq!(to.rank(), Rank::R8.relative_to(self.stm));
                debug_assert_eq!(self.colored_piece_on(from, self.stm), Some(PieceType::Pawn));

                self.toggle_square(from, self.stm, PieceType::Pawn);
                self.toggle_square(to, self.stm, promotion);
                self.mailbox[from] = None;
                self.mailbox[to] = Some(promotion);
            }
        }

        self.stm = !self.stm;
        self.hash ^= ZOBRIST.black_to_move;
        self.calc_pinned_and_checkers();
    }

    pub fn start_pos() -> Self {
        Self::read_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub fn read_fen(fen: &str) -> Option<Self> {
        let mut parts = fen.trim().split(' ');

        let pieces = parts.next()?;
        let stm = parts.next()?;
        let castles = parts.next()?;
        let epts = parts.next()?;
        let hmc = parts.next()?;
        let fmc = parts.next()?;
        if parts.next().is_some() {
            return None;
        }

        let mut board = Board {
            pieces: Default::default(),
            mailbox: Default::default(),
            occupied: Default::default(),
            castles: Default::default(),
            en_passant: None,
            pinned: Bitboard::EMPTY,
            checkers: Bitboard::EMPTY,
            halfmove_clock: 0,
            fullmove_count: 0,
            stm: Color::White,
            hash: 0,
        };

        let mut rank = 8u8;
        for line in pieces.split('/') {
            rank = rank.checked_sub(1)?;
            if line == "8" {
                continue;
            }

            let mut file = 0;
            let chars = line.bytes();
            for ch in chars {
                if matches!(ch, b'1'..=b'7') {
                    file += ch - b'0';
                    continue;
                }
                if file >= 8 {
                    return None;
                }

                let pt = match ch.to_ascii_lowercase() {
                    b'p' => PieceType::Pawn,
                    b'n' => PieceType::Knight,
                    b'b' => PieceType::Bishop,
                    b'r' => PieceType::Rook,
                    b'q' => PieceType::Queen,
                    b'k' => PieceType::King,
                    _ => return None,
                };
                let color = Color::from_idx(ch.is_ascii_lowercase() as u8);

                let sq = Square::from_file_rank(File::from_idx(file), Rank::from_idx(rank));
                board.toggle_square(sq, color, pt);
                if board.mailbox[sq].replace(pt).is_some() {
                    return None;
                }

                file += 1;
            }
        }

        // Sanity check that both sides have a king.
        if board.colored_pieces(PieceType::King, Color::White).popcnt() != 1
            || board.colored_pieces(PieceType::King, Color::Black).popcnt() != 1
        {
            return None;
        }

        if rank != 0 {
            return None;
        }

        board.stm = match stm {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return None,
        };

        if board.stm == Color::Black {
            board.hash ^= ZOBRIST.black_to_move;
        }

        if castles != "-" {
            for ch in castles.bytes() {
                let color = Color::from_idx(ch.is_ascii_lowercase() as u8);
                let king = board.king(color);

                let file = match ch.to_ascii_lowercase() {
                    b'a'..=b'h' => File::from_idx(ch.to_ascii_lowercase() - b'a'),
                    b'k' => (king.file().idx()..8).map(File::from_idx).find(|&f| {
                        board
                            .colored_pieces(PieceType::Rook, color)
                            .contains(Square::from_file_rank(f, king.rank()))
                    })?,
                    b'q' => (0..king.file().idx())
                        .rev()
                        .map(File::from_idx)
                        .find(|&f| {
                            board
                                .colored_pieces(PieceType::Rook, color)
                                .contains(Square::from_file_rank(f, king.rank()))
                        })?,
                    _ => return None,
                };

                let king_sq = board.king(color);
                let rook_sq = Square::from_file_rank(file, king_sq.rank());
                if !board
                    .colored_pieces(PieceType::Rook, color)
                    .contains(rook_sq)
                {
                    return None;
                }

                board.set_castles(color, Some(file), file > king_sq.file());
            }
        }

        if epts != "-" {
            let mut chars = epts.bytes();
            let file = match chars.next()? {
                ch @ b'a'..=b'h' => File::from_idx(ch - b'a'),
                _ => return None,
            };

            if !matches!(chars.next(), Some(b'3' | b'6')) {
                return None;
            }

            board.set_en_passant(Some(file));
        }

        board.halfmove_clock = hmc.parse().ok()?;
        if board.halfmove_clock >= 100 {
            return None;
        }

        board.fullmove_count = fmc.parse().ok()?;

        board.calc_pinned_and_checkers();

        Some(board)
    }

    pub fn fen(&self, chess960: bool) -> String {
        use std::fmt::Write;
        let mut res = String::new();

        for &rank in Rank::ALL.iter().rev() {
            let mut gap = 0;

            for &file in File::ALL {
                let sq = Square::from_file_rank(file, rank);
                if let Some(pt) = self.piece_on(sq) {
                    if gap != 0 {
                        write!(res, "{gap}").unwrap();
                        gap = 0;
                    }
                    let color = Color::from_idx(self.occupied[Color::Black].contains(sq) as u8);
                    res.push(pt.to_char(color));
                } else {
                    gap += 1;
                }
            }
            if gap != 0 {
                write!(res, "{gap}").unwrap();
            }
            res.push(if rank != Rank::R1 { '/' } else { ' ' });
        }

        write!(res, "{:?} ", self.stm).unwrap();

        let mut castles = String::new();
        if let Some(f) = self.castles[Color::White].short {
            let ch = if !chess960 { 'K' } else { f.to_char() };
            castles.push(ch);
        }
        if let Some(f) = self.castles[Color::White].long {
            let ch = if !chess960 { 'Q' } else { f.to_char() };
            castles.push(ch);
        }
        if let Some(f) = self.castles[Color::Black].short {
            let ch = if !chess960 { 'K' } else { f.to_char() };
            castles.push(ch.to_ascii_lowercase());
        }
        if let Some(f) = self.castles[Color::Black].long {
            let ch = if !chess960 { 'Q' } else { f.to_char() };
            castles.push(ch.to_ascii_lowercase());
        }

        if castles.is_empty() {
            castles.push('-');
        }

        write!(res, "{castles} ").unwrap();

        let ep = if let Some(f) = self.en_passant {
            format!("{:#?}{:?}", f, Rank::R6.relative_to(self.stm))
        } else {
            "-".to_string()
        };

        write!(res, "{ep} {} {}", self.halfmove_clock, self.fullmove_count).unwrap();

        res
    }

    pub fn print(&self, chess960: bool) {
        println!("╔═══╤═══╤═══╤═══╤═══╤═══╤═══╤═══╗");

        for &rank in Rank::ALL.iter().rev() {
            print!("║");
            for &file in File::ALL {
                let sq = Square::from_file_rank(file, rank);
                let mut ch = match self.piece_on(sq) {
                    None => ' ',
                    Some(PieceType::Pawn) => 'P',
                    Some(PieceType::Knight) => 'N',
                    Some(PieceType::Bishop) => 'B',
                    Some(PieceType::Rook) => 'R',
                    Some(PieceType::Queen) => 'Q',
                    Some(PieceType::King) => 'K',
                };

                if self.occupied[Color::Black].contains(sq) {
                    ch = ch.to_ascii_lowercase();
                }

                print!(" {ch} ");
                print!("{}", if file == File::H { '║' } else { '│' });
            }
            if rank != Rank::R1 {
                println!(" {rank:?}\n╟───┼───┼───┼───┼───┼───┼───┼───╢");
            }
        }
        println!(" {:?}\n╚═══╧═══╧═══╧═══╧═══╧═══╧═══╧═══╝", Rank::R1);

        for file in File::ALL {
            print!("  {file:?} ");
        }

        println!("\n\nFEN: {}", self.fen(chess960));
        println!("Zobrist key: {:#018x}", self.hash)
    }

    #[inline]
    pub fn parse_move(&self, lan: &str, chess960: bool) -> Option<Move> {
        if !(4..=5).contains(&lan.len()) {
            return None;
        }

        let from = Square::parse(&lan[..2])?;
        let to = Square::parse(&lan[2..4])?;
        let promote_to = match lan.as_bytes().get(4) {
            Some(&b) => Some(PieceType::from_char(b as char)?),
            None => None,
        };

        if let Some(pt) = promote_to {
            return Some(Move::new_promotion(from, to, pt));
        }

        let castle_file = 'castle: {
            if self.piece_on(from) != Some(PieceType::King) {
                break 'castle None;
            }

            if !chess960 && from.file() == File::E && [File::C, File::G].contains(&to.file()) {
                break 'castle Some(to.file());
            }

            if chess960 && self.colored_piece_on(to, self.stm) == Some(PieceType::Rook) {
                let is_short = to.file() > from.file();
                let dst = if is_short { File::G } else { File::C };
                break 'castle Some(dst);
            }

            None
        };

        if let Some(cf) = castle_file {
            return Some(Move::new(
                from,
                Square::from_file_rank(cf, to.rank()),
                MoveFlag::Castle,
            ));
        }

        let is_ep = self.piece_on(from) == Some(PieceType::Pawn)
            && from.rank() == Rank::R5.relative_to(self.stm)
            && self.en_passant == Some(to.file());

        Some(Move::new(
            from,
            to,
            if is_ep {
                MoveFlag::EnPassant
            } else {
                MoveFlag::None
            },
        ))
    }
}
