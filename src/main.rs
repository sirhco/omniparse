//! Omniparse CLI entry point

mod cli;

use clap::Parser;
use cli::args::{Cli, Command, OutputFormat};
use cli::output::{format_detection_result, format_extraction_result};
use omniparse::extract_from_path;
use std::fs::File;
use std::io::{self, Write};
use std::process;

fn main() {
    let args = Cli::parse();

    // Subcommand path bypasses the extraction flow entirely.
    if let Some(Command::Models { action }) = &args.command {
        let code = cli::models::run(action);
        process::exit(code);
    }

    if let Err(e) = run(args) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run(args: Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Determine output writer
    let mut output: Box<dyn Write> = if let Some(output_path) = &args.output {
        Box::new(File::create(output_path)?)
    } else {
        Box::new(io::stdout())
    };
    
    // Process files
    if args.detect_only {
        // Detection-only mode
        process_detection_only(&args.files, &mut output, &args.format, args.verbose)?;
    } else if args.parallel && args.files.len() > 1 {
        // Parallel processing mode
        process_files_parallel(&args, &mut output)?;
    } else {
        // Sequential processing mode
        process_files_sequential(&args, &mut output)?;
    }
    
    Ok(())
}

fn process_detection_only(
    files: &[std::path::PathBuf],
    output: &mut dyn Write,
    format: &OutputFormat,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let detector = omniparse::detection::TypeDetector::new();
    
    for (i, path) in files.iter().enumerate() {
        if verbose {
            eprintln!("Detecting type for: {}", path.display());
        }
        
        match detector.detect_from_path(path) {
            Ok(detection) => {
                if i > 0 && !matches!(format, OutputFormat::Json | OutputFormat::Yaml) {
                    writeln!(output, "\n---\n")?;
                }
                
                if files.len() > 1 && !matches!(format, OutputFormat::Json | OutputFormat::Yaml) {
                    writeln!(output, "File: {}", path.display())?;
                }
                
                format_detection_result(output, &detection, format)?;
            }
            Err(e) => {
                eprintln!("Error detecting {}: {}", path.display(), e);
            }
        }
    }
    
    Ok(())
}

fn process_files_sequential(
    args: &Cli,
    output: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut success_count = 0;
    let mut error_count = 0;
    
    for (i, path) in args.files.iter().enumerate() {
        if args.verbose {
            eprintln!("Processing: {}", path.display());
        }
        
        match extract_from_path(path) {
            Ok(result) => {
                if i > 0 && !matches!(args.format, OutputFormat::Json | OutputFormat::Yaml) {
                    writeln!(output, "\n---\n")?;
                }
                
                if args.files.len() > 1 && !matches!(args.format, OutputFormat::Json | OutputFormat::Yaml) {
                    writeln!(output, "File: {}", path.display())?;
                }
                
                format_extraction_result(output, &result, &args.format, args.metadata_only)?;
                success_count += 1;
                
                if args.verbose {
                    eprintln!("✓ Successfully processed {}", path.display());
                }
            }
            Err(e) => {
                eprintln!("Error processing {}: {}", path.display(), e);
                error_count += 1;
            }
        }
    }
    
    if args.verbose && args.files.len() > 1 {
        eprintln!("\nSummary: {} succeeded, {} failed", success_count, error_count);
    }
    
    if error_count > 0 && success_count == 0 {
        return Err("All files failed to process".into());
    }
    
    Ok(())
}

fn process_files_parallel(
    args: &Cli,
    output: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    if args.verbose {
        eprintln!("Processing {} files in parallel...", args.files.len());
    }
    
    let extractor = omniparse::core::Extractor::new();
    let results = omniparse::utils::parallel::process_files_parallel(&extractor, &args.files);
    
    let mut success_count = 0;
    let mut error_count = 0;
    
    for (i, file_result) in results.iter().enumerate() {
        match &file_result.result {
            Ok(result) => {
                if i > 0 && !matches!(args.format, OutputFormat::Json | OutputFormat::Yaml) {
                    writeln!(output, "\n---\n")?;
                }
                
                if args.files.len() > 1 && !matches!(args.format, OutputFormat::Json | OutputFormat::Yaml) {
                    writeln!(output, "File: {}", file_result.path)?;
                }
                
                format_extraction_result(output, result, &args.format, args.metadata_only)?;
                success_count += 1;
                
                if args.verbose {
                    eprintln!("✓ Successfully processed {}", file_result.path);
                }
            }
            Err(e) => {
                eprintln!("Error processing {}: {}", file_result.path, e);
                error_count += 1;
            }
        }
    }
    
    if args.verbose {
        eprintln!("\nSummary: {} succeeded, {} failed", success_count, error_count);
    }
    
    if error_count > 0 && success_count == 0 {
        return Err("All files failed to process".into());
    }
    
    Ok(())
}

