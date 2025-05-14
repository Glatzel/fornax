use std::path::PathBuf;

use assert_cmd::Command;
#[test]
fn test_dnc() {
    let root = fornax_devtool::root_dir();
    let exe = fornax_devtool::get_example_exe("dnc");
    Command::new(exe).current_dir(root).assert().success();
}
