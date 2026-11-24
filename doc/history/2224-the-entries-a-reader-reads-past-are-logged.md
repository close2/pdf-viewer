# 2224 — The entries a reader reads past are logged by the table that gives them

Lane `gwg`, the silent-repair campaign's fix round 6 (ADR 3477), on round 2218's census. ADR 3597.

- **The census.** Four Sonnet helpers read the sites round 2218 left in `xref.rs`, the decoders,
  `tree.rs`, `text_string.rs`, `lexer.rs`, `object_streams.rs`, sixteen `pdf-font` files,
  `appearance.rs`, `annotation.rs`, `image/unpack.rs`, `viewer_preferences.rs`, `action.rs`,
  `forms_data.rs` and `form.rs`: comment edits only, a site marked where it is no file's value, the
  clause's default, or a repair already logged out of the window (the class named beside it). The
  round removed one marker (an unknown line ending reported only through `owed`).
- **Classes** (ADR 3597): five for entries of the wrong type or value, one per table family
  (catalog display, viewer preference, action, annotation, field); T05's remainder (a form's
  `/Resources`, the alpha constants, a halftone, its `TransferFunction`); `font/wmode-unusable`.
  `/FirstChar` missing beside `/Widths` is `font/widths-malformed`; an object stream read for its
  members logs `syntax/object-stream-header` as its expansion does. All section 5.1 alone.
- **Not a repair**: `/PrintScaling`'s unrecognised value is the clause's `AppDefault`.
- **The three documents**, read with a scratch example printing each repair's site: 050734's
  blockers are `syntax/string-unterminated` in pages 53 and 59 (the eighteenth and twentieth of 22,
  both in the tree), which the photograph survey is asked for and does not take; `issue17679_2`
  is `model/operand-of-the-wrong-type`, `issue6342` `model/operator-short-of-operands` beside
  `syntax/number-salvaged`. None is answered here.
- Left: the truncated `hmtx`, the UTF-16 surrogates, the unpackers' short data, `forms_data`, 32
  sites of `appearance.rs`, and an `Indexed` sample past `hival` painted black where §8.6.6.3
  adjusts it to the nearest index — a pixel change for its own round.
- **Census**, with and without the pack: every line as round 2220's, none newly refused or
  converted. veraPDF 1.30.3 over
  5493 outputs: 40 not compliant and 12 unfinished, the same list as round 2222's.
- Habit: a helper's "case 3" (logged elsewhere) is the cheapest marker to write and the easiest to
  get wrong; read the path that records before accepting it.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-font -p pdf-archive -p pdf-transform -p
viewer-qt -p conformance`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0,
nextest `--lib --bins --tests` 0 (6165 of 6165; the manifest is empty), rustdoc 0, fuzz 0;
`cargo test -p conformance` 0. `tools/state.sh --round 2224 launch` queued past the wall budget
behind the post-merge train's walk, so the launch counts are owed to the merge.
