# 1382 — A character the interface's faces lack is asked of the machine before it is a box

Session 1272. Status: **accepted** and built.
Context: `crates/viewer-ui/src/chrome.rs` (`Chrome::new`, `Chrome::compiled_in_only`, `Chrome::set`,
`machine_face`), `crates/viewer-ui/tests/panel.rs`, `doc/todo/27-the-interfaces-own-font.md`.
Amends: `doc/todo/27`'s "[f]alling back to a face on the machine is no longer on this list"; keeps
ADR 0133, ADR 0195 and ADR 0326.
Clauses: ISO 32000-2 §9.6.2.2, §9.7.4.2 (the search reused), Table 121, Table 124.

## 1. Why the refusal is revisited

`quorra`'s tab label drew a CJK file name as boxes. A file name is the person's own string, not a
document's, and no clause states how an interface draws it. `doc/todo/27` refused a machine fallback
because it "costs ADR 0133's argument outright — the interface would stop looking the same on two
machines". Tested against the tree, that cost is narrower than stated: the fallback is asked only
after both of `Chrome::set`'s routes into the compiled-in face have failed, so every character the
fourteen state is drawn identically everywhere, and what differs between machines is only what is a
box on all of them. The native hosts already draw these strings with the platform's font stack, which
is todo/27's own option 2; this is the same answer for the one host with no platform to ask.

## 2. Decision

- `Chrome::new` puts the machine's faces behind the four compiled-in ones. The search is
  `pdf_font::substitute::installed_covering` for the one character (§9.7.4.2's covering search), the
  face is loaded through `LoadedFont::load` from an assembled `/TrueType` dictionary whose `/FontFile2`
  holds it (`LoadedFont::standard`'s route, so there is no second font reader), and every answer,
  including "none", is kept per character and style. Faces already found are asked first.
- Where the machine offers no face the character is the box, as before, and nothing else is said.
  U+FFFD stays a box: it is §7.9.2.2's report about the file.
- `Chrome::compiled_in_only` is the chrome whose every pixel is the binary's, for callers that
  measure the compiled-in faces.
- A test needing a machine face skips when the machine offers none (ADR 1154).

## 3. Cost

The first label holding a character the fourteen lack walks the machine's catalogue once (the search
is memoised; ADR 0152 measured 215 ms cold). A label the fourteen cover never asks, so the launch path
of a Latin-named file is unchanged; a CJK-named one pays the walk before its first strip is drawn.
