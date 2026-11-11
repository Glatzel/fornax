use std::path::PathBuf;
use std::process::ExitCode;

use fornax_devtool::canon_crx_audit::{audit_file, audit_manifest, render_csv, summarize};

fn main() -> ExitCode {
    let mut corpus = None;
    let mut manifest = None;
    let mut output = None;
    let mut paths = Vec::new();
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--corpus") => corpus = args.next().map(PathBuf::from),
            Some("--manifest") => manifest = args.next().map(PathBuf::from),
            Some("--out") => output = args.next().map(PathBuf::from),
            _ => paths.push(PathBuf::from(arg)),
        }
    }
    let rows = if let (Some(corpus), Some(manifest)) = (corpus, manifest) {
        match audit_manifest(&corpus, &manifest) {
            Ok(rows) => rows,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        let mut rows = Vec::new();
        for path in paths {
            match audit_file(&path) {
                Ok(row) => rows.push(row),
                Err(error) => {
                    eprintln!("{}: {error}", path.display());
                    return ExitCode::FAILURE;
                }
            }
        }
        rows
    };
    if rows.is_empty() {
        eprintln!("usage: canon-crx-audit FILE...");
        return ExitCode::from(2);
    }
    let csv = render_csv(&rows);
    if let Some(output) = output {
        if let Err(error) = std::fs::write(&output, csv) {
            eprintln!("writing {}: {error}", output.display());
            return ExitCode::FAILURE;
        }
    } else {
        print!("{csv}");
    }
    let (lossless, lossy, ambiguous) = summarize(&rows);
    eprintln!(
        "lossless={lossless} lossy={lossy} ambiguous={ambiguous} total={}",
        rows.len()
    );
    ExitCode::SUCCESS
}
