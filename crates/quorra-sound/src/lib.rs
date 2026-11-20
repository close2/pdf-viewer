//! The audio output a window plays ISO 32000-2 §13.3's sounds on, in a helper program of its own.
//!
//! One responsibility: *carry samples a window has already decoded and converted to a process that
//! plays them on this machine's audio device*. The window links no audio library: the C under the
//! device (`cpal` reaches ALSA through `alsa-sys`, which links `libasound` at load time) is linked
//! into the helper alone, so a window maps nothing for sound at its start and starts on a machine
//! that has no audio library at all (ADR 3317, `doc/questions/Q972`). Two halves:
//!
//! - **The window's half** ([`Helper`]) is started by the window's player at the first sound the
//!   reader's level lets play, never at open (`CLAUDE.md`'s startup rules, RFC 0015 section 4.6).
//!   It spawns the program, reads the device's format from it, and hands it each sound.
//! - **The helper's half** ([`serve`]) opens the device first, says its format or why it could not
//!   be opened, then plays each sound it is handed in place of the one before, until its input
//!   ends — which is the window closing the pipe, or the window's process having gone.
//!
//! [`wire`] is what crosses between them. **Nothing a document wrote crosses it**: the samples were
//! decoded in the confined media worker and converted by `viewer_host::sound`, so the helper parses
//! no byte of a document, which is the standing ADR 3297 gave the C on the output side.
//!
//! # Why a program and not a library loaded at the first sound
//!
//! `alsa-sys` names `libasound` to the linker; there is no switch in it to open the library at the
//! first use instead, as `winit`, `wgpu` and `softbuffer` open theirs. A program of its own is the
//! construction that keeps the link and moves it out of the window (ADR 3317).

#![forbid(unsafe_code)]

mod client;
mod helper;
pub mod wire;

pub use client::{Helper, HelperError};
pub use helper::{Device, serve};
pub use wire::Format;

/// The helper program's name, which is also the name its diagnostics are reported under.
pub const HELPER_PROGRAM: &str = "quorra-sound";

/// The variable that names the helper program where it is not beside the running executable.
pub const HELPER_PATH_VARIABLE: &str = "QUORRA_SOUND";
