# 480 — The nine entries a group shares, and the window that came up blank

**Finding.** `doc/todo/01`'s fifteenth sweep, re-run: 182 rows in the population, 24 stating an
entry their own `code = [...]` does not name, 43 entries. Its strongest hit outside §14.8 and
clauses 8–11 is **§12.5.6.2's `/IRT` and `/RT`**, disposed of by a row that says they "reach a
comments pane rather than a raster" — true of a reply relationship and a reply type, and the
reason nobody read the paragraph four below Table 172, where the same two entries make a **group**
and give it nine shared entries. `/C` is ink and `/Contents` is what §12.5.6.6 draws, so the
sentence was wrong about two of the nine and silent about the rest. A subordinate annotation's own
entries were read from the subordinate and nothing said so.

**Date.** 2026-08-13.
**ADR.** [0315](../adr/0315-the-nine-entries-a-group-shares-and-the-window-that-came-up-blank.md).
**Touched.** `crates/pdf-model/src/markup.rs` (new, `group_source` and four tests),
`crates/pdf-model/src/popup.rs` (`read`, `popup_of`, `opens_with_the_page`, the module header, two
tests), `crates/pdf-model/src/appearance.rs` (the seven markup `/C` reads and free text's
`/Contents`), `crates/pdf-model/src/lib.rs` (one module line),
`crates/pdf-model/examples/annotation_group_census.rs` (new),
`crates/viewer-core/src/open.rs` and `interact.rs` (`popup_of`'s new argument),
`doc/conformance/ledger.toml` (§12.5.6.2, §12.5.6.4, §12.5.6.14),
`doc/todo/01-ledger-partial-rows.md` (the sweep's second run and what it taught),
`doc/adr/0315-*`, this file.

## What the round verified rather than assumed

- **Counted before believed** (trap 11). `examples/annotation_group_census` over the 964 openable
  documents: **one** `/IRT` in 34 835 annotations — `issue13447.pdf`, a strike-out grouped with the
  caret that replaces it, whose primary states the same `/C`. Zero across the other three corpora
  (273 documents). So the corpus **cannot rank this rule**, and the fixtures are a pair differing
  only in `/RT` (trap 8's fourth shape). Each half was watched fail with the rule removed: four
  tests, all four red.
- **The witness is ISO 32000-2's own PDF**, for the second time in twenty sessions. 2074 `/IRT` in
  11 462 annotations, 322 `/RT /Group` with every primary on the same page, 323 entries disagreeing
  with the primary — `/Popup` 213, `/RC` 109, `/M` 1 — and 1752 replies each naming a popup of its
  own.
- **The effect was measured end to end with an example that already existed.**
  `examples/spec_annotation_census`: **1535 of its 2552 windows carried text before, 1748 after** —
  exactly the 213 that hang off a subordinate. Each is an editorial change where the caret carries
  the replacement text and the strike-out a reader clicks carries `<p></p>`.
- **The conversion was checked before the clause was trusted.** `pdftotext -layout` over
  `doc/ISO_32000-2_sponsored_EC3.pdf` agrees with `doc/md/` word for word here; the only difference
  is the spaces the conversion puts around an italicised key name, which is why the quotations read
  `Group .` and `Open .`
- **The rule moves nine entries and no others**, which is a test of its own: a subordinate keeps
  its `/Rect`, `/Subtype`, `/F` and `/AP`, and `square_or_circle` takes `/C` from the group and
  `/IC` from the annotation in adjacent lines.
- **Run in the real window under `Xvfb`**, because no gate here sees a host and a popup is drawn by
  `viewer_ui::chrome::popup_windows`. A fixture whose strike-out states `/T (nobody)`, `/C [0 1 0]`
  and no text, grouped with a caret that carries the words: the window came up titled **the
  editor**, bodied **the primary's words**, with a **blue** title bar, and the strike-out on the
  page drawn blue rather than green. Four of the nine entries in one photograph.
- **Gates.** `fmt`, `clippy --workspace --all-targets` (silent), `nextest` 1729 passed / 11 skipped,
  the doctests, the corpus gate, the oracle, both text gates, dates, XMP, JPEG 2000, the quorra
  corpus gate and `cargo test -p conformance` — all pass, none moved.

## What the next round should know

- **The reply half is still owed and is a panel.** "[I]nteractive PDF processors shall not display
  replies to an annotation individually but together in the form of threaded comments", and this
  hands a host 1752 separate windows on ISO 32000-2. It is the same missing panel round 460 named
  for §12.5.6.15's attachments, and §12.5.6.2's row is what keeps it `partial`.
- **The sweep's list is shorter but its hits need a different question.** Question two prints an
  entry the row's own files do not name; what decides whether that is *work* is whether the row's
  disposal of the entry is a claim about the entry or about the clause. Both kinds pass the grep and
  only the first is legible as wrong.
- **`/DS`, `/Subj` and `/CreationDate` have no reader in this tree**, so three of the ten group
  attribute keys are named in `crate::markup`'s header and wired nowhere. Whoever adds one finds the
  rule beside the key.
