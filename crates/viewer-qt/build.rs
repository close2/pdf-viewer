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
}
