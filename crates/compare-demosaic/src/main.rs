use std::path::PathBuf;

use compare_demosaic::results::{
    CANONICAL_RESULTS_FILE, ComparisonResults, DEFAULT_RESULTS_DIR, SUMMARY_RESULTS_FILE,
};
use compare_demosaic::{ALGORITHM_COUNT, ProgressEvent, data};
use indicatif::{ProgressBar, ProgressStyle};

fn main() -> mischief::Result<()> {
    let input = data::canonical()?;
    let progress = progress_bar(ALGORITHM_COUNT)?;
    let results = ComparisonResults::build(input, |event| report_progress(&progress, event))?;

    let results_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_RESULTS_DIR);
    let canonical_path = results_dir.join(CANONICAL_RESULTS_FILE);
    let summary_path = results_dir.join(SUMMARY_RESULTS_FILE);
    results.write(&canonical_path)?;
    results.summary().write(&summary_path)?;
    progress.finish_with_message("comparison complete");
    println!("wrote {}", canonical_path.display());
    println!("wrote {}", summary_path.display());
    Ok(())
}

fn progress_bar(total: usize) -> mischief::Result<ProgressBar> {
    let progress = ProgressBar::new(total as u64);
    progress.set_style(
        ProgressStyle::with_template("{elapsed_precise} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .map_err(|error| mischief::mischief!("invalid progress template: {error}"))?
            .progress_chars("=>-"),
    );
    Ok(progress)
}

fn report_progress(progress: &ProgressBar, event: ProgressEvent) {
    match event {
        ProgressEvent::Started {
            algorithm,
            reference,
        } => progress.set_message(format!("{algorithm} / {reference}")),
        ProgressEvent::Finished => progress.inc(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_tracks_algorithm_and_completion() -> mischief::Result<()> {
        let progress = progress_bar(3)?;
        progress.set_draw_target(indicatif::ProgressDrawTarget::hidden());
        assert_eq!(progress.length(), Some(3));

        report_progress(
            &progress,
            ProgressEvent::Started {
                algorithm: "AHD",
                reference: "LibRaw",
            },
        );
        assert_eq!(progress.message(), "AHD / LibRaw");
        assert_eq!(progress.position(), 0);

        report_progress(&progress, ProgressEvent::Finished);
        assert_eq!(progress.position(), 1);
        progress.finish_with_message("comparison complete");
        assert!(progress.is_finished());
        assert_eq!(progress.message(), "comparison complete");
        Ok(())
    }
}
