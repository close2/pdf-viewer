//! Local and CI platform checks cover the binaries Cargo says the snapshots ship.
//! The audit calibrates missing packages, targets and libraries against copies of those inputs.

use std::path::Path;
use std::process::Command;

#[test]
fn the_platform_checks_cover_the_shipped_surfaces() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the conformance crate is two directories below the repository root");
    let output = Command::new("python3")
        .args(["-I", "-B"])
        .arg(root.join("tools/platform-check-audit.py"))
        .output()
        .expect("the platform coverage audit starts");
    assert!(
        output.status.success(),
        "platform coverage audit failed ({}):\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("six calibrated omissions rejected"),
        "the audit must finish its calibration: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}
