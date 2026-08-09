# ADR 0251 — The space in force, and the one a group declares

Status: accepted, 2026-08-09 (session 415).

## Context

`doc/todo/23` had one population left of the four it opened with: **a blending colour space that
is not the device's three components, for a *painted* group**, four corpus documents, all
`/DeviceCMYK`. ADR 0217 priced it in one sentence — a mask group's result is one number and
§10.4.2.3's reduction to it is linear, so it fits in a channel a rasteriser already composites;
a painted group's result is three components, so it "wants the group's raster in its own
components, which is a second raster format".

That sentence was a **prediction**, and the three rounds before this one each found that a clause's
own arithmetic collapsed something an earlier round had called structural: ADR 0220 found every term
of §10.4.2.4's black generation cancelling out of §10.4.2.3, ADR 0234 found §11.4.6's two stages
collapsing to two ordinary compositing operations, ADR 0237 found §11.4.4's NOTE 4 accumulators
cancelling under `Normal` to 5.6 × 10⁻¹⁶ over 200 000 inputs. `doc/todo/23`'s own last paragraph
said what to do about that: *transcribe §11.6.6 and §11.3.3 the same way and find out what actually
cancels.*

This round did, and found two things — one about the *population* and one about the *prediction*.

## The census, which is the finding that came first

`crates/pdf-model/examples/group_space_census.rs` reads every painted transparency group in the
corpus and prints the space it actually composites in rather than the one its dictionary carries.
Over 964 documents that open:

```text
115 page group(s) state a /CS, 7 of them not RGB — every one /DeviceCMYK
 44 document(s) hold a painted transparency group
 declared /CS: /DeviceCMYK 71, /DeviceRGB 32, [/ICCBased] 9, [/CalRGB] 1, absent 598
 effective:    /DeviceCMYK 96, /DeviceRGB 564, [/ICCBased] 8, [/CalRGB] 1, the device's 42
  1 group introduces a space that is not a three-component RGB one
```

**The declared column and the effective column are different populations**, and §11.6.6 says so in
the sentence this tree had never applied:

> For non-isolated groups, or if no group colour space is specified, the group colour space shall
> be inherited from the parent group or page.

§11.7.2 repeats it and gives the reason — "the use of an explicit colour space in a non-isolated
group would require converting colours from the backdrop's colour space to that of the group in
order to perform the compositing computations" — and §11.4.7 names the root of the inheritance:

> That initial colour space shall serve as the default blending colour space for each page, unless
> the page explicitly specifies an alternative default by means of its page dictionary containing a
> Group key that contains a CS key whose value represents a different colour space from the initial
> blending colour space.

So the four documents `doc/todo/23` named were the wrong four:

| | |
|---|---|
| `bug1721218_reduced.pdf` | 31 groups, **all `/I true`**, `/CS /DeviceCMYK`. §11.6.6's case, correctly reported before and after. |
| `issue18032.pdf`, `bug1755507.pdf` | every group **non-isolated**; what makes them depart is the *page* group, which states `/CS /DeviceCMYK`. Reported for the right reason now. |
| `issue14200.pdf` | one non-isolated group declaring `/DeviceCMYK`, on a page that states no `/Group` at all. **Nothing on it composites anywhere but the device's components, and the report was false.** |

And five documents were departing in silence, which is trap 5's failure rather than a missing
feature: `bug1365930.pdf`, `bug1703683_page2_reduced.pdf`, `issue12798_page1_reduced.pdf`,
`issue13520.pdf` and `personwithdog.pdf` all state a page group of `/DeviceCMYK`, so **every mark on
those pages composites in ink** — top-level objects as much as groups — and this tree read the entry
nowhere.

**Three of those five were already in the oracle's ambiguous bucket with a hand-written diagnosis
about transparency**, which is the corroboration worth having:
`AMBIGUOUS_STACKED_SCREEN_UNDER_MASKS` had written of `issue13520.pdf` that its five renderers show
"five readings of a stack of `Screen` blends under luminosity masks, which is §11.6.6's blending
space". Right about the clause family, wrong about which clause, and the entry it named is not the
one in force. The hypothesis was three sessions old and no gate could have told it from a diagnosis.

## Decision

### The interpreter carries the space in force, not the space declared

`Interpreter::blending` is `Option<String>` — the name of the effective blending space where it is
one this tree does not composite in, and `None` where it is. It is seeded from §11.4.7's page group
by `page_blending_space` and replaced for the duration of a group's content by `group_blending`,
which applies §11.6.6's two bullets: an **isolated** group with a `/CS` introduces its own space,
everything else inherits.

The report fires **once, where the file introduces the departure** — at the page for a page group,
at the group for an isolated one whose space differs from what it inherited — and, as before, only
where something composites, because an opaque `Normal` paint carries its colour through whatever
space it is carried through. A group that inherits a departing space is drawn no better and no worse
than the ancestor that introduced it, and that ancestor carries the report.

Two consequences fall out of reading the clause rather than the entry:

- The flattening path in `run_transparency_group` loses its `/CS` question outright. That branch is
  reached only for a non-isolated group, and a non-isolated group introduces nothing.
- Inside a *mask* group `blending` is cleared, and that is ADR 0220's finding rather than a
  simplification: a subtractive mask group is painted in the ink §10.4.2.3 weighs, that weighting is
  linear in the components, and a linear functional of a convex combination is the convex
  combination of the functional. The compositing this tree performs in there **is** the compositing
  the clause asks for; the one thing that is not — a blend function, which is not affine — is
  `note_blended_luminosity`'s report and not this one.

### A second raster format is genuinely required, and the reason is not the one ADR 0217 gave

§11.3.3's compositing formula under `Normal` is a weighted average, which §11.3.6 states outright:
"the compositing formula collapses to a simple weighted average of the backdrop and source colours".
A convex combination passes through an **affine** map unchanged. So the whole question is whether
the conversion out of the blending space is affine over the colours the group composites — nothing
about "three components" enters it, and ADR 0217's reason was the wrong reason for a true
conclusion.

Written down and run, per component, over 200 000–300 000 random colour pairs and alphas:

| the conversion | `Normal`, worst `|`CMYK route − device route`|` |
|---|---|
| §10.4.2.5's classic `1 − min(1, c + k)`, no channel over one unit of ink | **3.3 × 10⁻¹⁶** |
| §10.4.2.5 unrestricted (the clamp reached) | 0.459 = **117 of 255** |
| §10.4.2.5 with the clamp **deferred** onto three unclamped components | **3.4 × 10⁻¹⁶** |
| **this tree's multilinear interpolation of the ink cube** (ADRs 0009, 0042) | 0.189 = **48 of 255** |

So under the standard's own classic formula the collapse is exact away from the clamp, and the clamp
is deferrable by the identical trick ADR 0220 used one clause over — `1 − (c + k)` composites
linearly, so the `min` can wait for the compositing. **Under the conversion this project actually
uses it does not collapse at all**, because multilinear interpolation over a 4-cube carries products
of the inks and is affine on no face of it.

The simplest fixture there is says it without a corpus: half of registration black over paper is
`[76.0, 66.1, 63.9]` composited in `DeviceCMYK` and `[127.5, 127.5, 127.5]` composited on the device
— **51.5 of 255**, and in the direction a printer would expect, because half the ink of registration
black is still most of the way to black. `compositing_in_cmyk_is_not_compositing_in_the_device_and_this_is_the_gap`
pins both halves: that gap, and the fact that two colours differing only in `k` — one edge of the ink
cube, where multilinear interpolation *is* affine — agree to under half a level.

**Which makes the second raster format a cost of a decision rather than a demand of the clause**, and
that is worth writing down where the decision lives. §10.4.2.1 ranks §10.3's ICC route above
§10.4.2's "crude approximations"; ADRs 0009 and 0042 took the first and stand in for a profile with a
sixteen-corner interpolation. That choice is not reopened here — it is measured against three other
renderers and it wins — but it is now recorded in §10.4.2.5's ledger row as costing §11.6.6 its
cheap answer.

### What is therefore *not* built

Compositing in the blending space. It needs a four-component raster per group, with §11.3.4's
complement-before-and-after around every blend function, images and shadings painted into it, and
three backends taught the format — `render-quorra`'s `GroupSpec` and Vello's layers are both RGBA.
`doc/todo/23` keeps it, with the arithmetic above instead of a prediction.

Nothing is asked of `render-quorra` this round: no display-list command changed, so its refusal list
is where sessions 397 and 400 left it (`QUORRA_FEEDBACK.md` §14 and §16, both still open and both
still unanswered).

## Consequences

### What moved on the gates

Every raster is unchanged and that is checkable rather than asserted: the whole diff under
`crates/src` is report paths and one field that feeds nothing else, no `Command` is constructed
differently, and the oracle's verdict counts are **identical** — 905 agreeing, 68 contradicted, 786
ambiguous, 1 our-geometry, 2 reference-geometry, 14 not comparable, 18 no render. Only the
complete/incomplete split moved.

- **corpus 974 with 65 → 68 incomplete**, and every step of that is a report: `issue14200.pdf`
  leaves because its report was false, and `bug1703683_page2_reduced.pdf`,
  `issue12798_page1_reduced.pdf`, `issue13520.pdf` and `personwithdog.pdf` arrive because §11.4.7's
  entry is read at last. Trap 5's kind of rise, on four pages that were departing in silence.
  `bug1365930.pdf` states the same entry and does **not** report: nothing on its first page
  composites, so the space cannot change a pixel.
- The transparency population is **9 documents** rather than 6, and 7 of the nine name a blending
  space: one by §11.6.6 and six by §11.4.7.
- **oracle 1693 → 1690 complete, 101 → 104 incomplete**, with `agrees` 905 (863 complete),
  `contradicted` 68 (66 complete) and `ambiguous` 786 (753 → **750** complete). Four diagnosed pages
  left the ambiguous population by becoming incomplete and their groups are emptied with the
  measurement kept; `issue14200.pdf` arrived in it by ceasing to report and is diagnosed as
  `AMBIGUOUS_ICC_MATRIX_PROFILE`'s second member — a 918 × 427 `ICCBased` photograph where ours is a
  flat +0.148 / +0.157 / +0.160 of 255 per channel above two references that agree with each other to
  0.007, on a 3144-byte `mntrRGB` matrix-shaper profile `lcms` evaluates for both of them.
- **quorra 912 / 36 / 9 / 17 unmoved**, **text 99.2% (24043/24243 words) unmoved** with 62 → 65
  documents ungated, **dates 1514 of 1545**, **XMP 318 read**, **JPEG 2000 14 byte-identical** — all
  unmoved.
- **conformance 5980 → 6032 citations, 569 → 576 quotations**, 875 ledger rows and all six status
  counts unmoved. **workspace tests 1491 → 1493.**
- **`doc/todo/00`'s step 7 over all 786 ambiguous pages**: twenty at or past −1 and sixteen of them
  incomplete, head `issue16038.pdf` −5.758, `issue12295.pdf` −1.712, `checkbox_no_appearance.pdf`
  −1.200, `issue14297.pdf` −1.146, `issue7821.pdf` −1.000 — **the same five names in the same order
  to the thousandth as the three-hundred-and-ninety-seventh's and four-hundred-and-sixth's runs**,
  the alarm holding for the eleventh consecutive time. The four newly-reporting pages are unmoved to
  the thousandth as well — `issue13520.pdf` +0.695, `personwithdog.pdf` +0.719,
  `bug1703683_page2_reduced.pdf` +0.141, `issue12798_page1_reduced.pdf` +0.068 — and only their
  labels gained `[incomplete]`, which is ADR 0220's observation run backwards.

### The lesson, which is about *where* a value is read rather than what it says

Every one of the last four transparency rounds found a clause whose arithmetic was cheaper than the
tree assumed. This one found something else: the tree was reading the right entry from the wrong
place. `/CS` is a key in a dictionary and it means nothing on its own — §11.6.6 makes it conditional
on `/I`, §11.7.2 makes the condition a rule about backdrops, and §11.4.7 puts a root under the whole
inheritance that this tree had never opened. **A report can be false and a silence can sit beside it,
from one entry read literally**, and no gate in this project could see either: `issue14200.pdf`'s
false report kept a page out of the oracle's judgement for as long as it stood, and five pages
composited in ink with nothing said.

The instrument that found it is `doc/todo/13`'s rule — census the population before pricing the
work — and what made the census say anything was printing the *effective* space beside the declared
one rather than counting the declared one. **A census that counts what the file writes has counted a
spelling.**
