use colored::Colorize;
use core::fmt;
use futures::stream::{self, RepeatWith, StreamExt};
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::{
    collections::HashSet,
    io::{self, Write},
    num::ParseIntError,
    sync::Arc,
};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
struct ProbProgress {
    bingo_count: i128,
    not_bing_line_set: Vec<HashSet<i32>>,
}

async fn evaluate_bingo_board_and_calculate(
    card: &BingoCard,
    unchosen_number_set: HashSet<i32>,
    timer_state: Arc<Mutex<bool>>,
) {
    // 当たればビンゴの数字たち
    // 例: [[5], [1,5], [43,12], [12,55], [12,23,69]]
    let bingo_line_list: Vec<_> = LINE_PATTERNS
        .into_iter()
        .map(|pattern| {
            pattern
                .into_iter()
                .filter(|(row_index, column_index)| !card.state[*row_index][*column_index])
                .map(|(row_index, column_index)| card.numbers[row_index][column_index])
                .collect::<HashSet<i32>>()
        })
        .collect();

    tokio::spawn(async move {
        let mut prob_stream = _calculate_probs(bingo_line_list, &unchosen_number_set, 3000).await;
        loop {
            let (n, pattern_count, prob) = prob_stream.next().await.unwrap();

            if *timer_state.lock().await {
                return;
            }

            println!(
                "{}回目までにBINGOになる確率: {}% ({}パターン)",
                n,
                (prob * 100.).to_string().bold(),
                (pattern_count).to_string().bold(),
            );
        }
    });
}

async fn _calculate_probs<'a>(
    bingo_line_list: Vec<HashSet<i32>>,
    unchosen_number_set: &'a HashSet<i32>,
    chunk_size: usize,
) -> RepeatWith<impl FnMut() -> (i128, i128, f64) + 'a> {
    let mut prob_progress = ProbProgress {
        bingo_count: 0,
        not_bing_line_set: vec![HashSet::new()],
    };

    let mut n: i128 = 1;
    let prob_stream = stream::repeat_with(move || {
        let mut new_prob_progress = ProbProgress {
            bingo_count: 0,
            not_bing_line_set: vec![],
        };

        let all_pattern_count = pattern(unchosen_number_set.len() as i128, n as i128);

        let already_bingo_number = prob_progress.bingo_count;

        new_prob_progress.bingo_count +=
            already_bingo_number * (unchosen_number_set.len() as i128 - (n - 1));

        // println!("{}", &prob_progress.not_bing_line_set.len());

        let chunked_not_bing_line_set = prob_progress
            .not_bing_line_set
            .chunks(chunk_size)
            .collect::<Vec<_>>();

        let aggregated_prob_progress = chunked_not_bing_line_set
            .par_iter()
            .map(|bingo_line_set| {
                let mut temporary_prob_progress = ProbProgress {
                    bingo_count: 0,
                    not_bing_line_set: vec![],
                };

                for chosen_number_set_not_bingo in bingo_line_set.iter() {
                    for i in unchosen_number_set {
                        if chosen_number_set_not_bingo.contains(i) {
                            continue;
                        }

                        let mut new_chosen_number_set = chosen_number_set_not_bingo.clone();
                        new_chosen_number_set.insert(*i);

                        let is_bingo = bingo_line_list.iter().any(|group| {
                            group.len() <= (n as usize) && group.is_subset(&new_chosen_number_set)
                        });

                        if is_bingo {
                            temporary_prob_progress.bingo_count += 1;
                        } else {
                            temporary_prob_progress
                                .not_bing_line_set
                                .push(new_chosen_number_set);
                        }
                    }
                }

                temporary_prob_progress
            })
            .reduce(
                || ProbProgress {
                    bingo_count: 0,
                    not_bing_line_set: vec![],
                },
                |mut acc, x: ProbProgress| {
                    acc.bingo_count += x.bingo_count;
                    acc.not_bing_line_set.extend(x.not_bing_line_set);
                    acc
                },
            );
        new_prob_progress.bingo_count += aggregated_prob_progress.bingo_count;
        new_prob_progress
            .not_bing_line_set
            .extend(aggregated_prob_progress.not_bing_line_set);

        prob_progress = new_prob_progress;

        n += 1;

        // ラウンドと手先と確率のペアを返す
        (
            (n - 1),
            all_pattern_count,
            (prob_progress.bingo_count as f64) / (all_pattern_count as f64),
        )
    });

    prob_stream
}

struct BingoCard {
    numbers: [[i32; 5]; 5],
    state: [[bool; 5]; 5],
}

impl fmt::Display for BingoCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result = String::new();

        let types: [&str; 5] = ["B", "I", "N", "G", "O"];

        for (row_index, row) in self.numbers.iter().enumerate() {
            result.push_str(&format!("{}  ", types[row_index]));
            for (column_index, number) in row.iter().enumerate() {
                let mut number_text = format!("{:02}", number)
                    .to_string()
                    .on_black()
                    .bold()
                    .green();

                if self.state[row_index][column_index] {
                    number_text = number_text.magenta();
                }
                result.push_str(&format!("{} ", number_text,));
            }
            result.push_str("\n");
        }
        write!(f, "{}", result)
    }
}

#[tokio::main]
async fn main() {
    let all_number_set = (1..=75).collect::<HashSet<i32>>();
    let mut unchosen_number_set = all_number_set.clone();
    let mut choosen_number_set: HashSet<i32> = HashSet::new();

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

    loop {
        println!(
            "{}",
            format!(
                "----------------[Current Turn is {}]----------------",
                choosen_number_set.len() + 1
            )
            .on_white()
            .black()
            .bold()
        );
        println!(
            "rest number count is {}",
            unchosen_number_set.len().to_string().bold()
        );
        println!("{}", card);

        let choosen_number_result = get_user_input();
        let choosen_number = match choosen_number_result {
            Ok(num) => num,
            Err(_) => {
                println!("1~75の数字を入力してください");
                continue;
            }
        };

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

        choosen_number_set.insert(choosen_number);
        unchosen_number_set.remove(&choosen_number);

        if check_exists_any_line(&card) {
            print_bingo();
            break;
        }

        let timer_state = Arc::new(Mutex::new(false));
        evaluate_bingo_board_and_calculate(&card, unchosen_number_set.clone(), timer_state.clone())
            .await;

        wait_user_any_input();
        (*timer_state.lock().await) = true;
    }
}

fn print_bingo() {
    println!(
        "--------------------------\n\n          {}          \n\n--------------------------",
        format!(
            "{}{}{}{}{}{}",
            "B".green(),
            "I".red(),
            "N".blue(),
            "G".cyan(),
            "O".yellow(),
            "!".magenta(),
        )
        .bold(),
    );
}

static LINE_PATTERNS: [[(usize, usize); 5]; 12] = [
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

fn pattern(n: i128, r: i128) -> i128 {
    let mut temp_n = n;
    let mut sum = 1;

    // rの回数だけループ
    for _ in 0..r {
        sum *= temp_n;
        temp_n -= 1;
    }

    sum
}

fn check_exists_any_line(card: &BingoCard) -> bool {
    let has_any_pattern = LINE_PATTERNS.into_iter().any(|pattern| {
        pattern
            .into_iter()
            .all(|(row_index, column_index)| card.state[row_index][column_index])
    });

    return has_any_pattern;
}

fn get_user_input() -> Result<i32, ParseIntError> {
    print!("Chosen number: ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input.trim().parse::<i32>()
}

fn wait_user_any_input() -> () {
    println!(
        "{}",
        "Enterを押すと確率予測を終えて次のターンに進みます"
            .italic()
            .underline()
    );
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
}

mod test {
    use super::*;

    #[test]
    fn test_pattern() {
        let pattern_count = pattern(18, 2);
        assert_eq!(pattern_count, 306);
    }

    #[test]
    fn test_pattern_2() {
        let pattern_count = pattern(18, 15);
        assert_eq!(pattern_count, 1067062284288000);
    }

    #[tokio::test]
    async fn test_stream_1() {
        let unchosen_number_set = HashSet::from([1, 2, 3, 4, 5]);
        let prob_stream = _calculate_probs(
            vec![
                HashSet::from([1]),          // 1.
                HashSet::from([2, 3]),       // 2.
                HashSet::from([1, 4, 5]),    // 3.
                HashSet::from([3, 4, 5, 2]), // 4.
            ],
            &unchosen_number_set,
            3000,
        );

        let mut prob_stream = prob_stream.await;
        // 1. 1patterns
        assert_eq!(prob_stream.next().await, Some((1, 5, 0.2)));

        // 1. 1*4*2=8patterns
        // 2. 1*2=2patterns
        assert_eq!(prob_stream.next().await, Some((2, 20, 0.5)));

        // 1. 12*3=36patterns
        // 2. 2 x3 x2 = 12patterns
        assert_eq!(prob_stream.next().await, Some((3, 60, 0.8)));
    }

    #[tokio::test]
    async fn test_latency_375() {
        _test_latency(375).await;
    }

    #[tokio::test]
    async fn test_latency_750() {
        _test_latency(750).await;
    }

    #[tokio::test]
    async fn test_latency_1500() {
        _test_latency(1500).await;
    }

    #[tokio::test]
    async fn test_latency_3000() {
        _test_latency(3000).await;
    }

    #[tokio::test]
    async fn test_latency_6000() {
        _test_latency(6000).await;
    }

    #[tokio::test]
    async fn test_latency_12000() {
        _test_latency(12000).await;
    }

    #[tokio::test]
    async fn test_latency_24000() {
        _test_latency(24000).await;
    }

    #[tokio::test]
    async fn test_latency_48000() {
        _test_latency(48000).await;
    }

    #[tokio::test]
    async fn test_latency_96000() {
        _test_latency(96000).await;
    }

    #[tokio::test]
    async fn test_latency_192000() {
        _test_latency(192000).await;
    }

    async fn _test_latency(chunk_size: usize) {
        let unchosen_number_set: HashSet<i32> = (1..70).collect();
        let prob_stream = _calculate_probs(
            vec![
                HashSet::from([1]),
                HashSet::from([40]),
                HashSet::from([50]),
                HashSet::from([60]),
                HashSet::from([2, 3]),
                HashSet::from([4, 5]),
                HashSet::from([5, 6]),
                HashSet::from([6, 7, 8]),
                HashSet::from([8, 9, 10]),
                HashSet::from([10, 11, 12]),
                HashSet::from([11, 12, 13]),
                HashSet::from([13, 14, 15]),
            ],
            &unchosen_number_set,
            chunk_size,
        );

        let mut prob_stream = prob_stream.await;
        prob_stream.next().await;
        prob_stream.next().await;
        prob_stream.next().await;
        prob_stream.next().await;
    }
}
