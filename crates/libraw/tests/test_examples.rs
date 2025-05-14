use std::path::PathBuf;

use assert_cmd::Command;

#[test]
fn test_bayer_image() {
    let root = fornax_devtool::root_dir();
    let exe = fornax_devtool::get_example_exe("bayer_image");
    Command::new(exe).current_dir(root).assert().success();
}
#[test]
fn test_image_sizes() {
    let root = fornax_devtool::root_dir();
    let exe = fornax_devtool::get_example_exe("image_sizes");
    Command::new(exe).current_dir(root).assert().success();
}
#[test]
fn test_imgother() {
    let root = fornax_devtool::root_dir();
    let exe = fornax_devtool::get_example_exe("imgother");
    Command::new(exe).current_dir(root).assert().success();
}
#[test]
fn test_iparams() {
    let root = fornax_devtool::root_dir();
    let exe = fornax_devtool::get_example_exe("iparams");
    Command::new(exe).current_dir(root).assert().success();
}
#[test]
fn test_raw_image() {
    let root = fornax_devtool::root_dir();
    let exe = fornax_devtool::get_example_exe("raw_image");
    Command::new(exe).current_dir(root).assert().success();
}
#[test]
fn test_rawdata() {
    let root = fornax_devtool::root_dir();
    let exe = fornax_devtool::get_example_exe("rawdata");
    Command::new(exe).current_dir(root).assert().success();
}
