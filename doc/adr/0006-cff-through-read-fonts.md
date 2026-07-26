# ADR 0006 — Bare CFF is read through `read-fonts`, not parsed in-tree

Status: accepted, 2026-07-26. Supersedes the in-tree sfnt synthesis added in `1aa702d`.

## Context

`/FontFile3` may hold a bare CFF font program rather than a complete `OpenType` file. A
CFF has no `cmap` table, so a character code does not reach a glyph the way it does in a
`TrueType` font. Two routes exist, and which applies depends on the font:

- A **name-keyed** CFF names every glyph in its *charset*. A code becomes a glyph name —
  through the PDF `/Encoding`, or through the encoding the font itself carries — and the
  name becomes a glyph index.
- A **CID-keyed** CFF has no glyph names. Its charset assigns a CID per glyph, and a code
  reaches a glyph by inverting that.

The previous session implemented half of the first route: `cff::wrap_in_sfnt` synthesised
a minimal `OTTO` container so that `skrifa`'s `FontRef` would accept the program, with
`head` and `maxp` tables built by hand and the glyph count read out of the `CharStrings`
INDEX. It worked, was tested, and was deliberately kept off the loading path, because a
synthesised container has no `cmap` and lookup would therefore fall through to treating
the character code as a glyph index — loading cleanly, reporting nothing unsupported, and
drawing the wrong glyphs. The handover recorded that as the single most valuable next
task, estimated as roughly 80 lines of charset parsing plus two 256-entry encoding tables.

Completing that route as described would have meant writing, in this tree: the charset
formats 0, 1 and 2; the encoding formats 0 and 1 with their supplements; the String INDEX;
and Adobe's 391 standard strings, transcribed by hand.

## Decision

Do none of that. `skrifa` re-exports `read-fonts` as `skrifa::raw`, and
`read_fonts::ps::cff::CffFontRef::new_cff` parses a **bare** CFF directly. It already
provides every structure listed above, plus CID-keyed detection, the charset inverse, and
`draw` — so it produces outlines from the bare program with no container at all.

`crates/pdf-font/src/cff.rs` was therefore rewritten as a thin adapter over
`CffFontRef`, and `wrap_in_sfnt` and its supporting INDEX and DICT byte parsing were
deleted. What remains in-tree is only the PDF-specific part: choosing which route applies,
and resolving `/Encoding` — a base encoding plus `/Differences` — onto the font's charset.

## Consequences

The clear benefit is that untrusted font bytes are parsed by a fuzzed, memory-safe,
maintained library instead of by code written here. That is the same reasoning that chose
`skrifa` over `FreeType` in the first place; applying it to `skrifa` and then hand-rolling
CFF parsing beside it would have been inconsistent. It also removes a 391-entry table that
would have been transcribed from memory, where a single wrong entry maps a real document to
a real but wrong glyph — silently.

The net change is a **reduction**: `cff.rs` went from 364 lines of byte parsing to 194
lines of adapter, while covering strictly more (CID-keyed fonts, encoding supplements,
custom charsets) than the hand-written version was ever going to.

The cost is a deeper dependency on `read-fonts` internals. `skrifa::raw` is a documented
re-export and `ps` is public and unconditional, but it is a lower-level surface than
`skrifa`'s own API and may move faster between versions. The mitigation is that the
corpus tests exercise it against real documents on every run, so a breaking change fails
loudly at compile or test time rather than quietly changing glyph output.

A second cost: `CffFontRef::new_cff` re-parses the program on each outline extraction,
because it borrows and so cannot be cached in `LoadedFont`. Outlines are already cached
per glyph, so this happens once per distinct glyph rather than once per character drawn,
and the corpus fonts are 0.5–5 KB. No measurement has yet shown it to matter; if one does,
the fix is to cache the parsed metadata rather than to reinstate the container.

## What stayed in-tree, and why

PDF's `WinAnsiEncoding` and `MacRomanEncoding` are in `crates/pdf-font/src/encoding.rs`.
`read-fonts` has no reason to carry them — they are PDF's Annex D, not CFF's — and they
are genuinely PDF-specific: `MacRomanEncoding` is *not* Mac OS Roman, omitting sixteen
mathematical and symbol glyphs and keeping code 219 as `currency` where Apple reassigned
it to the euro.

They were extracted from `doc/md/ISO_32000-2_sponsored_EC3.md` rather than written from
memory, including the assignments that appear only in Table D.2's notes: `space` at 160
(WinAnsi) and 202 (MacRoman), `hyphen` at 173, and every otherwise unused WinAnsi code
above 32 falling to `bullet`. `StandardEncoding` is not transcribed at all — it is the
same table as the CFF standard encoding, which `read-fonts` carries.

A test cross-validates the result against a second, independent document: every glyph name
in both tables must appear in Adobe's CFF standard strings, with `Euro` the single
documented exception. The two encodings were derived from ISO 32000-2 and the standard
strings from Adobe TN 5176, so a transcription slip has to survive both to get through.
