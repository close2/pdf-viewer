# 846 — The profile owns five of them, and the pair is one raster

Date: 2026-09-01. ADR **0773**. An oracle round on `doc/todo/12` item 3, which is now answered.

Touched: `tools/pdfref/src/reference.rs` (`substituted_cmyk_profile`),
`crates/pdf-model/tests/oracle.rs` (`Substitution::page`, `print_the_substitutions`),
`doc/conformance/ledger.toml` (§8.6.4.4), `doc/todo/12-one-bound-two-jobs.md`,
`doc/traps/oracle-and-references.md`, `doc/verify.md`, `doc/adr/0773-…`. **No rendering code, no
pixel, no verdict**: the oracle prints 980 agrees / 60 contradicted / 836 ambiguous before and
after, and the substitution knob is unset in every gate.

## The item, and the population it was handed as a number

The substitution table's vector row: the `mupdf` + `ghostscript` consensus contradicts `poppler`
on **119 of 226** vector pages, against 13 of us. Trap 9's shared-data bullet — one Artifex
`default_cmyk.icc` in both binaries — was the hypothesis, and nobody had run the removal.

**The first act was ADR 0772's lesson applied to the row it handed on.** `Substitution` carries the
page now and `print_the_substitutions` names every row's contradicted pages under the table. The
119 are 76 of the `bitmap-*` JBIG2 conformance family, 10 pages of `issue11878_reduced.pdf`, 3 of
`issue5481.pdf`, 2 apiece of `colors.pdf` and `function_based_shading_cmyk.pdf`, and 26 documents
with one page each. That is a population and not a diagnosis (trap 9's fourth bullet), so
everything after it is measured.

## The removal, and the control that made a null result readable

`gs -sDefaultCMYKProfile=` is the removal ADR 0048 never ran. It is a harness knob rather than a
shell script — `PDFREF_GS_CMYK_PROFILE`, in `Reference::build_command` — because `Cache` keys on
`command_signature`, so an invocation it changes is a different key and cannot be answered out of
the baseline's renders. Unset it adds no argument, and passing `gs` its own profile explicitly
reproduces the default render **byte for byte**, which is the control run first.

| gs's `DeviceCMYK` profile | vector pages | `poppler` contradicted | ours |
|---|---|---|---|
| `default_cmyk.icc` (Artifex) — baseline | 226 | **119 (52.7%)** | 13 |
| `CGATS001Compat-v2-micro.icc` (hayro's) | 226 | **119 (52.7%)** | 13 |
| `ps_cmyk.icc` (Ghostscript's own) | 220 | **114 (51.8%)** | 8 |

**The middle row is a null, and the null is a finding rather than a failure.** The page lists are
`diff`-clean, all 119 names. Artifex's `desc` says SWOP and CGATS TR 001 is the data SWOP
publishes, so substituting one for the other takes the *file* away and leaves the *press*: on
`function_based_shading_cmyk.pdf` page 1 the two renders are mean **0.1257 of 255** apart, max 7 —
two files of different authors, lengths and licences, twenty-two times apart in size — against
mean **12.9954**, max 116, 32.84% of channels for `ps_cmyk.icc`. Trap 9's sixth bullet, priced. A
shared-data removal has to be checked against the `desc` tags before its null is believed.

**Under the removal that removes something, five pages leave**, and the same five leave our own
row: `function_based_shading_cmyk.pdf` pages 1 and 2, `postscript_type4_many_outputs.pdf` page 1,
`transparent.pdf` page 1, `type4psfunc.pdf` page 1 — `CONTRADICTED_DEVICE_CMYK_CONVERSION` exactly.
**The shared profile owns 4.2% of the vector row and nothing outside the group ADR 0048 gave it
eight hundred sessions ago.** It owns five of our thirteen, which is the honest direction.

## What the row is actually made of

The per-measure columns said the hypothesis could not be right before any removal was run: of the
119, **116 are convicted on the worst tile and 109 on the structural similarity, against 45 on the
mean** — a texture disagreement, where a colour profile acts as an area times a level.

Rendering the three references over the 119 with the harness's own arguments and comparing every
pair: `mupdf` v `ghostscript` is at **`max 0` on 97 of the 117 pages that can be compared**, median
mean 0.0000 and ssim 1.00000, all 76 of the bitmap family among them, where `poppler` sits a median
0.8318 and 0.97549 from each. Not close — the same bytes. On `bitmap-halftone.pdf` page 1 the two
write 2 distinct colours where `poppler` writes 198, on a 399 × 400 media box holding one 399 × 400
`JBIG2Decode` image, sampled 1:1 and integer-aligned.

Two consequences. `Tolerance::widened_to` takes the larger of the class floor and twice the
consensus's spread, and **twice zero is zero** — so on those pages the relative bound, which is the
whole reason this gate judges the way it does, is inert and `poppler` is held to the bare
`Tolerance::VECTOR` floor. And **a consensus of two identical rasters is one reading counted
twice**: this pair forms a consensus on 226 vector pages where `poppler` + `ghostscript` forms one
on 120 and `poppler` + `mupdf` on 123, and the hundred-page difference is judged at a spread of
zero. That is trap 9's new bullet, and the tell is one command reading `max` rather than any of the
numbers that round to zero long before two rasters are equal.

Nothing here says `poppler` is wrong on the bitmap family: §8.9.5.3 leaves interpolation to the
processor ("only a hint, and a PDF processor may ignore it"), and ADR 0381 already answered who is
right there out of the documents themselves.

## The spec track

§8.6.4.4 `DeviceCMYK colour space`, `partial`, read against `colour.rs` rather than inherited: the
sixteen-corner table is `CMYK_CORNERS`, `initial_colour` still returns `[0.0, 0.0, 0.0, 1.0]` for
`Self::Cmyk`, and the three outranking sources each have a reader (`ColourSpace::named_default`,
`content/colour.rs`'s `DestOutputProfile`, the `ICCBased` route in `to_rgb_at`). Kept at `partial`,
which records the documented choice between the two answers §10.4.2.1 ranks. What is new on the row
is the round's own measurement from the other side: the reference agreement this tree's colour is
judged against is a **press** rather than a file, and two independently obtained copies of the same
press are worth an eighth of a level to each other — the strongest statement available about a
clause that states no destination at all.

## Gates

The whole §2 sequence, on `main`, on a quiet machine: `fmt` (both workspaces) clean, `clippy
--workspace --all-targets` and the `fuzz/` one under `RUSTFLAGS="-D warnings"` clean, `nextest`
2847 passed / 18 skipped, doctests clean, and every corpus gate green including the oracle at 980 /
60 / 836 and `conformance` at 218. The `pointers` and `quotations` sweeps were run for the
documents this round moved and print nothing new.

## For the next round

Item 1 of `doc/todo/12` is the only part of that file still open, and it is the consensus half —
ADR 0243's 278 arrivals. What this round adds to it is a second reason to look there: on the pages
above, the pair *forming* the consensus is the whole mechanism, and the bound plays no part.
