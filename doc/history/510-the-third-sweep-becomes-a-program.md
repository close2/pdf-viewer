# 510 — The third sweep becomes a program, and the generator that stamped a retired sentence back

**Finding.** A sweep round under `doc/todo/01`'s binding rule. The capability-reason sweep —
"this program has no ___", grepped by hand since session 122 — is `conformance --bin
capabilities` now, carrying the two judgements each run redid: the lacking noun grepped against
the tree with the witness path printed, and the program-versus-crate subject tell. Its first run
found §14.7.6.3 disposing of attribute revision numbers with "a processor that edits, which this
is not" — expired since session 135 made this an editing program; kept as a declined *may* on
the clause's own deprecation. The retired-claim sweep then found the ledger header's status
definition still reading "we do not create files" — the sentence session 137 amended — because
the header is *generated* and the retired sentence lived in the generator's `PREAMBLE`, which
stamped it back over the corrected file; `Exclusion::Xfa`'s doc held the XFA exclusion's
disproved pre-amendment wording the same way. A correction to generated text is not a correction
until it reaches the template. The blame band — §12.8.\*'s four freed rows plus commits 185–202 —
held the fifth failure shape at its cleanest: §9.6, §9.6.5 and §9.6.5.1 all still said a font
naming `MacExpertEncoding` "is refused and reported", fifty-nine sessions after session 451
transcribed Table D.4 (ADR 0286) and corrected §D and §D.4 one family over. All three are
`implemented` now; §12.5.6.23's redaction exclusion was re-argued on the amended exclusion's own
terms (§7.5.6's append-only update cannot express a phase whose verb is *destroy*).

**Date.** 2026-08-14.
**ADR.** [0345](../adr/0345-the-third-sweep-becomes-a-program-and-the-generator-that-stamped-a-retired-sentence.md).

**Sweep results, verbatim from the runs.**

- `capabilities` (new): ledger 47 sentences — 34 witnessed by the tree, 40 about the program,
  7 about one crate; source 142 — 116 witnessed, 78 program, 64 crate. One defect (§14.7.6.3).
- `blockers`: ledger 20 — 6 expired, 9 holding, 5 naming no clause; source 26 — 9, 10, 7.
  0 defects.
- `unread`: 63 rows claim, 172 keys; 53 confirmed, 119 quoted over 51 rows, 54 by the row's own
  code. 0 defects.
- `entries`: 232 rows in the population, 44 stating an entry their own `code` does not name,
  108 entries — 29 named nowhere, 79 only elsewhere, 37 not named by the row's own note. The
  known populations; nothing worked.
- `quotations`: 3049 quotations in 470 documents, 1441 verbatim, 22 diverging, 0 defects.
- Retired claim (4) over the twelve rounds' nouns: clean; over the retired exclusion wordings
  it paid twice — the generated header's "we do not create files" (with `doc/ledger-and-claims.md`
  recording the header as corrected all along) and `Exclusion::Xfa`'s "deprecated by ISO 32000-2
  itself and specified outside it". Caller sweep (5): 286 distinct `pub fn` names, 92 unnamed by
  hosts, 77 by nothing — the known three populations. Arithmetic (6): §7.9.2 and §O, clean.
  Inapplicable (7): 66 of 80 name source vocabulary on a session-local stop-list; no row in the
  population changed since 501's read (checked by `git diff` over status lines). Citations (8):
  3 hits, all the known correction-quoting-its-pointer shape. Table numbers (9): 1024 citations
  checked, 61 suspects, one defect (`spec_annotation_census.rs`'s "Table 353's `/MarkInfo`" —
  Table 29's entry given to the table its value points at). Parent counts (10): 4 hits, 0
  defects. Ledger quotation marks (11): 1245 spans (session-local normaliser), 695 verbatim, 34
  diverging, 0 defects. Sweep 14: 15 hits (session-local vocabulary), 0 defects. Errata (12):
  "151 struck passage(s) of 4 words or more that doc/md/ still carries as current text" over all
  fourteen PDFs, unchanged.

**Rows corrected in this commit.** §9.6, §9.6.5, §9.6.5.1 (the MacExpert refusal retired by ADR
0286, three rows one mechanism — all `implemented`, with the permitted-names test as evidence),
§12.5.6.23 (the exclusion re-argued on `CLAUDE.md`'s amended terms), §14.7.6.3 (the editing
capability acknowledged; the increment kept declined as the clause's own deprecated `may`), and
the ledger header's `writer-side` definition (regenerated from the corrected template). Kept
with evidence recorded: §7.10.2, §7.11.3, §8.6.4.4, §11.7.4.4, §12.7.8.3.1, §12.7.8.3.4,
§12.8.2.1, §12.8.4.1, §12.8.4.5, §12.8.5.3.

**Source corrected.** `tools/conformance/src/ledger.rs` (`Status::WriterSide`,
`Exclusion::WriterSide`, `Exclusion::Xfa` docs), `tools/conformance/src/bin/ledger.rs`
(`PREAMBLE`'s writer-side line), `crates/pdf-model/examples/spec_annotation_census.rs`
(Table 29/353).

**Code.** `tools/conformance/src/capabilities.rs` and `bin/capabilities.rs` (new, with eight
unit tests).

**Touched.** `doc/conformance/ledger.toml`, `doc/todo/01-ledger-partial-rows.md` (the run's
record; five commands now), `doc/todo/02-every-round.md` §4 (the new command's line),
`doc/ledger-and-claims.md` (four programs → five; the generated-header mechanism recorded),
`doc/adr/0345-*` (new), this file.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent;
`cargo nextest run --workspace` all green; `cargo test --workspace --doc` green;
`cargo test -p conformance -- --nocapture` green (5 checks). No change reaches a raster — the
one source edit outside `tools/conformance` is an example's doc comment — so the corpus and
oracle gates were not owed by this round's edits.
