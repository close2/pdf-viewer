# Specification audit of ISO 32000-2 clause 11 (transparency) — round 2171

The project's eighth specification audit, made beside the other lanes in the shape of the owner's
review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits 2128 to
2161 ([`2161-specification-audit-9-2-9-4.md`](2161-specification-audit-9-2-9-4.md)). Baseline:
main at `41a51cb0`. Evidence: [`2171-specification-audit-11/`](2171-specification-audit-11/) —
`findings.json` (2038's schema) and `expected-failures.json` (the failing tests of each
behaviour finding). The same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. Every `shall` and `should` of §11.1 to §11.7.5.3 (Tables 134 to 149) was read in
   `doc/md/ISO_32000-2_sponsored_EC3.md`, lines 9297 to 10482; §11.3 and §11.4's formulas are not
   decoded there and were taken from the PDF where a probe needed one.
2. 2038's clause 11 findings were read first and are not repeated: B14, B39, D29, T08, T44, T57,
   T60, T77, T102, T104, T105, T107, T122, T135, T170, T171; and 2161-B01 (§11.7.4.4 through a
   nested `BT`), since fixed.
3. The interpreter was traced through `pdf-model`'s `content/ext_gstate.rs` (Table 57's `/BM`,
   `/SMask`, `/CA`, `/ca`, `/AIS`), `soft_mask.rs` and `content/transparency/` (groups, knockout,
   the implicit groups of §11.7.4.3 and §11.7.4.4, presses, soft masks, blending spaces), and
   the backends' compositing in `render-cpu/src/blend.rs` and `groups.rs`, `raster-gpu`'s
   `composite.wgsl` and `reduce.wgsl`, and `render-gpu/src/scene.rs`.
4. The lens was the earlier audits' habits: a rule keyed on a name applies wherever the name is
   read (Table 147's `/K` on a page as on a form; `/BM` in an `/ExtGState`, an annotation, a
   pattern cell); a parameter has a type as well as a place (`/BC`'s length, `/BM`'s name); a
   value has a reader on the way out as on the way in; a bound has two sides (`gs` sets a mask
   and a later `gs` may or may not unset it).
5. About forty fixtures were probed end to end, content stream to pixel, on `render-cpu`, and the
   same display lists drawn by `render-raster` (quorra, on the RX 580) and `render-gpu` (Vello).
   Each B finding has failing tests with controls that pass on main, in
   `pdf-model/tests/transparency_clause11_spec_audit.rs`, a module of `transparency_suite` (ADR
   3086). No portable-suite test was added: two findings are reports on malformed input, which
   `portable-conformance/` does not state, and the two pixel findings are carried by the oracle.

What holds, probed and passing on main, is not listed one by one. Among it: §11.6.3's array
taking the first *recognised* name; `/SMask /None` removing the mask; a soft mask in force at a
`Do` applied once to the group's result, and the group's content starting at Normal, alpha 1 and
no mask (Table 147's "Initial blend mode"); `/CA` alone not applied to a group (§11.6.4.4: only
the nonstroking constant); `/AIS` true making `/ca` and a soft mask shape in a knockout group
(blue at shape ½ over red gives (127, 0, 128); opacity ½ gives (127, 127, 255)); an isolated group
compositing a Multiply onto transparency and a non-isolated one onto the page; a non-isolated
group inside a non-isolated knockout group taking the knockout group's initial backdrop (Table
147); a `/Group` whose `/S` is not `/Transparency` not grouping; the soft mask's coordinate system
fixed at the `gs`; an image's `/SMask`, and its `/Mask`, overriding the graphics state's soft mask
for that image (§11.6.4.3, "Either form of mask"); a tiling cell's transparency parameters reset
and the cell composited as a non-isolated group (a Multiply in the cell reads the page under it);
a `DeviceCMYK` mask group's default `/BC` black; `Hue`, `Saturation`, `Color` and `Luminosity`
in a `DeviceGray` and a `CalGray` page group answering as §11.3.5.3's RGB route does (the
backdrop, the backdrop, the backdrop, the source); §11.7.4.3's compatible mode under Multiply on a
`DeviceCMYK` page group (C 0.5 over C 0.5 gives C 0.75, M kept).

Two readings were checked and left. §11.5.3's CIE branch — "convert to the CIE 1931 XYZ space and
use the Y component" — is not what the tree does for a `CalRGB` mask group (the device formula
gives 0.5 where Y is 0.218); that is the owner's departure A23 (ADR 2552), stated, and not a
finding. And `render-gpu`'s knockout at a partially covered pixel keeps `(1 − a)(1 − a·αs)` of
the destination where §11.4.6 asks `(1 − a)`; `scene.rs`'s `knock_out` states the bound and the
cross-backend scene measures it, and antialiasing coverage is not a shape the clause names.

## Backends

At the centre of every fixture the three backends agree to within raster rounding, or one of them
refuses by name: Vello a non-isolated group, a four-component page group and a one-component
page group through a curve; quorra a non-isolated knockout group. Each refusal is a typed error
and the viewer's frame goes to another lane, so no disagreement between backends is a finding.

## Findings

Four behaviour gaps, all in `pdf-model` and therefore in all three backends. In order of how much
a person would see:

- **2171-B01** — a `gs` whose dictionary has no `/SMask` removes the soft mask in force. An absent
  key reads as `Null`, `entry_with_output_intent` answers `Null` with `SoftMaskEntry::None`, and
  `ext_gstate.rs` sets `state.soft_mask = None` for it as for `/None`. `/M gs /A gs` where `/A`
  states only `/ca` draws a red fill unmasked. §8.4.5 sets the parameters a dictionary names, and
  §11.6.4.3 gives the removal to the name `/None` alone.
- **2171-B02** — the page group's `/K` is not read. Table 147 alters `/I` and `/CS` for a page and
  not `/K`; a page with `/K true` draws its overlapping translucent squares composited as in a
  non-knockout group, and says nothing.
- **2171-B03** — a soft-mask dictionary that breaks Table 142 is drawn in silence: a `/BC` whose
  length is not the group space's component count is replaced by the default black, and a
  `/Luminosity` mask whose group states no `/CS` (Table 142's `/G` and Table 147's `/CS` both say
  it shall) is composited on device RGB. Neither reaches `unsupported`.
- **2171-B04** — a `/BM` name outside Tables 134 and 135 is drawn as Normal and not reported.
  §11.6.3's Normal fallback is the array form's; an unknown name is unsupported input.

## Ledger rows contradicted

All are `implemented` and stay so until the fix round, which re-reads each: `11.6.4.3` and `8.4.5`
(B01), `11.4.7` and `11.6.6` (B02), `11.6.5.1` (B03: its note says `/BC` "defaults to the initial
colour of the space `/CS` *states*", which is the absent case, not the malformed one), `11.6.3`
(B04: its note says all sixteen names reach both backends and is silent on the seventeenth).

## Suggested fix groups

1. **`soft_mask.rs`'s `entry_with_output_intent` with `content/ext_gstate.rs`** — B01: an absent
   `/SMask` returns a third answer, *not stated*, that leaves `state.soft_mask` alone; `/None` and
   an unusable value keep theirs. A small change with a wide reach: every page that sets a mask
   and then a second state is affected.
2. **`content.rs`'s page set-up with `content/transparency/knockout.rs`** — B02: the page group's
   `/K` read where its `/CS` is (`blending_space.rs`'s `page_blending_space`), and the page's
   content run as the elements of an isolated knockout group, or reported where a backend cannot.
3. **`soft_mask.rs`'s `read`** — B03: a `/BC` of the wrong length or with a non-number reported
   beside the default it falls back to, and a `/Luminosity` group without `/CS` reported where it
   is composited on device RGB.
4. **`content/ext_gstate.rs`'s `stated_blend_mode`** — B04: a name `known_blend_mode` does not
   know reported (the array form stays silent, as §11.6.3 gives it a fallback).
