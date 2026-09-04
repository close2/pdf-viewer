//! Font loading and glyph outline extraction.
//!
//! Maps a PDF font dictionary plus a character code to a glyph outline, applying the
//! encoding the document specifies.
//!
//! Outline extraction is delegated to `skrifa`, a memory-safe replacement for `FreeType` —
//! historically a steady source of vulnerabilities in every viewer that used it.
//!
//! # Text shaping is deliberately absent
//!
//! A PDF content stream carries glyphs the producer already positioned. Re-shaping them
//! would move glyphs away from the coordinates the document specifies, and would do so most
//! visibly on the complex-script documents where shaping seems most helpful. See
//! `doc/stack.md` on `rustybuzz`.
//!
//! # What is implemented, and what says so
//!
//! Embedded `TrueType` and CFF outlines, for simple fonts and for composite (Type0) fonts
//! under the Identity encoding or an embedded `CMap` (§9.7.5.3) — which between them cover
//! the overwhelming majority of modern documents. A font this crate cannot load returns an
//! error naming why, so the caller reports the text as undrawn rather than silently omitting
//! it. Table 116's predefined `CMap`s are carried as data (see [`predefined`]) rather than
//! refused, which is where this paragraph said the gap was for hundreds of sessions after the
//! hundred-and-fifty-sixth closed it.

#![forbid(unsafe_code)]

pub mod cff;
pub mod cmap;
pub mod collection;
mod composite;
pub mod encoding;
#[cfg(test)]
mod fixture;
mod glyph_names;
mod loading;
mod metrics;
pub mod name_keyed;
pub mod panose;
pub mod predefined;
mod program;
pub mod provider;
mod sfnt;
pub mod standard;
pub mod standard_metrics;
pub mod substitute;
mod substituted;
pub mod tounicode;
mod truetype;
pub mod type1;
mod vertical;

pub use crate::cmap::Code;
pub use crate::loading::{CharacterGlyph, FontError, LoadedFont, Meaning, NOTDEF_GLYPH, NamingGap};
pub use crate::metrics::measured_extent;
pub use crate::sfnt::repaired_font_program;
