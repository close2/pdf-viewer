//! The ABI, compiled and run from C.
//!
//! **The two Rust hosts prove the vocabulary; only a C program proves the ABI.** Everything else
//! in this crate is Rust calling Rust — the entry points are `extern "C"` and the argument types
//! are C's, but no C compiler has read the header and no linker has resolved the symbols. This
//! test is what closes that: `cc` compiles `c/open_a_page.c` against `include/pdf_viewer.h`, links
//! it against the `cdylib`, and runs it on a real document.
//!
//! What it therefore catches that nothing else does: a declaration in the header that does not
//! match the Rust signature by *name* (the linker says so), a struct tag colliding with a function
//! in C's one namespace (the compiler says so — and it did, which is why the frame struct is
//! called `pdfv_frame`), and a caller freeing a handle the library still owns (the allocator says
//! so, loudly, on exit).
//!
//! **Skipped rather than failed where there is no C compiler**, in the shape the rest of this tree
//! uses for a corpus that is not checked out: a machine without `cc` is a machine this test cannot
//! run on, and pretending otherwise would make the gate a coin toss. CI has one.

#![allow(
    clippy::expect_used,
    reason = "test code: a step that cannot be performed must fail loudly rather than pass by \
              doing nothing"
)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// Where the workspace's build output is, found from this test binary rather than assumed.
///
/// `CARGO_TARGET_DIR` is set to a directory outside the tree on the machine this project is
/// developed on, so the path is taken from the executable that is running: a test binary lives in
/// `<target>/<profile>/deps/`, and the `cdylib` beside it is two levels up.
fn artefacts() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let deps = exe.parent()?;
    Some(deps.parent()?.to_path_buf())
}

/// The first of `cc` and `gcc` that answers `--version`.
fn compiler() -> Option<String> {
    let named = std::env::var("CC").ok();
    let candidates = named.iter().map(String::as_str).chain(["cc", "gcc"]);
    for candidate in candidates {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            return Some(candidate.to_owned());
        }
    }
    None
}

#[test]
fn a_c_program_opens_a_document_turns_a_page_asks_a_query_and_gets_pixels() {
    let Some(cc) = compiler() else {
        println!("skipped: no C compiler on this machine");
        return;
    };
    let Some(artefacts) = artefacts() else {
        println!("skipped: this test binary is not where cargo puts one");
        return;
    };

    // The `cdylib` is a separate artefact from the rlib this test links against, and `cargo test`
    // does not necessarily produce it. Asking cargo for it is cheap when it is already there and
    // is the only way to be sure it is: a test that skipped because the library was missing would
    // be a gate that quietly stops running.
    let built = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .args(["build", "-p", "viewer-ffi", "--lib"])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .status();
    assert!(
        built.is_ok_and(|status| status.success()),
        "the cdylib has to exist for a C program to link against"
    );

    let library = artefacts.join("libviewer_ffi.so");
    assert!(
        library.exists(),
        "no cdylib at {}: this test needs one",
        library.display()
    );

    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let program = artefacts.join("pdfv_open_a_page");
    let compiled = Command::new(&cc)
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-O2")
        .arg("-I")
        .arg(crate_root.join("include"))
        .arg(crate_root.join("c/open_a_page.c"))
        .arg("-o")
        .arg(&program)
        .arg("-L")
        .arg(&artefacts)
        .arg("-lviewer_ffi")
        // The library is not installed anywhere a loader looks, so the path it was linked against
        // is recorded in the program. That is a test fixture's answer and not a shipping one.
        .arg(format!("-Wl,-rpath,{}", artefacts.display()))
        .output()
        .expect("the compiler runs");
    assert!(
        compiled.status.success(),
        "{cc} refused the header or the program:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    // `-Werror` above means this is already empty, and saying so is what makes the assertion
    // above about *errors* rather than about noise.
    assert!(
        compiled.stderr.is_empty(),
        "the C compiler had something to say:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let document = crate_root.join("../../doc/PDF20_AN001-BPC.pdf");
    let ran = Command::new(&program)
        .arg(&document)
        .output()
        .expect("the program runs");
    let said = String::from_utf8_lossy(&ran.stdout).into_owned();
    println!("{said}");
    assert!(
        ran.status.success(),
        "the C program failed:\n{}",
        String::from_utf8_lossy(&ran.stderr)
    );

    // The numbers it printed, checked here rather than in the C, so that a change to the ABI that
    // still *runs* cannot pass by printing something else. The application note is five pages,
    // its outline is fourteen rows, its third page is where a search for "black point" lands, and
    // the page after it draws.
    for expected in [
        "abi 1 (header 1), 16 event kind(s) (header 16)",
        "Opened says document 1 has 5 page(s)",
        "page 1 of 5 (5 page(s) in the document)",
        "outline: 14 row(s)",
        // Annex O's `search`, pumped one page at a time by the C loop. Three steps for three
        // pages — the phrase is not on the first two — and the view moves to the page the
        // occurrence is on, which is the annex's "selecting the first matching word in the
        // document" as far as a C caller can see it.
        "search: found on page 3, bytes 63..74, after 3 step(s)",
        "after the search: page 3 of 5",
        "after the turn: page 4 of 5, drawn in ",
        "frame: page 4, 708x1000, format 0, 2832000 byte(s)",
        "ok",
    ] {
        assert!(
            said.contains(expected),
            "the C program did not say {expected:?}"
        );
    }
    // And the page is not blank, which is the one thing a count of bytes cannot say. The line the
    // count is read out of also carries a wall clock, which is why it is split on the semicolon
    // rather than matched whole: a number that moves between runs is not something to assert.
    let inked = said
        .lines()
        .find(|line| line.starts_with("copied 2832000 byte(s) in "))
        .and_then(|line| line.split("); ").nth(1))
        .and_then(|rest| rest.split(' ').next())
        .and_then(|count| count.parse::<usize>().ok())
        .expect("the program says how many pixels are not white");
    assert!(inked > 1000, "only {inked} pixels of the page are inked");
    // The three wall clocks are printed rather than asserted on — a machine's speed is not this
    // test's business — but that they were *taken* is: a C host measures its own time to first
    // page, because the viewer has no clock (rule 3).
    for measured in [
        "first page drawn and handed back at ",
        "after the turn: page 4 of 5, drawn in ",
    ] {
        assert!(
            said.contains(measured),
            "the C program did not measure {measured:?}"
        );
    }
}
