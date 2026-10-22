# 2230 — A page override can require DefaultRGB beside a shipped RGB intent

Session 1698, archive. Status: implemented; focused preservation and external
validation pass; final scoped, conformance and canonical archive gates pass.

ISO 19005-4 §6.2.3 makes a page's PDF/A output intent current on that page.
Its §6.2.4.3 offers a device-independent default as another licence for RGB.
ISO 32000-2 §8.6.5.6 defines default-space substitution; §11.6.6 applies
current blending-space conversion. A prepared RGB catalog intent therefore
cannot alone answer every page's DeviceRGB failure. This amends ADR 2114's
RGB-intent exclusion only for the proved route below.

The held `6-2-4-3-t04-fail-n.pdf` has no catalog intent: page 6 has CMYK
intent 11/profile 16, independent gray group 10/profile 15, and ordinary Form 12
with its own empty resources. The old conversion added an RGB catalog
intent and its output still failed the RGB row. The new preparation can
supply the existing DefaultRGB writer with the shipped sRGB source profile.

Admission requires part 4 and an actually prepared shipped sRGB profile.
Every page intent is checked before placement: valid indirect profiles,
consistent destinations, no external reference, and byte-identical shipped
sRGB for any RGB page override. At least one page overrides with Gray/CMYK.
Producer/supplied catalog profiles and the old non-RGB path are unchanged.

The writer places defaults globally, so the proof closes its entire resource
population first. Only ordinary page/Form paths with literal device colours
and path operators are admitted. Page groups are absent or supported ICC
Gray tone curves with kTRC and no A2B tables; Form groups, annotations, optional content, fonts, images, graphics
states, patterns, shadings and arbitrary named colour spaces are refused.
Existing defaults must be byte-identical shipped sRGB and are not overwritten.
Malformed, cyclic, deep, undecodable, damaged or over-budget paths refuse.
The shared survey must be complete and show a newly affected direct RGB use
on a non-RGB-override page. Its per-page Form deduplication is safe only because
the structural proof excludes differing invocation targets/defaults/state.

No new consent, schema or writer placement policy is introduced. The decision
remains Stated under the documented sRGB convention. Failed proof retains
the existing output-intent fallback and public output-verdict refusal; a new
precise public refusal API is not claimed. Q566 remains unresolved and untouched.
