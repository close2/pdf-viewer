# 1194 — The grey a press separates, and the one §10.4.2.3 states

Status: accepted. Session 1178.
Context: ISO 32000-2 §10.4.2.1, §10.4.2.2, §10.4.2.3, §10.4.2.4, §10.4.2.5, §10.3, §11.3.4,
§11.5.3, §11.6.6; ADRs 0217, 0262, 0263, 0790, 0796, 1119 (the word), 1157 (what the zero test
reads).
Code: `crates/pdf-colour/src/colour.rs`.
Tests: `crates/pdf-colour/src/colour.rs::a_grey_is_separated_by_the_press_rather_than_by_the_clauses_nominal_black`.

## 1. The clause has two directions and only one of them is this tree's

§10.4.2.3 states both conversions between `DeviceGray` and `DeviceCMYK`, each as a `shall`:

> Nominally, a gray level is the complement of the black component of CMYK . Therefore, the CMYK
> colour value equivalent to a specific gray level shall be

with cyan, magenta and yellow at 0.0 and `black = 1.0 - grey`; and, the other way,

> To obtain the equivalent gray level for a given CMYK value, the contributions of all components
> shall be taken into account

with `gray = 1.0 - min(1.0, 0.3 × cyan + 0.59 × magenta + 0.11 × yellow + black)`.

**The second direction is executed exactly**, and on a route to a pixel rather than as arithmetic
kept for its own sake: `ColourSpace::ink` is the sum inside the `min` and `ColourSpace::luminosity`
applies the `min`, which is what §11.5.3's device branch asks of a `/Luminosity` soft mask whose
group blends in a subtractive space, and what `Compositing::Grey` asks of a `k` operator painted
into a `/DeviceGray` blending space (§11.3.4, §11.6.6; ADRs 0217, 0790). The linearity of the sum
is why a mask group can be *painted* in one number and composited by an ordinary rasteriser.

**The first direction is not.** A grey reaching `ColourSpace::to_cmyk` — the conversion
`Compositing::Subtractive` performs for every colour painted into a `DeviceCMYK` group — is
separated by `rgb_to_ink`, the right inverse of the press's own cube (ADR 0263), or by the
profile's `B2A` where the press has one (ADR 0796). Neither is this clause's black generation.

## 2. Which of the two honest words this is

§10.4.2.1 ranks the whole family:

> Although ICC enabled PDF processors should always follow the provisions and recommendations
> provided in 10.3, "CIE-Based colour to device colour", a less-capable PDF processor may choose to
> use the algorithms specified in the following subclauses 10.4.2.2 through 10.4.2.5. These
> algorithms are, however, very simple and as perceived by a human viewer they produce only crude
> approximations of the original colours.

Two readings were available and only one survives the sentence. **Not `implemented`**: the ranking
is a `should` on the processor's *route*, not a statement that the route it ranks second produces
the same answer, and this clause's own `shall` says what the answer is. A row calling the clause
implemented would be claiming a conversion nothing here performs. **`departed`** is the word (ADR
1119): every other requirement of the clause is executed, this one was decided against for a reason
the standard itself states, and the decision has a price somebody can read. That is the same
disposition §10.4.2.5's row has carried since ADR 0263, settled by the same sentence — which is the
point, because two clauses of one list decided two ways would be a ranking applied twice with
different results.

## 3. The price, measured

`a_grey_is_separated_by_the_press_rather_than_by_the_clauses_nominal_black` walks twenty-one greys
and takes two figures, because the departure costs in one of them and not in the other:

- **In components, up to 0.60.** A tenth-grey separates here to cyan 0.60, magenta 0.47, yellow
  0.44 and black 0.88 where the clause states 0.0, 0.0, 0.0 and 0.90; a mid grey to
  (0.12, 0.07, 0.08, 0.50) against (0, 0, 0, 0.50). Every grey but black and white carries
  chromatic ink the clause puts at zero. That is what §11.3.4's per-component compositing formula
  reads inside a `DeviceCMYK` group, so a grey blended against a coloured backdrop there is being
  blended from different operands than the clause's.
- **In pixels, nothing.** Taken back out through the press's cube the same grey is the grey the
  file stated, inside one level of an eight-bit channel across all twenty-one — the right-inverse
  property ADR 0263 built for. An opaque mark does not move.

**And it does not reach §8.6.7's zero test**, which is worth stating because that is where a
non-zero cyan would be loudest: ADR 1157 makes the nonzero-overprint test read the four tints
"defined within the PDF file", which `content/colour.rs::cmyk_tints` supplies only for a colour the
stream stated in `DeviceCMYK`. A grey never becomes such a colour, so no component this conversion
invents can leave a backdrop standing that the clause would have overwritten.

## 4. What would revisit it

The ranking, not the arithmetic. §10.4.2.1's preference is conditioned on being an "ICC enabled"
processor; a build of this program that had no press to search against would be the "less-capable
PDF processor" the sentence hands these algorithms to, and this clause's formula would then be the
answer rather than the nominal slice `rgb_to_cmyk` already computes on the way to the search. The
formula is therefore in the tree and evaluated (`colour.rs::rgb_to_cmyk`, §10.4.2.4's first step,
which on a grey *is* this clause's formula term for term) — what is departed from is answering
with it.
