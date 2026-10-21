//! The Windows host's C# must compile against its actual SDK references before a host change
//! reaches CI. The XAML compiler and packaging still run on Windows; this check substitutes
//! only XAML's generated fields and initialization methods, never an SDK type.

#![expect(
    clippy::print_stdout,
    reason = "a compiler check must fail if it cannot run, and an absent optional SDK is reported"
)]

use std::path::Path;
use std::process::Command;

#[test]
fn the_windows_csharp_compiles_when_the_sdk_is_available() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the conformance crate is two directories below the repository root");
    let script = root.join("tools/dotnet-check.sh");
    let probe = Command::new("bash")
        .arg(&script)
        .arg("--probe")
        .output()
        .expect("the SDK availability check starts");
    if probe.status.code() == Some(69) {
        println!(
            "C# check skipped: {}",
            String::from_utf8_lossy(&probe.stderr)
        );
        return;
    }
    assert!(
        probe.status.success(),
        "SDK discovery failed: {}",
        String::from_utf8_lossy(&probe.stderr)
    );
    let compiled = Command::new("bash")
        .arg(script)
        .output()
        .expect("the C# compiler check starts");
    assert!(
        compiled.status.success(),
        "C# compile failed ({}):\n{}\n{}",
        compiled.status,
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    println!("{}", String::from_utf8_lossy(&compiled.stdout));
}
