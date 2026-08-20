# 619 — Four ceilings, none of which was this program's

`doc/todo/03`'s chunk again, over the SafeDocs crawl, for the fourth round running. Eight whole
archives this time, and four defects — every one of them a *bound* that belonged to something
other than this program: a decoder library's default, a window's size, a filter parameter read as
a disagreement, and a font container's table list.

Date: 2026-08-20.
ADR: [0454](../adr/0454-four-ceilings-none-of-which-was-this-programs.md).

Touched: `crates/pdf-model/src/image.rs`, `crates/pdf-model/src/inline_image.rs`,
`crates/pdf-model/src/content/run.rs`, `crates/pdf-font/src/program.rs`,
`crates/pdf-model/tests/{ccitt_bound,dct_components,content_window,inline_images}.rs`,
`crates/pdf-model/examples/token_window_census.rs`, `doc/conformance/ledger.toml` (§7.4.6, §7.4.8,
§8.9.7, §9.9), `doc/todo/03-more-corpora.md` §19, `doc/todo/_image-codecs-and-the-sandbox.md` §7,
`doc/traps/oracle-and-references.md`, the ADR and this file.

## The chunk

**`0546`, `1284`, `2022`, `2760`, `3498`, `4236`, `4974`, `5712` — 8000 documents**, none of
603's two, 613's five or 615's seven. An archive is a hash bucket (ADR 0261), so any set is
unbiased. 603's instrument **reused rather than rewritten**: page one at 72 dpi against
`pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box, ranked by our ink
minus the lightest live reference's, panel sizes beside each number.

**Checked before it was trusted — and the check fired.** Sixteen documents named by ADRs 0438,
0448 and 0451 were re-measured first, and seven came back *worse than before their fix*:
`3252105.pdf` at −156.436 where 615 recorded −6.390, `2268946.pdf` at −15.621 where it recorded
+0.035. Nothing had regressed. **`pdf-sandbox-worker` is not built by building the example**, and
without it every codec behind the sandbox refuses by name and the ranking measures a tree with no
bilevel decoder. Built, all sixteen reproduce to the thousandth. A round that had skipped 615's
discipline here would have opened with seven invented regressions; it is a note in
`doc/traps/oracle-and-references.md` now.

## What the four defects were

**`2022009.pdf` at −84.152** — a full-page scan drawn blank, reporting `Image height 28341 greater
than height limit 16384`. That is `zune-jpeg`'s `DecoderOptions` default, reached before this
crate's own `MAX_SAMPLES`. §7.4.8 puts the dimensions "entirely under the control of the encoder"
and states no ceiling; ISO/IEC 10918-1 gives each axis sixteen bits. `jpeg_options` states 65535
and the budget with the argument written beside it is left to do its job — which still refuses the
bomb, 65535² being 4.29 G samples against 268 M. 615's `/Mask` lesson from a new direction.

**`3498294.pdf` at −26.015** — an architectural drawing whose 1024×716 unfiltered inline image
drew 37% and whose remaining 1.4 MB of samples were **tokenised as content operators**.
`inline_image`'s three answers were in the right order; what failed is that each derived end is
*checked* against the `EI` it predicts, and inside a 64 KiB window the check cannot happen — so
both derived answers were dropped and the forward search ran, and the doubling loop then stopped
because a guess had returned an answer. §8.9.7 makes the bytes a stream's data and §7.3.8.2 makes
that extent inferable; a derived end past the buffer is now a request for more bytes.

**`4236390.pdf` at −15.235 and `2022430.pdf` at −12.618** — scans refused for `/Columns 872`
against `/Width 869`, and `896` against `892`. Table 11's second sentence says the filter "shall
adjust the width of the unencoded image to the next multiple of 8", and 869 padded is 872. Both
files wrote the adjusted width; `ceil(869/8)` and `ceil(872/8)` are both 109, so the two premises
the refusal rested on — differing strides, nothing to decide between them — were both false.

**`0546308.pdf` at −6.785 and `3498231.pdf` at −7.131** — about 1550 text operations each lost to
`units per em is zero`. The font is a `/FontFile3` `/Subtype /OpenType` carrying `BASE`, `CFF `,
`GPOS`, `GSUB`, `OS/2` and `cmap` — no `head`, which is where every sfnt reader finds the em
square. Table 124 requires the `CFF ` and the `cmap` and says outright that "not all tables are
required in the font file", so the files are conforming and the refusal was ours. The `CFF ` table
is handed to the bare-CFF reader this crate already has.

## What moved

Twenty-two archives re-ranked whole and diffed row by row: **21 rows of 22 000 move**, and they
divide cleanly.

**Ten are documents one of the four fixes is about:**

| | before | after | fix |
|---|---|---|---|
| `2022009.pdf` | −84.152 | **−0.105** | JPEG budget |
| `3498294.pdf` | −26.015 | **−0.106** | inline extent |
| `4236390.pdf` | −15.235 | **+0.689** | `/Columns` |
| `2022430.pdf` | −12.618 | **+1.274** | `/Columns` |
| `3498231.pdf` | −7.131 | **+0.009** | no `head` |
| `3375550.pdf` | −7.099 | **−0.085** | no `head` |
| `0546308.pdf` | −6.785 | **−0.010** | no `head` |
| `5712943.pdf` | +2.451 | **+1.785** | no `head` |
| `3498460.pdf` | −1.971 | +0.322 | `/Columns` |
| `2268541.pdf` | −0.146 | +0.155 | JPEG budget |

**Two of the ten are in 615's own archives** — `3375550.pdf` and `2268541.pdf` — which is the
third round running that a fix has reached an earlier chunk.

**The other eleven are the instrument rather than the tree, and that is measured rather than
asserted.** Eight have our panel identical to the thousandth with a *reference* panel absent from
one of the two runs; three have **our** panel absent, and re-measured alone at three workers
instead of sixteen all three reproduce their earlier number exactly — `1161651.pdf` +1.537,
`6327464.pdf` +0.481, `1161228.pdf` −0.033. Thirty seconds is the per-renderer bound and a loaded
machine is what a sixteen-way run is; `doc/traps/oracle-and-references.md` already carries the
shape from the references' side.

Each fix is pinned by a test **run against the defect first** (trap 13): a generated 8 × 20000
frame above the library's default and far below the budget; an inline image two windows long whose
samples spell an `EI` in the first one, with a marker rectangle after the real `EI`; four
T.4-encoded lines of sixteen columns under a `/Width` of twelve, with a negative twin at
`/Columns 24`; and an `OTTO` wrapping this repository's own `FoxitSerif.pfb`, with and without a
`head`.

## What the head still holds

**Five documents of 22 000 now wait on `hayro-jbig2`'s flat 10 000-instance cap** — `0546561.pdf`
−30.018 and `4974796.pdf` −15.417 join the three `doc/todo/_image-codecs` §7 already had, and
these two reach it directly rather than from behind a bound of ours. One in 4400.

**Four silent rows diagnosed and not taken**, each named in `doc/todo/03` §19 with its evidence:
`2022794.pdf` −12.743 (1451 `DCTDecode` images, one of them 1400×2 — `doc/todo/11`'s subject),
`4236552.pdf` −10.930 (one command, `DCTDecode` under `ICCBased` with an `/SMask` — trap 9),
`4236836.pdf` −10.001 (a **text-only** page of five Type 1 subsets at 20.4 against 30.4 to 35.8,
the one head row that is not about an image) and `2022216.pdf` +20.141 (twenty `/SMask`s, the only
positive row where we are above three agreeing references). 615's two are still open.

**44 rows of the 8000 produce no number**, the same three shapes 613 and 615 opened by hand.

## Gates

The full §2 sequence, because the change is in `pdf-model` and `pdf-font`. `RUSTFLAGS="-D warnings"`
on the clippy line, which caught four lints this round's own code introduced. **No gate number
moves**, and `doc/todo/00`'s step 7 — owed because this round changes what gets drawn — reproduces
session 598's head and tail to the thousandth. §5's binaries were **not** rebuilt: this is not a
fifth round and nothing on the launch path was measured.
