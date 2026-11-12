import io
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).parent))

import benchmark_matrix


SHA_A = "a" * 64
SHA_B = "b" * 64


class BenchmarkMatrixTests(unittest.TestCase):
    def test_builds_one_item_per_golden_row(self):
        rows = benchmark_matrix.load_manifest(
            io.StringIO(
                "path,size,sha256\n"
                f"Canon/EOS R5/photo.CR3,10,{SHA_A}\n"
                f"Nikon/D850/photo.NEF,20,{SHA_B}\n"
            )
        )

        self.assertEqual(
            benchmark_matrix.build_matrix(rows),
            {
                "include": [
                    {
                        "path": "Canon/EOS R5/photo.CR3",
                        "format": "CR3",
                        "label": "CR3: Canon/EOS R5/photo.CR3",
                        "size": 10,
                        "sha256": SHA_A,
                    },
                    {
                        "path": "Nikon/D850/photo.NEF",
                        "format": "NEF",
                        "label": "NEF: Nikon/D850/photo.NEF",
                        "size": 20,
                        "sha256": SHA_B,
                    },
                ]
            },
        )

    def test_rejects_duplicate_paths(self):
        with self.assertRaisesRegex(ValueError, "duplicate path"):
            benchmark_matrix.load_manifest(
                io.StringIO(
                    "path,size,sha256\n"
                    f"Canon/EOS R5/photo.CR3,10,{SHA_A}\n"
                    f"Canon/EOS R5/photo.CR3,20,{SHA_B}\n"
                )
            )

    def test_rejects_noncanonical_headers(self):
        with self.assertRaisesRegex(ValueError, "path,size,sha256"):
            benchmark_matrix.load_manifest(io.StringIO("path,sha256\nfile.CR3,a\n"))


if __name__ == "__main__":
    unittest.main()
