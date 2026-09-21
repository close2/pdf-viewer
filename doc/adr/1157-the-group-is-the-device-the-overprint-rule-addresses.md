# 1157 — The group is the device §11.7.4's overprint rule addresses

Status: accepted. Session 1160.
Supersedes: ADR 0028's overprinting half, on the expiry condition that ADR wrote into its own
record. Its §10.7.5 half — stroke adjustment, and `Stroke::device_width` — is untouched.
Builds on: ADR 0262, which made a page or isolated group composite in the four components its
blending colour space names.
Context: `crates/pdf-model/src/content/overprint.rs`, `crates/pdf-model/src/content/ext_gstate.rs`,
`crates/pdf-model/src/content/colour.rs`, `crates/pdf-model/src/content/{path,text}.rs`,
`crates/pdf-render/src/paint.rs`, `crates/render-cpu/src/blend.rs`,
`crates/pdf-model/tests/overprint.rs`.
Clauses: ISO 32000-2 §8.4.5 (Table 57), §8.6.6.4, §8.6.7, §11.3.4, §11.3.6, §11.4.4, §11.4.7,
§11.7.2, §11.7.3, §11.7.4.1–§11.7.4.5 (Table 146).

## 1. The question, and what had changed under it

ADR 0028 left `/OP`, `/op` and `/OPM` deliberately unread and gave two independent readings for it.
The first was §8.6.7's, the opaque imaging model, and it still holds: this device has three
additive process colourants and produces no separations, so NOTE 1's "[i]f overprinting is not
supported, the value of the overprint parameter shall be ignored" applies, and the overprint mode
is settled by a `shall not` in the clause's body — "[i]t also shall not apply if the native colour
space of the output device does not include CMYK device colourants; in that case, source colours
shall be converted to the device's native colour space, and all components participate in the
conversion, whatever their values."

The second was §11.7.4's, the transparent model, and it was a *derivation over Table 146*: every
row that a group of three process components and no spot colourants can reach gives `B(C_b,C_s) =
C_s`, which is Normal. ADR 0028 stated the derivation's one premise and its expiry in the same
sentence: "[e]xactly one configuration breaks it: a group whose blending colour space *is*
`DeviceCMYK` … the day this renderer composites in a document's colourants it owes the whole of
Table 146."

ADR 0262 is that day. A page or isolated group whose blending space has four components is drawn as
two rasters and put back together where §11.4.7 puts the conversion. The premise expired and the
row did not move.

## 2. The clause, read again: the device the rule addresses is the group's space

Three sentences settle it, and none of them is about the screen.

**§11.7.4.3's first bullet conditions on the group, not the device.** "If the overprint mode is 1
(nonzero overprint mode) and the current colour space and group colour space are both DeviceCMYK,
then process colour components with nonzero values shall replace the corresponding component values
of the backdrop; components with zero values leave the existing backdrop value unchanged." No
clause of §11.7.4 mentions the output device's colourants as a condition on the mode.

**§11.7.4.2 states where the arithmetic happens, and it is a `shall`.** "If the group colour space
is different from the native colour space of the output device, its components are not the device's
actual process colourants; the blending computations shall affect the process colour ants only
after the group's results have been converted to the device colour space." That is exactly
`pdf_render::blending::resolve` running after every element has composited — so §8.6.7's remedy
("source colours shall be converted to the device's native colour space, and all components
participate in the conversion") is factually not what this renderer does inside such a group. The
sentence's condition and its consequence travel together, and the consequence is absent here.

**§11.7.4.5's NOTE 1 names this circumstance as the one where the two models differ.** "This
difference between opaque and transparent overprinting and erasing rules arises only within a
transparency group (including the page group, if its colour space is different from the native
colour space of the output device)." The screen's three colourants are not the reason the rule does
not apply; they are the circumstance in which the standard says the transparent rule *is*
different.

So the owner's revisit note is upheld on its own argument, and both of its factual claims held when
checked: ADR 0262 does composite in stated blending spaces including `DeviceCMYK`, and
`ext_gstate.rs` did reason from the screen's colourants where the clause reasons from the group's.

## 3. What is owed is one cell, not the whole table

ADR 0028 said the day would owe "the whole of Table 146". Read against what this renderer actually
builds, it owes one cell.

The group space here is four **process** components and nothing else: §11.4.7's pair carries `1−c`,
`1−m`, `1−y` in one raster and `1−k` in the other, and no spot colourant is maintained beside them.
§11.7.2 makes that space `DeviceCMYK` for the purposes of the bullet even when the file named an
`ICCBased` 'CMYK' profile — "DeviceCMYK shall be redefined within the transparency group to be the
same as the blending colour space and references to the process colourants Cyan , Magenta , Yellow
and Black are defined to be references to the corresponding colourants in the blending colour
space, even where the actual or simulated output device is not CMYK."

Row by row:

| row | reachable? | why |
|---|---|---|
| 1, `C, M, Y, or K` | **yes** | the group's four components; its `OP true, OPM 1` cell is `C_s if C_s ≠ 0, C_b if C_s = 0` — **the one cell owed** |
| 2, process component other than CMYK | no | the group has no fifth process component |
| 3, 5, 7, 8, spot colourant | no | no spot colourant exists beside the group's four |
| 4, any process space → process component | yes, and it is `C_s` in all three columns | already Normal |
| 6, `Separation`/`DeviceN` → process component | no | §11.7.3: "[i]f any other colour space has been specified for the group, the Separation or DeviceN colour space shall be converted to its alternate colour space", so such a space never *is* the current one here |
| 9, a group | yes, `C_s` | NOTE 2 reverts a group to Normal |

**`shall`, `may`, and what follows.** §11.7.4.3's first sentence is a permission — "a PDF processor
may consider implementing a special blend mode that consults the overprint-related graphics state
parameters" — so declining the mode entirely is something the standard allows, and §8.6.7's NOTE 1
is the sentence that would license it. Everything after the permission is a `shall` conditional on
taking it: the mode's value (the two bullets, Table 146), the implicit group for a non-Normal
current blend mode, and §11.7.4.4's first bullet. Declining is therefore *permitted* and is not
*inapplicable*; `CLAUDE.md`'s "[e]very PDF that exists renders as its producer specified" and
§6.3.2.1's conformance floor being "the floor, not the goal" decide between the two, in the
direction of building it. `pdf-archive`'s ISO 19005-2 processor row saying this program shall
respect the three entries is consistent with that and is not the reason.

## 4. The construction

**The zero test is made where the clause puts it.** §8.6.7: "Determination of whether a tint value
is zero or non-zero shall be made on the tint value defined within the PDF file, before
quantisation into a device tint value for the output device." A raster channel cannot answer that —
a tint below half a level quantises to the whitest level and is still nonzero — so the interpreter
keeps the four tints a `k` or `scn` operator stated in `DeviceCMYK` (`GraphicsState::fill_tints`),
and the command carries **which channels** the bullet leaves to the backdrop rather than a
predicate a backend would re-evaluate. That is `pdf_render::Overprint`, and
`BlendMode::Overprint(Overprint)` is the seventeenth blend mode — one no document can name, because
§11.7.4.3 says it "shall not be invoked explicitly".

**Table 146's first row is "DeviceCMYK , specified directly, not in a sampled image", and
"directly" is read as written.** The mode is chosen only where the current colour space is
`ColourSpace::Cmyk` and the content stream stated the four numbers. A `Separation` or `DeviceN`
reverting to a `DeviceCMYK` alternate is *not* this case even though §11.7.4.3's NOTE 2 makes the
alternate the current colour space: the numbers the zero test is about would then be a tint
transform's output rather than anything "defined within the PDF file", and the table gives such a
space rows of its own. §8.6.7 draws the same line in words — the mode "shall apply only to painting
operations that use the current colour in the graphics state when the current colour space is
DeviceCMYK", and "shall not, however, apply to the painting of images or shadings". Images and
shadings are excluded by both, and a patterned paint carries no tints here for that reason.

**One raster, both halves.** The chromatic pass keeps the channels whose `c`, `m`, `y` tints are
zero; the black pass keeps all three where `k` is zero, since that raster holds `1−k` in each. The
`geometry_digest` that guards the pair hashes a blend mode's discriminant only, so the two halves'
different masks do not part them.

**`render-cpu` computes it; the other two refuse by name** — ADR 1158.

## 5. Consequences

- §8.6.7 moves `inapplicable` → `implemented`, §11.7.4.1 `inapplicable` → `implemented`,
  §11.7.4.3 `implemented` → `partial`, §11.7.4.5's note is rewritten on the same status, and
  §11.7.4.4 keeps `partial` with the first bullet added. The aggregate §11.7.4 stays `partial`.
- **The lesson is ADR 0028's own, and it is the one `CLAUDE.md` states about a claim that decays.**
  The derivation was sound, it was written down with its premise, and its premise was retired by a
  round working on something else. What did not happen is the row being re-read when that happened.
  A status resting on a stated premise is a status that has to be re-read when the premise moves,
  and nothing in this tree watches for that but a person reading the clause again.
