import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent))

import tracker_data

SHA_A = "a" * 64
SHA_B = "b" * 64
SHA_C = "c" * 64
SHA_D = "d" * 64


def write_manifest(path):
    path.write_text(
        "path,size,sha256\n"
        f"Canon/Cam/a.DNG,10,{SHA_A}\n"
        f"Sony/Cam/a.ARW,20,{SHA_B}\n"
        f"Canon/Nested/b.DNG,30,{SHA_C}\n"
        f"Canon/Cam/c.DNG,40,{SHA_D}\n",
        encoding="utf-8",
    )


def write_baseline(path, width=10):
    path.write_text(
        f"{','.join(tracker_data.BASELINE_HEADER)}\n"
        "✅,Canon Cam,DNG,Canon/Cam/a.DNG,1,1,"
        f"{SHA_A},1,1,,{width},10,RGGB,7,lossless-jpeg,1,"
        f"strips,14,integer,1,10,,\n",
        encoding="utf-8",
    )


def test_select_filters_and_shards(tmp_path):
    manifest = tmp_path / "manifest.csv"
    write_manifest(manifest)
    selected = tmp_path / "selected.csv"
    selected_count = tracker_data.select_data(
        manifest,
        selected,
        ["canon"],
        [],
        ["Canon/**/*.DNG"],
        None,
    )
    assert selected_count == 3
    assert "Canon/Cam/c.DNG" in selected.read_text(encoding="utf-8")

    shards = []
    for index in range(2):
        shard = tmp_path / f"shard-{index}.csv"
        tracker_data.select_data(
            manifest,
            shard,
            [],
            [],
            [],
            (index, 2),
        )
        shards.append(shard.read_text(encoding="utf-8").splitlines()[1:])
    assert (len(shards[0]), len(shards[1])) == (2, 2)
    assert set(shards[0]).isdisjoint(shards[1])


def test_merge_diff_and_update(tmp_path):
    expected = tmp_path / "selected.csv"
    expected.write_text(
        f"path,size,sha256\nCanon/Cam/a.DNG,10,{SHA_A}\n",
        encoding="utf-8",
    )
    shard = tmp_path / "shard.csv"
    write_baseline(shard)
    merged = tmp_path / "merged.csv"
    tracker_data.merge_data(expected, [shard], merged)
    assert merged.read_text(encoding="utf-8") == shard.read_text(encoding="utf-8")

    previous = tmp_path / "baseline.csv"
    candidate = tmp_path / "candidate.csv"
    write_baseline(previous)
    write_baseline(candidate)
    diff = tmp_path / "diff.csv"
    assert tracker_data.diff_data(expected, previous, candidate, diff) == (0, 0, 0)
    assert len(diff.read_text(encoding="utf-8").splitlines()) == 1

    updated = tmp_path / "updated.csv"
    tracker_data.update_data(previous, candidate, updated)
    assert updated.read_text(encoding="utf-8") == previous.read_text(encoding="utf-8")


def test_merge_rejects_duplicate_rows(tmp_path):
    expected = tmp_path / "selected.csv"
    expected.write_text(
        f"path,size,sha256\nCanon/Cam/a.DNG,10,{SHA_A}\n",
        encoding="utf-8",
    )
    first = tmp_path / "first.csv"
    second = tmp_path / "second.csv"
    write_baseline(first)
    write_baseline(second)
    with pytest.raises(tracker_data.TrackerDataError, match="duplicate"):
        tracker_data.merge_data(expected, [first, second], tmp_path / "merged.csv")


def test_aggregate_uses_shared_python_operations(tmp_path):
    selected = tmp_path / "selected.csv"
    selected.write_text(
        f"path,size,sha256\nCanon/Cam/a.DNG,10,{SHA_A}\n",
        encoding="utf-8",
    )
    baseline = tmp_path / "baseline.csv"
    shard = tmp_path / "shard.csv"
    write_baseline(baseline, width=9)
    write_baseline(shard)
    work_dir = tmp_path / "work"
    output_file = tmp_path / "github-output"

    result = subprocess.run(
        [
            "bash",
            str(tracker_data.SCRIPT_DIR / "aggregate.sh"),
            "--selected",
            str(selected),
            "--baseline",
            str(baseline),
            "--corpus-manifest",
            str(selected),
            "--work-dir",
            str(work_dir),
            "--github-output",
            str(output_file),
            "--",
            str(shard),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    assert "changed=true" in result.stdout
    assert "changed=true" in output_file.read_text(encoding="utf-8")
    assert (work_dir / "candidate.csv").is_file()
    assert (work_dir / "diff.csv").is_file()
    assert (work_dir / "baseline.csv").is_file()
