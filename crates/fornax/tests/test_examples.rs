
use assert_cmd::Command;
#[test]
fn test_libraw_dalim() {
    let root = fornax_devtool::root_dir();
    let exe = fornax_devtool::get_example_exe("libraw_dalim");
    Command::new(exe).current_dir(root).assert().success();
}

#[test]
fn test_libraw_libraw() {
    let root = fornax_devtool::root_dir();
    let exe = fornax_devtool::get_example_exe("libraw_libraw");
    Command::new(exe).current_dir(root).assert().success();
}
