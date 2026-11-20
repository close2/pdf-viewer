//! `quorra` links no audio library: `libasound` is not among the shared libraries its loader maps
//! before `main` (ADR 3317, `doc/questions/Q972`).
//!
//! `CLAUDE.md`'s startup rules say nothing page one does not need is done at launch, and a window
//! that names `libasound.so.2` in its dynamic section has the loader map and relocate it at every
//! start — and cannot start at all on a machine without it — whether or not a document ever plays a
//! sound. The sound is played by the helper `quorra-sound`, started at the first sound, which is
//! where the library is linked; `quorra-sound`'s own `pipe` test reads the same section there and
//! finds it, so this reading is one that would see the library if it came back.
//!
//! The binary read is the test profile's `quorra`. Which shared libraries a program names is the
//! linker's decision from the crates linked into it, which no profile setting changes, so the
//! release binary names the same ones.

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
fn quorra_names_no_audio_library_among_what_its_loader_maps() {
    let needed = needed(env!("CARGO_BIN_EXE_quorra")).expect("readelf runs (binutils)");
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
        "quorra links an audio library at load time, which only the sound helper may: {audio:?}"
    );
}
