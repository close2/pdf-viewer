# 854 — The plant that failed nothing

2026-09-01. The `PARTIAL_FILE_ONLY_EVIDENCE_CEILING` reading list, finished: the last five rows off
it, 5 → 0, and the ratchet repointed the way this project retires one that has won. The reading's
own by-product is the round's title — a plant written to *calibrate* a named test passed instead,
and what it exposed is a `shall` that sixteen documents in the wild depend on and nothing in this
tree held.

No ADR: nothing here is a decision. The two lessons that generalise are written where they belong —
`doc/todo/01`'s sweeps section and the ratchet's own doc comment.

## The rows, and what each named test was calibrated against

All five are **aggregates**, which is why they outlived the renames: a row over a family cannot be
promoted by pointing at a better test in the same file, because its evidence is one named test per
child that states requirements. Every name below was calibrated by planting the rule it guards.

| row | new evidence | plant | what failed |
|---|---|---|---|
| §8.6.6 Special colour spaces | one test per family, four | a bare `/Pattern` refused at `by_name` | 19 of `tiling.rs`'s 26 |
| | | an `Indexed` lookup's `start` forced to 0 | both indexed tests, in two files |
| | | the `/All` complement dropped | `the_all_colourant_is_a_complemented_tint` + 1 |
| §8.6.6.5 DeviceN | `a_devicen_of_only_none_colourants_marks_nothing`, `a_devicen_passes_its_none_components_to_the_tint_transform` | `inputs` counted over the non-`None` names | both |
| | | the all-`None` branch removed | exactly the first |
| §11.7.5 Rendering parameters | one test per category §11.7.5.1 divides them into | every mark counted fully opaque | 3 of `transfer_functions.rs`'s 13 |
| | | a shading's ramp built under `Conversion::device()` | exactly 1 of `rendering_intent.rs`'s 11 |
| §14.9 Accessibility support | one test per entry read, four | each entry's `stated` call taken away, one at a time | `/Alt` 1, `/E` 1, `/Lang` 3, `/ActualText` 4 — and no plant failed a test about another entry |
| §7.7 Document structure | the page tree, a page's geometry, a page's content, the name dictionary | `MAX_TREE_DEPTH` cut to 1 | the ordering test + 1 |
| | | a production box left unintersected with the medium | exactly the boundary test |
| | | a non-stream `/Contents` part reported as nothing | 2 of `contents_entry.rs`'s 10 |
| | | `/Params` never read | 2 of `attachment.rs`'s |

§8.6.6.5's reverting rule had **no test at all**. §8.6.6.5 states the `/None` component's two fates
as a pair and only the second is reachable on a screen — "when the DeviceN colour space reverts to
its alternate colour space, those components shall be passed to the tint transformation function" —
and the fixture is a `[/DeviceN [/Spot /None] /DeviceRGB]` whose type 4 transform makes the second
component the whole of the green channel, so a discarded `/None` is black rather than a shade. The
same draw with the tints exchanged pins the clause's other sentence, the operand order.

## The plant that failed nothing

§8.6.6.2's named test is `tiling.rs::an_uncoloured_pattern_takes_its_colour_from_the_operator`, and
the plant written for it was `ColourSpace::parse_at`'s `/Pattern` arm reduced to `base: None` — the
underlying colour space dropped from the parser outright. **Every test in the workspace passed.**

The reason is a fallback: §8.7.3.3's underlying space is optional in the *syntax* a bare `/Pattern
cs` writes, so `content::pattern` picks `DeviceGray`, `DeviceRGB` or `DeviceCMYK` by the operand
count beside the pattern name — and for a device base the fallback and the stated space agree on
every value. Both of §8.7.3.3's named tests write the bare form. So the clause's own `shall` — "A
Pattern colour space representing an uncoloured tiling pattern shall have a parameter: an object
identifying the underlying colour space in which the actual colour of the pattern shall be
specified" — was implemented, correct, and held by nothing.

`an_uncoloured_patterns_colour_is_read_in_its_underlying_space` closes it with a `Separation` base,
whose single operand the fallback would read as a grey level and the clause reads as a tint. The
same plant now fails exactly that one test of 70.

## The demand side: the population, and one witness taken to a verdict

`examples/absence_audit` carries the claim now — the only one of its blocks whose subject is an
*array* rather than a dictionary, which is why it is asked in the array arm.

| population | witnesses |
|---|---|
| curated (1251) | none |
| `SafeDocs` `CC-MAIN-2021-31` (65 944) | **16** — fifteen `[/ICCBased …]`, one `[/CalRGB …]` |

Calibrated rather than believed: relaxing the same filter to device bases names 13 curated
witnesses, so the zero is a measurement. The first run of the condition over-reported by one — a
document writing `[/DeviceRGB]` in *array* form, where the fallback stands in exactly — and the
condition asks the base's family rather than its spelling now (trap 11).

**And the count is taken to a verdict rather than left as one.** `0300357.pdf` rendered at scale 1
with the base read, and again with the base dropped: **page 1 of 12 differs and the other eleven do
not**, 2920 of 500 990 pixels, a colour shift across a hatched region — the profile against the
fallback's `DeviceRGB`. So the parameter decides a picture on a file that exists.

## The ratchet, retired the way this project retires one

`PARTIAL_FILE_ONLY_EVIDENCE_CEILING` is 0 and asserted with `==`, which is what
`FILE_ONLY_EVIDENCE_CEILING` became at zero: a `partial` row arriving with a file for evidence now
fails the build rather than raising a number. Calibrated at zero in both directions — §11.7.5's two
named tests truncated back to their files reports 1 and fails.

One false sentence corrected while doing it, in two places: the ratchet's doc comment and
`doc/todo/01` both said "§8.6.6's backwards fold-over rule sat under a row whose evidence was
`tests/colour_paths.rs`". The fold-over is §8.7.4.5.7's and its row's evidence was
`tests/shadings.rs`; §8.6.6 was on the list, but for a different debt. One row of a population
wearing another's defect.

## Gates

§2 whole, on a quiet machine — `pdf-model` is in the first row of the change→gate map, and although
nothing outside `tests/`, `examples/` and documents moved, the map is about the crate rather than
the diff. `--bin undenominated` was run because this round wrote counts over two corpora (both
named in their own sentences, and neither is a hit); `--bin pointers` and `--bin quotations` name
nothing this round added.

§5's binaries were not rebuilt: not a fifth round, and the one measurement taken here is a
before/after render from a `--release` example built in this tree with `touch` on the changed crate
between the arms (trap 10b).

## What is left

The reading list this ratchet was is exhausted. `--bin owed` still prints 222 `partial` rows whose
stated terms this tree already names, and that is the list a spec-driven round takes next.
