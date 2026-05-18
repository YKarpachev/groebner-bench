use crate::types::MutationRule;
use crate::types::Runner;
use crate::utils;
use num_bigint::BigUint;
use primal::StreamingSieve;
use rand::{self, random_range};
use regex::Regex;
use serde::Serialize;
use std::collections::VecDeque;
use std::fs::create_dir_all;
use std::fs::{File, create_dir, read_to_string, remove_dir_all};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{fs, thread};

/// Init deque of JSON file paths in target directory.
///
/// Errors:
/// - Couldn't read the target dir
fn init_deque(target_dir: &Path) -> Result<VecDeque<PathBuf>, Box<dyn std::error::Error>> {
    let mut deque = VecDeque::new();

    for entry in fs::read_dir(target_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
            deque.push_back(path);
        }
    }

    Ok(deque)
}

/// Returns true if `c` can be part of a variable name.
///
/// Variable characters are ASCII letters, digits, or `_`.
fn is_var_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Replaces occurrences of `variable` in `equation` with a scaled version.
///
/// If the variable has an exponent, the scale is `prime^exponent`.
/// When `negative` is true, odd powers receive a negative sign.
fn mutate_variable_in_equation(
    equation: &str,
    variable: &str,
    prime: u64,
    negative: bool,
) -> String {
    let escaped_var = regex::escape(variable);
    let pattern = format!(r"{}(?:\^([0-9]+))?", escaped_var);
    let re = Regex::new(&pattern).unwrap();

    let mut out = String::new();
    let mut last = 0;

    for caps in re.captures_iter(equation) {
        let m = caps.get(0).unwrap();
        let start = m.start();
        let end = m.end();

        let before_ok = equation[..start]
            .chars()
            .next_back()
            .map_or(true, |c| !is_var_char(c));

        let after_ok = equation[end..]
            .chars()
            .next()
            .map_or(true, |c| !is_var_char(c));

        if !before_ok || !after_ok {
            continue;
        }

        out.push_str(&equation[last..start]);

        let replacement = if let Some(exp_match) = caps.get(1) {
            let exp: u32 = exp_match.as_str().parse().unwrap();

            let scale = BigUint::from(prime).pow(exp);

            if negative && exp % 2 == 1 {
                format!("-{}*{}^{}", scale, variable, exp)
            } else {
                format!("{}*{}^{}", scale, variable, exp)
            }
        } else if negative {
            format!("-{}*{}", prime, variable)
        } else {
            format!("{}*{}", prime, variable)
        };

        out.push_str(&replacement);
        last = end;
    }

    out.push_str(&equation[last..]);
    out
}

/// Returns every `k`-sized combination of indexes from `0..n`.
fn index_combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    fn rec(n: usize, k: usize, start: usize, current: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if current.len() == k {
            out.push(current.clone());
            return;
        }

        let missing = k - current.len();
        for i in start..=n - missing {
            current.push(i);
            rec(n, k, i + 1, current, out);
            current.pop();
        }
    }

    if k == 0 || k > n {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut current = Vec::new();
    rec(n, k, 0, &mut current, &mut out);
    out
}

/// Split an expression into additive terms, preserving `+` and `-` signs.
fn split_terms(expression: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut current = String::new();

    for (i, ch) in expression.char_indices() {
        if (ch == '+' || ch == '-') && i != 0 {
            terms.push(current);
            current = String::new();
        }
        current.push(ch);
    }

    if !current.is_empty() {
        terms.push(current);
    }

    terms
}

/// Replaces a selected group of variables inside every monomial by `replacement`.
///
/// Example: with group `{x1, x3}` and replacement `z`, `x1*x2*x3` becomes
/// `z*x2`. The order of ungrouped factors is preserved.
fn replace_group_in_equation(equation: &str, group: &[String], replacement: &str) -> String {
    let group_set: std::collections::HashSet<&str> = group.iter().map(String::as_str).collect();

    split_terms(equation)
        .into_iter()
        .map(|term| {
            let (sign, body) = match term.as_bytes().first() {
                Some(b'+') => ("+", &term[1..]),
                Some(b'-') => ("-", &term[1..]),
                _ => ("", term.as_str()),
            };

            let factors: Vec<&str> = body.split('*').collect();
            let mut matched = std::collections::HashSet::new();

            for factor in &factors {
                if group_set.contains(*factor) {
                    matched.insert(*factor);
                }
            }

            if matched.len() != group.len() {
                return term;
            }

            let mut new_factors = Vec::with_capacity(factors.len() - group.len() + 1);
            new_factors.push(replacement.to_string());

            for factor in factors {
                if !group_set.contains(factor) {
                    new_factors.push(factor.to_string());
                }
            }

            format!("{}{}", sign, new_factors.join("*"))
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Serializable data for a polynomial-system test.
#[derive(Serialize)]
struct TestData {
    pub dimension: u64,
    pub variables: Vec<String>,
    pub equations: Vec<String>,
}

/// Mutate test according to rule.
///
/// 1. `file` considered a JSON file path (no checks),
/// 2. It also expented to have top level fields:
/// - dimension: Number
/// - variables: [Strings]
/// - equations: [Strings]
/// 3. Variable names should't overlap
///
/// Errors:
/// - Couldn't read file to string
/// - Could not parse the string as `serde_json::Value`
/// - Field is missing or invalid names
/// - Array data type is not string
/// - Couldn't collect variables and equations to `Vec<String>`
/// - Couldn't create new file
/// - Couldn't write data to new file
fn mutate(
    rule: &MutationRule,
    file: &Path,
    target_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let text = read_to_string(file)?;
    let data: serde_json::Value = serde_json::from_str(&text)?;
    match rule {
        MutationRule::Simple { offset, negative } => {
            if let Some(obj) = data.as_object() {
                let dimension: u64 = obj
                    .get("dimension")
                    .and_then(|v| v.as_u64())
                    .ok_or("missing or invalid names")?;
                let variables: Vec<String> = obj
                    .get("variables")
                    .and_then(|v| v.as_array())
                    .ok_or("missing or invalid field")?
                    .iter()
                    .map(|item| {
                        item.as_str()
                            .ok_or("variables must contain only strings")
                            .map(String::from)
                    })
                    .collect::<Result<_, _>>()?;
                let mut equations: Vec<String> = obj
                    .get("equations")
                    .and_then(|v| v.as_array())
                    .ok_or("missing or invalid field")?
                    .iter()
                    .map(|item| {
                        item.as_str()
                            .ok_or("equations must contain only strings")
                            .map(String::from)
                    })
                    .collect::<Result<_, _>>()?;

                let mut replace = |offset: usize, negative: bool| {
                    for (var_i, variable) in variables.iter().enumerate() {
                        let prime = StreamingSieve::nth_prime(var_i + variables.len() * offset + 1);

                        for equation in equations.iter_mut() {
                            *equation = mutate_variable_in_equation(
                                equation,
                                variable,
                                prime as u64,
                                negative,
                            );
                        }
                    }
                };

                replace(*offset, *negative);

                let data = TestData {
                    dimension,
                    variables,
                    equations,
                };

                let new_path = target_dir.join(file.file_name().unwrap());
                let new_file = File::create(new_path)?;

                let writer = BufWriter::new(new_file);
                serde_json::to_writer_pretty(writer, &data)?;
            }
        }
        MutationRule::Complex { length } => {
            if let Some(obj) = data.as_object() {
                let dimension: u64 = obj
                    .get("dimension")
                    .and_then(|v| v.as_u64())
                    .ok_or("missing or invalid names")?;
                let variables: Vec<String> = obj
                    .get("variables")
                    .and_then(|v| v.as_array())
                    .ok_or("missing or invalid field")?
                    .iter()
                    .map(|item| {
                        item.as_str()
                            .ok_or("variables must contain only strings")
                            .map(String::from)
                    })
                    .collect::<Result<_, _>>()?;
                let equations: Vec<String> = obj
                    .get("equations")
                    .and_then(|v| v.as_array())
                    .ok_or("missing or invalid field")?
                    .iter()
                    .map(|item| {
                        item.as_str()
                            .ok_or("equations must contain only strings")
                            .map(String::from)
                    })
                    .collect::<Result<_, _>>()?;

                if *length <= 1 || *length > variables.len() {
                    eprintln!(
                        "Couldn't complex-mutate {}: length must be in 2..={}, got {}",
                        file.display(),
                        variables.len(),
                        length
                    );
                    return Ok(());
                }

                for indexes in index_combinations(variables.len(), *length) {
                    let group: Vec<String> =
                        indexes.iter().map(|&i| variables[i].clone()).collect();
                    let replacement = "z";

                    let mutated_equations: Vec<String> = equations
                        .iter()
                        .map(|equation| replace_group_in_equation(equation, &group, replacement))
                        .collect();

                    if mutated_equations == equations {
                        continue;
                    }

                    let mut mutated_variables = variables.clone();
                    if !mutated_variables.iter().any(|v| v == replacement) {
                        mutated_variables.push(replacement.to_string());
                    }

                    let data = TestData {
                        dimension: mutated_variables.len() as u64,
                        variables: mutated_variables,
                        equations: mutated_equations,
                    };

                    let stem = file
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .ok_or("file name must be valid UTF-8")?;
                    let group_name = group.join("_");
                    let new_path =
                        target_dir.join(format!("{}_{}_{}.json", stem, replacement, group_name));
                    let new_file = File::create(new_path)?;

                    let writer = BufWriter::new(new_file);
                    serde_json::to_writer_pretty(writer, &data)?;
                }
            }
        }
    }

    Ok(())
}

/// Mutates tests and saves them.
///
/// Errors:
/// - Couldn't extract eligible tests
/// - Couldn't create directory
pub fn mutate_tests(
    rule: &MutationRule,
    min_time: f64,
    max_time: f64,
    baseline_results: &Path,
    target_dir: &Path,
    out_dir: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let eligible_tests =
        utils::extract_eligible_tests(baseline_results, target_dir, min_time, max_time)
            .map_err(|e| format!("Couldn't extract eligible tests: {e}"))?;

    if eligible_tests.is_empty() {
        println!("No eligible tests");
        return Ok(());
    }

    create_dir_all(&out_dir)
        .map_err(|e| format!("Couldn't create dir {}: {e}", out_dir.display()))?;

    for entry in eligible_tests {
        if let Err(e) = mutate(&rule, entry.as_path(), out_dir.as_path()) {
            eprintln!("Couldn't mutate {:?}: {}", entry, e);
        }
    }
    Ok(())
}

/// Run tests.
///
/// Errors:
/// - Could't read the target dir
pub fn run(
    num_threads: usize,
    runner: Runner,
    target_dir: &Path,
    out_dir: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let tests_queue = Arc::new(Mutex::new(init_deque(target_dir)?));
    let mut handles = Vec::new();
    let runner = Arc::new(runner);
    let out_dir = Arc::new(out_dir);

    for _ in 0..num_threads {
        let tests_queue = Arc::clone(&tests_queue);
        let runner = Arc::clone(&runner);
        let out_dir = Arc::clone(&out_dir);

        let handle = thread::spawn(move || {
            loop {
                let path = {
                    let mut q = match tests_queue.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => {
                            eprintln!("mutex was poisoned, recovering");
                            poisoned.into_inner()
                        }
                    };
                    q.pop_front()
                };

                match path {
                    Some(path) => runner.run(&path, &out_dir),
                    None => break,
                };
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}

/// Mutates tests according to rule and runs them.
///
/// Errors:
/// - Couldn't extract eligible tests
/// - Couldn't create a temp dir
/// - Couldn't read a temp dir
/// - Error during tests run
///
/// Warnings:
/// - Prints a warning if cleanup fails
pub fn run_with_mutations(
    rule: MutationRule,
    min_time: f64,
    max_time: f64,
    num_threads: usize,
    runner: Runner,
    baseline_results: &Path,
    target_dir: &Path,
    out_dir: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let eligible_tests =
        utils::extract_eligible_tests(baseline_results, target_dir, min_time, max_time)
            .map_err(|e| format!("Couldn't extract eligible tests: {e}"))?;

    if eligible_tests.is_empty() {
        println!("No eligible tests");
        return Ok(());
    }

    let tmp_dir_name = random_range(1_000_000..=10_000_000).to_string();
    let tmp_dir = target_dir.join(&tmp_dir_name);

    create_dir(&tmp_dir)
        .map_err(|e| format!("Couldn't create temp dir {}: {e}", tmp_dir.display()))?;

    for entry in eligible_tests {
        if let Err(e) = mutate(&rule, entry.as_path(), tmp_dir.as_path()) {
            eprintln!("Couldn't mutate {:?}: {}", entry, e);
        }
    }

    // FIXME: cleanup is skipped if read_dir fails
    let result: Result<(), Box<dyn std::error::Error>> =
        if std::fs::read_dir(&tmp_dir)?.next().is_none() {
            println!("No tests after mutations");
            Ok(())
        } else {
            run(num_threads, runner, tmp_dir.as_path(), out_dir)
                .map_err(|e| format!("Error during main tests run: {e}").into())
        };

    if let Err(e) = remove_dir_all(&tmp_dir) {
        eprintln!("Couldn't perform cleanup {}: {}", tmp_dir.display(), e);
    }

    result
}
