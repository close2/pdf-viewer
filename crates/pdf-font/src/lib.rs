//! Font loading and glyph outline extraction.
//!
//! Maps a PDF font dictionary plus a character code to a glyph outline, applying
//! the encoding, `Differences` array, and `CMap` that the document specifies.
//!
//! Outline extraction for OpenType, TrueType and CFF is delegated to `skrifa`, a
//! memory-safe replacement for `FreeType` — historically a steady source of
//! vulnerabilities. Two PDF font types fall outside skrifa's scope and are handled
//! here: Type1, which is legacy but still present in older documents, and Type3,
//! whose glyphs are arbitrary content streams and so are rendered rather than
//! loaded.
//!
//! Text shaping is explicitly *not* performed. A PDF content stream carries glyphs
//! that the producer already positioned; re-shaping them would move glyphs away
//! from the coordinates the document specifies, and would do so most visibly on
//! complex-script documents. See `CLAUDE.md` on `rustybuzz`.
//!
//! Implemented after Phase 5.

#![forbid(unsafe_code)]
