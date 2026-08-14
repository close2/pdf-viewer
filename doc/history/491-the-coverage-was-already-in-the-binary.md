# 491 — The coverage was already in the binary

**Finding.** `doc/todo/27` asked for *coverage* for the interface's own font and priced three
answers, each costing a licence, a megabyte or ADR 0133's reproducibility. None of them was
needed for most of it: the panel asked the compiled-in Helvetica for a character **code**, a
simple font's codes are one byte, so the route reached the 149 characters §9.6.5.2's
`StandardEncoding` names — while the face behind it is Liberation Sans and states 668. This
program had been drawing a placeholder box for `é` in a face that has one. A character with no
code is now looked up by character (`LoadedFont::character_glyph`), and of the 54 corpus documents
whose panels lost a character, **41 lose nothing at all**. 130 of the 144 characters recovered are
Latin-1 Supplement.

**Date.** 2026-08-14.
**ADR.** [0326](../adr/0326-the-coverage-was-already-in-the-binary.md).
**Touched.** `crates/pdf-font/src/loading.rs` (`CharacterGlyph`, `LoadedFont::character_glyph`,
`cached_outline`, one test), `crates/pdf-font/src/lib.rs` (the re-export),
`crates/viewer-ui/src/chrome.rs` (`Set::Character`, the module header, three stale doc comments),
`crates/viewer-ui/tests/panel.rs` (one test),
`crates/pdf-model/examples/interface_font_census.rs` (new), `doc/verify.md`,
`doc/conformance/ledger.toml` (§9.6.2.2 and §14.3.3), `doc/todo/27-the-interfaces-own-font.md`
and its `doc/todo/README.md` index line, `doc/adr/0326-*` (new), this file.

## What the census is, and why it is a second one

`pdf-model --example interface_font_census` opens every corpus document and takes the seven
populations a program draws *from* a document — §12.3.3's outline titles, §8.11.4.3's layer names,
§7.11.4's file names, §14.3.3's `/Info`, §14.3.2's XMP, §12.4.2's page labels and §12.5.6.14's
popup text — asking each character of **both** routes into the compiled-in face.

It does not go through `Chrome`, and that is the point rather than an oversight.
`viewer-ui --example chrome_coverage` measures what this host's own `Chrome` can set, which makes
it the instrument under test for exactly this question; a census whose predicate is the thing
being checked measures the instrument (`doc/HANDOVER.md` trap 8, and ADR 0215's shape). The two
examples ask different questions of different denominators and both are kept.

The cross-check that says the new one is right: §14.3.3's `/Info` loses 196 characters by the code
route here, and ADR 0195's independent count through `Chrome` was 195/196.

## What is left, and what it is worth

Thirteen documents, which the census names one by one: 213 characters of Hebrew, all in
`issue14046.pdf`; 81 of Thai, all in `issue13211.pdf`; 85 of Japanese and Chinese over six
documents; and 77 of U+FFFD, mostly in `bug1146106.pdf` — which writes its text strings as UTF-16
little-endian, so the boxes there are §7.9.2.2's undefined code point being *reported* rather than
a script this binary lacks. `doc/todo/27` stays open on the two answers that remain — compile in a
face, or let a native host draw the row — with a CJK face now buying six documents in 964 and a
Hebrew or Thai one buying a single document apiece.

## What the next round should know

- **The chrome is drawn through `pdf-render` display lists, and none of it is page content.** The
  corpus gate and the oracle were run to say so and are identical; that is the check the change
  had to pass, not a hope.
- **A capability can arrive and announce nothing, one directory over from the ledger.** The
  fourteen faces were compiled in in the hundred-and-forty-eighth session for reproducibility, and
  nobody afterwards asked what their character sets contained. `doc/habits.md`'s ledger section
  names that shape for a *row*; this was the same shape in an interface, and the tell was the same
  — a refusal whose stated reason ("the fourteen are Latin") had stopped being true.
- **`LoadedFont::character_glyph` is deliberately not reachable from the interpreter's route.** A
  document selects glyphs by code, and drawing one the file did not select would be inventing what
  the page says. Its doc comment says so where the next reader will be tempted.
