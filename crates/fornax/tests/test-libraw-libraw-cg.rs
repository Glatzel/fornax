use std::path::PathBuf;

use assert_cmd::Command;
#[test]
fn test_libraw_process_cg() {
    let root = PathBuf::from(std::env::var("CARGO_WORKSPACE_DIR").unwrap());
    let mut exe = root.clone();
    exe.push("target/llvm-cov-target/debug/examples/libraw-libraw-cg.exe");
    Command::new(exe).current_dir(root).assert().success();
}
