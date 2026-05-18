mod execution;
mod types;
mod utils;

use execution::{mutate_tests, run, run_with_mutations};
use std::path::{Path, PathBuf};
use types::{MutationRule, Runner};
use utils::{make_excel_book, make_graph};

fn main() {
    let rule = MutationRule::Complex {
        length: 7,
    };
    let min_time: f64 = 1.0;
    let max_time: f64 = 250.0;
    let num_threads: usize = 30;
    let runner = Runner::new(
        Path::new("algorithms/ginvdist/main.py"),
        Some(Path::new("algorithms/ginvdist/.venv/bin/python")),
    );
    let baseline_results = Path::new("results/results-baseline/");
    let target_dir = Path::new("algorithms/ginvdist/tests/");
    let out_dir = PathBuf::from("results/septuple/");

    let venv_python_path = Path::new("python/.venv/bin/python");
    let old_results = Path::new("results/results-baseline/");
    let new_results = Path::new("results/results-large/");
    let results: Vec<PathBuf> = vec![
        PathBuf::from("results/results-baseline/"),
        PathBuf::from("results/results-double/"),
        PathBuf::from("results/results-triple/"),
        PathBuf::from("results/results-quadruple/"),
        PathBuf::from("results/results-quintuple/"),
        PathBuf::from("results/results-sextuple/"),
        PathBuf::from("results/results-small/"),
        PathBuf::from("results/results-medium/"),
        PathBuf::from("results/results-signed/"),
        PathBuf::from("results/results-large/")
    ];

    // Mutates tests and saves them
    // if let Err(e) = mutate_tests(&rule, min_time, max_time, baseline_results, target_dir, out_dir) {
    //     eprintln!("{e}");
    // }

    // Runs tests
    // if let Err(e) = run(num_threads, runner, target_dir, out_dir) {
    //     eprintln!("{e}");
    // }

    // Mutates and runs new tests
    // if let Err(e) = run_with_mutations(
    //     rule,
    //     min_time,
    //     max_time,
    //     num_threads,
    //     runner,
    //     baseline_results,
    //     target_dir,
    //     out_dir,
    // ) {
    //     eprintln!("{e}");
    // }

    // Makes graph
    // make_graph(venv_python_path, old_results, new_results);

    // Makes excel book
    // make_excel_book(Path::new("python/.venv/bin/python"), results);
}
