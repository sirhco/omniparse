//! `omniparse models …` subcommand handlers.
//!
//! All real work happens in [`omniparse::ocr::ml`]; this module is a thin
//! presentation layer. When the binary is built without the `ocr-ml` feature
//! the subcommand still parses but emits a clear error.

use crate::cli::args::ModelsAction;

/// Dispatch a `models` subcommand. Returns an exit code (0 = success).
pub fn run(action: &ModelsAction) -> i32 {
    #[cfg(feature = "ocr-ml")]
    {
        match action {
            ModelsAction::Download { force } => download(*force),
            ModelsAction::Verify => verify(),
            ModelsAction::Path => print_path(),
            ModelsAction::List => list(),
        }
    }
    #[cfg(not(feature = "ocr-ml"))]
    {
        let _ = action;
        eprintln!(
            "omniparse: `models` subcommand requires the `ocr-ml` feature.\n\
             Rebuild with: cargo install omniparse --features ocr-ml"
        );
        2
    }
}

#[cfg(feature = "ocr-ml")]
fn download(force: bool) -> i32 {
    match omniparse::ocr::ml::prefetch_all(force) {
        Ok(paths) => {
            for p in &paths {
                println!("ok  {}", p.display());
            }
            0
        }
        Err(e) => {
            eprintln!("download failed: {e}");
            1
        }
    }
}

#[cfg(feature = "ocr-ml")]
fn verify() -> i32 {
    match omniparse::ocr::ml::verify_all() {
        Ok(()) => {
            println!("ok");
            0
        }
        Err(e) => {
            eprintln!("verify failed: {e}");
            1
        }
    }
}

#[cfg(feature = "ocr-ml")]
fn print_path() -> i32 {
    match omniparse::ocr::ml::model_dir() {
        Ok(p) => {
            println!("{}", p.display());
            0
        }
        Err(e) => {
            eprintln!("could not resolve cache dir: {e}");
            1
        }
    }
}

#[cfg(feature = "ocr-ml")]
fn list() -> i32 {
    let statuses = match omniparse::ocr::ml::list_models() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("list failed: {e}");
            return 1;
        }
    };
    println!(
        "{:<25} {:>10}  {:<64}  {}",
        "NAME", "SIZE", "SHA256", "STATUS"
    );
    let mut all_ok = true;
    for s in &statuses {
        let size = s.size.map(format_size).unwrap_or_else(|| "-".to_string());
        let sha = s.sha256.as_deref().unwrap_or("-");
        let status = if s.size.is_none() {
            "missing"
        } else if s.ok {
            "ok"
        } else {
            all_ok = false;
            "mismatch"
        };
        println!("{:<25} {:>10}  {:<64}  {}", s.spec.name, size, sha, status);
    }
    if all_ok { 0 } else { 1 }
}

#[cfg(feature = "ocr-ml")]
fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let b = bytes as f64;
    if b >= MB {
        format!("{:.1}M", b / MB)
    } else if b >= KB {
        format!("{:.1}K", b / KB)
    } else {
        format!("{}B", bytes)
    }
}
