# 2279 — A movie is turned by its /Rotate, and a press on 3D artwork is answered

Lane `gwg`: review 2278's four fix groups (clause 13). ADRs 3817 to 3820; no question.

- **B01 + D02** (`pdf-model` `media.rs`): §13.2.2's "unrecognised value for a recognised key" — an MH
  `/C` that is no dictionary, a Table 279 `/V`, `/P` or `/L` that is no array, an `/L` element that is
  no text string — makes the rendition non-viable; in BE each is read past and logged (§13.2.3.1).
  `/P`'s sentence now says this program plays the multimedia constructs but supports no version
  fully (the 13.2.5 row is partial). Test `a_criterion_of_the_wrong_type_is_non_viable_in_must_honour_only`.
- **B03**: XFDF `encoding="alaw"` writes `/E /ALaw` (printed Table 305, ADR 2398). The habit's grep
  found one other sentence repeating `doc/md/`'s three-row table: ADR 1297's XFDF table, amended above it.
- **B02** (ADR 3817): `/Rotate` is `Playing::quarter_turns` and `Event::Poster::quarter_turns`, crossing
  the confined protocol; `viewer_host::video::placed_turned` turns the placement's corners, so the
  device's affine and quorra's quad need nothing more, and `Placed::part_of` turns the samples for the
  upright windows (GTK, Qt, C ABI), `is_whole` false when turned. A value no multiple of 90 is said.
- **D01** (ADR 3818): a negative `/Rate` and `Palindrome`'s backward half are not built; the 13.4 row is
  `partial`, both planned in `doc/todo/80` section 2 and listed in `doc/todo/65` bucket 4.
- **B04** (ADR 3819): a press on a 3D or rich media annotation reports that its artwork is not shown and
  the normal appearance stays (`interact::deprecated_media::exhibited`).
- **D03** (ADR 3820): the 21 rich media notes no longer say "no round has planned"; `Sound` and `Video`
  follow the built players and are planned (`doc/todo/80` section 1, five rounds), not built: they need
  §13.7.2.2's activation, presentation and instance matching, which the players do not carry.
- 2278-B01..B04 left `expected-failures-current.json`; B01–B04 and D01–D03 are `fixed` in 2278's findings.
- Ledger: 13.4 `implemented` → `partial`; 13.2.2, 13.2.3.1, 13.3, 13.6.2, 13.7.2.1 stay `implemented`
  with tests added; the rich media rows stay `out-of-scope`, their reason restated.
- Left, in order: `doc/todo/80` section 2 (backward play), then section 1 (rich media's five rounds).

**Gates.** Over `-p pdf-model -p pdf-media-worker -p viewer-core -p viewer-host -p viewer-confined -p
viewer-qt -p viewer-ui` (clippy also `-p viewer-gtk -p viewer-ffi -p conformance`): fmt 0, clippy `-D
warnings --all-targets` 0, `pdf-sandbox --bins` 0, nextest `--lib --bins --tests` (`xvfb-run -a`) 4661
of 4661, failures equal to the manifest (none), rustdoc `-D warnings --document-private-items` (six
crates, private directory) 0, fuzz `cargo check --bins` 0, `cargo test -p conformance --no-fail-fast`
0 (485 passed). Launch (`tools/state.sh --round 2279
launch`, after the rebase onto `f4576bae9`): 101, one figure of 36 outside — `PDF20_AN001-BPC.pdf`
`peak_anon_mib` 6.629 in 4.000 .. 6.500 (the last reading was 5.992). No changed line runs when that file
opens and draws page one; the comparison against `f4576bae9` is owed before the merge.
