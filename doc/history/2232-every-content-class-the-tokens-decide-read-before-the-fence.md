# 2232 — Every content class the tokens decide read before the fence

Lane 3. ADRs 3629, 3630, 3631.

- **The trap** (round 2228): the converter's fence reads the repair log the survey's token walk
  filled, and the photograph step interprets every page afterwards, so a class only the
  interpreter logs arrives after the decision. `pdf_model::content::grammar` now reads every
  class a token decides (ADR 3629): `model/operands-left-over` beside the operator's own class
  (`Grammar::operator` answers a `Found`), counted as the interpreter counts — an array's
  brackets and elements one each, a keyword inside an array stepped over (§7.3.6), §8.6.8's
  colour operators ignored after `d1` and in an uncoloured tiling pattern — and
  `model/property-list-unresolved` for a well-typed `BDC`, the survey resolving the name. The
  survey also logs `model/content-past-a-bound` where its inline-image scan stops at the bound.
  The inline-object and inline-image classes were already read: both walks run the same reader.
- **The gate** (ADR 3630): `grammar::COVERAGE` names who reads each class logged inside content
  — grammar, shared reader, or interpreter with a reason (the five graphics-state-dictionary and
  three form-dictionary classes: facts about the object a name resolves to, no token states
  them). `tools/conformance/tests/content_classes.rs` holds it to the sources (planted, it fails).
- **The converter** (ADR 3631): `operands-left-over` and `marked-content-unbalanced` join the
  classes written out where the page draws the same; an `EMC` closing nothing is written out
  alone. A well-typed `BDC` naming nothing is answered by ADR 2684's definition of the name, not
  written out. `a_widget_the_producer_gave_an_appearance…`'s appearance was `1 0 g`, a leftover
  the walk now finds; the fixture says `0.5 g`.
- **The documents**: `issue15629` converts under `as-shown` at 2b and 4, its stray `EMC`
  written out; `issue6342` is refused there by `model/operands-left-over`'s name (its `c` runs
  draw, so the write is refused; 6.2.2's `c08.` stands too) and photographed under
  `as-if-printed` as before. Census with and without the pack (the same movement), against
  round 2228's: `as-if-printed` does not
  move; under `as-shown` `issue15629` converts, and `T02-05-01_008` and `_012`, written before
  with a stream needing a repair nobody had read, are refused by the leftover's name (2b and 4);
  ten more refusals now name the leftover or another class first.
- Left: writing out only the leftover operands (the operator kept: `as_drawn` must keep each
  operand's place), which would convert the two T02 files; the eight interpreter-only classes.

**Gates.** Over `-p pdf-model -p pdf-archive -p pdf-transform -p viewer-qt -p conformance`: fmt 0,
clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0, nextest `--lib --bins --tests` 100
(5298 of 5302; the four failures are the manifest's `structure_14_6_14_9_spec_audit` entries),
rustdoc 0 (three crates), fuzz 0, `cargo test -p conformance` 0. veraPDF 1.30.3 over 5493
outputs: 40 not compliant and 12 unfinished, the same documents as round 2228's. The launch
section: 6 documents, 36 figures banded, none outside.
