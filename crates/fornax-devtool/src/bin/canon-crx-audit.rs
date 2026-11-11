use std::process::ExitCode;

use fornax_devtool::canon_crx_audit::{audit_file, render_csv, summarize};

fn main() -> ExitCode {
    let mut rows = Vec::new();
    for path in std::env::args_os().skip(1) {
        let path = std::path::PathBuf::from(path);
        match audit_file(&path) {
            Ok(row) => rows.push(row),
            Err(error) => {
                eprintln!("{}: {error}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }
    if rows.is_empty() {
        eprintln!("usage: canon-crx-audit FILE...");
        return ExitCode::from(2);
    }
    print!("{}", render_csv(&rows));
    let (lossless, other) = summarize(&rows);
    eprintln!("lossless={lossless} unsupported={other} total={}", rows.len());
    ExitCode::SUCCESS
}
