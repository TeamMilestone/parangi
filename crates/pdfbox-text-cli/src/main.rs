use std::path::PathBuf;

use clap::Parser;

/// PDF text extraction CLI tool (Rust port of Apache PDFBox text extraction).
#[derive(Parser, Debug)]
#[command(name = "pdfbox-text", version, about)]
struct Cli {
    /// Input PDF file path(s)
    #[arg(required = true)]
    input: Vec<PathBuf>,

    /// Output file path (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Number of parallel threads (0 = auto)
    #[arg(short = 'j', long, default_value = "0")]
    threads: usize,
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

    for input in &cli.input {
        match pdfbox_text::extract_text(input) {
            Ok(text) => {
                if let Some(ref output_path) = cli.output {
                    if let Err(e) = std::fs::write(output_path, &text) {
                        eprintln!("Error writing {}: {}", output_path.display(), e);
                        std::process::exit(1);
                    }
                } else {
                    print!("{}", text);
                }
            }
            Err(e) => {
                eprintln!("Error processing {}: {}", input.display(), e);
                std::process::exit(1);
            }
        }
    }
}
