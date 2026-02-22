use std::io::Write;
use std::path::PathBuf;

use clap::Parser;

/// PDF text extraction CLI tool (Rust port of Apache PDFBox text extraction).
#[derive(Parser, Debug)]
#[command(name = "pdfbox-text", version, about)]
struct Cli {
    /// Input PDF file path(s)
    #[arg(required = true)]
    input: Vec<PathBuf>,

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

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    if cli.threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(cli.threads)
            .build_global()
            .ok();
    }

    let config = pdfbox_text::StripperConfig::default();

    if cli.input.len() == 1 && cli.output_dir.is_none() {
        // Single file mode: output to stdout or specified file
        let path = &cli.input[0];
        let result = if cli.sequential {
            pdfbox_text::extract_text_sequential(path, &config)
        } else {
            pdfbox_text::extract_text_with_config(path, &config)
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
        // Batch mode: parallel file processing
        let output_dir = cli.output_dir.as_deref();
        if let Some(dir) = output_dir {
            std::fs::create_dir_all(dir).unwrap_or_else(|e| {
                eprintln!("Error creating output directory {}: {}", dir.display(), e);
                std::process::exit(1);
            });
        }

        let results = pdfbox_text::extract_text_batch(&cli.input, &config);

        let mut success_count = 0u32;
        let mut error_count = 0u32;
        let stderr = std::io::stderr();

        for batch_result in results {
            match batch_result.result {
                Ok(text) => {
                    success_count += 1;
                    if let Some(dir) = output_dir {
                        let filename = batch_result
                            .path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy();
                        let out_path = dir.join(format!("{}.txt", filename));
                        if let Err(e) = std::fs::write(&out_path, &text) {
                            let mut err = stderr.lock();
                            let _ = writeln!(err, "Error writing {}: {}", out_path.display(), e);
                            error_count += 1;
                        }
                    } else {
                        // Multiple files to stdout: print with file header
                        println!("=== {} ===", batch_result.path.display());
                        print!("{}", text);
                        println!();
                    }
                }
                Err(e) => {
                    error_count += 1;
                    let mut err = stderr.lock();
                    let _ = writeln!(
                        err,
                        "Error processing {}: {}",
                        batch_result.path.display(),
                        e
                    );
                }
            }
        }

        if cli.input.len() > 1 {
            eprintln!(
                "Processed {} files: {} succeeded, {} failed",
                cli.input.len(),
                success_count,
                error_count
            );
        }

        if error_count > 0 {
            std::process::exit(1);
        }
    }
}
