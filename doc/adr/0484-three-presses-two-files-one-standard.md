# ADR 0484 — Three presses, two files, one standard

Status: accepted. Session 656.

`CONTRADICTED_DEVICE_CMYK_CONVERSION` holds five pages in four documents and calls itself "the group
with the most evidence behind it of any here". Two of the five carried all of it. This ADR records
what the other three are, measured; what the byte-level check says about the two references whose
agreement the group rests on; and a mechanism for trap 9 that none of the four already written
covers.

## 1. Why this group

Round 651 chose by `git blame` over the run of group comments and then by the sharper tell — the one
note that said what a reference *draws* with no number behind it. That group is done, so this round
asked the same question one level down: **how many of a group's own members does its note actually
measure?**

Counted over the fourteen non-empty groups, thirteen answer *all of them*.
`CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE`, `CONTRADICTED_REFERENCE_GLYPH_WIDTHS`,
`CONTRADICTED_SUBPIXEL_IMAGE` and `CONTRADICTED_VISIBILITY_EXPRESSION` hold one page each and
measure it; `CONTRADICTED_SHARED_JBIG2_DECODER` prints an ink table with a row per page for all
seven; `CONTRADICTED_GLYPH_EDGES` accounts for its twenty-six in five cohorts of 8, 1, 11, 1 and 5;
`CONTRADICTED_SUBSTITUTED_FONT` gives a `/BaseFont` and a cap-row count for each of its eight.

`CONTRADICTED_DEVICE_CMYK_CONVERSION` is the exception. `postscript_type4_many_outputs.pdf` has a
closed form written out and sampled twice, nine sessions apart; `transparent.pdf` has a five-renderer
pixel table. `function_based_shading_cmyk.pdf`'s two pages and `type4psfunc.pdf`'s one appear once,
in a sentence about what their *dictionaries* contain:

> All three reach `DeviceCMYK`: `type4psfunc.pdf` and `postscript_type4_many_outputs.pdf` through a
> `/DeviceN` whose alternate it is, `function_based_shading_cmyk.pdf` directly.

That is trap 9's fourth shape exactly — "**what a page's objects are is evidence about where to look
and never about who is right**" — and the trap's own text records that the same reasoning about the
same clause family cost six rounds once already (ADR 0456). The sentence also says *three* while the
array beneath it holds five in four documents, which is the second time this group's arithmetic has
had to be corrected against its own list.

## 2. What the three pages are, in closed form

Each admits one, so no renderer's opinion is needed for the colour.

**`function_based_shading_cmyk.pdf` page 1** is three §8.7.4.5.2 type 1 shadings on a 290 × 290 page.
`/Sh20` (130 pt at 10, 10) and `/Sh21` (60 pt at 150, 10) share object 10, a §7.10.2 sampled function
with `/Size [2 2]`, `/BitsPerSample 8` and sixteen bytes:

```text
  00 00 00 00   FF 00 00 00   00 FF 00 00   00 00 FF 40
```

`/Order` defaults to 1, so with the domain point `(u, v)` the colour is bilinear:

```text
  C = u(1 − v)      M = (1 − u)v      Y = uv      K = 64uv/255
```

`/Sh22` (130 pt at 10, 150) is the same construction one space out: `[/Separation /Spot /DeviceCMYK
12 0 R]`, a tint bilinear over 0, 128/255, 192/255 and 1, and a §7.10.3 transform with `/N 1` from
`/C0 [0 0 0 0]` to `/C1 [0.1 0.9 0.8 0.05]`, so the ink is `t · (0.1, 0.9, 0.8, 0.05)`.

**Page 2** is the same 600 × 600 square six times — `/Sh30` to `/Sh35` are object 10 again under
`/Matrix [600 0 0 600 …]` at six *integer* offsets on an 1880 × 1260 page. Section 5 is what that
buys.

**`type4psfunc.pdf`** is one §8.7.4.5.3 axial shading through `[/DeviceN [/Magenta /Yellow]
/DeviceCMYK 183 0 R 196 0 R]`. Object 183 is 292 bytes of §7.10.5 `roll`, `index`, `cvr` and `sub`;
hand-evaluated it leaves `(0, m, y, 0)` on the stack — the identity into two of the four channels,
which is what a `/DeviceN` naming `/Magenta` and `/Yellow` means. The `/Function` is a §7.10.4 stitch
over one exponential from `[.2 .8]` to `[0 0]`, and the two `cm` operators put the axis *vertical*,
44.42 points long, so the colour is `(0, 0.2(1 − t), 0.8(1 − t), 0)` with `/Extend [true true]`.

## 3. The measurement, and it is one question rather than three

**One statement comes out of the forms with no renderer in it.** Multilinear interpolation of ADR
0009's sixteen ink corners over the closed-form CMYK, compared with our own raster at all 125 sample
points, is **within one level of 255 everywhere**. Ours is that arithmetic carried out; whatever the
rest of this ADR says about anybody else, no part of the disagreement is a question about our
shading. The same fact validates the closed forms themselves — a wrong form would agree with nothing,
and this one agrees with one renderer exactly and, through the profiles below, with three more.

Sampled against those forms at 125 points over five shadings — 25 apiece on `/Sh20`, `/Sh21`,
`/Sh22`, page 2's `/Sh30` and the axial band — with both candidate press profiles evaluated by
`pdf_model::icc`, **this tree's own A2B evaluator**, and never by a reference's:

| max &#124;Δ&#124;, levels of 255 | ours | `poppler` | `mupdf` | `ghostscript` | `hayro` |
|---|---|---|---|---|---|
| Artifex SWOP profile | 48 | 51 | 8 | 8 | 8 |
| CGATS001Compat micro profile | 48 | 51 | 5 | 4 | 4 |
| ours | — | 4 | 48 | 48 | 48 |
| `poppler` | 4 | — | 51 | 51 | 51 |
| `mupdf` | 48 | 51 | — | 6 | 4 |
| `ghostscript` | 48 | 51 | 6 | — | 5 |
| `hayro` | 48 | 51 | 4 | 5 | — |

Two camps, on every page and at every sample point. Ours and `poppler` within four levels of each
other; `mupdf`, `ghostscript` and `hayro` within six; 48 and 51 across. Both profiles land inside the
second camp and 48 outside the first.

**So there is nothing here for the shading to be wrong about.** §8.7.4.5.2's interpolation, §7.10.2's
`/Order`, §7.10.5's operator set, §7.10.4's stitch and §8.6.6.4's and §8.6.6.5's tint transforms all
sit on the near side of the split — the colour every renderer paints is a function of the closed-form
CMYK alone, and the three sizes of the *same* colour field on page 1 (130 pt, 60 pt and page 2's
600 pt) are that function's own consistency check: each renderer agrees with itself across scales to
within the sampling offset. The group's name is right about all five members. It had been right about
three of them by assumption.

## 4. The agreement, checked at the byte and then found to be two files

The group's central sentence is ADR 0048's — "their agreement is one profile seen twice" — and it
rested on our evaluator reproducing `mupdf`'s and `ghostscript`'s numbers from
`/usr/share/ghostscript/iccprofiles/default_cmyk.icc`. That is an inference from an output, and it
is the right one; the file itself says more:

- `/usr/share/ghostscript/iccprofiles/default_cmyk.icc` — 187 484 bytes, `desc` **Artifex CMYK SWOP
  Profile**, `cprt` *Copyright Artifex Software 2011*, tags `A2B0 A2B1 A2B2 B2A0 B2A1 B2A2`,
  `md5 fd199526f0a7e0bceb294a777cd84252`.
- `libgs.so` embeds **no** ICC profile at all, so `ghostscript` reads that file off the disk.
- `libmupdf.so` embeds **five**, and one of them is **the same 187 484 bytes at the same digest**, at
  offset 3 360 896.

Neither reads the other's copy and they are the same bytes. The scan is worth keeping as an
instrument: every ICC profile is a four-byte big-endian length followed by `acsp` at offset 36, so
what a binary is reading can be found without its source and without `ldd`.

**And then `hayro` does not fit.** It sits with the Artifex pair — 4 levels from `mupdf`, 5 from
`ghostscript`, 48 from us — and shares nothing with either. `objdump -p` on `pdfref-hayro` names
`libgcc_s.so.1`, `libm.so.6` and `libc.so.6`; there is no `liblcms2`, no C colour library, nothing to
share. What it carries is `hayro-interpret`'s own `assets/CGATS001Compat-v2-micro.icc`: **8 464
bytes, `desc` `uCMY`, `cprt` `CC0`, one `A2B0` tag**, against Artifex's 187 484 bytes and three
`A2B` tables beside three `B2A` ones.
Different size, different author, different licence — and our evaluator on *either* file predicts all
three renderers.

### The sixth mechanism

Trap 9 lists five ways two references can agree without that being evidence: a shared gap, shared
data, a shared default argument, shared code wider than one decoder, and two unrelated wrong answers
coinciding at one angle (ADR 0480). This is none of them. Artifex's `desc` says **SWOP**, and CGATS TR 001 is the characterisation data
SWOP publishes: the two files describe **the same printing condition**. Three implementations that
share no code, no file and no library agree because each independently went and got a copy of the
same published standard.

That is invisible to every instrument the trap already names. A dependency graph shows nothing; a
digest comparison shows two different files; only reading the profiles' own `desc` tags, and knowing
what the names mean, connects them. It is recorded on trap 9 as its own bullet.

## 5. The page that asks each renderer a question about itself

Page 2's six shadings are one picture six times at six integer offsets, so the *document* states an
invariant about its own raster: every renderer owes six identical 600 × 600 squares. This is trap 9's
corpus-invariant instrument — each program compared only with itself, no renderer treated as truth.

Ours, `mupdf`, `ghostscript` and `hayro` return six squares differing in **zero** channels.
`poppler` returns two answers: the square's top row is painted on the three squares at `y` 640 and
left white on the three at `y` 20 — 600 pixels at up to 255 levels, one row — where the clip
rectangle and the shading `/Matrix` differ only by an integer translation of 620 points. Recorded,
not chased, and unreported upstream.

It is worth naming what this does *not* say. The invariant cannot rank the two camps of section 3,
because both camps satisfy it; a self-consistency check tells you a renderer is answering one
question rather than two, and says nothing about the answer. Its whole value here is that it removed
the shading from suspicion before any profile was opened.

## 6. The clause, and it is one subclause off from where the code cited it

The group's pages are not fixed, and the reason is principle 5 rather than difficulty. The sentence
that decides them is §10.3.2's NOTE:

> Establishing a CIE-based source colour space can happen based on a user-driven configuration, by
> assumptions made by the PDF processor software, by analysis of the colour values and other
> properties, or by other mechanisms.

Four processors, four assumptions about what a document's `DeviceCMYK` *means*, one licence. Three of
the four load a press profile; ours is `CMYK_CORNERS`, sixteen ink corners interpolated. The standard
ranks §10.3's route above §10.4.2's crude formulas (§10.4.2.1) and then declines to choose among the
assumptions §10.3 permits, so adopting somebody's press to close 48 levels would be curve-fitting
with a licence attached — the argument ADRs 0009 and 0042 already make, now with the corpus number
under it.

**`CMYK_CORNERS`'s comment cited §10.3.1's NOTE for this, and §10.3.1's NOTE is about the
destination.** The two are one subclause apart and differ by one word — *destination* against
*source* — and both contain "assumptions made by the PDF processor software", which is how a
citation lands next door and reads correctly for four hundred sessions. This table is a claim about
what the file's ink means, not about what the screen is; the destination here is sRGB. Corrected in
`colour.rs` and in the ledger's §10.3.1 and §10.3.2 rows, with §10.3.2's NOTE quoted verbatim in both
places it is load-bearing.

## 7. What `spec-errata emit` found on the way, and the blind spot it exposes

`doc/todo/02` §4 says a round runs `emit` over the family before it writes. Under the heading
*10.4.1 General* it prints one annotation pair, Issue #181, `Review`/`Completed`:

```text
  over: ISO 15076-1:2010 (ICC.1:2010)
  says: the appropriate ICC specification (see "Table 66 - ICC profile versions supported by
        ICCBased colour spaces")
```

It is not §10.4.1's. Page 376 carries §10.2 through §10.4.1, and the StrikeOut's
`/Rect [83.128 333.629 238.273 346.049]` lands on the words `pdftotext -bbox` places at
(86.42, 494.66)–(237.24, 507.60) on an 841.92-point page — the last line of **§10.3.1**, whose
closing sentence this tree quotes in two places: `colour.rs`'s `BRADFORD` and §10.3.1's ledger row.
Both are prose now, and the erratum *strengthens* what they claim: `icc.rs` reads the profile
header's version byte and accepts 2.x and 4.x alike, which is what "the appropriate ICC
specification" with a pointer to Table 66 asks for and what one dated edition did not.

**`spec-errata check` never named it, and could not have.** `MIN_WORDS` is 4 and the struck run is
two tokens. So this is a quotation sitting on retired text that passes the gate built to find
exactly that — a second blind spot beside #236's, and a different one: #236 was an erratum over text
nobody had written yet, this is an erratum too short to compare. `doc/errata-read.md` carries both
now, and the rule that `emit` runs before writing has a second reason.

## 8. What changed

- `crates/pdf-model/tests/oracle.rs` — the group note: the three closed forms and their table, the
  byte identity, `hayro`'s separate profile and its `objdump` line, §10.3.2's NOTE quoted, the
  six-square invariant and `poppler`'s two answers to it, and "all three" corrected to four.
- `doc/traps/oracle-and-references.md` — trap 9's shared-data bullet gains the digest and the
  header-scan instrument; a new bullet at the *end* of the list for the shared *standard*, placed
  there rather than beside shared data so that no existing "second shape" / "fourth shape"
  reference in `oracle.rs` shifts under it.
- `crates/pdf-model/src/colour.rs` — `CMYK_CORNERS` quotes §10.3.2's NOTE and says why it is not
  §10.3.1's; `BRADFORD`'s quotation of the retired ICC citation becomes prose.
- `doc/conformance/ledger.toml` — §10.3.1 and §10.3.2, both `implemented` and both re-read.
- `doc/errata-read.md` — Issue #181 at p. 376, and what its two-word strikeout says about `check`.

No pixel moves. The five pages stay contradicted, and the group's name survives its examination for
the second time running — what did not survive is a sentence that described three of its members
from their dictionaries.

## 9. What is owed

- `poppler`'s dropped row is unreported upstream, and so are `mupdf`'s and `ghostscript`'s press
  assumption and `hayro`'s. `doc/HAYRO_ISSUES.md` does not name the last, and it is not obviously a
  defect — it is a choice, made without a document asking for it, exactly as ours is.
- The membership rule that admitted three pages on their dictionaries is still the rule. Nothing in
  the harness asks a group whether its note has a number for every name it holds; section 1's count
  was done by hand.
