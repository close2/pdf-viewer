//! `kio/` built with `cmake`, loaded by KIO, and driven through the protocol.
//!
//! **This is the only instrument in the round that loads the plugin.** The Rust tests test the
//! core, `a_c_program_drives_the_abi` tests the ABI, and `header_and_library_agree` tests those
//! two against each other; none of them makes KIO read the plugin's metadata, decide that `pdf:`
//! is served there, fork `kioworker`, and send it a command over a socket. `kio/test/
//! drive_the_worker.cpp` is a KIO *client* — the same jobs a file manager runs — and this test
//! builds both halves and compares what came back with what RFC 0003 says the tree is.
//!
//! # What it does not establish, said plainly
//!
//! It is not Dolphin and it never sees a session. It says nothing about how a listing is
//! rendered, nothing about the `archiveMimetype` association that makes a click on a PDF enter it
//! as a folder, and nothing about a person's experience of any of it. Those need a KDE session;
//! this needs a `QCoreApplication`.
//!
//! # Why a skip here is not a hole
//!
//! `doc/todo/02` §2's sequence has to stay green on a machine with no KDE, which is the whole
//! reason `kio/` is outside the cargo workspace. So: no `cmake` on the machine, or no ECM, Qt 6
//! or KF6 for it to find, and this test **prints what is missing and returns** — the shape
//! `viewer-ffi`'s C driver uses for a machine with no `cc`. Anything else that goes wrong in the
//! configure or the build is a failure, because a plugin that will not compile against a
//! toolchain that *is* there is a defect rather than an absence.

// `clippy.toml` sets `allow-expect-in-tests`, so this attribute is fulfilled by the `.expect`s in
// `what_came_back` — an ordinary function — and by none of the ones inside the `#[test]` itself.
// Worth knowing, because an expectation nothing fulfils is *itself* an error under
// `RUSTFLAGS="-D warnings"`: this file had no helper for one revision of it and the attribute
// failed the build.
#![expect(
    clippy::expect_used,
    reason = "test code: a figure the harness did not print must fail loudly rather than pass by \
              doing nothing"
)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// Where the workspace's build output is, found from this test binary rather than assumed.
fn artefacts() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let deps = exe.parent()?;
    Some(deps.parent()?.to_path_buf())
}

/// Whether a program answers `--version`.
fn present(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// The sentence `cmake` prints when a `find_package` found nothing.
///
/// Told apart from every other configure failure on purpose: a missing package is this machine's
/// business and a syntax error in `kio/CMakeLists.txt` is ours.
fn a_package_is_missing(said: &str) -> bool {
    said.contains("Could not find a package configuration file") || said.contains("Could NOT find")
}

#[test]
fn kio_loads_the_plugin_browses_the_tree_and_writes_two_verbs_through_it() {
    if !present("cmake") {
        println!("skipped: no cmake on this machine");
        return;
    }
    let Some(artefacts) = artefacts() else {
        println!("skipped: this test binary is not where cargo puts one");
        return;
    };
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = crate_root.join("../..");

    // The shared library the plugin links against, and — trap 10 — the confined generator, which
    // Cargo will not build for another package's test and without which nothing can be answered.
    for package in [
        ["build", "-p", "pdf-vfs-ffi", "--lib"],
        ["build", "-p", "pdf-vfs", "--bins"],
    ] {
        let built = Command::new(&cargo)
            .args(package)
            .current_dir(&workspace)
            .status();
        assert!(
            built.is_ok_and(|status| status.success()),
            "{package:?} has to succeed before the plugin can link or answer"
        );
    }
    let library = artefacts.join("libpdf_vfs_ffi.so");
    assert!(library.exists(), "no cdylib at {}", library.display());

    let build = artefacts.join("kio-face");
    let configured = Command::new("cmake")
        .arg("-S")
        .arg(crate_root.join("../../kio"))
        .arg("-B")
        .arg(&build)
        .arg(format!(
            "-DPDFVFS_INCLUDE_DIR={}",
            crate_root.join("include").display()
        ))
        .arg(format!("-DPDFVFS_LIBRARY={}", library.display()))
        .output()
        .expect("cmake runs");
    if !configured.status.success() {
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&configured.stdout),
            String::from_utf8_lossy(&configured.stderr)
        );
        assert!(
            a_package_is_missing(&said),
            "cmake refused kio/CMakeLists.txt for a reason that is not a missing package:\n{said}"
        );
        println!("skipped: this machine has cmake and not the KDE toolchain:\n{said}");
        return;
    }
    let compiled = Command::new("cmake")
        .args(["--build"])
        .arg(&build)
        .output()
        .expect("cmake builds");
    assert!(
        compiled.status.success(),
        "the plugin did not build:\n{}{}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );

    // KIO finds a worker by scanning `kf6/kio/` under every plugin directory, so the plugin is
    // put where an installed one would be and `QT_PLUGIN_PATH` names the root of that layout.
    // Nothing here installs into the system: the whole point is that this runs in a build tree.
    let plugins = build.join("plugins/kf6/kio");
    std::fs::create_dir_all(&plugins).expect("a plugin directory beside the build");
    std::fs::copy(build.join("pdf.so"), plugins.join("pdf.so")).expect("the plugin is placed");

    let document = workspace.join("doc/PDF20_AN001-BPC.pdf");
    let scratch = build.join("scratch.pdf");
    std::fs::copy(&document, &scratch).expect("a scratch copy the write verbs may change");

    let ran = Command::new(build.join("drive_the_worker"))
        .arg(document.canonicalize().expect("the note is on disk"))
        .arg(&scratch)
        .env("QT_PLUGIN_PATH", build.join("plugins"))
        // The confined generator is beside cargo's output rather than beside `kioworker`, so it
        // is named. `pdf_vfs::WORKER_PATH_VARIABLE` exists for exactly this.
        .env("PDF_VFS_WORKER", artefacts.join("pdf-vfs-worker"))
        // The plugin was linked against a library in the build tree, which no loader looks in.
        .env("LD_LIBRARY_PATH", &artefacts)
        .output()
        .expect("the harness runs");
    let said = String::from_utf8_lossy(&ran.stdout).into_owned();
    println!("{said}");
    assert!(
        ran.status.success(),
        "the KIO client failed:\n{}",
        String::from_utf8_lossy(&ran.stderr)
    );

    what_came_back(&said);
}

/// What KIO handed the client, checked here rather than in the C++.
fn what_came_back(said: &str) {
    for expected in [
        // RFC 0003 section 4's tree, through a `listDir` a file manager would make.
        "root: pages renders images text attachments meta",
        "pages: 0001.pdf 0002.pdf 0003.pdf 0004.pdf 0005.pdf",
        // A `get` of exactly as many bytes as the `stat` promised, which is what "agreeing with
        // the stat 1" says and is RFC 0003 section 5.5's whole point: an estimate would truncate
        // the page for every reader, the ffmpegfs lesson that section records. **The number is
        // not written here**, and that is a correction rather than laziness — it was, and the
        // merge that brought rounds 910 and 911 in moved an extracted page from 36 265 bytes to
        // 36 997 with nothing about this face changed. A derived file's length is the writer's
        // figure, so pinning it here would make this face's gate fail for the transform suite's
        // reasons. What binds is the agreement, and it is read off both lines below.
        "stat: 0001.pdf, directory 0, ",
        ", type application/pdf",
        "beginning %PDF-, agreeing with the stat 1",
        // Section 5.3's three refusals, each reaching the *job's* error string as the core's own
        // sentence rather than as KIO's canned category. This is the half FUSE cannot carry, and
        // the reason this face is worth building at all.
        "mkdir: /fonts: this directory is the document's own shape",
        "rename: /pages/0001.pdf -> /pages/0002.pdf: a rename inside pages/ is a reorder",
        "put into text/: /text/0001.txt: editing a page's text through a byte stream",
        // `CLAUDE.md` principle 3's *warn* channel, arriving at a KIO job as `KJob::warning`.
        "the document said: §7.5.6 leaves a deleted object's bytes in the file",
        // Section 5.2's verbs, through KIO's own `del` and `put`, over a scratch copy — and the
        // listing renumbers, which is what makes an ordinal a position.
        "after the delete: 4 page(s)",
        "after the insert: 5 page(s)",
        "ok",
    ] {
        assert!(
            said.contains(expected),
            "KIO did not answer with {expected:?}"
        );
    }
    // And every refusal names its `errno` beside the sentence, so a script reading the error
    // string still has the number the core chose. Counted rather than matched once, because
    // three refusals sharing one substring is trap 27's shape.
    assert_eq!(
        said.matches("(EPERM)").count(),
        3,
        "each of section 5.3's three refusals carries the core's own errno"
    );
    // The two byte counts, read off the lines rather than written down. They have to be equal —
    // that is RFC 0003 section 5.5 — and the page has to be a page rather than an empty answer
    // that would satisfy equality trivially.
    let number_after = |prefix: &str| -> Option<u64> {
        said.lines()
            .find_map(|line| line.strip_prefix(prefix))
            .and_then(|rest| rest.split(' ').next())
            .and_then(|count| count.parse::<u64>().ok())
    };
    let stated = number_after("stat: 0001.pdf, directory 0, ")
        .expect("the harness prints the size the stat stated");
    let read = number_after("get: ").expect("the harness prints the size the get returned");
    assert_eq!(
        stated, read,
        "a stat that states a size the get does not deliver truncates the page for every reader"
    );
    assert!(
        stated > 1000,
        "{stated} bytes is not a page of the application note"
    );
}
