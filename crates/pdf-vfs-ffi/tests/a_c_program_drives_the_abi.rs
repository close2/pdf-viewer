//! The ABI, compiled and run from C.
//!
//! **The FUSE face proves the core; only a C program proves the ABI.** Everything else in this
//! crate is Rust calling Rust — the entry points are `extern "C"` and the argument types are C's,
//! but no C compiler has read the header and no linker has resolved the symbols. This test is
//! what closes that: `cc` compiles `c/browse_a_document.c` against `include/pdf_vfs.h`, links it
//! against the `cdylib`, and runs it on a real document and a scratch copy of one.
//!
//! What it therefore catches that nothing else does: a declaration in the header that does not
//! match the Rust signature by *name* (the linker says so), a struct tag colliding with a
//! function in C's one namespace (the compiler says so), and a caller freeing a handle the
//! library still owns (the allocator says so, loudly, on exit).
//!
//! **Skipped rather than failed where there is no C compiler**, in the shape `viewer-ffi` uses
//! for the same gate: a machine without `cc` is a machine this test cannot run on, and pretending
//! otherwise would make it a coin toss.

#![expect(
    clippy::expect_used,
    reason = "test code: a step that cannot be performed must fail loudly rather than pass by \
              doing nothing"
)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// Where the workspace's build output is, found from this test binary rather than assumed.
///
/// `target-dir` is a directory outside the tree on the machine this project is developed on, so
/// the path is taken from the executable that is running: a test binary lives in
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
fn a_c_program_browses_a_document_reads_a_page_out_of_it_and_writes_two_verbs_back() {
    let Some(cc) = compiler() else {
        println!("skipped: no C compiler on this machine");
        return;
    };
    let Some(artefacts) = artefacts() else {
        println!("skipped: this test binary is not where cargo puts one");
        return;
    };
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    // The `cdylib` is a separate artefact from the rlib this test links against, and `cargo test`
    // does not necessarily produce it. Asking cargo for it is cheap when it is already there and
    // is the only way to be sure it is: a test that skipped because the library was missing would
    // be a gate that quietly stops running.
    let built = Command::new(&cargo)
        .args(["build", "-p", "pdf-vfs-ffi", "--lib"])
        .current_dir(&workspace)
        .status();
    assert!(
        built.is_ok_and(|status| status.success()),
        "the cdylib has to exist for a C program to link against"
    );
    // **Trap 10, and it bites harder here than anywhere else in this crate**: not one question
    // below can be answered without `pdf-vfs-worker`, because RFC 0003 section 6 puts every byte
    // of parsing in it. Cargo will not build another package's binary for this test.
    let worker = Command::new(&cargo)
        .args(["build", "-p", "pdf-vfs", "--bins"])
        .current_dir(&workspace)
        .status();
    assert!(
        worker.is_ok_and(|status| status.success()),
        "the confined generator has to exist for the tree to answer anything"
    );

    let library = artefacts.join("libpdf_vfs_ffi.so");
    assert!(
        library.exists(),
        "no cdylib at {}: this test needs one",
        library.display()
    );

    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let program = artefacts.join("pdfvfs_browse_a_document");
    let compiled = Command::new(&cc)
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-O2")
        .arg("-I")
        .arg(crate_root.join("include"))
        .arg(crate_root.join("c/browse_a_document.c"))
        .arg("-o")
        .arg(&program)
        .arg("-L")
        .arg(&artefacts)
        .arg("-lpdf_vfs_ffi")
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
    let scratch = artefacts.join("pdfvfs_scratch.pdf");
    std::fs::copy(&document, &scratch).expect("a scratch copy is made beside the test binary");

    // `bug1815476.pdf` is encrypted with `/P -1084`, so §7.6.4.2's Table 22 bit 11 is clear and
    // taking a page out of the mount is withheld — which is what makes the *ask* level reachable
    // from C. A hyphen where `doc/pdf.js` is not checked out; the C program says which it did.
    let restricted = crate_root.join("../../doc/pdf.js/test/pdfs/bug1815476.pdf");
    let restricted = if restricted.exists() {
        restricted.display().to_string()
    } else {
        String::from("-")
    };

    let ran = Command::new(&program)
        .arg(&document)
        .arg(&scratch)
        .arg(&restricted)
        // The confined generator is beside `cargo build`'s output rather than beside this
        // program, so it is named rather than searched for. `pdf_vfs::WORKER_PATH_VARIABLE`
        // exists for exactly this case.
        .env("PDF_VFS_WORKER", artefacts.join("pdf-vfs-worker"))
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
    what_it_asked(&said, &restricted);
}

/// `CLAUDE.md` principle 3's *ask* level, driven from C the way a face drives it (ADR 0874): the
/// question, a no that leaves the document alone, and a yes that performs the operation.
fn what_it_asked(said: &str, restricted: &str) {
    if restricted == "-" {
        assert!(said.contains("restricted: skipped, no corpus"), "{said}");
        println!("the *ask* round trips were skipped: the pdf.js corpus is not checked out");
    } else {
        for expected in [
            "consulted: verdict 2, question 'This document restricts assembling a document out \
             of these pages: Table 22 bit 11 is clear. Do it anyway?'",
            "after a no: EACCES —",
            "after a yes: ",
        ] {
            assert!(
                said.contains(expected),
                "the C program did not say {expected:?}"
            );
        }
    }
}

/// The lines the C program printed, checked here rather than in the C.
///
/// So that a change to the ABI that still *runs* cannot pass by printing something else. The
/// application note is five pages; RFC 0003 section 4's tree has six directories at its root.
fn what_it_printed(said: &str) {
    for expected in [
        "abi 1 (header 1), 13 errno kind(s) (header 13)",
        // The split is the face's own question and the only one that asks the file system.
        "and the rest is '/pages/0001.pdf'",
        "a path with no file in it: no part of this path is a file, so there is no document here",
        "the document has 5 page(s)",
        "/: 6 entries: pages/ renders/ images/ text/ attachments/ meta/",
        "/pages: 5 entries: 0001.pdf 0002.pdf 0003.pdf 0004.pdf 0005.pdf",
        // RFC 0003 section 5.5: a `stat` generates, so the size is the file's own and the read
        // below is exactly that many bytes.
        "stat /pages/0001.pdf: kind 1, size stated 1,",
        "beginning %PDF-",
        // What the *core* says the verbs mean, which is what a file manager's access bits are.
        "meaning of /pages/0001.pdf: write 1, delete 2",
        "meaning of /text/0001.txt: write 0, delete 0",
        // Section 5.3's refusals, each reaching C as a number **and** a sentence — which is the
        // half FUSE cannot carry and the whole reason this face is worth building.
        "writing into text/: EPERM — /text/0001.txt: editing a page's text through a byte stream",
        "rename: EPERM — /pages/0001.pdf -> /pages/0002.pdf: a rename inside pages/ is a reorder",
        "mkdir: EPERM — /fonts: this directory is the document's own shape",
        "a shortfall that is not there is refused rather than answered",
        // Section 5.2's verbs, over a scratch copy, from C.
        "deleted a page: 4 page(s) now,",
        "inserted a page at 0001: 5 page(s) now",
        "ok",
    ] {
        assert!(
            said.contains(expected),
            "the C program did not say {expected:?}"
        );
    }
    // Trap 5 across a boundary: the layout declares things this build does not do, and a face is
    // able to print them. The *number* is not asserted — it moves as the core grows — but that
    // there are some, and that the first is a sentence rather than a code, is.
    let shortfalls = said
        .lines()
        .find_map(|line| line.strip_prefix("shortfalls: "))
        .and_then(|count| count.parse::<usize>().ok())
        .expect("the program says how many shortfalls the core declares");
    assert!(
        shortfalls > 0,
        "the core declares none, which it should not"
    );
    assert!(
        said.contains("the first shortfall: "),
        "a shortfall reached C as a number and not as a sentence"
    );
}
