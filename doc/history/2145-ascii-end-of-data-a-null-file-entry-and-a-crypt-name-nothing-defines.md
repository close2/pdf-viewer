# Round 2145 — ASCII end-of-data, a null `/F`, and a crypt filter name nothing defines

A fix round in the `audit` worktree for audit 2142's groups B, D and E (§7.4): B02, B06, B07, D02
and D03, and the `pdf-model` corpus ratchet red on main.

## What changed

- ADR 3281 (B02): `ascii_hex`, `ascii85` and the windowed `AsciiHex` / `Ascii85` stages report
  `Damage::Truncated` for data that ends before `>` / `~>`; the odd digit is not padded and the
  partial group is not flushed at a cut, since §7.4.2 and §7.4.3 state both rules at the marker.
- ADR 3282 (B06): `Document::is_external` takes the document and reads a null `/F`, direct or a
  reference to an undefined object, as absent (§7.3.9, §7.3.10). Public signature; the callers in
  `serialize.rs` and `pdf-transform` changed with it.
- ADR 3283 (B07, D02): in a document with no encryption dictionary, a `/Crypt` stage whose `/Name`
  is not `Identity` marks the stream `decryption_failed` when it is read, as ADR 3233 does in an
  encrypted one. Ledger 7.4.10's "commutes with everything" is restated as a choice.
- ADR 3284 (D03): `jbig2_bilevel` reports a `/JBIG2Globals` that is not a stream; a null one is
  absent.
- The corpus ratchet: `INCOMPLETE` is held by name and admits a document that joins with a new
  report beside its clause, so `issue17929.pdf` (a font outside the fourteen spelling the key
  `/GontDescriptor`, ADR 3119's report, already the file's in `whose_defect`) is admitted under
  Table 109. ADR 3281 adds `PDFBOX-3148-2-fuzzed.pdf`, whose widget appearance is `/A85` with
  no `~>` and is now reported cut; the list is 67, every other name unchanged.

**Ledger.** 7.3.8, 7.3.8.2, 7.4.2, 7.4.3 and 7.4.10 stay `implemented`, each with the audit test
and a sentence; 7.4.10's note is restated (D02).

**Gates.** fmt exit 0; clippy `-D warnings --all-targets` over `-p pdf-syntax -p pdf-model -p
pdf-transform` exit 0; `pdf-sandbox --bins` exit 0; nextest `--lib --bins --tests` of the three
(`xvfb-run -a`): 4592 run, 4592 passed ("failed 0, expected 0"; the manifest is empty); rustdoc
exit 0; fuzz `check --bins` exit 0; conformance exit 101 on the first run (this record had no
Gates paragraph yet), exit 0 on the second. The `corpus` walk: before (main, the train
at `fcd1d05b`) 66 incomplete against 65; after, 67 named, exit 0 (`PDFBOX-3148-2-fuzzed.pdf`
joined by this round's ADR 3281, read, and admitted).

**Left.** A `/DecodeParms` array shorter than its chain still gives the stages past its end their
defaults (ADR 3274's recorded choice); audit 2142 has no open finding.

**Habit.** A rule keyed on a name (§7.3.9's null, a crypt filter name) is applied wherever
the name is read, and that includes the document that lacks the dictionary the name points into.
