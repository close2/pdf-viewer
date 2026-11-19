# 3165 — An image's table converts only the levels it draws, where its space may fault

Session 2116. Status: **accepted and built** in `pdf-colour` (`colour/queries.rs`: `ColourSpace::may_fault`;
`black_generation.rs`: `BlackGeneration::may_fault`) and `pdf-model` (`image/unpack.rs`: `table_of_levels`,
`levels_drawn`, `palette`'s `drawn`). Builds ADR 2848's first item ("An image's table of levels").
Clauses: ISO 32000-2 §7.10.5.3, §8.9.3, §8.9.6.4, §7.3.8.2. Row §7.10.5.3 stays `implemented`.
Instruments: `pdf-model/examples/callgrind_interpret.rs` (the cost), `pdf-model/examples/type4_operator_census.rs`
(the population).

## The clause, and what over-reported

§7.10.5.3: "It shall be an error for the number of remaining operands to differ from the number of output variables
specified by Range or for any of them to be objects other than numbers." Since ADR 2848 every type 4 evaluation a
page makes is asked and its fault said. A one-component image of eight bits or fewer is converted as a table of
every level its depth holds (ADR 1557's route), so a tint transform that errs only at a level no sample uses was
evaluated there and said: the report stated more than the drawing.

## Decision

The table converts only the levels the image draws **where a conversion could fault**, and every level elsewhere.
`ColourSpace::may_fault` walks the space — a `Separation` or `DeviceN` tint transform, and the bases and alternates
an `Indexed`, `ICCBased` alternate, pattern or simulated space is built over — asking `Function::may_fault` (ADR
2940's proof that no input errs); the conversion's §10.4.2.4 black generation is asked the same. Where either may
fault, `levels_drawn` reads the samples once: a byte at a time where a byte holds only samples its row has (the
set of bytes seen expanded into levels at the end), a sample at a time in a row's last, part-filled byte. It leaves
out §8.9.3's padding ("A PDF processor shall ignore these padding bits."), the samples a short stream never carried
(unpainted, §7.3.8.2 and ADR 0343), and the levels §8.9.6.4's colour key masks ("An image sample shall be masked
(not painted)"). A level left out is one no pixel reads, and its entry is black.

## The cost, measured

Callgrind, `callgrind_interpret` on page one, release, HEAD's binary against this tree's, each with its own
`pdf-sandbox-worker` beside it. The fixture is a 2000 × 2000 eight-bit `Separation` image whose samples take 100
levels (`(7x + y) mod 100`), all tints below 0.5, unfiltered, drawn under two programs:

| program | HEAD | this tree | |
|---|---|---|---|
| `{ dup dup }` — `may_fault` clears it | 35 704 136 | 35 699 862 | the scan is skipped; the walk costs nothing measurable |
| `{ dup 0.5 gt { pop 1 1 } { dup dup } ifelse }` — it may fault | 35 767 804 | 45 200 696 | +9.43 M: 2.36 instructions a sample |

Each cell is the modal count of three to six runs; single runs read up to 1.7 M higher in either arm, which moves
neither conclusion.
`issue9940.pdf`, the page ADR 1557 measured, is not usable as an A/B here: both arms read either about 80 M or
about 102 M from one run to the next. The second row is the price of the narrowing, paid by an image only where its
space carries a program ADR 2940's walk cannot clear; every other image pays the `may_fault` walk and nothing else.

**The population.** `type4_operator_census` now counts the image `XObject`s of one component and at most eight bits
whose space parses, and of those the ones `ColourSpace::may_fault` leaves uncleared. Over the pdf.js suite and
`doc/corpora-own` (976 paths): 1 127 such images, none uncleared; over `doc/corpora` (503 paths): 404, none. Over the
corpus cache (98 835 paths) the run was stopped by `tools/bounded.sh`'s 6 GiB tree ceiling after 1 495 s, before it
printed, and is not taken here; a sharded run is the way to take it. The argument does not rest on it: an image whose
space `may_fault` clears pays nothing measurable, so the cost falls only on the images whose report the narrowing
corrects, whatever their number.

## Alternatives not taken

- **Narrowing every image's table**: the scan would be paid by every one-component image for a report only a
  faulting program can change.
- **Converting every level and withdrawing the faults no sample meets**: a fault is recorded in the function's
  shared cell at evaluation (ADR 2848), and nothing records which input met it; undoing it would need a second
  evaluation per level, which is the scan's cost again and a write path into the cell.

## Held by

`image::unpack::tests`: `a_rows_padding_is_no_level_drawn`, `a_sample_the_data_never_carried_is_no_level_drawn`,
`a_level_the_colour_key_masks_is_no_level_drawn`. `function_faults` (in `syntax_suite`):
`an_images_tint_transform_that_errs_for_a_level_it_draws_is_said` (0x10 and 0xF0 under a program that errs above 0.5
say the fault; 0x10 and 0x20 do not; the conforming program says nothing),
`a_level_the_colour_key_masks_says_no_fault`, `an_indexed_images_base_that_errs_for_an_entry_it_draws_is_said`.
Planted back — `palette` handed `None` for `drawn` — the three `function_faults` tests fail.
