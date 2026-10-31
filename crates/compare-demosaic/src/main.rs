use std::path::PathBuf;

use compare_demosaic::results::{ComparisonResults, DEFAULT_RESULTS_FILE};
use compare_demosaic::{ALGORITHM_COUNT, ProgressEvent};
use indicatif::{ProgressBar, ProgressStyle};

fn main() -> mischief::Result<()> {
    ensure_no_arguments(std::env::args().skip(1))?;
    write_results()
}

fn ensure_no_arguments(arguments: impl IntoIterator<Item = String>) -> mischief::Result<()> {
    if arguments.into_iter().next().is_some() {
        return Err(mischief::mischief!(
            "usage: compare-demosaic (writes results/canonical.json)"
        ));
    }
    Ok(())
}

fn write_results() -> mischief::Result<()> {
    let progress = progress_bar()?;
    let results =
        ComparisonResults::canonical_with_progress(|event| report_progress(&progress, event))?;

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_RESULTS_FILE);
    results.write(&path)?;
    progress.finish_with_message("comparison complete");
    println!("wrote {}", path.display());
    Ok(())
}

fn progress_bar() -> mischief::Result<ProgressBar> {
    let progress = ProgressBar::new(ALGORITHM_COUNT as u64);
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
    fn command_accepts_no_options() {
        ensure_no_arguments(Vec::<String>::new()).expect("no arguments are valid");
        for removed_option in ["--stdout", "--output"] {
            let error = ensure_no_arguments([removed_option.to_owned()])
                .expect_err("comparison command options are unsupported");
            assert!(error.to_string().contains("results/canonical.json"));
        }
    }

    #[test]
    fn progress_tracks_total_case_and_completion() -> mischief::Result<()> {
        let progress = progress_bar()?;
        progress.set_draw_target(indicatif::ProgressDrawTarget::hidden());
        assert_eq!(progress.length(), Some(ALGORITHM_COUNT as u64));
        assert_eq!(progress.position(), 0);

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
