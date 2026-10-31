use std::collections::BTreeMap;
use std::path::Path;

use crate::results::CanonicalResults;

pub const README_FILE: &str = "crates/compare-demosaic/README.md";
pub const NOTES_FILE: &str = "crates/compare-demosaic/notes.json";

pub fn render_baseline() -> mischief::Result<String> {
    let baseline =
        CanonicalResults::read(&crate::results::repo_path(crate::results::BASELINE_FILE))?;
    let notes_path = crate::results::repo_path(NOTES_FILE);
    let notes = serde_json::from_slice::<BTreeMap<String, String>>(&std::fs::read(notes_path)?)
        .map_err(|error| mischief::mischief!("cannot parse notes.json: {error}"))?;
    Ok(render(&baseline, &notes))
}

pub fn write_baseline() -> mischief::Result<()> {
    std::fs::write(crate::results::repo_path(README_FILE), render_baseline()?)?;
    Ok(())
}

pub fn check_baseline() -> mischief::Result<()> {
    check_text(&crate::results::repo_path(README_FILE), &render_baseline()?)
}

pub fn render(results: &CanonicalResults, notes: &BTreeMap<String, String>) -> String {
    let mut output = String::from(
        "# compare-demosaic\n\n\
Compares Fornax demosaic implementations with LibRaw and darktable at the\n\
demosaic stage. The canonical comparison uses the full active area returned by\n\
`fornax_devtool::raw_file()` and evaluates the same mosaic under all four Bayer\n\
labels. White balance and the default output crop are not applied.\n\n\
`canonical.json` is the source of truth for quality verdicts. The tables below\n\
are generated from the committed baseline; do not edit their numbers by hand.\n\
See [AGENTS.md](AGENTS.md) for the investigation workflow.\n\n\
```bash\n\
source scripts/setup.sh\n\
cargo run --release -p compare-demosaic\n\
cargo run --release -p compare-demosaic -- --artifacts\n\
cargo run --release -p compare-demosaic -- diff <old.json> <new.json>\n\
cargo run --release -p compare-demosaic -- --bless\n\
```\n\n\
The default run writes `target/compare-demosaic/canonical.json`. Failed rows\n\
write log-scaled error maps and tile summaries under\n\
`target/compare-demosaic/artifacts/`; `--artifacts` writes them for passing rows\n\
too. Filtered runs write `target/compare-demosaic/canonical.filtered.json` and\n\
are not valid baselines.\n\n\
## Evaluation\n\n\
Robust verdicts require all three conditions:\n\n\
- MAE at most 2 codes.\n\
- P99 absolute error at most 64 codes.\n\
- At most a 0.001 fraction of channel samples above 1024 codes.\n\n\
All errors are 16-bit codes. Exact means every sample is bit-identical.\n\
Rounding-only means every difference is one code upward from the truncating\n\
LibRaw reference. A warning means every robust limit passed but at least one\n\
sample exceeded 1024 codes.\n\n",
    );

    let mut algorithms = BTreeMap::<&str, Vec<&crate::results::ComparisonResult>>::new();
    for result in &results.results {
        algorithms
            .entry(result.algorithm.as_str())
            .or_default()
            .push(result);
    }
    for (algorithm, rows) in algorithms {
        output.push_str(&format!("## {algorithm}\n\n"));
        output.push_str(
            "| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | P999 | >1024 frac | Changed frac |\n\
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
        );
        for row in rows {
            output.push_str(&format!(
                "| {} | [{}]({}) | {} | {:.6} | {:.6} | {} | {} | {} | {:.6} | {:.6} |\n",
                row.pattern,
                row.reference,
                row.source_url,
                row.verdict.symbol(),
                row.metrics.mae,
                row.metrics.rmse,
                row.metrics.max,
                row.metrics.p99,
                row.metrics.p999,
                row.metrics.frac_gt1024,
                row.metrics.changed_frac,
            ));
        }
        if let Some(note) = notes.get(algorithm) {
            output.push_str(&format!("\n> [!NOTE]\n> {}\n", note.replace('\n', "\n> ")));
        }
        output.push('\n');
    }

    output.push_str(&format!(
        "Reference builds: LibRaw {}; darktable {} ({}).\n\nLegend:\n\n{}",
        results.meta.references.libraw.version,
        results.meta.references.darktable.version,
        results.meta.references.darktable.commit,
        concat!(
            "- `✅`: every compared sample is bit-identical or every robust limit passes\n",
            "  without high-error outliers.\n",
            "- `🟢`: differences are one code upward because Fornax rounds to nearest while\n",
            "  LibRaw truncates.\n",
            "- `⚠️`: every robust limit passes, but isolated high-error samples remain.\n",
            "- `❌`: at least one limit in `failed_limits` failed.\n",
        ),
    ));
    output
}

fn check_text(path: &Path, expected: &str) -> mischief::Result<()> {
    let actual = std::fs::read_to_string(path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(mischief::mischief!(
            "{} is out of date; run `cargo run -p compare-demosaic -- readme`",
            path.display()
        ))
    }
}
