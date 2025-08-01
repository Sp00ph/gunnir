use gunnir::*;

#[cfg(test)]
mod perft;

fn main() {
    // let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    // let fen = "rnb1k1n1/pppp1ppp/8/3Pp3/4r3/8/PPP2PPP/R3K1qR w KQq - 0 1";
    let fen = "rqbbk1r1/1ppp2pp/p3n1n1/4pp2/P7/1PP5/1Q1PPPPP/R1BBKNRN b Kkq - 3 10";
    let board = Board::read_fen(fen).unwrap();

    println!("{:?}", board.castles);

    board.print();

    let mut moves = vec![];
    board.gen_moves(|pm| {
        moves.extend(pm);
    });

    println!("\n\n{} moves found", moves.len());

    for mov in moves {

        println!(
            "\n\n\n{}: {:?} -> {:?} ({:?})",
            board.piece_on(mov.from()).unwrap().to_char(board.stm),
            mov.from(),
            mov.to(),
            mov.move_flag()
        );

        let mut board = board.clone();
        board.make_move(mov);
        board.print();
    }
}
