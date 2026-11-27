# Round 2278 — the twenty-third specification audit: clause 13, multimedia features

An audit round in the `gwg` worktree, in the shape of review 2038 and audits 2128 to 2271: every
`shall` and `should` of §13.1 to §13.5 against `pdf_model::media`, the XFDF sound writer,
`pdf-media-worker`, `viewer_host`'s sound and video players and `viewer-core`'s activation, and
§13.6 and §13.7 for what the page shows and the refusal by name. It fixes nothing.

## What it found

- `doc/reviews/2278-specification-audit-13.md` and its directory: 4 B, 0 T, 3 D.
- B03 XFDF's `alaw` sound is written with no `/E`, so Table 305's default `Raw` plays A-law codes.
  B02 a movie this program shows ignores Table 306's `/Rotate`. B04 a press on a 3D or rich media
  annotation says nothing of the excluded artwork. B01 a wrong-typed `/V`, `/P`, `/L` or `/C` in a
  rendition's `MH` criteria is read as met. D01 13.4 is `implemented` over `/Rotate`, negative
  `/Rate` and `Palindrome`; D02 `/P`'s refusal says "plays none"; D03 rich media's `Sound` and
  `Video` configurations are unblocked and their rows' reason is stale.
- §13.2.2 at every `MH`/`BE` site but B01's, the selector, the offsets, Table 286's policy, the
  sample conversion, the freeze rules and stop-on-close in every host hold.
- Four failing tests with controls: `pdf-model/tests/clause_13_multimedia_spec_audit.rs` (in
  `navigation_suite`), `viewer-core/tests/clause_13_activation_spec_audit.rs`; listed in the
  audit's `expected-failures.json` and appended to `expected-failures-current.json`.

## Ledger

Rows before and after: unchanged. The findings contradict 13.2.2, 13.2.3.1, 13.3, 13.4, 13.6.2,
13.7.2.1 and the rich media `out-of-scope` rows' reason.

**Gates.** Over `-p pdf-model -p pdf-media-worker -p viewer-core -p viewer-host -p viewer-qt -p
conformance`: fmt 1 and clippy 101 on the new tests, fixed, then 0 and 0. `pdf-sandbox` 0.
nextest `--lib --bins --tests` (`xvfb-run -a`): 4861 run, 5 failed — the manifest's four and this
record before its gates paragraph. Rustdoc of `pdf-model` and `viewer-core` 0. `cargo test -p
conformance --no-fail-fast` 0. Fuzz not owed (no public type changed).

## Habit

A comment that says what the standard's table holds is a reading of `doc/md/`, and the extraction
drops rows: ADR 2398 found Table 305's `ALaw` missing and fixed the reader, and the one writer of
the same table kept the extraction's word. When a premise about the text is found false, grep the
tree for every other sentence that states it.
