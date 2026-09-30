# 1412 — A transcoded JPX image's opacity channel is its soft-mask image, and three more rows share the transcode

Session 1287. Status: accepted and **built**.
Context: ISO 32000-2 Table 87 (`SMaskInData`, `SMask`), §11.6.5.2, Table 143, Table 144; ISO 19005-2
section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 (cited, paraphrased). Code:
`crates/pdf-transform/src/archive/transcode.rs` (`SITES`, `soft_mask_image`, `zero_matte`),
`archive/prepare.rs` (the soft masks are added objects), `archive/mod.rs` (`kept_where_it_is`),
`archive/config.rs` (`PRESERVABLE_IN_PLACE`), `archive/decision.rs` (`SHAPES`, the refusal
sentence), `doc/profiles/keep-everything.toml`, `doc/pdf-a-mitigations.md`. Tests:
`crates/pdf-transform/tests/archive.rs`. Amends ADR 1400 section 3 (its `SMaskInData` refusal) and
section 4's last sentence (the three rows it left un-re-classed).

## 1. The opacity channel becomes the object Table 87 names

Table 87's code 1: "A PDF processor shall create a soft-mask image from the information to be used
as a source of mask shape or mask opacity in the transparency imaging model." A Flate copy has no
channel for it, so the transcode creates that image — `/Subtype /Image`, `/ColorSpace /DeviceGray`
as Table 143 requires, the parent's `/Width` and `/Height`, the opacity channel's own depth where
Table 87 can state it and the widest's field with a widened `/Decode` otherwise — takes a spare
object number, and names it in the copy's `/SMask`. `SMaskInData` goes, as before. The mask's
decoded bytes are proved equal to the decoder's opacity samples before anything is promised (ADR
0973), exactly as the colour's are.

**Code 2** multiplied the colour by the opacity. §11.6.5.2's pre-blending is computed "using actual
colour component values, with the effects of the Filter and Decode transformations already
performed", so a multiplication with nothing added is the `/Matte` whose every component is zero —
which is also how this tree undoes code 2 when it draws (`jpx_samples_to_rgba` divides the decoded
values by the opacity). The redaction writer made the same construction (ADR 1277). Refused by
sentence where Table 144's "valid colour components in that colour space" cannot hold zero: an
`Indexed` space, or one a component of whose range excludes zero.

Also refused by sentence: an image stating both `SMask` and a non-zero `SMaskInData`, which Table 87
forbids ("If this entry has a non-zero value, SMask shall not be specified"); an image mask whose
data carries opacity, which Table 87 gives no `SMask` to carry.

## 2. Three more rows are the same transcode for the same shape

The channel-count, bit-depth and CIEJab rows have JPEG 2000 data as their subject, as the baseline
row does (ADR 1400 section 1), and a stated `ColorSpace` puts the samples in its domain whatever the
data's specification says. So `transcode::SITES` holds all four; a `preserve` at any of them
transcodes the images its findings name, `PRESERVABLE_IN_PLACE` admits them with no mechanism key,
`SHAPES` splits each into `stated-colour-space` (answered) and `data-colour-space` (refused), and
`keep-everything.toml`'s three rows lose their unbuilt `codec` and `max-growth` keys. A component
deeper than sixteen bits keeps the refusal, so the bit-depth row is answered only for the shape
Table 87 can hold (a mixed-depth codestream). The fixture for the channel-count row is a two-channel
codestream under a two-colourant `DeviceN`, which converts and validates clean.

## Consequences

- The growth bound `doc/pdf-a-mitigations.md` asked for (`max-growth`) is not built for any of the
  four; the report names each image's size under both filters, which is what an operator judges by.
- What is left of the transcode is the data-colour-space shape, which needs a colour space this
  converter would choose, and a component deeper than sixteen bits.
