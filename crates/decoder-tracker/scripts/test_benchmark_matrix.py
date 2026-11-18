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
                "cargo-options": "--no-default-features",
                "label": "CR3: Canon/EOS R5/photo.CR3",
                "size": 10,
                "sha256": SHA_A,
            },
            {
                "path": "Nikon/D850/photo.NEF",
                "format": "NEF",
                "cargo-options": "--no-default-features",
                "label": "NEF: Nikon/D850/photo.NEF",
                "size": 20,
                "sha256": SHA_B,
            },
        ]
    }


def test_filters_to_requested_formats_without_format_features():
    rows = benchmark_matrix.load_manifest(
        io.StringIO(f"path,size,sha256\nAdobe/photo.DNG,10,{SHA_A}\nSony/photo.ARW,20,{SHA_B}\n")
    )

    matrix = benchmark_matrix.build_matrix(rows, benchmark_matrix.parse_formats("arw"))

    assert matrix["include"] == [
        {
            "path": "Sony/photo.ARW",
            "format": "ARW",
            "cargo-options": "--no-default-features",
            "label": "ARW: Sony/photo.ARW",
            "size": 20,
            "sha256": SHA_B,
        }
    ]


def test_accepts_extended_golden_metadata_columns():
    rows = benchmark_matrix.load_manifest(
        io.StringIO(
            f"path,size,sha256,decoder_path,reason,overhead_only\n"
            f"Canon/EOS R5/photo.CR3,10,{SHA_A},CR3.crx,large CRX,0\n"
        )
    )

    assert rows == [{"path": "Canon/EOS R5/photo.CR3", "size": 10, "sha256": SHA_A}]


def test_filters_performance_groups():
    rows = benchmark_matrix.load_manifest(
        io.StringIO(
            f"path,size,sha256,decoder_path,reason,overhead_only,group\n"
            f"Canon/EOS R5/core.CR3,10,{SHA_A},CR3.crx,core,0,core\n"
            f"Canon/EOS R6/extra.CR3,20,{SHA_B},CR3.crx,extra,0,full\n"
        )
    )

    matrix = benchmark_matrix.build_matrix(rows, groups=benchmark_matrix.parse_groups("full"))

    assert [item["path"] for item in matrix["include"]] == ["Canon/EOS R6/extra.CR3"]


def test_rejects_duplicate_paths():
    with pytest.raises(ValueError, match="duplicate path"):
        benchmark_matrix.load_manifest(
            io.StringIO(f"path,size,sha256\nCanon/EOS R5/photo.CR3,10,{SHA_A}\nCanon/EOS R5/photo.CR3,20,{SHA_B}\n")
        )


def test_rejects_noncanonical_headers():
    with pytest.raises(ValueError, match="path,size,sha256"):
        benchmark_matrix.load_manifest(io.StringIO("path,sha256\nfile.CR3,a\n"))
