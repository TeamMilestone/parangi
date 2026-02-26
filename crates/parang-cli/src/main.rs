use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use clap::Parser;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
use rayon::prelude::*;

/// PDF text extraction CLI tool (Rust port of Apache PDFBox text extraction).
#[derive(Parser, Debug)]
#[command(name = "parang", version, about)]
struct Cli {
    /// Input PDF file path(s)
    #[arg()]
    input: Vec<PathBuf>,

    /// Read input file paths from a file (one path per line, use '-' for stdin)
    #[arg(short = 'L', long)]
    file_list: Option<PathBuf>,

    /// Output directory for extracted text files (one .txt per PDF)
    #[arg(short, long)]
    output_dir: Option<PathBuf>,

    /// Output file path (only for single input file, default: stdout)
    #[arg(short = 'f', long)]
    output_file: Option<PathBuf>,

    /// Number of parallel threads (0 = auto)
    #[arg(short = 'j', long, default_value = "0")]
    threads: usize,

    /// Disable parallelism (single-threaded mode)
    #[arg(long)]
    sequential: bool,
}

fn read_file_list(path: &PathBuf) -> Vec<PathBuf> {
    let reader: Box<dyn BufRead> = if path.as_os_str() == "-" {
        Box::new(std::io::stdin().lock())
    } else {
        let file = std::fs::File::open(path).unwrap_or_else(|e| {
            eprintln!("Error opening file list {}: {}", path.display(), e);
            std::process::exit(1);
        });
        Box::new(std::io::BufReader::new(file))
    };
    reader
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(PathBuf::from(trimmed))
            }
        })
        .collect()
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    if cli.threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(cli.threads)
            .build_global()
            .ok();
    }

    let config = parang::StripperConfig::default();

    // Collect all input paths
    let mut all_inputs = cli.input.clone();
    if let Some(ref list_path) = cli.file_list {
        all_inputs.extend(read_file_list(list_path));
    }

    if all_inputs.is_empty() {
        eprintln!("Error: no input files specified");
        std::process::exit(1);
    }

    if all_inputs.len() == 1 && cli.output_dir.is_none() {
        // Single file mode: output to stdout or specified file
        let path = &all_inputs[0];
        let result = if cli.sequential {
            parang::extract_text_sequential(path, &config)
        } else {
            parang::extract_text_with_config(path, &config)
        };
        match result {
            Ok(text) => {
                if let Some(ref output_path) = cli.output_file {
                    if let Err(e) = std::fs::write(output_path, &text) {
                        eprintln!("Error writing {}: {}", output_path.display(), e);
                        std::process::exit(1);
                    }
                } else {
                    print!("{}", text);
                }
            }
            Err(e) => {
                eprintln!("Error processing {}: {}", path.display(), e);
                std::process::exit(1);
            }
        }
    } else {
        // Batch mode: streaming parallel file processing
        let output_dir = cli.output_dir.as_deref();
        if let Some(dir) = output_dir {
            std::fs::create_dir_all(dir).unwrap_or_else(|e| {
                eprintln!("Error creating output directory {}: {}", dir.display(), e);
                std::process::exit(1);
            });
        }

        let total = all_inputs.len();
        let success_count = AtomicU32::new(0);
        let error_count = AtomicU32::new(0);

        // Streaming parallel: process and write immediately per file
        all_inputs.par_iter().for_each(|path| {
            let result = parang::extract_text_with_config(path, &config);
            match result {
                Ok(text) => {
                    success_count.fetch_add(1, Ordering::Relaxed);
                    if let Some(dir) = output_dir {
                        let filename = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy();
                        let out_path = dir.join(format!("{}.txt", filename));
                        if let Err(e) = std::fs::write(&out_path, &text) {
                            eprintln!("Error writing {}: {}", out_path.display(), e);
                            error_count.fetch_add(1, Ordering::Relaxed);
                        }
                    } else {
                        // Multiple files to stdout: print with file header
                        let stdout = std::io::stdout();
                        let mut out = stdout.lock();
                        let _ = writeln!(out, "=== {} ===", path.display());
                        let _ = write!(out, "{}", text);
                        let _ = writeln!(out);
                    }
                }
                Err(e) => {
                    error_count.fetch_add(1, Ordering::Relaxed);
                    eprintln!("Error processing {}: {}", path.display(), e);
                }
            }
        });

        let success = success_count.load(Ordering::Relaxed);
        let errors = error_count.load(Ordering::Relaxed);

        if total > 1 {
            eprintln!(
                "Processed {} files: {} succeeded, {} failed",
                total, success, errors
            );
        }

        parang::print_profile_summary();

        if errors > 0 {
            std::process::exit(1);
        }
    }

    parang::print_profile_summary();
}
