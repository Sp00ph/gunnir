use std::time::Instant;

use gunnir::*;

#[cfg(test)]
mod perft;

fn main() {
    // // let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    // // let fen = "rnb1k1n1/pppp1ppp/8/3Pp3/4r3/8/PPP2PPP/R3K1qR w KQq - 0 1";
    // let fen = "rqbbk1r1/1ppp2pp/p3n1n1/4pp2/P7/1PP5/1Q1PPPPP/R1BBKNRN b Kkq - 3 10";
    // let board = Board::read_fen(fen).unwrap();

    // println!("{:?}", board.castles);

    // board.print(false);

    // let mut moves = vec![];
    // board.gen_moves(|pm| {
    //     moves.extend(pm);
    // });

    // println!("\n\n{} moves found", moves.len());

    // for mov in moves {

    //     println!(
    //         "\n\n\n{}: {:?} -> {:?} ({:?})",
    //         board.piece_on(mov.from()).unwrap().to_char(board.stm),
    //         mov.from(),
    //         mov.to(),
    //         mov.move_flag()
    //     );

    //     let mut board = board.clone();
    //     board.make_move(mov);
    //     board.print();
    // }

    fn perft(board: &Board, depth: u8) -> u64 {
        let mut nodes = 0;

        if depth == 0 {
            return 1;
        }

        if depth == 1 {
            board.gen_moves(|moves| {
                nodes += moves.len() as u64;
            });
        } else {
            board.gen_moves(|moves| {
                for mv in moves {
                    let mut board = *board;
                    board.make_move(mv);

                    nodes += perft(&board, depth - 1);
                }
            });
        }

        nodes
    }

    let board = Board::start_pos();

    for depth in 1.. {
        let t0 = Instant::now();
        let n = perft(&board, depth);
        let d = t0.elapsed();
        println!("{depth}: {n} (took {d:.2?})");
    }
}
