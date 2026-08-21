//! Finds Qt 6, generates the bridge, runs `moc`, and compiles the host's C++.
//!
//! This is the whole of what `doc/todo/30` meant by "Qt costs a C++ bridge and a build step",
//! and it is worth reading as the cost itself. `cxx-qt-build` locates Qt through `qmake6` on
//! `PATH`, hands `cxx` the bridge module to generate `extern "C"` shims from, runs `moc` over
//! every header named here — `cpp/window.h` declares four `Q_OBJECT` classes and moc is what
//! makes their metaobjects — compiles the C++, and links `QtCore`, `QtGui` and `QtWidgets`.
//!
//! **A machine without Qt 6's development files cannot build this crate**, exactly as a machine
//! without GTK 4's cannot build `viewer-gtk`. That is a host binding a platform rather than a
//! defect: ADR 0246 records where it is excluded (the three cross-target checks) and where it is
//! installed (CI's two jobs, beside GTK's).
//!
//! `cc_builder` is deliberately not used. It is an `unsafe fn` in `cxx-qt-build` 0.9, and
//! `include_dir` plus `cpp_file` reach the same result through the safe interface — which
//! matters here more than usual, because this crate's `unsafe` position is one of the things the
//! round exists to establish.

// A build script's job is to abort the build when its environment is not what Cargo promises,
// and the panic message is the diagnostic a developer reads — the same argument
// `pdf-font/build.rs` and `pdf-spec/build.rs` make.
#![expect(
    clippy::expect_used,
    reason = "aborting the build is the intended and only useful failure mode here"
)]

/// Forces the linker to want `cxx-qt`'s initializer *before* it reads the archive that has it.
///
/// **This is a workaround for somebody else's ordering, and it is here because the alternative
/// is a machine requirement made silently.** `cxx-qt-build` puts the *call* to the crate
/// initializer in its own static library linked `+whole-archive`, and `rustc` places that
/// library **after** the rlib whose bundled C++ object *defines* the initializer. A linker that
/// resolves archives in one left-to-right pass has already walked past the definition by the
/// time it meets the reference, so it reports
///
/// ```text
/// call-initializers.cpp:2: error: undefined reference to 'cxx_qt_init_crate_viewer_qt'
/// ```
///
/// `lld` does not, because it keeps every archive member as a lazy symbol for the whole link and
/// fetches one when a later reference needs it — which is why this never appeared on a machine
/// that has `lld` and appeared on every GitHub runner, which does not. `qt-build-utils` picks
/// the first of `lld`, `ld.gold`, `mold` that it can run (`QtPlatformLinker::init`), so the
/// runner gets `gold` and this tree got a link error in `test` while `check` passed — `clippy`
/// links no binaries. Session 630 reproduced it here by removing `lld` from `PATH`.
///
/// `-u SYMBOL` enters the symbol as undefined at the start of the link, so the definition is
/// pulled out of the rlib when it is read rather than skipped. It is a no-op under `lld`, where
/// the symbol is resolved anyway, and it costs one linker argument.
///
/// **One property worth knowing before an upgrade**: `-u` on a symbol nothing defines fails the
/// link. So if `cxx-qt-build` ever renames its initializer, this says so at once, by name, rather
/// than quietly leaving the initializer uncalled — which is the direction to fail in.
///
/// Linux only, because the linker choice this repairs is Linux's: `-Wl,` is not Windows' spelling
/// and Mach-O prefixes an underscore to the symbol.
fn want_the_initializer_before_the_archive_is_read() {
    if std::env::var_os("CARGO_CFG_TARGET_OS").as_deref() != Some("linux".as_ref()) {
        return;
    }
    // `cxx-qt-build`'s own `crate_init_key`: `crate_` and the package name, hyphens replaced.
    let package =
        std::env::var("CARGO_PKG_NAME").expect("cargo sets CARGO_PKG_NAME for a build script");
    println!(
        "cargo::rustc-link-arg=-Wl,-u,cxx_qt_init_crate_{}",
        package.replace('-', "_")
    );
}

fn main() {
    cxx_qt_build::CxxQtBuilder::new()
        // Three modules, named one by one: `qt_module` adds an include directory per module and
        // does not follow Qt's own dependencies, so asking for Widgets alone finds `QWidget` and
        // not `QImage`. Qt Quick is deliberately absent — this is a widget host, and §12.7's
        // controls are `QLineEdit` and `QCheckBox` rather than QML items.
        .qt_module("Core")
        .qt_module("Gui")
        .qt_module("Widgets")
        .file("src/bridge.rs")
        // `cpp/` so that the bridge's own `include!("viewer-qt/host.h")` and the C++'s
        // `#include "window.h"` both resolve.
        .include_dir("cpp")
        // A header, so `cxx-qt-build` runs `moc` over it and compiles what moc writes.
        .cpp_file("cpp/window.h")
        .cpp_file("cpp/window.cpp")
        .build();

    // After `build()`, so that this argument is the last word on the subject.
    want_the_initializer_before_the_archive_is_read();
}
