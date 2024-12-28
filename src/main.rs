use core::fmt;
use std::{
    collections::HashSet,
    io::{self, Write},
    num::ParseIntError,
};
// use textplots::{Chart, Plot, Shape};
use futures::stream::{self, RepeatWith, StreamExt};
use std::hash::{Hash, Hasher};

const PROB_COUNT: usize = 3;

#[derive(Clone, Debug)]
struct ProbProgress {
    bingo_count: i128,
    already_bingo_line_set: Vec<HashSet<i32>>,
    // chosen_numbers_bingo: Vec<Vec<i32>>,
    not_bing_line_set: Vec<HashSet<i32>>,
}

async fn evaluate_bing_board_and_calculate(card: &BingoCard, unchosen_number_set: &HashSet<i32>) {
    // ex.
    // 当たればビンゴの数字たち
    // [5],
    // [1,5],
    // [43,12],
    // [12,55],
    // [12,23,69],
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

    let mut prob_stream = _calculate_probs(bingo_line_list, unchosen_number_set).await;

    loop {
        let (n, pattern_count, prob) = prob_stream.next().await.unwrap();
        println!(
            "{}回目までにBINGOになる確率: {}% ({}パターン)",
            n,
            prob * 100.,
            pattern_count,
        );
    }
}

async fn _calculate_probs<'a>(
    // card: &BingoCard,
    bingo_line_list: Vec<HashSet<i32>>,
    unchosen_number_set: &'a HashSet<i32>,
) -> RepeatWith<impl FnMut() -> (i128, i128, f64) + 'a> {
    let mut prob_progress = ProbProgress {
        bingo_count: 0,
        already_bingo_line_set: vec![HashSet::new()],
        not_bing_line_set: vec![HashSet::new()],
    };

    let mut n: i128 = 1;
    let prob_stream = stream::repeat_with(move || {
        let mut new_prob_progress = ProbProgress {
            bingo_count: 0,
            already_bingo_line_set: prob_progress.already_bingo_line_set.clone(),
            not_bing_line_set: vec![],
        };

        // println!("------{}-----", &n);

        let all_pattern_count = pattern(unchosen_number_set.len() as i128, n as i128);

        // dbg!(&prob_progress.not_bing_line_set);

        let already_bingo_number = prob_progress.bingo_count;

        new_prob_progress.bingo_count +=
            already_bingo_number * (unchosen_number_set.len() as i128 - (n - 1));
        // println!("加算されました, {}", &new_prob_progress.bingo_count);

        for chosen_number_set_not_bingo in prob_progress.not_bing_line_set.iter() {
            // dbg!(&chosen_number_set_not_bingo);
            // [1,2,3,4]
            for i in unchosen_number_set {
                if chosen_number_set_not_bingo.contains(i) {
                    // println!("スキップ, {:?} {:?}", &i, &chosen_number_set_not_bingo.0);
                    continue;
                }

                let mut new_chosen_number_set = chosen_number_set_not_bingo.clone();
                new_chosen_number_set.insert(*i);

                // if new_prob_progress
                //     .already_bingo_line_set
                //     .iter()
                //     .any(|group| group.0.is_subset(&chosen_number_set_not_bingo.0))
                // {
                //     continue;
                // }

                // [2,4]
                // dbg!(&new_chosen_number_set.0);
                let is_bingo = bingo_line_list.iter().any(|group| {
                    // dbg!(&group);
                    // dbg!(&new_chosen_number_set.0);
                    group.len() <= (n as usize) && group.is_subset(&new_chosen_number_set)
                });

                if is_bingo {
                    // println!("{:?}はビンゴです！", &new_chosen_number_set);
                    new_prob_progress
                        .already_bingo_line_set
                        .push(new_chosen_number_set);

                    new_prob_progress.bingo_count += 1;
                } else {
                    // println!("{:?}はビンゴになりませんでした", &new_chosen_number_set);
                    new_prob_progress
                        .not_bing_line_set
                        .push(new_chosen_number_set);
                }
            }
        }

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
            "----------------[Current Turn is {}]----------------",
            choosen_number_set.len() + 1
        );
        println!("rest number count is {}", unchosen_number_set.len());
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
            println!("Bingo!");
            break;
        }

        // let probs = calculate_probs(&card, &unchosen_number_set);
        evaluate_bing_board_and_calculate(&card, &unchosen_number_set).await;

        // probs.iter().enumerate().for_each(|(i, prob)| {
        //     println!("{:03}回目までにBINGOになる確率: {}%", i + 1, prob * 100.);
        // });

        // let mut points = [(0f32, 0f32); PROB_COUNT];
        // probs.iter().enumerate().for_each(|(i, prob)| {
        //     points[i] = (i as f32, (*prob as f32) * 100.);
        // });
        // Chart::default().lineplot(&Shape::Steps(&points)).display();
    }
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

fn calculate_probs(card: &BingoCard, unchosen_number_set: &HashSet<i32>) -> Vec<f64> {
    // ex.
    // 当たればビンゴの数字たち
    // [5],
    // [1,5],
    // [43,12],
    // [12,55],
    // [12,23,69],
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

    let prob_list: Vec<f64> = (1..=PROB_COUNT)
        .map(|i| {
            let prob = calc_probability(unchosen_number_set, &bingo_line_list, i);
            prob
        })
        .collect();

    prob_list
}

fn recursion(
    unchosen_number_set: &HashSet<i32>,
    bingo_line_list: &Vec<HashSet<i32>>,
    picked_number_set: &HashSet<i32>,
    n: usize,
) -> i64 {
    if picked_number_set.len() == n {
        // 合否判定する
        let is_bingo = bingo_line_list
            .iter()
            .any(|group| group.len() <= n && group.is_subset(&picked_number_set));

        return is_bingo as i64;
    }

    // 再帰実行
    let mut bing_count = 0;
    for i in unchosen_number_set {
        if picked_number_set.contains(&i) {
            continue;
        }

        let mut new_picked_number_set = picked_number_set.clone();
        new_picked_number_set.insert(*i);
        bing_count += recursion(
            unchosen_number_set,
            bingo_line_list,
            &new_picked_number_set,
            n,
        );
    }

    return bing_count;
}

fn calc_probability(
    unchosen_number_set: &HashSet<i32>,
    bingo_line_list: &Vec<HashSet<i32>>,
    n: usize,
) -> f64 {
    let all_pattern_count = pattern(unchosen_number_set.len() as i128, n as i128);

    // 枝切り
    if bingo_line_list.iter().all(|group| group.len() > n) {
        return 0.0;
    }

    let pattern_count = recursion(
        unchosen_number_set,
        bingo_line_list,
        &HashSet::new(),
        n as usize,
    );

    (pattern_count as f64) / (all_pattern_count as f64)
}

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

mod test {
    use super::*;
    #[test]
    fn test_recursion_minimum_case() {
        let count = recursion(
            &HashSet::from([1, 2, 3, 4, 5]),
            &vec![HashSet::from([1]), HashSet::from([2])],
            &HashSet::new(),
            1,
        );

        assert_eq!(count, 2)
    }

    #[test]
    fn test_recursion_minimum_zero() {
        let count = recursion(
            &HashSet::from([1, 2, 3, 4, 5, 6, 7, 8]),
            &vec![HashSet::from([1, 2, 3, 4]), HashSet::from([4, 5, 6, 7])],
            &HashSet::new(),
            3,
        );

        assert_eq!(count, 0)
    }

    #[test]
    fn test_recursion() {
        let count = recursion(
            &HashSet::from([1, 2, 3, 4]),
            &vec![
                HashSet::from([1]), // 1-2, 1-3, 1-4, 2-1, 3-1, 4-1
            ],
            &HashSet::new(),
            2,
        );

        assert_eq!(count, 6)
    }

    #[test]
    fn test_recursion_multi_group() {
        let count = recursion(
            &HashSet::from([1, 2, 3, 4]),
            &vec![
                HashSet::from([1]),    // 1-2, 1-3, 1-4, 2-1, 3-1, 4-1
                HashSet::from([2, 3]), // 2-3, 3-2
            ],
            &HashSet::new(),
            2,
        );

        assert_eq!(count, 8)
    }

    #[test]
    fn test_recursion_multi_group_2() {
        let count = recursion(
            &HashSet::from([1, 2, 3, 4]),
            &vec![
                HashSet::from([1]),    // 1-2, 1-3, 1-4, 2-1, 3-1, 4-1
                HashSet::from([2, 3]), // 2-3, 3-2
                HashSet::from([3, 4]), // 3-4, 4-3
                HashSet::from([4, 1]), // (すでに列挙済み)
            ],
            &HashSet::new(),
            2,
        );

        assert_eq!(count, 10)
    }

    #[test]
    fn test_recursion_multi_group_complicated() {
        let count = recursion(
            &HashSet::from([1, 2, 3, 4, 5]),
            &vec![
                HashSet::from([1]),       // 12patterns x3 = 36patterns
                HashSet::from([2, 3]),    // 2patterns x3 x2 = 12patterns
                HashSet::from([1, 4, 5]), // (列挙済み)
                HashSet::from([3, 4, 5, 2]),
            ],
            &HashSet::new(),
            3,
        );

        assert_eq!(count, 48)
    }

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

    #[test]
    fn test_calc_probability() {
        let pattern_count = calc_probability(
            &HashSet::from([1, 2, 3, 4, 5]),
            &vec![HashSet::from([1]), HashSet::from([2])],
            1,
        );
        assert_eq!(pattern_count, 0.4);
    }

    #[test]
    fn test_calc_probability_2() {
        let pattern_count = calc_probability(
            &HashSet::from([1, 2, 3, 4, 5]),
            &vec![
                HashSet::from([1]),       // 12patterns x3 = 36patterns
                HashSet::from([2, 3]),    // 2patterns x3 x2 = 12patterns
                HashSet::from([1, 4, 5]), // (列挙済み)
                HashSet::from([3, 4, 5, 2]),
            ],
            3,
        );
        assert_eq!(pattern_count, 48. / 60.);
    }

    #[tokio::test]
    async fn test_stream_1() {
        let a = HashSet::from([1, 2, 3, 4, 5]);
        let prob_stream = _calculate_probs(
            vec![
                HashSet::from([1]),          // 1.
                HashSet::from([2, 3]),       // 2.
                HashSet::from([1, 4, 5]),    // 3.
                HashSet::from([3, 4, 5, 2]), // 4.
            ],
            &a,
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
}
