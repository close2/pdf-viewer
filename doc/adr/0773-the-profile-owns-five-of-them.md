# ADR 0773 — The profile owns five of them, and the pair is one raster

Status: accepted, 2026-09-01. Session 846, an oracle round on `doc/todo/12` item 3.

The substitution table's vector row — the `mupdf` + `ghostscript` consensus contradicting
`poppler` on **119 of 226** vector pages, against 13 of us — is read. Trap 9's shared-data bullet
was the hypothesis and it is **wrong about 114 of the 119**: taking the shared profile away moves
exactly five pages, and they are the five ADR 0048 named when it found the mechanism. What owns
the rest is measured instead of guessed, and it is a stronger mechanism than any on that list:
on **97 of the 117** of those pages the comparison can read, `mupdf` and `ghostscript` are
**pixel-identical**. No verdict, bound or pixel of this tree moves.

## 1. The population came out of the gate this time

ADR 0772's lesson, applied to the row it handed on. `Substitution` now carries the page, and
`print_the_substitutions` prints every row's contradicted pages by name under the table. The 119
are: **76 of the `bitmap-*` JBIG2 conformance family**, 10 pages of `issue11878_reduced.pdf`, 3 of
`issue5481.pdf`, 2 apiece of `colors.pdf` and `function_based_shading_cmyk.pdf`, and 26 documents
contributing one page each. That composition is a population and not a diagnosis — trap 9's fourth
bullet — and everything below is measured rather than read off the file names.

## 2. The removal, and the instrument it needed

ADR 0048 established the shared profile by **evaluating** it: this tree's own A2B evaluator
pointed at `/usr/share/ghostscript/iccprofiles/default_cmyk.icc`, which `libgs` reads off the
disk and `libmupdf` carries compiled in at the same 187 484 bytes and the same digest. What it
never did was take the file away, and a mechanism named is not a mechanism priced (ADR 0497).

`gs -sDefaultCMYKProfile=` is the removal, and it is now a knob rather than a shell script:
`PDFREF_GS_CMYK_PROFILE` in `pdfref::Reference::build_command`. It is in the harness because
`Cache` keys a remembered render on `Reference::command_signature`, so an invocation this changes
is a **different key** and cannot be answered out of the baseline's cache — which is the one way
an experiment like this measures nothing and says it measured something. Unset, it adds no
argument at all.

**The control first** (trap 13). Passing `gs` its own `default_cmyk.icc` explicitly reproduces the
default render **byte for byte** — same MD5 on `function_based_shading_cmyk.pdf` page 1 — so the
knob is inert on its own and anything the substitutions move is the profile.

## 3. Two removals, because the first one removed the wrong thing

| gs's `DeviceCMYK` profile | vector pages judged | `poppler` contradicted | ours |
|---|---|---|---|
| `default_cmyk.icc` (Artifex, 187 484 B) — baseline | 226 | **119 (52.7%)** | 13 |
| `CGATS001Compat-v2-micro.icc` (hayro's, 8 464 B) | 226 | **119 (52.7%)** | 13 |
| `ps_cmyk.icc` (Ghostscript's PostScript CMYK, 5 340 B) | 220 | **114 (51.8%)** | 8 |

**The middle row moves nothing, and the page lists are identical** — all 119 names, `diff`-clean.
That is not a failed experiment; it is trap 9's *sixth* bullet arriving as a number. Artifex's
`desc` says **SWOP** and CGATS TR 001 is the characterisation data SWOP publishes, so substituting
one for the other removes the **file** and leaves the **press**. Priced on
`function_based_shading_cmyk.pdf` page 1, the two renders are **mean 0.1257 of 255 apart, max 7**:
two files of different authors, lengths and licences, twenty-two times apart in size, agreeing to
an eighth of a level. Against `ps_cmyk.icc` the same page is **mean 12.9954, max 116, 32.84% of
channels differing** — that is a different press, and it is the removal that removes something.

**Under the removal that removes something, five pages leave**, and the same five leave our own
row (13 → 8):

`function_based_shading_cmyk.pdf` pages 1 and 2, `postscript_type4_many_outputs.pdf` page 1,
`transparent.pdf` page 1, `type4psfunc.pdf` page 1.

That is `CONTRADICTED_DEVICE_CMYK_CONVERSION` exactly — the group ADR 0048 created and ADR 0510
extended. **The shared profile owns 5 of the 119: 4.2% of the vector row, and not one page
outside the group already attributed to it.** It owns a much larger share of *our* thirteen —
five of them, 38% — which is the honest way round: the mechanism is real, it is where it was
always said to be, and it is not what the vector row is made of.

## 4. What the vector row is made of, measured

The per-measure columns say the hypothesis could not have been right before any removal was run.
Of the 119 convictions, **116 are on the worst tile and 109 on the structural similarity, against
45 on the mean** — a texture disagreement, where a colour profile acts as an area times a level
and would have shown up in the mean and the differing fraction first.

Rendering the three voting references over the 119 pages by hand with the harness's own arguments
(trap 3) and comparing every pair with `examples/compare_rasters`:

| pair | median mean | median ssim |
|---|---|---|
| `mupdf` v `ghostscript` | **0.0000** | **1.00000** |
| `poppler` v `mupdf` | 0.8318 | 0.97549 |
| `poppler` v `ghostscript` | 0.8318 | 0.97549 |

**`max 0` on 97 of the 117 pages that could be compared, and on all 76 of the bitmap family.** Not
close: the same bytes. The panels are not blank and the identity is not an artefact of the
comparison — on the 20 pages where the two differ the same instrument says so, and on
`bitmap-halftone.pdf` page 1 `mupdf` and `ghostscript` both write 2 distinct colours where
`poppler` writes 198 and 694 pixels of 159 600 differ.

Two consequences, and the second is the finding.

**The bound has no widening in it.** `Tolerance::widened_to` takes the larger of the class floor
and the consensus's own spread times two, and twice zero is zero — so on those 97 pages what
`poppler` is held to is the bare `Tolerance::VECTOR` floor, with the whole relative-bound
mechanism inert. It fails on structural similarity at a median 0.97549 against a floor of 0.9900.

**And a consensus of two identical rasters is one reading counted twice.** ADR 0005's inference is
that two implementations sharing no code agreeing about a page is evidence; two implementations
agreeing to the *byte* over a page of images are not two implementations for the purpose of that
inference, whatever their source trees say. The population asymmetry is the same fact from
outside: this pair forms a consensus on **226** vector pages where `poppler` + `ghostscript` forms
one on 120 and `poppler` + `mupdf` on 123. The extra hundred pages are pages no pair containing
`poppler` agrees on at all, and on every one of them the verdict is decided by a spread of zero.

`bitmap-halftone.pdf` is the shape in one file: a 399 × 400 media box holding one 399 × 400
`JBIG2Decode` image, so at 72 dpi the sampling is 1:1, axis-aligned and integer-aligned, and the
picture is bilevel by construction. §8.9.5.3 leaves interpolation to the processor — it is "only a
hint, and a PDF processor may ignore it" — so nothing in the standard forbids `poppler`'s grey,
and nothing in this measurement says who is right. What it says is that the pair convicting it is
`jbig2dec` twice (trap 9's fifth bullet), and ADR 0381 has already answered *right* out of the
documents themselves.

## 5. What this changes

- Trap 9's shared-data bullet carries the removal and its number.
- Trap 9 gains the mechanism above: **a pair whose rasters are identical is not a consensus**, with
  the count and the zero-widening consequence.
- Trap 9's shared-press bullet carries the two-file control: 0.1257 of 255 between SWOP and SWOP,
  13.00 between SWOP and another press — which is what makes a "shared data" removal credible or
  null, and there was no way to tell those apart before.
- `doc/todo/12` item 3 is answered for the profile and restated for what is left.
- `PDFREF_GS_CMYK_PROFILE` and the named substitution rows are in the tree, so neither the removal
  nor the population has to be rebuilt by hand again.

## 6. What this does not claim

That any page is drawn differently, that any verdict was wrong, or that `poppler` is wrong on the
bitmap family — the second is ADR 0381's answer and it is not this measurement's. Nor that the
identity of the pair *should* change a verdict: acting on it would need a rule, a rule would need
a population, and this round measured one row of one table. The oracle prints 980 / 60 / 836
before and after.
