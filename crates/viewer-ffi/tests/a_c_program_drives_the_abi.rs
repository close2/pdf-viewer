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

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A one-page document with §12.7's two commonest fields, written out beside the test binary.
///
/// **The form half of this gate needs a form, and the application note has none.** The other
/// candidate was a corpus document — `issue17492.pdf` is what `viewer-gtk` and `viewer-qt` were
/// driven against — and it is refused for trap 8's reason turned around: `doc/pdf.js` is optional
/// in a checkout, and a gate that skipped when it is absent would be a gate that quietly stops
/// running. Eleven objects of hand-written PDF cost nothing and are always there.
///
/// The check box carries `/AP /N << /Yes … /Off … >>` because that is the whole point of the
/// exercise: §12.7.5.2.3 makes `/V` "a name object representing the check box's appearance state",
/// and `Yes` is the *file's* invention — a C caller has to be handed it by
/// `pdfv_field_widget_text`, and a guess would tick nothing.
fn form_fixture() -> Vec<u8> {
    let appearance = |colour: &str| {
        let contents = format!("{colour} 0 0 20 20 re f");
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length {} >>\nstream\n\
             {contents}\nendstream",
            contents.len().saturating_add(1)
        )
    };
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R 6 0 R] \
         /DA (/Helv 0 Tf 0 g) >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> \
         /Contents 4 0 R /Annots [5 0 R 6 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Tx /T (typed) /V (hello) \
         /Rect [10 10 90 30] /F 4 >>\nendobj\n\
         6 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Btn /T (ticked) /Rect [110 10 130 30] \
         /F 4 /AS /Off /AP << /N << /Yes 7 0 R /Off 8 0 R >> >> >>\nendobj\n\
         7 0 obj\n{}\nendobj\n\
         8 0 obj\n{}\nendobj\n",
        appearance("0 0 1 rg"),
        appearance("1 1 1 rg"),
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

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
    let form = artefacts.join("pdfv_form_fixture.pdf");
    std::fs::write(&form, form_fixture()).expect("the fixture is written beside the test binary");
    let ran = Command::new(&program)
        .arg(&document)
        .arg(&form)
        .output()
        .expect("the program runs");
    let said = String::from_utf8_lossy(&ran.stdout).into_owned();
    println!("{said}");
    assert!(
        ran.status.success(),
        "the C program failed:\n{}",
        String::from_utf8_lossy(&ran.stderr)
    );

    what_it_printed(&said);
}

/// The numbers the C program printed, checked here rather than in the C.
///
/// So that a change to the ABI that still *runs* cannot pass by printing something else. The
/// application note is five pages, its outline is fourteen rows, its third page is where a search
/// for "black point" lands, and the page after it draws.
fn what_it_printed(said: &str) {
    for expected in [
        "abi 1 (header 1), 19 event kind(s) (header 19)",
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
        // What the five-hundred-and-eleventh session added, each line read off the library. The
        // two counted enumerations, the name it gives a number it does not define, and the refusal
        // an enumeration this ABI *takes* answers with are the whole of what C has in place of a
        // build failure — so they are asserted rather than printed.
        "control kinds 8 (header 8), row kinds 4 (header 4), unknown is unknown",
        "an undefined pointer action: the message at that index is not of the kind this accessor \
         reads",
        // The note states no optional content and no `/EmbeddedFiles` tree. Both answer an
        // **empty list** rather than `PDFV_NO_ANSWER`, which is `viewer-core`'s existing choice
        // and worth pinning here rather than assuming: a document with no layers has answered the
        // question, and the two would be the same picture in a panel and different sentences in a
        // status bar.
        // Where the reader is looking, taken away and put back (ADR 0737). **The values are
        // asserted as well as the verdict**, and that is not belt and braces: "exactly" compares
        // one answer with another, so an accessor that reported the same wrong number twice would
        // satisfy it. The numbers below are the note's own first page at 2.5 logical pixels per
        // user unit in an 800x1000 window — larger than the window on both axes, so a scroll of
        // (40, 120) is clamped to the page's own corner and the offset is not something a caller
        // could have asked for.
        "view: page 0, zoom 3 at 2.500, scroll 385.1,672.4",
        "view restored: exactly",
        "layers: 0 row(s)",
        "attachments: 0 row(s)",
        // §12.7's form, on the fixture beside the test binary. Two fields, the check box ticked
        // with the name the library handed over, and the edit read back rather than assumed.
        "form: 2 field(s)",
        "[0] Entry flags 0",
        "ticking ticked with the state Yes",
        "after the edit: 1 widget(s) on",
        "dirty after the edit: 1",
        // **The C program saves between the two**, so the undo takes back an edit the file
        // already holds and the document is unsaved again. This read 0 until the
        // five-hundred-and-twenty-fifth session, when `Open::dirty` stopped meaning "the log is
        // not empty" and started meaning "the cursor is not where the last save left it" — the
        // old answer was the same for a document saved and one never saved at all.
        "dirty after the undo: 1",
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
