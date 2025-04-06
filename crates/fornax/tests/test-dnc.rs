use assert_cmd::Command;
#[test]
fn test_wgs84_to_gcj02() {
    Command::cargo_bin("examples/dnc").unwrap().assert().success();
}
