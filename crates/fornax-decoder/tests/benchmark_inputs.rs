#[path = "../benches/decoder_inputs.rs"]
mod decoder_inputs;

use std::fs;
use std::path::{Path, PathBuf};

use decoder_inputs::{
    GoldenInput, ensure_materialized, parse_golden_manifest, select_golden_inputs,
};

const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

struct TempTree {
    root: PathBuf,
}

impl TempTree {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "fornax-benchmark-inputs-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn path(&self) -> &Path { &self.root }

    fn write(&self, relative: &str, contents: &[u8]) -> PathBuf {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for TempTree {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.root); }
}

fn manifest() -> Vec<u8> {
    format!(
        "path,size,sha256\n\
         Canon/EOS R5/photo.CR3,10,{SHA_A}\n\
         Nikon/D850/photo.NEF,20,{SHA_B}\n"
    )
    .into_bytes()
}

#[test]
fn parses_canonical_manifest_and_derives_case_labels() {
    let rows = parse_golden_manifest(&manifest()).unwrap();
    assert_eq!(
        rows,
        vec![
            GoldenInput {
                path: "Canon/EOS R5/photo.CR3".to_owned(),
                size: 10,
                sha256: SHA_A.to_owned(),
            },
            GoldenInput {
                path: "Nikon/D850/photo.NEF".to_owned(),
                size: 20,
                sha256: SHA_B.to_owned(),
            },
        ]
    );
    assert_eq!(rows[0].format(), "CR3");
    assert_eq!(rows[1].format(), "NEF");
    assert_eq!(rows[0].case_label(), "CR3__Canon--EOS_R5--photo.CR3");
}

#[test]
fn rejects_duplicate_golden_paths() {
    let csv = format!(
        "path,size,sha256\n\
         Canon/EOS R5/photo.CR3,10,{SHA_A}\n\
         Canon/EOS R5/photo.CR3,20,{SHA_B}\n"
    );
    let error = parse_golden_manifest(csv.as_bytes()).unwrap_err();
    assert!(error.contains("duplicate path"), "{error}");
}

#[test]
fn rejects_invalid_sha256_values() {
    let csv = "path,size,sha256\nCanon/EOS R5/photo.CR3,10,not-a-sha\n";
    let error = parse_golden_manifest(csv.as_bytes()).unwrap_err();
    assert!(error.contains("SHA-256"), "{error}");
}

#[test]
fn selects_all_golden_inputs_when_no_path_is_requested() {
    let tree = TempTree::new("select-all");
    tree.write("corpus/Canon/EOS R5/photo.CR3", b"canon");
    tree.write("corpus/Nikon/D850/photo.NEF", b"nikon");
    let rows = parse_golden_manifest(&manifest()).unwrap();
    let selected = select_golden_inputs(&rows, None, &tree.path().join("corpus")).unwrap();
    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].0.path, rows[0].path);
    assert_eq!(selected[1].0.path, rows[1].path);
}

#[test]
fn selects_the_exact_requested_golden_input() {
    let tree = TempTree::new("select-one");
    tree.write("corpus/Canon/EOS R5/photo.CR3", b"canon");
    tree.write("corpus/Nikon/D850/photo.NEF", b"nikon");
    let rows = parse_golden_manifest(&manifest()).unwrap();
    let requested = tree.path().join("corpus/Nikon/D850/photo.NEF");
    let selected =
        select_golden_inputs(&rows, Some(&requested), &tree.path().join("corpus")).unwrap();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].0.path, rows[1].path);
}

#[test]
fn rejects_an_explicit_path_outside_the_golden_manifest() {
    let tree = TempTree::new("outside");
    tree.write("corpus/Canon/EOS R5/photo.CR3", b"canon");
    tree.write("corpus/Nikon/D850/photo.NEF", b"nikon");
    let requested = tree.write("outside.CR3", b"outside");
    let rows = parse_golden_manifest(&manifest()).unwrap();
    let error =
        select_golden_inputs(&rows, Some(&requested), &tree.path().join("corpus")).unwrap_err();
    assert!(error.contains("not present in the golden manifest"), "{error}");
}

#[test]
fn rejects_an_unmaterialized_lfs_pointer() {
    let tree = TempTree::new("pointer");
    let pointer = tree.write(
        "corpus/Canon/EOS R5/photo.CR3",
        b"version https://git-lfs.github.com/spec/v1\noid sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nsize 10\n",
    );
    let rows = parse_golden_manifest(&manifest()).unwrap();
    let error = ensure_materialized(&[(rows[0].clone(), pointer)]).unwrap_err();
    assert!(error.contains("LFS pointer"), "{error}");
}
