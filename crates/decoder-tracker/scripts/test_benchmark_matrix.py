import io
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))

import benchmark_matrix
import pytest

SHA_A = "a" * 64
SHA_B = "b" * 64


def test_builds_one_item_per_golden_row():
    rows = benchmark_matrix.load_manifest(
        io.StringIO(f"path,size,sha256\nCanon/EOS R5/photo.CR3,10,{SHA_A}\nNikon/D850/photo.NEF,20,{SHA_B}\n")
    )

    assert benchmark_matrix.build_matrix(rows) == {
        "include": [
            {
                "path": "Canon/EOS R5/photo.CR3",
                "format": "CR3",
                "feature": "canon",
                "cargo-options": "--no-default-features --features canon",
                "label": "CR3: Canon/EOS R5/photo.CR3",
                "size": 10,
                "sha256": SHA_A,
            },
            {
                "path": "Nikon/D850/photo.NEF",
                "format": "NEF",
                "feature": "nikon",
                "cargo-options": "--no-default-features --features nikon",
                "label": "NEF: Nikon/D850/photo.NEF",
                "size": 20,
                "sha256": SHA_B,
            },
        ]
    }


def test_filters_to_requested_formats_and_maps_dng_to_no_feature():
    rows = benchmark_matrix.load_manifest(
        io.StringIO(f"path,size,sha256\nAdobe/photo.DNG,10,{SHA_A}\nSony/photo.ARW,20,{SHA_B}\n")
    )

    matrix = benchmark_matrix.build_matrix(rows, benchmark_matrix.parse_formats("arw"))

    assert matrix["include"] == [
        {
            "path": "Sony/photo.ARW",
            "format": "ARW",
            "feature": "sony",
            "cargo-options": "--no-default-features --features sony",
            "label": "ARW: Sony/photo.ARW",
            "size": 20,
            "sha256": SHA_B,
        }
    ]
    assert benchmark_matrix.FEATURES["DNG"] == ""


def test_rejects_duplicate_paths():
    with pytest.raises(ValueError, match="duplicate path"):
        benchmark_matrix.load_manifest(
            io.StringIO(f"path,size,sha256\nCanon/EOS R5/photo.CR3,10,{SHA_A}\nCanon/EOS R5/photo.CR3,20,{SHA_B}\n")
        )


def test_rejects_noncanonical_headers():
    with pytest.raises(ValueError, match="path,size,sha256"):
        benchmark_matrix.load_manifest(io.StringIO("path,sha256\nfile.CR3,a\n"))
