import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import tracker_data

SHA_A = "a" * 64
SHA_B = "b" * 64
SHA_C = "c" * 64
SHA_D = "d" * 64


class TrackerDataTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)

    def tearDown(self):
        self.temp.cleanup()

    def write_manifest(self, path):
        path.write_text(
            "path,size,sha256\n"
            f"Canon/Cam/a.DNG,10,{SHA_A}\n"
            f"Sony/Cam/a.ARW,20,{SHA_B}\n"
            f"Canon/Nested/b.DNG,30,{SHA_C}\n"
            f"Canon/Cam/c.DNG,40,{SHA_D}\n",
            encoding="utf-8",
        )

    def write_baseline(self, path, width=10):
        path.write_text(
            f"{','.join(tracker_data.BASELINE_HEADER)}\n"
            "✅,Canon Cam,DNG,Canon/Cam/a.DNG,1,1,"
            f"{SHA_A},1,1,,{width},10,RGGB,7,lossless-jpeg,1,"
            f"strips,14,integer,1,10,,\n",
            encoding="utf-8",
        )

    def test_select_filters_and_shards(self):
        manifest = self.root / "manifest.csv"
        self.write_manifest(manifest)
        selected = self.root / "selected.csv"
        selected_count = tracker_data.select_data(
            manifest,
            selected,
            ["canon"],
            [],
            ["Canon/**/*.DNG"],
            None,
        )
        self.assertEqual(selected_count, 3)
        self.assertIn("Canon/Cam/c.DNG", selected.read_text(encoding="utf-8"))

        shards = []
        for index in range(2):
            shard = self.root / f"shard-{index}.csv"
            tracker_data.select_data(
                manifest,
                shard,
                [],
                [],
                [],
                (index, 2),
            )
            shards.append(shard.read_text(encoding="utf-8").splitlines()[1:])
        self.assertEqual((len(shards[0]), len(shards[1])), (2, 2))
        self.assertTrue(set(shards[0]).isdisjoint(shards[1]))

    def test_merge_diff_and_update(self):
        expected = self.root / "selected.csv"
        expected.write_text(
            f"path,size,sha256\nCanon/Cam/a.DNG,10,{SHA_A}\n",
            encoding="utf-8",
        )
        shard = self.root / "shard.csv"
        self.write_baseline(shard)
        merged = self.root / "merged.csv"
        tracker_data.merge_data(expected, [shard], merged)
        self.assertEqual(
            merged.read_text(encoding="utf-8"),
            shard.read_text(encoding="utf-8"),
        )

        previous = self.root / "baseline.csv"
        candidate = self.root / "candidate.csv"
        self.write_baseline(previous)
        self.write_baseline(candidate)
        diff = self.root / "diff.csv"
        self.assertEqual(
            tracker_data.diff_data(
                expected,
                previous,
                candidate,
                diff,
            ),
            (0, 0, 0),
        )
        self.assertEqual(len(diff.read_text(encoding="utf-8").splitlines()), 1)

        updated = self.root / "updated.csv"
        tracker_data.update_data(previous, candidate, updated)
        self.assertEqual(
            updated.read_text(encoding="utf-8"),
            previous.read_text(encoding="utf-8"),
        )

    def test_merge_rejects_duplicate_rows(self):
        expected = self.root / "selected.csv"
        expected.write_text(
            f"path,size,sha256\nCanon/Cam/a.DNG,10,{SHA_A}\n",
            encoding="utf-8",
        )
        first = self.root / "first.csv"
        second = self.root / "second.csv"
        self.write_baseline(first)
        self.write_baseline(second)
        with self.assertRaisesRegex(tracker_data.TrackerDataError, "duplicate"):
            tracker_data.merge_data(
                expected,
                [first, second],
                self.root / "merged.csv",
            )

    def test_aggregate_uses_shared_python_operations(self):
        selected = self.root / "selected.csv"
        selected.write_text(
            f"path,size,sha256\nCanon/Cam/a.DNG,10,{SHA_A}\n",
            encoding="utf-8",
        )
        baseline = self.root / "baseline.csv"
        shard = self.root / "shard.csv"
        self.write_baseline(baseline, width=9)
        self.write_baseline(shard)
        work_dir = self.root / "work"
        output_file = self.root / "github-output"

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
        self.assertIn("changed=true", result.stdout)
        self.assertIn("changed=true", output_file.read_text(encoding="utf-8"))
        self.assertTrue((work_dir / "candidate.csv").is_file())
        self.assertTrue((work_dir / "diff.csv").is_file())
        self.assertTrue((work_dir / "baseline.csv").is_file())


if __name__ == "__main__":
    unittest.main()
