use std::fs;

use dircpy::copy_dir;
use tempfile::tempdir;
use try_v2_xtasks::commands::fmt;

#[test]
fn fmt_fixture() {
    let tmp = tempdir().expect("couldn't create temp dir for test");
    copy_dir("tests/fixture", tmp.path()).expect("couldn't copy fixture");
    let original = fs::read_to_string("tests/fixture/src/lib.rs").unwrap();
    let copied = fs::read_to_string(tmp.path().join("src/lib.rs")).unwrap();
    assert_eq!(original, copied);
    let cmd = fmt(tmp.path());
    let output = cmd.result.expect("`cargo fmt` failed to run");
    assert!(
        output.status.success(),
        "`cargo fmt` exited with status {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
    let formatted = fs::read_to_string(tmp.path().join("src/lib.rs")).unwrap();
    assert_ne!(original, formatted);
    dbg!(tmp.path());
}
