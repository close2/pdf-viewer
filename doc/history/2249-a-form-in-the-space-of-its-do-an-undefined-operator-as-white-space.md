# 2249 — A form in the space of its `Do`, an undefined operator as white space, and the archive's reals through one formatter

Lane 3, round 2244's PDF/A leftovers. ADRs 3697–3699.

- **ADR 3697**: §8.10.1 starts a form in "the graphics state that is in effect at the time Do is
  invoked"; the grammar (`Grammar::starting_in`), the survey (`State::spaces`, `OpenedStream::began_in`)
  and `as_drawn`'s walk now start a form there, a glyph description at its show, a pattern cell where
  its parent began, a page at §8.4.1. The poster's form needed no repair. Its soft-mask group's
  `0 0 0 scn` did, and the interpreter logged it at the page (a `SoftMaskRequest` held `/G` resolved);
  it is logged at the group now, where ADR 3679's `rg` answers it. A stream begun in two states is
  judged in neither (`Spaces::UNCOUNTED`).
- **ADR 3698**: ADR 2859 generalised. A keyword no table defines, with no operand before it, is
  written as white space under ADR 3558's proof, only where every operator of 6.2.2's row is then
  answered, and not counted by `wrote_out_of` (the trap ADR 3679's withdrawn write met). It does not
  end a text object ADR 3661 adds. `sci-notation`'s `ETBT` was inside its producer's text object.
  A fixture's `) Frobnicate` became `) 1 Frobnicate` to stay unanswered.
- **ADR 3699**: `derived.rs`'s matrix, `preserve.rs`'s text page and §12.5.5 matrix and the photograph's
  placement go through `archive::operand`/`widened_operand` (`real_number`), refusing a non-finite
  number by `NUMBER_NOT_FINITE`.
- **Not done**: the data-extent and stray-hex writes (ADR 3680's carriers hold other classes); closing
  an open section; 2240's `Td` across a cut.
- **Probes** (both parts, `as-shown` and `as-if-printed`): `sci-notation` and the `MICOM` poster convert
  with no page photographed; `050734` and `504076` convert under `as-if-printed`, photographed, and
  stay refused under `as-shown`. veraPDF 1.30.3: all twelve outputs compliant.
- **Census** (pack). Opening, main's tree, against 2244's log: only `050734` moved — it converts under
  `as-if-printed` at both parts and `as-shown` refuses it by `marked-content-unbalanced`; `504076`
  converts under `as-if-printed`. Closing against opening: `as-if-printed` unchanged (pdf.js 939,
  corpora 462 at 2b); `as-shown` pdf.js 908 → 912 written at 2b, 39 → 35 refused at both, corpora
  419 → 421, 48 → 46: `MICOM`, `T02-05-01_009`, `issue17554`, `issue2391-1`, `issue9252` and
  `sci-notation` convert, nothing moved against. veraPDF 1.30.3: their 12 outputs compliant. A first
  closing run, before ADR 3698 item 5, also wrote `issue19484_2` (an encrypted stream decrypting to
  noise) into an output failing 5.1; the guard returns it to main's answer.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-archive -p pdf-transform -p viewer-qt -p conformance`:
fmt 0; clippy `-D warnings --all-targets` 0; sandbox `--bins` 0; nextest `--lib --bins --tests` (`xvfb-run
-a`) 5801 of 5804, the three `pdf-syntax` audit tests the manifest lists; rustdoc `pdf-model`,
`pdf-archive`, `pdf-transform` 0; fuzz 0; `cargo test -p conformance` 0. The first run's two
conformance failures were this record's placeholder and a quotation attributed to §8.4.1; its clippy
101s (a function past 100 lines, three `assert!` on emptiness, a fourth bool) were fixed before it.
