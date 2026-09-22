# 1188 — Three instruments, and a population recounted

## A message that names a flag names one the program accepts (ADR 1213)

`tools/conformance/src/flags.rs`, `--bin flags`, `tools/state.sh flags`, gated by `tests/flags.rs`.
Both populations derived — the programs from the workspace's manifests (two renamed by a `[[bin]]`
block), the accepted flags from each binary's own source set followed through its `mod`
declarations. The first run gave 80 findings and none was the rule; three conditions took it to
zero, each added because the run without it produced noise: a `#[cfg(test)]` module is not the
program (80 → 24), a literal with no words around the flag is another program's argument (24 → 16),
and a command span belongs to the program it names, cargo's `--` deciding where the named program's
flags begin (16 → 0). Attribution outside a command span is by crate — name-on-the-line gave a
`quorra-transform` message to the `counts` binary, on the word *counts*.

The gate holds the population at zero rather than ratcheting, and can: round 1186 added `--font` to
`VALUED` and `KNOWN` while this was being built, so the sweep named six sites mid-build and names
none now. The plant runs every time all the same (`flags::calibrate`), into the functions rather
than into a file, and named both a message and a documented command line.

## §11.4.4's second element run, recounted on the interpreter's condition (ADR 1214)

`crates/pdf-model/examples/non_isolated_group_census.rs` asks `CpuRasterizer::group_buffer`'s own
conjunction of the display list's group commands, so §11.7.4's synthesised groups (ADR 1170) are
counted beside a file's own and apart from them. Curated: **2** groups on **1** of 1477 first pages,
both the implicit construction under `/Multiply`, on
`doc/pdf.js/test/pdfs/issue12798_page1_reduced.pdf` — where the file-stated count is 0, ADR 1107's
number. Crawl: **1233** on **205** of 89 286, 781 built and 452 file-stated. The calibration caught
a false zero first: the `/BM` pre-filter read only each object's top level and so excluded the
planted fixture, whose `ExtGState` is direct inside a `/Resources`.

## `tools/state.sh remedies` shows ADR 1199's wrinkle, and it is not zero

Per profile and target it now prints how many answers `not carried out yet` sit at a site the
target's own listing does not name — three for `keep-everything` at every target, three or four for
`as-if-printed`, one or two for `only-metadata-loss`. One awk pass over the same `--config` output.

## The departed rows, and what is left

All 17 notes name their deciding ADR in the first sentence, both where a row names two. Left:
`doc/pdf-a-mitigations.md` and `doc/pdf-a-conversion-limits.md` still say `--font` is not accepted.
