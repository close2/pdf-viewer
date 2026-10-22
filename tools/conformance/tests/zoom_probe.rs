//! Zoom evidence must reject stale, ambiguous and mismatched callback records.
//! This checks the parser without a display or access to the owner's GPU loop.

use std::path::Path;
use std::process::Command;

#[test]
fn zoom_receipts_reject_planted_association_failures() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the conformance crate is two directories below the repository root");
    let output = Command::new("python3")
        .args(["-I", "-B"])
        .arg(root.join("tools/zoom-probe-records.py"))
        .output()
        .expect("the zoom receipt self-test starts");
    assert!(
        output.status.success(),
        "zoom receipt self-test failed ({}):\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("zoom records: every case holds"),
        "the zoom receipt self-test must finish its controls"
    );
}

/// A successful process exit with an empty discovery population is not calibration.
#[test]
fn zoom_receipts_refuse_empty_calibration() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the conformance crate is two directories below the repository root");
    let output = Command::new("python3")
        .args(["-I", "-B", "-c"])
        .arg("import runpy, sys, unittest; unittest.defaultTestLoader.loadTestsFromTestCase = lambda _: unittest.TestSuite(); runpy.run_path(sys.argv[1], run_name='__main__')")
        .arg(root.join("tools/zoom-probe-records.py"))
        .output()
        .expect("the empty-discovery control starts");
    assert!(!output.status.success(), "empty discovery must fail");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("zoom receipt calibration requires all nine tests"),
        "the negative must fail at its intended calibration guard"
    );
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("every case holds"),
        "empty discovery must not print a successful calibration receipt"
    );
}
