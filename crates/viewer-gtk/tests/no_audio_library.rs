//! `quorra-gtk` links no audio library: `libasound`, `libpulse` and `libjack` are not among the
//! shared libraries its dynamic section names (ADRs 3317, 3329).
//!
//! The GTK window plays §13.3's sound through the helper `quorra-sound`, started at the first
//! sound, which alone links an audio library; a window that named one would have the loader map it
//! at every start, and refuse to start on a machine without it, whether or not a document ever
//! plays a sound (`CLAUDE.md`'s startup rules). `quorra-sound`'s own `pipe` test reads the same
//! section of the helper and finds `libasound` there, so this reading is one that would see the
//! library if it came back.
//!
//! What is read is the program's own `NEEDED` entries, the libraries its link named: what the
//! toolkit's shared libraries name in turn, or load as modules, is the toolkit's. The binary read
//! is the test profile's; which libraries a program names is the linker's decision from the crates
//! linked into it, which no profile setting changes, so the release binary names the same ones.

#![cfg(target_os = "linux")]

/// The `NEEDED` entries of `program`'s dynamic section, as `readelf -d` prints them.
fn needed(program: &str) -> std::io::Result<Vec<String>> {
    let dynamic = std::process::Command::new("readelf")
        .args(["-d", program])
        .output()?;
    assert!(dynamic.status.success(), "readelf -d {program} failed");
    Ok(String::from_utf8_lossy(&dynamic.stdout)
        .lines()
        .filter(|line| line.contains("(NEEDED)"))
        .map(str::to_owned)
        .collect())
}

#[test]
fn quorra_gtk_names_no_audio_library_among_what_its_loader_maps() {
    let needed = needed(env!("CARGO_BIN_EXE_quorra-gtk")).expect("readelf runs (binutils)");
    // The reading found the section: every program here names the C library.
    assert!(
        needed.iter().any(|line| line.contains("libc.so")),
        "no NEEDED entries were read: {needed:?}"
    );
    let audio: Vec<&String> = needed
        .iter()
        .filter(|line| {
            line.contains("libasound") || line.contains("libpulse") || line.contains("libjack")
        })
        .collect();
    assert!(
        audio.is_empty(),
        "quorra-gtk links an audio library at load time, which only the sound helper may: {audio:?}"
    );
}
