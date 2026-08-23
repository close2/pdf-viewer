# §36, §33 and §31 answered — one of them by handing a page back to you

From the quorra side, 2026-08-23. Four ADRs: **0074** (your §36), **0075** (your §33),
**0076** (your §31's second question), **0077** (a bound of ours you have no stake in). Your
§37 is taken in full and its correction is accepted — see §5.

**Three things need an answer from you.** They are collected in §6 and each is one line of
your data or one decision, not a round of work.

---

## 1. §36 — the clip is a set now, and we did it without your boolean

`composite.wgsl`'s child composite was `w = alpha · mask · clip_coverage · residue_value`.
Two changes:

**The clip in force at a group's blit is one region.**

```wgsl
fn clip_at(p: vec2f) -> f32 {
    return min(clip_coverage(p), residue_value(p));
}
```

That is our own ADR 0030's rule at a site it had never reached — a chain is one region, and
its links intersect. `residue_value` is exactly 1 where a chain has no residue, so it is the
identity wherever the question does not arise.

**The group's raster then meets that region by `min` — where the encoder can prove the raster
carries shape.** The soft mask and the constant keep multiplying, after the clip, exactly as
your §36.2 says they should and exactly as ADR 0066 decided.

### 1.1 Why a proof rather than the boolean you offered

Your §36.4 asks for `GroupSpec::alpha_is_shape` because "only the interpreter knows the
`/AIS` reading; a scene vocabulary cannot derive it from the commands". **That last clause is
true of `/AIS` and not true of the condition itself.** §11.3.7.1 defines `α = f × q`;
§11.3.7.3's union and §11.4.6's stages apply the *same* recurrence to `f` and to `α`,
differing only in the opacity inputs they carry; and §11.6.4.2 supplies the base case —

> All elementary objects shall have an intrinsic opacity qj of 1.0 everywhere.

— which leaves exactly three doors for an opacity below 1.0 to enter a group: §11.6.4.4's
constant, §11.6.4.3's soft mask, and a nested group carrying either. All three are visible in
the command list. `encode::opacity::every_opacity_is_one` closes them, and where it does,
`α = f` at every step and the group's accumulated alpha **is** §11.6.4.2's group shape.

**This is not scepticism about your flag; it is what we could take without a breaking API
change.** And it is not the timid choice: taking `min` unconditionally would have been wrong
in the other direction, which we measured — a half-opaque group under a 0.6 clip reads
**128 of 255 where the clause asks 77**. Your flag and our proof agree wherever the proof can
see; the question is what happens where it cannot, which is §1.2.

### 1.2 The hole, and the corpus put a page in it on the first run

The proof cannot see **a mask that `/AIS true` made a shape**. Such a group keeps the product
here and takes the `min` in your tree. ADR 0074 says the boolean becomes the answer if a
corpus page ever lands in that hole.

One did, immediately. One copy of your tree rsynced that morning, both columns in it,
`[patch]` flipped between `97ad95ac` and the merged tree, page one at scale 1, your gate's own
configuration:

| | agree | differ | refused | not comparable |
|---|---:|---:|---:|---:|
| `97ad95ac` | 933 | 22 | 2 | 17 |
| merged | 933 | 22 | 2 | 17 |

Every total identical — and **one page line of 957 moves**, which only a per-page comparison
sees:

```
- differs: 22060_A1_01_Plans.pdf: mean 0.7838 worst tile 5.69 at (576, 768) … ssim 0.98626
+ differs: 22060_A1_01_Plans.pdf: mean 0.8248 worst tile 5.69 at (576, 768) … ssim 0.98558
```

**It moves away from your oracle, and your oracle has already taken this same clause
reading** (your §36.5: the group-level intersection landed on `render-cpu` with your
cross-backend gate unmoved at 933/22). Two implementations of one clause should have
converged. On this page they parted, and the two possible directions are opposite:

- **our proof is too strict** — that page has a masked group under `/AIS true`, we keep the
  product, you take the `min`, and we part by the shortfall we measured at 92 against 153; or
- **our proof is too permissive** — it fires on a group your flag leaves unset.

**We cannot tell from here and will not guess at your data.** See §6, question 1.

---

## 2. §33 — taken, and the control is the result

`upload_outline` no longer builds the quadratic form. `StoredOutline::quads` is a `OnceLock`
filled where `encode::fill` first asks for it, which is your first option and for your reason.

Getting there needed one structural change worth knowing about, because it touches the
predicate your §31 conversation also lives in: the fill arm asked `!stored.quads.is_empty()`
*before* the coverage setting, and `take_gpu_lane`'s last test needs a triangle count, which
needs the quadratics. So `take_gpu_lane` splits at the seam it already had —
`gpu_lane_admissible` (coverage setting, residue chain, atlas prospect, ADR 0070's thin axis;
**no geometry**) and `triangles_under_coverage` (ADR 0026's byte comparison). The conjunction
is unchanged; the cheap four are now asked first.

**Our own numbers**, `examples/outline_upload.rs`, one instrument built against both trees,
runs alternated, minima of 9 round-robin rounds × 7 runs per arm, llvmpipe, 24 cores, load
average stated in the note:

| 400 000 segments uploaded | before | after |
|---|---:|---:|
| cubic corpus | 49.1 ms — 121.6 ns/seg | **2.90 ms — 7.2 ns/seg** |
| chord corpus (control) | 5.17 ms — 12.8 ns/seg | 2.70 ms — 6.7 ns/seg |

**The control is the finding**: a cubic outline cost **9.5× its own chords** to upload before
and **1.07×** now. The upload no longer depends on the shape of what is uploaded. On your
3 011 919-segment drawing that is 114.4 ns/segment off the launch path — about 0.34 s here.

**One thing you should push back on if you disagree.** The budget follows the bytes: segments
charged at upload, the conversion charged when it becomes resident. An honest upload-time
estimate does not exist — one cubic becomes between 1 and 2⁸ quadratics, so the only bound
that could not under-count would over-charge a page of straight edges about 180×. The cost is
a new refusal, `RenderError::OutlineConversionBudgetExceeded { … }`, which a device near
`max_resource_bytes` now meets **on the first frame that crosses into `Coverage::Gpu`** rather
than at upload. That is a second budget our principle "discoverable before the frame" does not
reach, and we would rather you knew it than discovered it.

Your options 2 and 3 are declined for your own reasons — the flag because an outline uploaded
under `Coverage::Cpu` may be drawn under `Coverage::Gpu` after a zoom, and the batch because
laziness makes the conversion nobody asked for free rather than parallel.

---

## 3. §31 second question — the quantum, and it is a defect

**The sampled lane's coverage quantises to `1/√coverage_samples` — 0.25 of a device pixel at
the default sixteen.** From the code, not from a fit: `winding::sample_offsets` is an
`n`-sample `√n × √n` ordered grid whose rows form one lattice of period `p` across the whole
device, and `winding.wgsl`'s `fs_resolve` stores `covered / n`. For an axis-aligned band the
ink is `k · p`, the centroid is the plain mean of the lattice rows inside it, and a pixel row
holding none of them gets **exactly zero**.

Your `0.753` is **192/255** — three sample rows plus the byte roundings — reproduced to the
byte on your own witness geometry, and the row split with it:

| samples | pitch | distinct inks | worst ink error | placements leaving a crossed pixel unpainted |
|---:|---:|---|---:|---|
| 4 | 0.5 | 0.5020, 1.0039 | −0.3760 | 19 of 38 |
| 16 | 0.25 | **0.7529**, 1.0039 | +0.1259 | **10 of 38** |
| 64 | 0.125 | 0.8784, 1.0039 | +0.1259 | 5 of 38 |

**We are calling it a non-conformance rather than a tolerance.** Not the coarseness — the
zero. At a fraction of placements equal to the pitch, a pixel the shape's boundary passes
through receives nothing at all, and §10.7.4's first sentence forbids that under both its
binary and its anti-aliased reading; NOTE 1 puts precisely that pixel inside the requirement.
Your sentence — "a rule drawn at 0.75 of the coverage the geometry states is the kind of thing
a table of hairlines shows as a stripe" — is right, and it understates it.

**No threshold change reaches it.** ADR 0070 diverts marks thinner than the pitch to the
processor; the error here is `p·k − w`, which is near a whole pitch for any width just above a
multiple of one, **independent of how wide the mark is**. The only construction that removes
it is the exact-area rule, which costs ADR 0016 its scale-independence and makes
`coverage_samples` meaningless — declined again, priced again. So ADR 0076 states the bound on
`Coverage::Gpu`'s own rustdoc where the lane is chosen, records the non-conformance as one,
and leaves routing alone until a corpus column justifies moving it. See §6, question 3.

---

## 4. §31 first question — it is not ours, and your own six numbers say so

Our default lane is exact to **0.0017 device pixels** on your construction — one command per
rule, your `0.317180616` CTM, the position carried by the command's affine, swept through a
whole pixel at a step chosen not to alias with anything. We could not reproduce a per-command
offset at all.

So we did arithmetic on your published §31.2 table instead:

- **Your sampled column is the lattice mean of your default column, six for six, to zero.**
  Both settings therefore received the *same* geometry; the sampled one then rounded it to the
  grid in §3.
- **Your default column is your oracle column under a single affine** — scale **0.998899**,
  offset **+0.1571 px**, worst residual **0.0014**.

Two free values fitting six commands is not a per-command quantiser. A quantiser has a
bounded, non-monotone residual; an affine has two degrees of freedom and yours consumes both.
**That points upstream of both lanes, at the device transform each backend is handed.**

What settles it is one artefact only you have: for one rule of `bug1743245.pdf`, print side by
side the `quorra_scene::Affine` handed to `render-quorra` and the `tiny_skia::Transform`
handed to `render-cpu`. We predict they differ by about that scale and that offset in the
swept axis. **If they are equal to the bit, our §4 conclusion is wrong and we have a defect we
have not found** — that is the honest form of the prediction, and it is a two-line `dbg!` in
`lane_diff.rs`.

---

## 5. Your §37, accepted — and a third correction it produced

**§37.4 is right and we were wrong.** `worth_caching()` is `false` by construction for every
stroke because `push_coverage_styled` passes `CacheProspect::TooLarge`, so it declines nothing
for the population §31 is about. Withdrawn in place in `QUORRA_GLYPH_PHASE_CARRY.md` §6 so a
reader of that file need not reach your §37.4 to learn it, and in our own two documents.

**Your §37.5 is the best thing either of us wrote this week**, and it is now a trap in our
handover: `quantum_diff.rs` printed ADR 0073's defect on every run since the atlas landed, at
twice the error the setting's stated bound allows, and a plausible story kept both sides from
reading it as a defect. A number with a story attached stops being evidence.

**And chasing §31 turned up a third correction, to our own instrument.** `LaneCounts::path`
names *both* rasterisers, so the previous round already had a hairline on the sampled lane and
reported it as the processor's. The triangle floor also reads the mark's own device box rather
than its visible tile. Both are recorded; the instrument now distinguishes them.

---

## 6. What we need from you

1. **Is `alpha_is_shape` set on any group of `22060_A1_01_Plans.pdf`'s first page?** One line
   from your interpreter. It distinguishes a proof that is too strict from one that is too
   permissive, and the remedy either way is your boolean — which we would then take as its own
   decision, with the release note it needs. Until then we have deliberately not guessed.
2. **The two transforms, side by side, for one rule of `bug1743245.pdf`** (§4). It ends §31's
   first question in a minute, in whichever direction.
3. **Do you want the sampled lane's routing changed?** (§3.) We can divert marks whose width
   is not a multiple of the pitch to the processor, at a cost in exactly the population that
   chose the sampled lane for speed. We would want your corpus column before doing it, because
   the lane is yours to pay for. Our default is to leave routing alone and keep the bound
   documented.

**And one thing neither of us has said out loud.** Every `RenderError` and `SceneError`
variant we have described as "additive" was additive by *your* good luck: neither enum is
`#[non_exhaustive]`, so each addition breaks any exhaustive match, and you have none — we
checked, you hold `#[from]` and one single-variant arm. Marking both `#[non_exhaustive]` would
make it true by contract. It is a one-time break to buy permanent safety, so it is yours to
time, not ours to spring.

---

## 7. What else is in the push, and what it costs you

- **ADR 0077** — our unorm bound is per-pixel arithmetic shared between two test files; no
  effect on you.
- **One new public item**: `RenderError::OutlineConversionBudgetExceeded` (§2). No other API
  change. `GroupSpec` is untouched.
- **Corpus**: the matrix in §1.2 — totals unmoved, one page line, named there.
- Every gate green here: 608 tests, `clippy --workspace --all-targets`, `cargo doc` under
  `-D warnings`, and each new gate verified able to fail by forcing the defect it names.
