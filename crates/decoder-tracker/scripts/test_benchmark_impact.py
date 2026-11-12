import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))

import benchmark_matrix


def test_direct_vendor_changes_select_only_that_vendor():
    assert benchmark_matrix.impacted_formats(["crates/fornax-raw-sony/src/codec/mod.rs"]) == frozenset({"ARW"})
    assert benchmark_matrix.impacted_formats(["crates/fornax-raw-panasonic/src/codec/v8.rs"]) == frozenset({"RW2"})


def test_bmff_selects_canon_and_codec_selects_everything():
    assert benchmark_matrix.impacted_formats(["crates/fornax-bmff/src/lib.rs"]) == frozenset({"CR3", "CRM"})
    assert benchmark_matrix.impacted_formats(["crates/fornax-codec/src/lib.rs"]) == (benchmark_matrix.ALL_FORMATS)


def test_shared_benchmark_and_manifest_changes_select_everything():
    for path in (
        "crates/fornax-raw-decoder/benches/decoder.rs",
        "crates/fornax-tiff/src/raster.rs",
        "Cargo.toml",
        "crates/decoder-tracker/golden/selected-manifest.csv",
        ".github/workflows/ci.yml",
    ):
        assert benchmark_matrix.impacted_formats([path]) == benchmark_matrix.ALL_FORMATS
