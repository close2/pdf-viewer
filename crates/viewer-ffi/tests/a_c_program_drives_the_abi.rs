//! The ABI, compiled and run from C.
//!
//! **The two Rust hosts prove the vocabulary; only a C program proves the ABI.** Everything else
//! in this crate is Rust calling Rust — the entry points are `extern "C"` and the argument types
//! are C's, but no C compiler has read the header and no linker has resolved the symbols. This
//! test is what closes that: `cc` compiles `c/open_a_page.c` against `include/quorra.h`, links
//! it against the `cdylib`, and runs it on a real document.
//!
//! What it therefore catches that nothing else does: a declaration in the header that does not
//! match the Rust signature by *name* (the linker says so), a struct tag colliding with a function
//! in C's one namespace (the compiler says so — and it did, which is why the frame struct is
//! called `quorra_frame`), and a caller freeing a handle the library still owns (the allocator says
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

/// A one-page document with §12.7's two commonest fields and a push-button whose §12.7.6.2
/// submit-form action the keyboard presses, written out beside the test binary.
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
/// `quorra_field_widget_text`, and a guess would tick nothing.
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
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R 6 0 R 11 0 R] \
         /DA (/Helv 0 Tf 0 g) >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> \
         /Contents 4 0 R /Annots [5 0 R 6 0 R 9 0 R 10 0 R 11 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Tx /T (typed) /V (hello) \
         /Rect [10 10 90 30] /F 4 >>\nendobj\n\
         6 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Btn /T (ticked) /Rect [110 10 130 30] \
         /F 4 /AS /Off /AP << /N << /Yes 7 0 R /Off 8 0 R >> >> >>\nendobj\n\
         7 0 obj\n{}\nendobj\n\
         8 0 obj\n{}\nendobj\n\
         9 0 obj\n<< /Type /Annot /Subtype /Text /Rect [150 70 160 80] /F 4 /T (Drive) \
         /Contents (a red word) /Popup 10 0 R /RC ({}) >>\nendobj\n\
         10 0 obj\n<< /Type /Annot /Subtype /Popup /Rect [10 40 190 95] /Parent 9 0 R /Open true \
         /F 4 >>\nendobj\n\
         11 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (sent) \
         /Rect [150 10 190 30] /F 4 /A << /S /SubmitForm /F << /FS /URL \
         /F (https://example.org/submit) >> /Flags 1024 >> >>\nendobj\n",
        appearance("0 0 1 rg"),
        appearance("1 1 1 rg"),
        // Table 172's `/RC`: one run red, bold, set in Courier and spaced two points (ADR 1655),
        // and a tab at the paragraph's end to its one right-aligned stop (ADR 1667), a dashed
        // two-point rule its leader (ADR 1726).
        "<body xmlns=\"http://www.w3.org/1999/xhtml\"><p style=\"tab-interval:36pt;\
         tab-stops:right leader(rule(dashed 2pt)) 72pt\">a <span style=\"color:#ff0000;font-weight:bold;\
         font-family:Courier;letter-spacing:2pt\">red</span> word<span \
         style=\"xfa-tab-count:1\"/></p></body>",
    );
    written(&body)
}

/// One text field whose validation asks with `app.alert` — RFC 0008 section 4.2's question, the
/// icon a question's and the buttons Yes and No, in Adobe's numbers 2 and 2 — for the window's
/// script dialogue (ADR 2121).
fn script_fixture() -> Vec<u8> {
    written(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
         /DA (/Helv 0 Tf 0 g) >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> \
         /Annots [4 0 R] >>\nendobj\n\
         4 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Tx /T (kept) /V (1) \
         /Rect [10 10 90 30] /F 4 /AA << /V << /S /JavaScript \
         /JS (app.alert\\('Keep it?', 2, 2\\);) >> >> >>\nendobj\n",
    )
}

/// `body`'s objects behind a header, with the cross-reference table and the trailer that index
/// them.
fn written(body: &str) -> Vec<u8> {
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
    let program = artefacts.join("quorra_open_a_page");
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
    let form = artefacts.join("quorra_form_fixture.pdf");
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
/// application note is five pages, its outline is fourteen rows, its first page is where a search
/// for "black point" lands, and the page after it draws.
fn what_it_printed(said: &str) {
    for expected in [
        "abi 2 (header 2), 30 event kind(s) (header 30)",
        // ISO 19005's conversion asked for at the reader's default level, granted once, and its
        // report's first line naming the profile the library listed second (ADR 1954), and that
        // profile's own sentence as the windows offer it (ADR 1966).
        "archive: 6 profile(s), the second as-if-printed: Discard what printing would not have \
         carried; touch nothing it would.",
        "archive: 1 grant(s), ",
        "line(s), the first: converting to PDF/A-2b under the profile \"as-if-printed\"",
        "Opened says document 1 has 5 page(s)",
        "page 1 of 5 (5 page(s) in the document)",
        // The open read no outline, so its announcement had no section; the preparation, run and
        // handed back through the ABI, announces page one again under §12.3.3's item whose
        // destination is page one — the note's title item, whose `/Title` breaks the line after
        // "001:" (ADR 1553).
        "prepared: page 1 is in \"PDF 2.0 Application Note 001:",
        "outline: 14 row(s)",
        // Annex O's `search`, pumped one page at a time by the C loop, and the view moves to the
        // page the occurrence is on, which is the annex's "selecting the first matching word in
        // the document" as far as a C caller can see it. The first occurrence is page one's title,
        // set as `[(B) -5 (la) 1 (c)] TJ` in a `1 Tf` font under `32 0 0 32 Tm`: §9.4.4's
        // displacement is in text space, where `-5` is 0.005 em and no word gap, so the readback
        // is "Black Point" at bytes 57..68 after "Application Note\n2018-09\nPDF 2.0 Application
        // Note 001: \n\n" (ADR 1490). One step, for the one page searched.
        "search: found on page 1, bytes 57..68, after 1 step(s)",
        "after the search: page 1 of 5",
        "after the turn: page 2 of 5, drawn in ",
        "frame: page 2, 708x1000, format 0, 2832000 byte(s)",
        // ADR 0346's enumerations, each line read off the library. The two counted enumerations,
        // the name it gives a number it does not define, and the refusal an enumeration this ABI
        // *takes* answers with are the whole of what C has in place of a build failure — so they
        // are asserted rather than printed.
        "control kinds 8 (header 8), row kinds 4 (header 4), unknown is unknown",
        "an undefined pointer action: the message at that index is not of the kind this accessor \
         reads",
        // The note states no optional content and no `/EmbeddedFiles` tree. Both answer an
        // **empty list** rather than `QUORRA_NO_ANSWER`, which is `viewer-core`'s existing choice
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
        // Table 22's bit 5 is asked by `quorra_copy` and nowhere else, so the copy text has no
        // answer before one; after it, the readback is the `QUORRA_EVENT_COPIED`'s own string, in
        // the logical order the note's structure tree gives (ADR 2087).
        "copy before quorra_copy: there is nothing to answer with",
        "in logical content order, the event's own text: 1",
        "layers: 0 row(s)",
        "attachments: 0 row(s)",
        // §12.7's form, on the fixture beside the test binary. Two fields, the check box ticked
        // with the name the library handed over, and the edit read back rather than assumed.
        "form: 3 field(s)",
        "[0] Entry flags 0",
        "ticking ticked with the state Yes",
        "after the edit: 1 widget(s) on",
        "dirty after the edit: 1",
        // **The C program saves between the two**, so the undo takes back an edit the file already
        // holds and the document is unsaved again: `Open::dirty` means "the cursor is not where the
        // last save left it", not "the log is not empty", which would answer the same for a
        // document saved and one never saved at all.
        "dirty after the undo: 1",
        // Table 172's `/RC` as runs (ADR 1655): three pieces, the middle one the note's red bold
        // Courier run with its two points of spacing, read through the header's own structs.
        "rich: 1 paragraph(s), 0 unapplied; paragraph 0 align 0, 3 piece(s)",
        "rich piece 1: \"red\" bold 1 colour 1.00,0.00,0.00 family Courier spacing 2.0pt",
        "rich piece 2: \" word\t\"",
        // The paragraph's stops as it states them: the interval and one right-aligned stop.
        "rich tabs: 1 stated, interval 1 at 36.0pt",
        "rich tab 0: align 2 at 72.0pt",
        // Its leader crosses with it (ADR 1726): a rule, dashed, two points thick, and broken
        // across the stop's 72 points from the margin as a dash every eight — nine pieces, each
        // a left edge and a width.
        "rich leader 0: pattern 2 style 1 thickness 1 at 2.0pt; 18 float(s) across 72pt",
        // The window names its note, object 9, and the note retyped through it is what the
        // window shows next (ADR 1726).
        "note 9 0 retyped: \"retyped\"",
        // The eight commands RFC 0010 section 3.1 priced (ADRs 2060, 2061). The clock is what the
        // save writes as Table 166's `/M`, a server's FDF answer is the field's value, and an
        // answer to a script that is not waiting reaches nothing.
        "clock: the save writes /M as the caller's clock said: 1",
        "respond: the field holds the server's value: 1",
        "an answer to a script that is not waiting: 0 event(s)",
        // `viewer_host::keys`' table through `quorra_key_press` (ADR 2072): the counts against the
        // header's, the acts the windows' keys come to, the markup key's sentence with nothing
        // selected, and Space on the push-button the Tab key reached — whose §12.7.6.2 submission
        // owes Table 240 bit 11, set without bit 8, as a sentence beside the request.
        "keys 33 (header 33), acts 22 (header 22), unknown is unknown",
        "keyboard: Down scrolls 60.0, Control + F is Find, F presenting is Nothing, Control + T is \
         Nothing, Escape warned is AbortDrawing",
        "keyboard: H with nothing selected says \"select some text first",
        "keyboard: Space on the focused button 11 submits to https://example.org/submit, owing 1 \
         sentence(s): Table 240 bit 11's ExclNonUserAnnots shall be used only when",
        // A page past `MAX_PIXELS` under `QUORRA_OVERSIZE_LISTED`: asked for whole at 40 logical
        // pixels per unit rather than refused, and drawn as a window of the viewport grown by a
        // quarter on each side — 1200 x 1500 and the pixel either span reaches into.
        "oversize: page 2 is 23812x33676, a window 1201x1501 at ",
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
        "after the turn: page 2 of 5, drawn in ",
    ] {
        assert!(
            said.contains(measured),
            "the C program did not measure {measured:?}"
        );
    }
}

/// `quorra_window` from C: `c/drive_a_window.c` runs the loop a toolkit host runs — a resize, the
/// window's timer, some keys, an answer — and every one of them is one call and the same reading of
/// what the window owes, then the rows, the marks and a control's commit read and sent through it
/// (RFC 0010 section 3.2a, ADRs 2088 and 2104).
#[test]
fn a_c_program_drives_a_window_one_call_per_thing_that_happened() {
    let Some(cc) = compiler() else {
        println!("skipped: no C compiler on this machine");
        return;
    };
    let Some(artefacts) = artefacts() else {
        println!("skipped: this test binary is not where cargo puts one");
        return;
    };
    // The `cdylib`, for the first test's reason.
    let built = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .args(["build", "-p", "viewer-ffi", "--lib"])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .status();
    assert!(
        built.is_ok_and(|status| status.success()),
        "the cdylib has to exist for a C program to link against"
    );
    // The script worker the window's runner starts, which `PDF_SCRIPT_WORKER` names below: the
    // engine is the worker program's alone, so no library this test links is built with it (ADR
    // 1616).
    let worker = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .args([
            "build",
            "-p",
            "pdf-script-worker",
            "--features",
            "engine",
            "--bin",
            "pdf-script-worker",
        ])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .status();
    assert!(
        worker.is_ok_and(|status| status.success()),
        "the script worker has to exist for a script's question to be asked"
    );
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let program = artefacts.join("quorra_drive_a_window");
    let compiled = Command::new(&cc)
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-O2", "-I"])
        .arg(crate_root.join("include"))
        .arg(crate_root.join("c/drive_a_window.c"))
        .arg("-o")
        .arg(&program)
        .arg("-L")
        .arg(&artefacts)
        .arg("-lviewer_ffi")
        .arg(format!("-Wl,-rpath,{}", artefacts.display()))
        .output()
        .expect("the compiler runs");
    assert!(
        compiled.status.success() && compiled.stderr.is_empty(),
        "{cc} refused the header or the program:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let document = crate_root.join("../../doc/PDF20_AN001-BPC.pdf");
    // A file of the window's own, beside the first test's, so the two may run at once.
    let form = artefacts.join("quorra_window_form_fixture.pdf");
    std::fs::write(&form, form_fixture()).expect("the fixture is written beside the test binary");
    let script = artefacts.join("quorra_window_script_fixture.pdf");
    std::fs::write(&script, script_fixture())
        .expect("the fixture is written beside the test binary");
    // The reader's directory: one file that is neither a certificate nor a PDF, so that both the
    // anchors and the target documents refuse it by name (ADR 2156).
    let directory = artefacts.join("quorra_window_reader_words");
    std::fs::create_dir_all(&directory).expect("the directory is made beside the test binary");
    std::fs::write(
        directory.join("words.txt"),
        b"neither a certificate nor a PDF\n",
    )
    .expect("the file is written");
    let ran = Command::new(&program)
        .arg(&document)
        .arg(&form)
        .arg(&script)
        .arg(&directory)
        .env("PDF_SCRIPT_WORKER", artefacts.join("pdf-script-worker"))
        // Two seconds rather than the reader's wait, so the question left unanswered is withdrawn
        // inside the test (ADR 1643).
        .env("PDF_VIEWER_SCRIPT_ANSWER_WAIT_MS", "2000")
        .output()
        .expect("the program runs");
    let said = String::from_utf8_lossy(&ran.stdout).into_owned();
    println!("{said}");
    assert!(
        ran.status.success(),
        "the C program failed:\n{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    what_the_window_owed(&said);
}

/// What the window owed the C program, checked here rather than in the C, as
/// [`what_it_printed`] checks the vocabulary's program.
fn what_the_window_owed(said: &str) {
    for expected in [
        // The counted kinds against the header's, and a path that names no file refused by its
        // own status rather than opened into a window that says nothing.
        "owed kinds 17 (header 17), unknown is unknown",
        // `viewer_host::Tab`'s six, in the order and the words every window shows them.
        "panels offered 6 (header 6): Contents Pages Layers Files Read About",
        "an unreadable file: the file named could not be opened for reading",
        // Table 29's `/PageLayout`, obeyed when the document opened and said, as every window says
        // it.
        "said: this document opens in the OneColumn page layout (§7.7.2)",
        // The first resize joined the open and the timer ran until the window asked for none: page
        // one, drawn on the drawing thread at the 800 x 1000 viewport, the size the vocabulary's
        // program gets for page two at the same viewport.
        "window frame: 1 picture(s), page 1, 708x1000 shown 708x1000, 2832000 byte(s); ",
        // §12.2's `/DisplayDocTitle` title, then the caption: §12.4.2's label, the page counted,
        // and §12.3.3's item the page is in.
        "window title: PDF 2.0 Application Note 001: Usage of Black Point Compensation — Cover — \
         page 1 of 5",
        // The panels' rows, read on the record the open owed through the vocabulary's own
        // handles: §12.3.3's fourteen items (mutool lists the same fourteen, as evidence), the
        // five pages, and §12.4.2's label for the first (ADR 2104).
        "window panels: 2 owed, 14 outline row(s), 14 property row(s), 5 page(s); page 1 is \
         labelled \"Cover\" (ok), its thumbnail: ok",
        // Page Down is a message the window sent itself, so nothing is handed back.
        "window frame: 1 picture(s), page 2, 708x1000",
        "— page 2 of 5 — PDF 2.0 Application Note 001: Black Point Compensation, act Nothing",
        // Control + F is the find bar, a widget, so it is handed back; H with nothing selected is
        // the sentence `viewer_host::decide` owes, said by the window.
        "window keys: Control + F is Find, H is Nothing and said \"select some text first",
        // The find bar's search, from page two, read a page per tick: the phrase is not on the
        // copyright page and is on the third, which `pdftotext` agrees with as evidence.
        "said: searching for \"Black Point\": 5 page(s) left",
        "window search: found \"Black Point\" on page 3",
        // What a repaint draws over the frames: the page's three occurrences of the needle, any
        // case, as the search counts them, the one it landed on selected; and none once the bar
        // closed, while the selection stays.
        "window marks with the find bar open: 3 match(es), 6 selection quad(s), 0 highlight(s)",
        "window marks after it closed: 0 match(es), 6 selection quad(s), 0 highlight(s)",
        "window pointer at the corner: over a link 0",
        // An outline row's click: `/Dest` names page 3, so the caption and the count follow.
        "window outline row \"Preface\" activated: PDF 2.0 Application Note 001: Usage of Black \
         Point Compensation — 2 — page 3 of 5",
        // Control and the wheel about the viewport's middle: three events of two fifths of a line
        // are one step, no event a step on its own, and the page is drawn a quarter larger; a
        // touchpad's sixty pixels back, with the fifth of a line carried, are one step out and the
        // page is drawn at the size it had (ADR 2152).
        "page 3, 884x1250 shown 884x1250",
        "window wheel: 1.2 lines in three events, 1 step(s); sixty pixels back, -1; unit 7: the \
         message at that index is not of the kind this accessor reads; not a number: a number \
         does not fit the type this boundary states for it",
        "page 3, 708x1000 shown 708x1000",
        // A file chooser's answer opens in a tab of its own and takes the front; the first is
        // chosen and closed, and closing the other is the last, which the window leaves open for
        // its caller to close.
        "said: quorra_window_form_fixture.pdf opened beside — 2 document(s) open",
        "window strip: 2 document(s), quorra_window_form_fixture.pdf in front at 1, the strip \
         owed yes",
        "window closed one: closed PDF20_AN001-BPC.pdf — 1 document(s) open; the last 0",
        "window closing the other: the last 1, 1 document(s) left open",
        // Tab reaches the push-button and Space presses it: the submission's owed sentence, then
        // the reader's default level for a form leaving this machine, which asks.
        "said: page 1: this item submits — Table 240 bit 11's ExclNonUserAnnots",
        // `viewer_host::may_submit`'s question, its reasons and its choice as one record.
        "window question: This document asks to send its form from this mach | You have set this \
         reader to ask before a form is sent",
        "window answered no: submit-form: declined — you answered \"Do not\". It would have been \
         POST https://example.org/submit",
        // And the question was taken by the answer, so a dialogue closed twice answers once.
        "a second answer: there is nothing to answer with",
        // And a password for a document no prompt is out for opens nothing.
        "a password nobody asked for: there is nothing to answer with",
        // The fixture's text field is a control placed over the page — the open owed the controls
        // once — and toggles nothing; a field the page does not show refuses an edit by status;
        // typed into and finished with, the field holds what was typed.
        "window form: 3 field(s), typed; 1 control record(s); toggling it: the message at that \
         index is not of the kind this accessor reads; an unknown field: there is nothing to \
         answer with",
        "window control committed: typed is \"typed in C\" (ok)",
        // ISO 19005's chooser, a record on the six targets and the shipped profiles, starting on
        // the first of each; an index past the list leaves it out, closing it says nothing was
        // converted, and a second answer finds nothing to answer (ADR 2121).
        "archive chooser: Convert to ISO 19005 (PDF/A)\nA profile is the least a conversion may \
         lose",
        "window chooser: 6 target(s) from PDF/A-2b, ",
        "starting on 0 and 0, act Nothing; past the list: the index names nothing; closed: nothing \
         was converted: no target and profile were chosen; again: there is nothing to answer with",
        // Control + P asks §7.6.4.2's bit 3 on the window's own default sheet, as `viewer-qt`
        // asks it; the grant is a record, the print stands until it is ended, and once.
        "print: printing, over 5 page(s)",
        "window print: act Nothing, 5 page(s), degraded 0; finished again: there is nothing to \
         answer with; a script answer nobody asked for: there is nothing to answer with",
        // The levels a host states, by word and by number, read as every window reads them; the
        // refusal is `--links=`' own sentence, and the form's submission is refused under them
        // where the default level asked (ADR 2120).
        "settings: --submissions=refuse taken 1, --cpu taken 0; --links=sometimes: the message at \
         that index is not of the kind this accessor reads, \"--links=sometimes: no such level. \
         One of refuse, ask, warn, open\"",
        "window under the stated levels: 0 question(s); ",
        // The reader's other words, by word and by number, refused in the command line's
        // sentences; the file in the directory said by name for the anchors and for the target
        // documents, as the toolkit windows say it; the conversion stated is where the chooser
        // starts; and the print key ends the print that stands (ADRs 2156, 2157).
        "reader: --reader-name used 2, --accept-unknown-revocation used 1; --trust-anchors \
         alone: the message at that index is not of the kind this accessor reads, \
         \"--trust-anchors wants a directory of PEM or DER certificates\"; no such directory: \
         the message at that index is not of the kind this accessor reads; word 6: the message at \
         that index is not of the kind this accessor reads; target 99: the index names nothing",
        "said: words.txt in the trust anchor directory is neither DER nor a PEM CERTIFICATE block",
        "window under the reader's words: the chooser starts on 1 and 0",
        "window print key again: \"the printed page is no longer what this window shows\"; then \
         finished: there is nothing to answer with",
        // A script's `app.alert` at the scripts' PROCEED level: the record in Adobe's numbers —
        // the question icon 2, Yes and No 2, closed answering No 3 — answered once; the second,
        // left, withdrawn by its wait (ADRs 1628, 1643).
        "window script: \"Keep it?\", icon 2, buttons 2, closed answers 3",
        "window script again: there is nothing to answer with; the second withdrawn 1, then \
         answered: there is nothing to answer with",
        "ok",
    ] {
        assert!(
            said.contains(expected),
            "the C program did not say {expected:?}"
        );
    }
    let inked = said
        .lines()
        .find(|line| line.starts_with("window frame: 1 picture(s), page 1, "))
        .and_then(|line| line.split("; ").nth(1))
        .and_then(|rest| rest.split(' ').next())
        .and_then(|count| count.parse::<usize>().ok())
        .expect("the program says how many pixels of page one are not white");
    assert!(inked > 1000, "only {inked} pixels of page one are inked");
}
