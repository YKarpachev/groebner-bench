use std::io::BufWriter;
use std::{
    fs::File,
    path::{Path, PathBuf},
    process::{Command, Output},
};

/// Mutation rull that is required by `mutate` func.
/// Possible values:
/// - Small
/// - Medium
/// - Large
/// - Signed
pub enum MutationRule {
    Simple { offset: usize, negative: bool },
    Complex { length: usize },
}

/// Represents a program runner.
///
/// A runner can either execute a script through an interpreter or execute a
/// binary directly. In both cases, the program receives a JSON file path as an
/// argument and is expected to write JSON output to `stdout`.
pub enum Runner {
    Script {
        interpreter: PathBuf,
        program: PathBuf,
    },
    Bin {
        program: PathBuf,
    },
}

impl Runner {
    /// Create new `Runner`:
    /// - provide program path and interpreter path to make `Runner::Script`
    /// - provide program path and `None` to make `Runner::Bin`
    pub fn new(program: &Path, interpreter: Option<&Path>) -> Runner {
        match interpreter {
            Some(val) => Runner::Script {
                interpreter: PathBuf::from(val),
                program: PathBuf::from(program),
            },
            None => Runner::Bin {
                program: PathBuf::from(program),
            },
        }
    }

    /// Writes `Output` (JSON) to new file
    ///
    /// Errors:
    /// - Couldn't read to string
    /// - Couldn't convert to `serde_json::Value`
    /// - Couldn't create new file
    /// - Couldn't write to new file
    ///
    /// Panics:
    /// - Couldn't create output dir for results
    /// - json_path doesn't have a file name
    fn write_to_file(
        output: Output,
        out_dir: &Path,
        json_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let text = String::from_utf8(output.stdout)?;
        let data: serde_json::Value = serde_json::from_str(&text)?;

        std::fs::create_dir_all(&out_dir).expect("Couldn't create a new dir for results");

        let new_file = File::create(
            out_dir.join(Path::new(
                json_path
                    .file_name()
                    .expect("is_file() check should exist outside."),
            )),
        )?;

        let writer = BufWriter::new(new_file);
        serde_json::to_writer_pretty(writer, &data)?;

        Ok(())
    }

    /// Runs the configured program with the given JSON file as input.
    ///
    /// The program is expected to write JSON to `stdout`, which is then saved
    /// as a pretty-printed JSON file in `out_dir`.
    pub fn run(&self, json_path: &Path, out_dir: &Path) {
        let output = match self {
            Runner::Script {
                interpreter,
                program,
            } => Command::new(interpreter)
                .args([program, json_path])
                .output(),
            Runner::Bin { program } => Command::new(program).args([json_path]).output(),
        };
        match output {
            Err(e) => {
                eprintln!("failed to execute process: {}", e);
            }
            Ok(output) => {
                if !output.status.success() {
                    eprintln!("command failed: {}", output.status);
                    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                }
                if let Err(e) = Self::write_to_file(output, out_dir, json_path) {
                    eprintln!("Could't write stdout to new file: {}", e);
                }
            }
        };
        ()
    }
}
