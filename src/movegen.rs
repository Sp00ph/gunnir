use crate::*;

impl Board {
    /// Returns a bitboard of all feasible target squares for non-king moves for the current side to move to.
    /// If we are not in check, then this is all squares not occupied by our pieces.
    /// If we are in check, it is instead only the squares between the checker and our king.
    /// If we are in double check, this function isn't applicable, as the king itself has to move away.
    #[inline]
    fn targets<const IN_CHECK: bool>(&self) -> Bitboard {
        let mask = if IN_CHECK {
            let checker = self.checkers.next();
            let our_king = self.king(self.stm);

            between_inclusive(checker, our_king)
        } else {
            Bitboard::UNIVERSE
        };

        mask & !self.occupied[self.stm]
    }

    #[inline]
    fn add_pawn_moves<const IN_CHECK: bool, V: FnMut(PieceMoves)>(&self, visitor: &mut V) {
        let targets = self.targets::<IN_CHECK>();

        let blockers = self.occupied();
        let pawns = self.colored_pieces(PieceType::Pawn, self.stm);
        let their_pieces = self.occupied[!self.stm];

        for from in pawns & !self.pinned {

            let to = (pawn_pushes(from, self.stm, blockers)
                | (pawn_attacks(from, self.stm) & their_pieces))
                & targets;

            if to.is_non_empty() {
                let promotes = (to & (Rank::R1.bitboard() | Rank::R8.bitboard())).is_non_empty();
                let move_flag = if promotes {
                    MoveFlag::Promotion
                } else {
                    MoveFlag::None
                };
                visitor(PieceMoves::new(move_flag, PieceType::Pawn, from, to))
            }
        }

        if !IN_CHECK {
            // We're not in check, so we can move pinned pawns as long as they stay pinned.
            // To stay pinned, they must stay on the same line (orthogonal or diagonal)
            // with the king.

            let our_king = self.king(self.stm);
            for from in pawns & self.pinned {
                let to = (pawn_pushes(from, self.stm, blockers)
                    | (pawn_attacks(from, self.stm) & their_pieces))
                    & targets
                    & line(our_king, from);

                if to.is_non_empty() {
                    let promotes =
                        (to & (Rank::R1.bitboard() | Rank::R8.bitboard())).is_non_empty();
                    let move_flag = if promotes {
                        MoveFlag::Promotion
                    } else {
                        MoveFlag::None
                    };
                    visitor(PieceMoves::new(move_flag, PieceType::Pawn, from, to))
                }
            }
        }

        if let Some(ep) = self.en_passant {
            let (dst_rank, push_dir) = (Rank::R6.relative_to(self.stm), self.stm.signum());

            let dst = Square::from_file_rank(ep, dst_rank);
            let taken = Square::from_file_rank(ep, dst_rank.offset(-push_dir));

            let our_king = self.king(self.stm);

            let orth = self.colored_orth_sliders(!self.stm);
            let diag = self.colored_diag_sliders(!self.stm);

            // One of our pawns can take en passant iff an opposite colored pawn on the destination square could take our pawn.
            for from in pawn_attacks(dst, !self.stm) & pawns {
                // En passant can introduce checks on our king even if our pawn isn't pinned, since a slider could
                // give discovered check through the taken pawn. Thus, we don't check which of our pawns are pinned,
                // an instead just simulate the move and see if we end up attacked by any sliders afterwards.

                // Remove our pawn and the taken pawn, and add the moved pawn to the blockers.
                let blockers = (blockers ^ from ^ taken) | dst;
                // Check whether we are checked by an opposing slider piece. We first check
                // the whole rays, to prevent unnecessary expensive slider move lookups.
                let on_rook_ray = (rook_rays(our_king) & orth).is_non_empty();
                if on_rook_ray && (rook_moves(our_king, blockers) & orth).is_non_empty() {
                    continue;
                }

                let on_bishop_ray = (bishop_rays(our_king) & diag).is_non_empty();
                if on_bishop_ray && (bishop_moves(our_king, blockers) & diag).is_non_empty() {
                    continue;
                }

                visitor(PieceMoves::new(
                    MoveFlag::EnPassant,
                    PieceType::Pawn,
                    from,
                    dst.bitboard(),
                ))
            }
        }
    }

    #[inline]
    fn add_knight_moves<const IN_CHECK: bool, V: FnMut(PieceMoves)>(&self, visitor: &mut V) {
        let targets = self.targets::<IN_CHECK>();
        let knights = self.colored_pieces(PieceType::Knight, self.stm);

        for from in knights & !self.pinned {
            let to = knight_moves(from) & targets;
            if to.is_non_empty() {
                visitor(PieceMoves::new(MoveFlag::None, PieceType::Knight, from, to));
            }
        }
    }

    #[inline]
    fn add_slider_moves<
        const IN_CHECK: bool,
        S: Fn(Square, Bitboard) -> Bitboard,
        V: FnMut(PieceMoves),
    >(
        &self,
        from: Bitboard,
        slider_moves: S,
        visitor: &mut V,
    ) {
        let targets = self.targets::<IN_CHECK>();
        let blockers = self.occupied();
        let our_king = self.king(self.stm);

        for from in from {
            let targets = if self.pinned.contains(from) {
                targets & line(our_king, from)
            } else {
                targets
            };

            let pseudolegals = slider_moves(from, blockers);
            let to = pseudolegals & targets;

            if to.is_non_empty() {
                visitor(PieceMoves::new(MoveFlag::None, self.piece_on(from).unwrap(), from, to));
            }
        }
    }


    #[inline]
    fn add_orth_sliders<const IN_CHECK: bool, V: FnMut(PieceMoves)>(&self, visitor: &mut V) {
        self.add_slider_moves::<IN_CHECK, _, _>(self.colored_orth_sliders(self.stm), rook_moves, visitor);
    }

    #[inline]
    fn add_diag_sliders<const IN_CHECK: bool, V: FnMut(PieceMoves)>(&self, visitor: &mut V) {
        self.add_slider_moves::<IN_CHECK, _, _>(self.colored_diag_sliders(self.stm), bishop_moves, visitor);
    }

    #[inline]
    fn king_safe_on(&self, sq: Square, color: Color, blockers: Bitboard) -> bool {
        // Pawn checks
        if (pawn_attacks(sq, color) & self.colored_pieces(PieceType::Pawn, !color)).is_non_empty() {
            return false;
        }

        // Knight checks
        if (knight_moves(sq) & self.colored_pieces(PieceType::Knight, !color)).is_non_empty() {
            return false;
        }

        let orth = self.colored_orth_sliders(!color);
        let diag = self.colored_diag_sliders(!color);

        // Check whether we are checked by an opposing slider piece. We first check
        // the whole rays, to prevent unnecessary expensive slider move lookups.
        let on_rook_ray = (rook_rays(sq) & orth).is_non_empty();
        if on_rook_ray && (rook_moves(sq, blockers) & orth).is_non_empty() {
            return false;
        }

        let on_bishop_ray = (bishop_rays(sq) & diag).is_non_empty();
        if on_bishop_ray && (bishop_moves(sq, blockers) & diag).is_non_empty() {
            return false;
        }

        !king_moves(self.king(!color)).contains(sq)
    }

    #[inline]
    fn can_castle(
        &self,
        king: Square,
        rook: Square,
        king_dst: Square,
        rook_dst: Square,
        blockers: Bitboard,
    ) -> bool {
        let blockers = blockers ^ rook;

        let must_be_safe = between(king, king_dst) | king_dst;
        let must_be_empty = must_be_safe | between(king, rook) | rook_dst;

        !self.pinned.contains(rook)
            && (blockers & must_be_empty).is_empty()
            && must_be_safe
                .into_iter()
                .all(|sq| self.king_safe_on(sq, self.stm, blockers))
    }

    #[inline]
    fn add_king_moves<const IN_CHECK: bool, V: FnMut(PieceMoves)>(&self, visitor: &mut V) {
        let targets = !self.occupied[self.stm];
        let king = self.king(self.stm);
        let blockers = self.occupied() ^ king;

        let to: Bitboard = (targets & king_moves(king))
            .into_iter()
            .filter(|&sq| self.king_safe_on(sq, self.stm, blockers))
            .collect();

        if to.is_non_empty() {
            visitor(PieceMoves::new(MoveFlag::None, PieceType::King, king, to));
        }

        if !IN_CHECK {
            let castles = self.castles[self.stm];
            let mut to = Bitboard::EMPTY;

            if let Some(short) = castles.short
                && self.can_castle(
                    king,
                    Square::from_file_rank(short, king.rank()),
                    Square::from_file_rank(File::G, king.rank()),
                    Square::from_file_rank(File::F, king.rank()),
                    blockers,
                )
            {
                to |= Square::from_file_rank(File::G, king.rank());
            }

            if let Some(long) = castles.long
                && self.can_castle(
                    king,
                    Square::from_file_rank(long, king.rank()),
                    Square::from_file_rank(File::C, king.rank()),
                    Square::from_file_rank(File::D, king.rank()),
                    blockers,
                )
            {
                to |= Square::from_file_rank(File::C, king.rank());
            }

            if to.is_non_empty() {
                visitor(PieceMoves::new(MoveFlag::Castle, PieceType::King, king, to));
            }
        }
    }

    #[inline]
    fn add_legal_moves<const IN_CHECK: bool, V: FnMut(PieceMoves)>(&self, visitor: &mut V) {
        self.add_pawn_moves::<IN_CHECK, V>(visitor);
        self.add_knight_moves::<IN_CHECK, V>(visitor);
        self.add_orth_sliders::<IN_CHECK, V>(visitor);
        self.add_diag_sliders::<IN_CHECK, V>(visitor);
        self.add_king_moves::<IN_CHECK, V>(visitor);
    }

    pub fn gen_moves<V: FnMut(PieceMoves)>(&self, mut visitor: V) {
        match self.checkers.popcnt() {
            0 => self.add_legal_moves::<false, V>(&mut visitor),
            1 => self.add_legal_moves::<true, V>(&mut visitor),
            _ => self.add_king_moves::<true, V>(&mut visitor),
        }
    }

    #[inline]
    pub fn calc_pinned_and_checkers(&mut self) {
        let our_king = self.king(self.stm);
        let occupied = self.occupied();

        self.checkers = ((pawn_attacks(our_king, self.stm) & self.pieces[PieceType::Pawn])
            | (knight_moves(our_king) & self.pieces[PieceType::Knight]))
            & self.occupied[!self.stm];

        self.pinned = Bitboard::EMPTY;

        let orth = self.pieces[PieceType::Rook] | self.pieces[PieceType::Queen];
        let diag = self.pieces[PieceType::Bishop] | self.pieces[PieceType::Queen];

        let attackers = self.occupied[!self.stm]
            & ((orth & rook_rays(our_king)) | (diag & bishop_rays(our_king)));

        for attacker in attackers {
            let blockers = between(attacker, our_king) & occupied;
            match blockers.popcnt() {
                0 => self.checkers |= attacker,
                1 => self.pinned |= blockers,
                _ => {}
            }
        }

        let their_king = self.king(!self.stm);
        let attackers = self.occupied[self.stm]
            & ((orth & rook_rays(their_king)) | (diag & bishop_rays(their_king)));

        for attacker in attackers {
            let blockers = between(attacker, their_king) & occupied;
            if blockers.popcnt() == 1 {
                self.pinned |= blockers & self.occupied[!self.stm]
            }
        }
    }
}
