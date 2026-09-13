# 1054 — Two parameters a conversion must see, and one it must not

Session 1037. Status: **accepted**. Two decisions about which parameters reach a colour
conversion in the transparent model, taken together because they are the same question asked in
opposite directions: §11.7.5.3's second bullet, whose parameters a group's conversion *out* had
never read, and §8.6.5.6's default colour space mechanism, which had been reaching a quantity
that is not a colour at all. Moves no page of the tracked corpus.

`§N` is ISO 32000-2 and nothing else.

## 1. A soft-mask image is read against no resource dictionary

§11.6.5.2's mask is a subsidiary image `XObject`, and `image::colour_space` resolved its
`/ColorSpace` against the resources in force at the *parent's* `Do`. Those resources may carry
§8.6.5.6's `/DefaultGray`, whose value "shall be used as the colour space for the operation
currently being performed" — so a `CalGray` default with a tone curve remapped the mask's samples
as if they were greys. ADR 1008 found this in session 987, priced it, and did not take it.

**The clause settles it in its own arithmetic**, which is a stronger answer than any appeal to
what a mask is for:

> Colour values in the original device colour space shall be passed unchanged to the default
> colour space, which shall have the same number of components as the original space.

A remapping therefore changes which *colour* a value denotes and never the value. §11.6.5.2 uses
the value: Table 143 fixes the mask's `/ColorSpace` at "Required; shall be `DeviceGray`" and
states no conversion anywhere on the way from a sample to a mask value, where §11.5.3 spells one
out in full for the other kind of mask. So there is nothing here for §8.6.5.6 to act on.

`image::mask_colour_space` is that reading in one function: the mask's own dictionary, an empty
resource dictionary, and `Conversion::device()`. `soft_mask_entry`,
`eligible_for_the_device_scale` and `apply_soft_mask`'s own `decode` all take it, so the
question the interpreter's report asks and the one the eager route answers stay the same
question. `matte_colour` deliberately keeps the parent's resources: Table 144 states `/Matte` in
"the colour space specified by the `ColorSpace` entry … in the parent image's image dictionary",
and the parent *is* painted.

**What a malformed mask loses is a guess rather than a right answer.** A mask naming its space by
a resource key — which Table 143 forbids — now resolves to nothing and `soft_mask_entry` names it
unusable, where before it resolved against a dictionary the clause does not point at. That is
trap 5's rule: the refusal is loud and typed.

**The population is zero on this disk and the fixture is what holds the rule** (trap 8): no
`doc/pdf.js` document states a `/DefaultGray` at all. `image_masks.rs::a_soft_masks_samples_are_
not_remapped_by_a_default_colour_space` states a *linear* `/DefaultGray`, which is the widest gap
one number can show, and the calibration is the defect planted back — the parent's resources
passed in again fails that one test of the file's eighteen, naming 188 of 255 where the clause
gives 128.

## 2. A press is keyed on the rendering it was sampled under

§11.7.5.3's second bullet:

> When painting a transparency group whose colour space is CIE-based into a parent group having
> a different colour space, the rendering intent used shall be the current rendering intent in
> effect at the time the Do operator is applied to the group.

An `ICCBased` 'CMYK' group's conversion out is `colour::Press`'s sampled grid, and
`sample_press` took it through `Profile::to_rgb`, which is `A2B1`-else-`A2B0` with black point
compensation on, whatever the file stated. The reason was structural rather than argued: the
press was cached on the profile's identity alone, so there was nowhere to put a second answer.

**The key gains the rendering.** `PressIdentity::Profile` carries a `crate::icc::Rendering`
beside the profile's identity, `sample_press` takes the grid through `Profile::to_rgb_with`, and
`Interpreter::group_press` reads the rendering off the state at the `Do` — `outer`, the state
§11.6.6 has *not* reset, `inner` being the group's own. One profile under two renderings is two
presses, which is what it is: two different conversions out of the same four components.

**The conversion in moves with it.** `xyz_to_ink` asked `Profile::to_device` with compensation
hard-coded on, matched to what `sample_press` did; it now asks `Press::rendering` for the black
point. The invariant ADR 0263 established — the two directions are inverses — is kept by
construction rather than by two constants agreeing.

**Three callers pass `Rendering::compensating()` and each has a reason rather than a default.**
§11.4.7's page group has no `Do`: nothing has run, so the parameters are the initial graphics
state's, which Table 51 makes `RelativeColorimetric` with `/UseBlackPtComp` at a `Default` this
processor compensates. §11.5.3's mask group is stated in an `/SMask` dictionary (§11.6.5.1) and
reduced to a luminosity rather than painted at a `Do`. `Press::assumed` interpolates sixteen
compile-time constants and has no profile for an intent to select, which is why
`PressIdentity::Assumed` carries no rendering at all.

**What it costs the budget is stated rather than hidden.** §11.7.2's `MAX_PRESSES` counts
distinct presses per interpretation, so a page painting one profile's group under nine different
intents now names nine — nine conversions out, which is what that budget is a budget on.

`transparency_groups.rs::the_intent_at_the_do_selects_the_conversion_out_of_a_groups_press` holds
it on a fixture profile whose `A2B0` and `A2B1` are two *different* presses, both expected
colours computed from the profile rather than read off this tree's raster; the calibration is
`Rendering::compensating()` passed at the `Do` instead, which fails that one test of the file's
fifty-nine and names the transform.

## 3. What this does not do

Neither §11.7.5.3's black generation and undercolour removal, which remain read as a condition
and reported rather than evaluated, nor any part of §11.6.6's remaining spaces. `raster_golden`
holds 973 of 974 tracked first pages unchanged across both changes; the one that moved is another
round's font-error reclassification, confirmed by planting both of these back and watching that
document's reports stay identical.
