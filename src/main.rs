use gunnir_board::*;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use rustyline::{Config, Editor, history::MemHistory};

fn main() {
    let mut b = Board::read_fen("1n4nr/p1Np2pp/1p6/4pQ2/3P3k/bP1B4/P1P2PPP/R3K2R w KQ - 6 18").unwrap();
    b.print(false);

    let mut editor = Editor::<(), MemHistory>::with_history(
        Config::builder().auto_add_history(true).build(),
        MemHistory::new(),
    )
    .unwrap();

    let mut rng = SmallRng::from_os_rng();

    'outer: while let Ok(line) = editor.readline("") {
        for s in line.trim().split_ascii_whitespace() {
            let Some(mv) = b.parse_move(s.trim(), false) else {
                eprintln!("Invalid move!");
                continue;
            };

            dbg!(mv);

            b.make_move(mv);

            let mut movs = vec![];
            b.gen_moves(|m| movs.extend(m));

            if movs.is_empty() {
                break 'outer;
            }
            let i = rng.random_range(0..movs.len());
            println!("{:?}", movs[i]);
            b.make_move(movs[i]);

            b.print(false);
        }
    }

    b.print(false);
    println!("Wowie!");
}
