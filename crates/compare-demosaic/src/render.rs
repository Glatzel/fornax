use std::collections::BTreeMap;
use std::path::Path;

use crate::results::CanonicalResults;

pub const README_FILE: &str = "crates/compare-demosaic/README.md";
pub const NOTES_FILE: &str = "crates/compare-demosaic/notes.json";
pub const GENERATED_BEGIN: &str = "<!-- BEGIN GENERATED -->";
pub const GENERATED_END: &str = "<!-- END GENERATED -->";

pub fn render_baseline() -> mischief::Result<String> {
    render_from(&crate::results::repo_path(crate::results::BASELINE_FILE))
}

pub fn render_from(path: &Path) -> mischief::Result<String> {
    let baseline = CanonicalResults::read(path)?;
    let notes_path = crate::results::repo_path(NOTES_FILE);
    let notes = serde_json::from_slice::<BTreeMap<String, String>>(&std::fs::read(notes_path)?)
        .map_err(|error| mischief::mischief!("cannot parse notes.json: {error}"))?;
    Ok(render(&baseline, &notes))
}

pub fn write_baseline() -> mischief::Result<()> { write(None) }

pub fn write(from: Option<&Path>) -> mischief::Result<()> {
    let path = crate::results::repo_path(README_FILE);
    let rendered = from.map_or_else(render_baseline, render_from)?;
    let actual = std::fs::read_to_string(&path).unwrap_or_default();
    std::fs::write(path, replace_generated(&actual, &rendered))?;
    Ok(())
}

pub fn check_baseline() -> mischief::Result<()> { check(None) }

pub fn check(from: Option<&Path>) -> mischief::Result<()> {
    let path = crate::results::repo_path(README_FILE);
    let rendered = from.map_or_else(render_baseline, render_from)?;
    let actual = std::fs::read_to_string(&path)?;
    check_text(&path, &replace_generated(&actual, &rendered))
}

pub fn render(results: &CanonicalResults, notes: &BTreeMap<String, String>) -> String {
    let mut output = String::from(
        "<!-- BEGIN GENERATED -->\n\
# compare-demosaic\n\n\
Compares Fornax demosaic implementations with LibRaw and darktable at the\n\
demosaic stage. The canonical comparison uses the full active area returned by\n\
`fornax_devtool::raw_file()` and evaluates the same mosaic under all four Bayer\n\
labels. White balance and the default output crop are not applied.\n\n\
`canonical.json` is the source of truth for quality verdicts. The tables below\n\
are generated from the committed baseline; do not edit their numbers by hand.\n\
See [AGENTS.md](AGENTS.md) for the investigation workflow.\n\n\
```bash\n\
source scripts/setup.sh\n\
cargo build --release -p compare-demosaic\n\
target/release/compare-demosaic run [--artifacts] [--jobs N]\n\
target/release/compare-demosaic diff <old.json> <new.json> [--fail-on regression|any|never]\n\
target/release/compare-demosaic bless <canonical.json> --yes\n\
```\n\n\
The default run writes `target/compare-demosaic/canonical.json`. Failed rows\n\
and `--artifacts` write log-scaled error maps and tile summaries under\n\
`target/compare-demosaic/artifacts/`. A filtered selection must pass\n\
`--shard NAME`, writes `NAME.shard.json`, and is never valid as a baseline.\n\
Use `merge` to combine shards into `canonical.json`.\n\n\
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
        results.meta.stable.references.libraw.version,
        results.meta.stable.references.darktable.version,
        results.meta.stable.references.darktable.commit,
        concat!(
            "- `✅`: every compared sample is bit-identical or every robust limit passes\n",
            "  without high-error outliers.\n",
            "- `🟢`: differences are one code upward because Fornax rounds to nearest while\n",
            "  LibRaw truncates.\n",
            "- `⚠️`: every robust limit passes, but isolated high-error samples remain.\n",
            "- `❌`: at least one limit in `failed_limits` failed.\n",
        ),
    ));
    output.push_str("<!-- END GENERATED -->\n");
    output
}

fn replace_generated(actual: &str, rendered: &str) -> String {
    let Some(begin) = actual.find(GENERATED_BEGIN) else {
        return rendered.to_owned();
    };
    let Some(relative_end) = actual[begin..].find(GENERATED_END) else {
        return rendered.to_owned();
    };
    let end = begin + relative_end + GENERATED_END.len();
    if begin == 0 && actual[end..].trim().is_empty() {
        return rendered.to_owned();
    }
    let mut output = String::new();
    output.push_str(&actual[..begin]);
    output.push_str(rendered);
    output.push_str(&actual[end..]);
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
