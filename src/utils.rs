use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{read_dir, read_to_string};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Finds tests in `target_dir` whose matching baseline result files are eligible.
///
/// Iterates over JSON files in `baseline_results` and marks a test as eligible
/// when:
/// - the top-level `status` field is `"ok"`
/// - `solver.solve_time_sec` exists
/// - `min_time` < `solver.solve_time_sec` < `max_time`
///
/// A file in `target_dir` is returned if its filename matches the filename of
/// an eligible JSON file in `baseline_results`.
///
/// Per-file errors, invalid JSON, missing fields, and non-JSON files are skipped.
///
/// Errors:
/// - Couldn't read `baseline_results`
/// - Couldn't read `target_dir`
pub fn extract_eligible_tests(
    baseline_results: &Path,
    target_dir: &Path,
    min_time: f64,
    max_time: f64,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut eligible_test_names: HashSet<OsString> = HashSet::new();
    let mut eligible_tests: Vec<PathBuf> = Vec::new();

    for entry in read_dir(baseline_results)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("failed to process file: {}", e);
                continue;
            }
        };

        let path = entry.path();

        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let text = match read_to_string(&path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("failed to read file: {}", e);
                continue;
            }
        };

        let data: serde_json::Value = match serde_json::from_str(&text) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("failed to parse JSON: {}", e);
                continue;
            }
        };

        let status_is_ok = data.get("status").and_then(|v| v.as_str()) == Some("ok");

        let solve_time = data
            .get("solver")
            .and_then(|v| v.get("solve_time_sec"))
            .and_then(|v| v.as_f64());

        if status_is_ok {
            if let Some(time) = solve_time {
                if min_time < time && time < max_time {
                    eligible_test_names.insert(entry.file_name());
                }
            }
        }
    }

    for entry in read_dir(target_dir)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("failed to process target file: {}", e);
                continue;
            }
        };

        if eligible_test_names.contains(&entry.file_name()) {
            eligible_tests.push(entry.path());
        }
    }

    Ok(eligible_tests)
}

/// Generate a comparison graph from validated input data.
///
/// This function assumes all arguments and JSON fields are present,
/// correctly typed, and structurally valid. It does not perform input
/// validation.
pub fn make_graph(venv_python_path: &Path, old_results: &Path, new_results: &Path) {
    let program_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("python")
        .join("graph.py");

    match Command::new(venv_python_path)
        .args([program_path.as_path(), old_results, new_results])
        .output()
    {
        Err(e) => eprintln!("failed to start process: {}", e),
        Ok(output) => {
            if !output.stderr.is_empty() {
                eprintln!(
                    "python stderr:\n{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
}

/// Generate an excel book from results
///
/// This function assumes all arguments and JSON fields are present,
/// correctly typed, and structurally valid. It does not perform input
/// validation.
pub fn make_excel_book(venv_python_path: &Path, result_dirs: Vec<PathBuf>) {
    let program_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("python")
        .join("excel_book.py");

    let args: Vec<PathBuf> = std::iter::once(program_path).chain(result_dirs).collect();

    match Command::new(venv_python_path).args(args).output() {
        Err(e) => eprintln!("failed to start process: {}", e),
        Ok(output) => {
            if !output.stderr.is_empty() {
                eprintln!(
                    "python stderr:\n{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
}
