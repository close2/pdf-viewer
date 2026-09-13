# ADR 1031 — §8.7.3.1's `/TilingType` is read and counted, not reported

## Status

Accepted.

## Context

ISO 32000-2 §8.7.3.1's Table 74 makes `/TilingType` a required entry of every Type 1 pattern
dictionary: "[a] code that controls adjustments to the spacing of tiles relative to the device
pixel grid". Its three values are

1. constant spacing, cells "spaced consistently" by a whole number of device pixels, with the cell
   distorted if need be and "[t]he amount of distortion shall not exceed 1 device pixel";
2. no distortion — "[t]he pattern cell shall not be distorted, but the spacing between pattern
   cells may vary by as much as 1 device pixel, both horizontally and vertically, when the pattern
   is painted";
3. constant spacing as in 1, with more distortion permitted.

The entry was read by nothing. `pdf_model::content::pattern::Interpreter::tiling` placed every site
at a multiple of `/XStep` and `/YStep` in pattern space and left the pixel grid to whichever backend
rasterised the display list, which is value 2's behaviour whatever a file asks for — and said
nothing about it. The ledger's §8.7.3.1 row was `partial` for exactly this.

`doc/traps/parsers-and-streams.md`'s trap 5 has one answer to that shape: a reader that substitutes
its own behaviour for the producer's says so. The obvious change is therefore an `Unsupported`
raised where the value is 1 or 3.

## Decision

**Read the entry, do not report it, and count the population with a command.**

`content/pattern.rs` reads `/TilingType` beside the other entries of Table 74 and states, under the
clause's own sentences, what each value gets. `crates/pdf-model/examples/tiling_type_census.rs`
prints how many Type 1 pattern dictionaries a corpus holds, how many ask for each code, and which
documents ask for 1 or 3. The ledger's §8.7.3.1 row carries the departure and stays `partial`.

## Consequences, and the measurement that decided it

The report was **written, run and then taken out again**, which is why this ADR exists: the
argument against it is a number rather than a preference.

- Over `doc/pdf.js`, `cargo test --profile gates -p pdf-model --test corpus` with the report in
  place added **seven documents** to the incomplete list — `pattern_text_embedded_font.pdf`,
  `tiling-pattern-box.pdf`, `tiling-pattern-large-steps.pdf`, `quadpoints.pdf`,
  `tiling_patterns_variations.pdf`, `scorecard_reduced.pdf` and `bug1795263.pdf`. Every one of them
  is a document the tiling clause is *about*. (`MAX_INCOMPLETE` was not breached; the cost is not
  that gate.)
- `tests/oracle.rs` filters its contradiction and ambiguity ratchets over `e.complete`, so an
  incomplete page leaves the judged population. Two of the seven are pinned, diagnosed entries:
  `tiling-pattern-box.pdf page 1` in `AMBIGUOUS_SUB_PIXEL_LINE_WORK` and
  `tiling_patterns_variations.pdf page 1` in `AMBIGUOUS_ONE_LADDER`. Reporting would have deleted
  both diagnoses to record a departure.
- What the departure is worth is bounded by the clause itself. Type 1 may not distort by more than
  one device pixel; type 2 may vary the spacing by as much as one device pixel. Every tile the
  producer asked for is painted, in the geometry the file states; what differs is a placement
  inside the tolerance the standard prints for all three codes.
- `examples/tiling_type_census` over `doc/pdf.js` reports 955 Type 1 pattern dictionaries, of which
  **392 ask for 1, 557 for 3 and 5 for 2**, one stating nothing readable. The value this tree
  performs is written by half of one per cent of the population, so a report here is not an
  exception — it is the ordinary case, and it would fire on almost every patterned page in the
  world.

That is ADR 0563's shape exactly, one clause over: Table 58's `h` on an empty path is not counted
as a lost segment, because the page keeps every mark and saying otherwise would take it out of the
oracle's judgement for a mark nobody lost. Trap 5's test is about a mark standing in place of the
producer's. Nothing stands in place of anything here.

**What a later round must not do is re-derive the report from the trap without re-deriving the
measurement.** The way to honour `/TilingType` is not a report at all: it is a rasteriser that
snaps a lattice to the device pixel grid, because only a backend knows the pixel. That work would
take the entry out of `content/pattern.rs` and into `pdf_render::Cell`'s consumers, and it would
close the row without costing anybody's judgement.

## Alternatives rejected

- **Raise the report and edit the two oracle pins.** It buys a sentence in a report pane and pays
  two diagnoses and seven documents' judgement for it.
- **Report only value 3.** The split between 1 and 3 is how much distortion is permitted, not
  whether any is; there is no clause under which 3 is a departure and 1 is not.
- **Leave the entry unread.** That is where the row was, and it is the one state in which nobody
  can tell whether the entry was considered.
