use std::io::BufRead;

use gunnir_board::*;

fn main() {
    let mut b = Board::start_pos();
    b.print(false);

    let mut s = String::new();
    let mut stdin = std::io::stdin().lock();
    loop {
        s.clear();
        stdin.read_line(&mut s).unwrap();
        if s.is_empty() {
            break;
        }

        for s in s.trim().split_ascii_whitespace() {
            let Some(mv) = b.parse_move(s.trim(), false) else {
                eprintln!("Invalid move!");
                continue;
            };

            dbg!(mv);

            b.make_move(mv);
            b.print(false);
        }
    }
}
