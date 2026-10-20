//! `quorra-gtk`'s launch on both lanes, each figure held to its band in
//! [`doc/checks/toolkit-launch.toml`](../../../doc/checks/toolkit-launch.toml) (ADR 2125).
//!
//! The harness is shared with the other toolkit window and lives beside `viewer-host`'s tests,
//! whose module comment says what each figure is, how it is taken and how to run it; this file
//! names the window and the trace's words for its first size.

#[path = "../../viewer-host/tests/toolkit_launch/mod.rs"]
mod toolkit_launch;

/// GTK's launch, device and `--cpu`, against its bands.
#[test]
#[ignore = "a clock gate: run alone under tools/bounded.sh --clock, with Xvfb installed (ADR 2125)"]
fn the_launch_is_inside_its_bands() {
    let window = toolkit_launch::Window {
        name: "quorra-gtk",
        binary: env!("CARGO_BIN_EXE_quorra-gtk"),
        sized: "first allocation",
    };
    if let Err(said) =
        toolkit_launch::require_the_sandbox().and_then(|()| toolkit_launch::hold(&window))
    {
        panic!("{said}");
    }
}
