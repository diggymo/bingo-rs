use core::fmt;
use std::io;

struct BingoCard {
    numbers: [[i16; 5]; 5],
    state: [[bool; 5]; 5],
}

impl fmt::Display for BingoCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result = String::new();

        let types: [&str; 5] = ["B", "I", "N", "G", "O"];

        for (row_index, row) in self.numbers.iter().enumerate() {
            result.push_str(&format!("{}  ", types[row_index]));
            for (column_index, number) in row.iter().enumerate() {
                let state = if self.state[row_index][column_index] {
                    "o"
                } else {
                    "x"
                };
                result.push_str(&format!("{:02}({}) ", number, state));
            }
            result.push_str("\n");
        }
        write!(f, "{}", result)
    }
}

fn main() {
    let all_number_count = 75;
    let mut rest_number_count = 75;

    // TODO: 入力として与えたい
    let mut card: BingoCard = BingoCard {
        numbers: [
            [1, 11, 3, 2, 14],
            [30, 19, 22, 29, 17],
            [42, 38, 0, 44, 46],
            [60, 53, 59, 55, 58],
            [74, 68, 61, 67, 73],
        ],
        state: [
            [false, false, false, false, false],
            [false, false, false, false, false],
            [false, false, true, false, false],
            [false, false, false, false, false],
            [false, false, false, false, false],
        ],
    };

    while (true) {
        println!(
            "[Current Turn is {}]",
            all_number_count - rest_number_count + 1
        );
        let choosen_number = get_user_input();

        card.numbers
            .iter()
            .enumerate()
            .for_each(|(row_index, row)| {
                row.iter().enumerate().for_each(|(column_index, number)| {
                    if *number == choosen_number {
                        card.state[row_index][column_index] = true;
                    }
                });
            });

        rest_number_count -= 1;

        if (check_exists_any_line(&card)) {
            println!("Bingo!");
            break;
        }

        println!("{}", card);
    }
}

fn check_exists_any_line(card: &BingoCard) -> bool {
    let patterns = [
        [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4)],
        [(1, 0), (1, 1), (1, 2), (1, 3), (1, 4)],
        [(2, 0), (2, 1), (2, 2), (2, 3), (2, 4)],
        [(3, 0), (3, 1), (3, 2), (3, 3), (3, 4)],
        [(4, 0), (4, 1), (4, 2), (4, 3), (4, 4)],
        [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)],
        [(0, 1), (1, 1), (2, 1), (3, 1), (4, 1)],
        [(0, 2), (1, 2), (2, 2), (3, 2), (4, 2)],
        [(0, 3), (1, 3), (2, 3), (3, 3), (4, 3)],
        [(0, 4), (1, 4), (2, 4), (3, 4), (4, 4)],
        [(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)],
        [(0, 4), (1, 3), (2, 2), (3, 1), (4, 0)],
    ];

    let has_any_pattern = patterns.into_iter().any(|pattern| {
        pattern
            .into_iter()
            .all(|(row_index, column_index)| card.state[row_index][column_index])
    });

    return has_any_pattern;
}

fn get_user_input() -> i16 {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    dbg!(&input);
    let parsed = input.trim().parse::<i16>().expect("cant parse ");
    parsed
}
