# 1636 — The cubic spline is not-a-knot

Session 1400. Status: **accepted**. Carries out ADR 1623 §1, which answered ADR 0098's objection to
choosing a spline; neither is edited.
Context: ISO 32000-2 §7.10.2 (Table 39 and the sentences under it); ADRs 0098, 0356, 1599, 1623;
`doc/questions/A72`; `crates/pdf-colour/src/function/cubic_spline.rs`.

## 1. The choice

Table 39 reads: "Valid values shall be 1 and 3, specifying linear and cubic spline interpolation,
respectively." That names the kind of interpolation and not which spline. The brief offered two: a
natural cubic spline, which is C2, against Catmull-Rom, which is local and C1. The tree takes
**neither**. It takes the **not-a-knot cubic spline**, as a tensor product over the inputs. Three
reasons, each read against the clause:

1. **"Spline" in numerical practice means the C2 piecewise cubic through every sample.** Catmull-Rom
   is a Hermite curve whose tangents are estimated. It is a different interpolant that happens to
   share the word, so it is out.
2. **The clause states its own threshold**: "If Size is less than 4, cubic spline interpolation is
   not possible and Order 3 shall be ignored if specified." A natural spline exists through two or
   three samples, so four is not where it becomes possible. The not-a-knot spline needs four,
   because its two end pieces are each one cubic and a cubic is fixed by four values. Through
   exactly four samples it *is* the cubic through them. Of the C2 splines, this end condition is the
   one the sentence describes.
3. **It reproduces every cubic exactly, end pieces included.** A natural spline sets the second
   derivative to zero at both ends, a condition no sample stated, and so bends the end pieces of
   any function whose curvature there is not zero. The brief asked for a fixture that reproduces a
   known cubic between its samples. A natural spline fails that fixture, and Catmull-Rom fails it
   too: it reproduces quadratics, not cubics.

**The cost** is the one every C2 spline has, and the clause's general sentence names it:
"Interpolation shall be used to determine output values from the nearest surrounding values in the
sample table." A C2 spline's value depends on every sample on its line. So the coefficients are
solved once, at parse time, as uniform B-spline coefficients (`n + 2` per line of `n`). An
evaluation then reads the four nearest of them per input, which is as local as an evaluation can
be.

Two readings complete the choice:

- **One order per function.** The entry is ignored for the whole function when any dimension has
  fewer than four samples. Table 39 gives one order, and a function of mixed order would be a value
  it does not list.
- **An unlisted order is refused**, as an unlisted `/BitsPerSample` is. The crawl has none.

## 2. A second departure the row did not name

`Sampled::eval_into` encoded each input as if every `/Domain` were [0 1]. The clause instead maps
from the function's own domain: e = Interpolate(x′, Domain2i, Domain2i+1, Encode2i, Encode2i+1).
The clause's EXAMPLE 2 has the domain [-1.0 1.0], and the tree read column 0 where the clause puts
column 1. It is fixed, and that example is now a fixture.

## 3. Measured

- **The crawl's `/Order` population** (a byte census of type 0 stream dictionaries, which are never
  inside object streams): 126 393 functions. 79 476 state `/Order 1`, 46 747 state none, **170
  state `/Order 3`** across 60 documents, and none states any other value. Of the 170, 79 have a
  dimension under four, 81 are one-input functions of 512 samples, one has five, and 9 take two or
  four inputs. ADR 0098's "not one" was over the tracked 974, which still hold none.
- **Pages that move.** Nine documents reach the spline on page one (CPU raster, scale 1, A/B by
  rewriting `/Order 3` as `/Order 1` in place). All are under `corpus-cache/tika-issue-tracker/`:
  - `GHOSTSCRIPT-687352-0.pdf`: 29.2% of pixels, by up to 27 levels. Its five samples step from
    192 to 64, and any C2 spline rings through a step.
  - `poppler-235-0.pdf` and `poppler-68861-0.pdf`: 1.9%, by up to 7 levels.
  - Six by one level, at most 779 pixels: `GHOSTSCRIPT-689095-2.pdf`, `GHOSTSCRIPT-695427-1.pdf`,
    `GHOSTSCRIPT-689794-1.pdf`, `REDHAT-1802479-0.pdf`, `REDHAT-1801314-0.pdf` and
    `GHOSTSCRIPT-692884-1.zip-0.pdf`.
  - `raster_golden`: 974 held, 0 moved.
- **Cost.** These are callgrind instruction counts, which no load moves.
  - `poppler-235-0.pdf` (three two-input functions, 522 × 72 and 204 × 54): 531 M against 320 M
    linear. Evaluation is 245 M against 102 M (16 terms against 4 per output), and solving is 66 M.
  - `GHOSTSCRIPT-687352-0.pdf`: 101.8 M against 98.9 M.
  - `REDHAT-1802479-0.pdf` (one input, 512 samples): 644.7 M against 643.5 M.
  - No optimisation is taken: the one page where the cost is large is one page of the crawl.
- **The domain fix's reach.** 585 crawl functions in 65 documents state a domain other than [0 1]:
  368 state [-0.001 1], where the shift is a thousandth of the table; 177 state [-1 1 -1 1], the
  shape of a §10.6 spot function, which this tree does not apply; 34 state [0 3], and six others.
  The tracked population holds none.

## Consequences

§7.10.2 is `implemented`, and so is §7.10, which ADR 1599 derives. `doc/todo/65` and `doc/todo/01`
drop both. A later round that prefers a different spline re-reads §1's three reasons, not this
census.
