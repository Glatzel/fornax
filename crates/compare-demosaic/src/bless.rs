use std::io::{BufRead, Write};
use std::path::Path;

use crate::diff::{self, DiffReport, FailOn};
use crate::error::{CompareError, Result};
use crate::results::{self, CanonicalResults};

#[derive(Debug)]
pub struct BlessReport {
    pub baseline: std::path::PathBuf,
    pub diff: Option<DiffReport>,
}

pub fn bless(path: &Path, yes: bool) -> Result<BlessReport> {
    let validation = crate::validate::validate_file(path, false, false)?;
    let candidate =
        CanonicalResults::read(path).map_err(|error| CompareError::invalid(error.to_string()))?;
    if candidate.meta.stable.dirty {
        return Err(CompareError::invalid(
            "refusing to bless evidence produced from a dirty working tree",
        ));
    }

    let baseline_path = results::repo_path(results::BASELINE_FILE);
    let report = if baseline_path.is_file() {
        Some(
            diff::compare_files(&baseline_path, path, FailOn::Never, false)
                .map_err(|error| CompareError::invalid(error.to_string()))?,
        )
    } else {
        None
    };
    if let Some(report) = &report {
        println!("{}", report.render_text());
    } else {
        println!(
            "creating the initial baseline with {} rows",
            validation.rows
        );
    }
    if !yes {
        print!("Bless this evidence and regenerate README tables? [y/N] ");
        std::io::stdout().flush()?;
        let mut answer = String::new();
        std::io::stdin().lock().read_line(&mut answer)?;
        if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
            return Err(CompareError::check("bless cancelled"));
        }
    }

    if let Some(parent) = baseline_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(path, &baseline_path)?;
    crate::render::write_baseline().map_err(|error| CompareError::usage(error.to_string()))?;
    Ok(BlessReport {
        baseline: baseline_path,
        diff: report,
    })
}
