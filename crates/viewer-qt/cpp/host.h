// The one function Rust calls, declared without naming a single Qt type.
//
// `#[cxx::bridge]`'s `include!` names this file, and the header cxx generates from the bridge
// includes it — so anything Qt in here would put Qt in front of the generated header and make the
// dependency circular. The Qt is in `window.h`, which only `window.cpp` includes.
//
// The forward declarations below are the bridge's own types. `Host` is the opaque Rust type;
// the five structs are the shared ones, defined in the generated header.
#pragma once

#include <cstdint>

#include "rust/cxx.h"

namespace pdf_viewer_qt {

struct Host;
struct QtRow;
struct QtFrame;
struct QtControl;
struct QtMeasure;
struct QtQuad;
struct QtUpdate;

/// Builds the window around `host`, shows it, and runs `QApplication::exec`.
///
/// `quit_after` is a millisecond count after which the application quits by itself, which is what
/// makes a run under `Xvfb` terminate; zero means never. Returns what `exec` returned.
std::int32_t run_qt_host(rust::Box<Host> host, std::int32_t quit_after);

} // namespace pdf_viewer_qt
