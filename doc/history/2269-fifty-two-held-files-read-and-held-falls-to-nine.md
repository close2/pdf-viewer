# 2269 — Fifty-two held files read, and `HELD` falls to nine

Lane `gwg`, the silent-repair census burn-down, tenth pass (ADRs 3477, 3521, 3765). ADR 3777.

- **New classes**, each section 5.1 alone: `model/requirement-entry-unusable` (Table 273's
  `/Penalty`), `model/media-criterion-unusable` (Table 279's `/L`), `font/opentype-cff-unreadable`
  (a `Type1` OpenType program whose `CFF ` table does not read, mapped by its `cmap`).
- **On existing classes**: an unpaired surrogate in a `/ToUnicode` destination now maps nothing
  and damages the map, in all three statements (`font/to-unicode-damaged`); a `/Style` or `/Panose`
  of another shape (`descriptor-style-unusable`); `/Opt`, `/MaxLen`, `/Ff` in the view and the
  scripts (`field-entry-unusable`); Table 212's `/Volume` and flags, said only in notes before
  (`action-entry-unusable`); PrintField `/Checked`, `/TextDecorationType`, `/TextPosition`
  (`structure-attribute-unusable`); a script's `/Rect` (`annotation-entry-unusable`); an appearance
  stream's `/Resources` of another type (`form-resources-not-a-dictionary`).
- **Marked**, each against its source or its function's doc: catalog declines, this reader's
  slices and counts, named declines, the `Tf`/`sh` operand shortfalls the loop and the survey log,
  a field with no value as a script's empty text, XFA's omitted `xdpContent`.
- `HELD` 74 → 9: the 52 files read hold none; the nine are rounds 2267's and 2268's. ADR 3777
  argues the ratchet at zero: when those fall, `HELD` is deleted, not kept empty.

- `tools/state.sh --round 2269 launch`: 6 documents, 36 figures banded, 0 outside.
- Instructions, one rayon thread, `callgrind_interpret` against main's `41c9f29d1`: ISO 32000-2
  page 101 1 439 396 101 → 1 435 379 758; `ArabicCIDTrueType` page 1 6 864 746 → 6 863 479;
  `bug1937438_from_word` page 1 6 420 748 → 6 419 881. No rise.
- **Census, a movement, and it is the `/ToUnicode` log**: identity maps stating `<d800> <d8ff>
  <d800>` and three like it (surrogate destinations for codes no string shows) now log the map
  damaged, refused by section 5.1 (ADR 3570 point 5). With the pack, as if printed: 2b 938 → 929
  converted (`basicapi`, `bug1186827`, `bug1473809`, `issue12963`, `issue18099_reduced`,
  `issue2074`, `issue2956`, `issue5677`, `mixedfonts`), corpora 456 → 455 (`499039`); either part,
  eight pdf.js and `499039` written at 4 instead, `issue18099_reduced` refused at both. As shown:
  2b 911 → 903, 415 → 414. Without the pack, the same documents. veraPDF 1.30.3 over 5503
  outputs (2266's 5523 less the twenty): 40 not compliant, 12 unfinished, every line as 2266's.
- Left, in order: ADR 3570's unbuilt route (write a damaged map's mappings that read as a new
  `CMap`), which wins those back; the nine held sites with their rounds; the ratchet at zero.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-font -p pdf-archive -p pdf-transform -p
viewer-qt -p conformance`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0,
nextest `--lib --bins --tests` (`xvfb-run -a`) 6329 of 6332 — the manifest's two §10.5 entries and
a test of this round's, fixed and re-run (38 of 38); rustdoc of the four library crates 0;
`cargo test -p conformance` 0. No public type changed, so no fuzz check.
