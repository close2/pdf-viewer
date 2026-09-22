# 1169 — The question a half of the page cannot answer

Status: accepted. Session 1166.
Amends: ADR 1157's construction, on a premise it stated correctly and a consequence it did not
follow — and ADR 1158 §1's sentence about what the pair's guard can see.
Context: `crates/pdf-render/src/paint.rs`, `crates/pdf-model/src/content/overprint.rs`,
`crates/pdf-model/src/content/transparency.rs`,
`crates/viewer-confined/src/protocol/display_list.rs`.
Clauses: ISO 32000-2 §8.6.7, §11.4.4, §11.4.6, §11.4.7, §11.7.4.3, §11.7.4.5 (Table 146).

## 1. The defect

A page whose group states `DeviceCMYK` is interpreted twice and the two lists are put back
together per pixel (§11.4.7, ADR 0262): one carries `1−c`, `1−m`, `1−y` and the other `1−k` in
all three channels. `DisplayList::geometry_digest` guards the pair, and what it hashes of each
command includes `std::mem::discriminant(&command.blend())` — because the halves are resolved
per pixel and a command present in one and absent from the other would be composited against a
shape that never drew it.

ADR 1157 chose the special overprinting blend mode per half, from that half's own three tints:

```rust
let kept = match half {
    Half::Chromatic => [tints[0] == 0.0, tints[1] == 0.0, tints[2] == 0.0],
    Half::Black => [tints[3] == 0.0; 3],
};
let Some(overprint) = Overprint::new(kept) else { return state.blend };
```

`Overprint::new` declined to build a mode that keeps no channel, on the true observation that
such a mode is Normal's arithmetic. So a source colour of `0.9 0.1 0.2 0` — three nonzero tints
and a zero black — gave `BlendMode::Overprint` in the black half and `BlendMode::Normal` in the
chromatic one. Two discriminants, one page: the digests differed, the pair was refused, and the
page **fell back to the device's three components with nothing reported**. Its overprinting was
gone, its four-component compositing was gone, and `is_complete()` was true.

ADR 1158 §1 wrote that the guard "hashes a blend mode's discriminant only, so the two halves'
different masks do not part them". That is right about `Overprint(a)` against `Overprint(b)` and
says nothing about `Overprint` against `Normal`, which is the pair the construction could
actually produce.

`crates/pdf-model/tests/overprint.rs::a_source_whose_only_zero_tint_is_black_keeps_the_pages_pair`
is the fixture, and it fails on the half-local question with the planted arm checked (trap 13).

## 2. The fix is to ask the clause's own question

§11.7.4.3's first bullet decides the **group space's four components**:

> If the overprint mode is 1 (nonzero overprint mode) and the current colour space and group
> colour space are both DeviceCMYK , then process colour components with nonzero values shall
> replace the corresponding component values of the backdrop; components with zero values leave
> the existing backdrop value unchanged.

Four components, one object, one decision. Which of a *raster's* three channels each decides is
a fact about this renderer's representation, not about the clause. So the question "is the
special mode in force here" is asked of the four tints — `tints.contains(&0.0)` — and it answers
the same in both runs; the per-half `kept` is then only the value the mode carries.

`Overprint::new` becomes infallible and the empty set is one of its values. A mode that keeps no
channel of *this* raster is still the special mode: it is `C_s` in every channel of this half and
the backdrop in every channel of the other, which is exactly what the colour asked for. The
decision that the mode is in force at all belongs where all four tints are visible, which is the
interpreter.

Two consequences follow and both are in this round:

- **The confined protocol's tags.** `blend_tag` encoded the seven non-empty subsets as 16 to 22
  by adding the bits to 15, so the empty set would have crossed the process boundary as
  `Luminosity`. The eight subsets are now 16 to 23.
- **Nothing else moves.** A page reaches the mode only where some tint is zero, and then the
  other half keeps something, so `DisplayList::overprints` — which the two refusing backends read
  — flags exactly the pages it flagged before.

## 3. A second guarantee that was not enforced

`Command::Group`'s `isolated` documents that `pdf-model` emits `false` "only where no enclosing
group is a knockout group", because §11.4.6's NOTE 6 gives a nested group the *outer* group's
initial backdrop rather than the immediate one. `implicit_knockout_group` — which builds §9.3.8's
and §11.7.4.4's groups out of commands already emitted — never checked its elements for one.
Nothing put a non-isolated group there before ADR 1170; a form XObject inside a Type 3 glyph
could, and would have been drawn on transparency rather than on its backdrop.

It now refuses such a list, which leaves the enclosing group the report it already has rather
than a picture computed against the wrong backdrop.

## 4. What this says about the instrument

The pair's guard did its job: it saw a structural divergence and refused. What was missing is
that **its refusal is silent** — the page falls through to the device's components, which is a
legitimate interpretation of a different question, and nothing says the file's own blending
colour space was dropped. A round that widens what a command may carry inside §11.4.7's pair
should ask what the two runs can now disagree about *before* asking what the page looks like;
this one found it by writing the fixture for the clause's own table rather than for the code.
