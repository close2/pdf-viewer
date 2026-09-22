# 1242 — The samples arrive at the depth the file states, and the graphics state reaches the cube

Status: accepted. Session 1202.
Context: ISO 32000-2 §8.9.6.4, §8.9.5.1, §8.9.5.2, §7.4.9, Table 87, §11.7.5.3, §10.4.2.4,
§11.6.6, §11.7.2; ADRs 0023, 0796, 0832, 1121, 1193, 1207 (amended here), 1232 (which found the
first half).
Code: `crates/pdf-sandbox/src/protocol.rs`, `crates/pdf-sandbox/src/decode.rs`,
`crates/pdf-model/src/image.rs`, `crates/pdf-model/src/content/transparency.rs`,
`crates/pdf-model/src/content.rs`.
Tests: `crates/pdf-sandbox/src/protocol.rs::a_raster_of_more_than_eight_bits_round_trips_at_its_own_precision`,
`::a_worker_cannot_send_eight_bit_samples_at_a_wider_precision`,
`::a_precision_outside_one_to_sixteen_is_refused`,
`crates/pdf-model/tests/image_masks.rs::a_colour_key_over_a_twelve_bit_jpeg_2000_image_is_compared_in_twelve_bits`,
`::a_sixteen_bit_jpeg_2000_colour_key_reaches_the_top_of_its_domain`,
`crates/pdf-model/tests/transparency_groups.rs::a_group_painted_into_a_cmyk_parent_is_converted_by_the_functions_at_the_do`.

Two decisions a later round must not re-litigate, and they are one round's because they are the
same shape: a value the clause requires to travel was stopped by this tree's own plumbing, and
the refusal each carried named a place where the value did not exist rather than the place it
was dropped.

## 1. `pdf_sandbox::Raster` carries a precision

**Decided: a JPEG 2000 raster leaves the confined worker at the codestream's own bit depth,
with that depth on the wire, and the eight-bit raster is reached above it.**

ISO 32000-2 Table 87 makes the depth this processor's: `/BitsPerComponent` "shall be ignored if
present" and "[t]he bit depth is determined by the PDF processor in the process of decoding the
JPEG 2000 image." What this processor determines is the file's, because §8.9.6.4's colour-key
ranges are integers in *that* domain — "[e]ach integer shall be in the range 0 to 2
`BitsPerComponent` - 1, representing colour values before decoding with the Decode array" — and
a narrowing in the decoder cannot be undone above it.

The wire gains one byte (`precision`, refused outside 1..=16) and the payload is sized from it:
one byte a sample at eight bits or fewer, two big-endian above, which is how a PDF image stream
writes a wider sample itself (§8.9.5.1). `Raster::sample`, `Raster::max_sample` and
`Raster::bytes_per_sample` are the readers, so no caller re-derives the stride.

**Above it nothing else changed shape.** `jpx_decode` builds §8.9.5.2's `/Decode` tables at the
raster's precision — the map is linear in the sample, so `D min` still lands at sample 0 and
`D max` at the largest — and the opacity channel, which has no `/Decode` entry, takes the one
linear map its domain admits (`scaled_to_byte`).

**Where a single domain does not exist, the refusal stays and says which of three reasons it
is**: signed samples, components that disagree on a depth (§8.9.5.2's "can have different values
per colour component"), or more than sixteen bits — the last named as this tree's limit rather
than as a clause's, because it is one.

**Two readings of one codestream, and the guard between them.** `jpx_sample_domain` reads the
`SIZ` precision to bound the ranges at parse time; the confined decoder reads the same field to
deliver the samples. They are one number in every codestream either can read, so a range above
what the raster can hold means the two parted — and `colour_key_in_the_rasters_domain` then drops
the key and says so beside the painted image, rather than comparing in a domain the samples are
not in. This is the shape trap 5 asks for: a silent fallback here would be a mask that quietly
matched the wrong pixels.

**`MAX_SAMPLES` is unchanged and the doubling is priced rather than ignored.** A codestream at
the budget's 2^26 samples now leaves the worker as 134 MB instead of 67 MB, beside the `f32`
coefficient buffers the decode already holds, which are four bytes a sample and dominate it. The
confinement's address-space limit is what a worker that exceeds its room meets, and it meets it
as a refusal with a sentence rather than as a crash of the parent — so the budget stays where the
decode's own arithmetic put it. Nothing in the standard states a number this bound must be at
least as large as (trap 38), and a producer's depth is not a size the bound is about.

**The fixture was regenerated rather than reinterpreted.** `JPX_TWELVE_BIT` used to decode to
4095 — saturated white — because its generating command wrote the raw samples little-endian and
`opj_compress` reads them big-endian, so the intended 3000 became 47115 and clipped. The
fixture now carries a true 3000 in twelve bits, which is what lets the test state three ranges
that separate the twelve-bit reading from the eight-bit one in three directions, the middle one
being the stretched value 187 that used to mask the whole image and must now mask nothing. **A
sixteen-bit fixture pins the top of the widest domain beside it**, every sample 65535, where a
range of exactly that masks and one stopping at 65534 does not: `2^n - 1` at sixteen bits is the
one value a shift computed in `u16` answers short of, and no twelve-bit fixture can reach it.

## 2. §11.7.5.3's second bullet is carried out

**Decided: the black-generation and undercolour-removal functions in force at the `Do` reach the
conversion of a group's result into a four-component parent, and `Unsupported::BlackGeneration`
is no longer about that case.**

> When painting a transparency group whose colour space is DeviceRGB into a parent group whose
> colour space is DeviceCMYK , the functions used shall be the ones in effect at the time the Do
> operator is applied to the group.

ADR 1207 recorded this bullet as owed, with the reason that "the conversion is a cube resolved
per pixel in a backend where no colour space exists". **The first half is true and the second
does not follow**, which is trap 40 exactly: the cube is *resolved* per pixel in a backend and
it is *sampled* in `content/transparency.rs::into_parent_cube`, forty lines from the graphics
state the bullet names. `parent_channels` was already calling `Compositing::paint` — with `None`
where the pair goes.

So the pair travels: `conversion_into_parent` takes it off `outer`, which is the state at the
`Do` (§11.6.6 has reset the group's own parameters on `inner`), beside the rendering intent ADR
1054 already reads there. It reaches §10.4.2.4's arithmetic through `Compositing::Subtractive`'s
arm of `paint` and no other, which is the clause's own "shall be applied only during conversion
from DeviceRGB to DeviceCMYK colour spaces" holding by construction.

**No backend changed.** What a backend receives is `pdf_render::ColourCube` either way; only its
contents differ. A round reaching for the per-pixel machinery `doc/todo/23` prices should know
that this row did not need it.

**The memoisation key gains the pair by pointer identity** (`BlackGeneration::identity`, the same
key `crate::colour::Conversion` uses), so two `/ExtGState` dictionaries stating one pair of
functions sample the cube twice. That costs a sampling and cannot cost a colour.

**One departure is left under §11.7.5.3 and it is the one ADR 1207 named second**: a press
sampled from the document's own bi-directional profile, whose `B2A` table is the conversion in
(§8.6.5.5, ADR 0796) and is the document's measured transform rather than the device default a
stated pair substitutes for. The report now fires on that alone, and
`a_stated_black_generation_is_reported_and_the_page_keeps_its_space` still holds it firing.

## 3. Consequences

- §8.9.6.4 moves `partial` → `implemented`; §11.7.5.3 moves `partial` → `departed`, its note's
  first sentence naming the press-profile case.
- `raster_golden` held 974 and moved 0. Neither population has a witness in `doc/pdf.js`: the
  colour-key census (§8.9.6.4's row) found not one `JPXDecode` witness in 90 535 documents, and
  both halves are pinned by generated fixtures alone.
- `pdf_sandbox::Raster::data` is no longer "always eight bits", and every comment that said so
  is gone rather than annotated.
