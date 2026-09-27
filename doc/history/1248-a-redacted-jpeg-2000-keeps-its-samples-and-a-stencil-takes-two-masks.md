# 1248 — A redacted JPEG 2000 image keeps its own samples, and a stencil takes two masks

Session 1248, 2026-09-26/27. ADRs 1333, 1334.

**Finding.** The four codec cases §12.5.6.23 still refused came from one cause. Every codec picture
was written back as eight-bit `DeviceRGB`, and the file's integers, colour-key ranges and matte are
not in that domain. The answer was to keep the domain rather than translate into a new one. The
refusal "two masks on one command" was also not what it said: the product of two masks is itself
one mask, one drawn through the other.

- **§12.5.6.23, partial (narrowed).** A `JPXDecode` image is written as the decoder's own integers
  (`pdf_model::image::jpx_samples`): at 8 bits, or at 16 with a widened `/Decode`, in its own colour
  space. It is decoded at full resolution only (`Request::JpxWhole`), within `--image-samples`. A
  budget above the viewer's starts a separate confined worker, `Sandbox::whole`, whose ceiling,
  answer size and timeout come from that budget. A re-expressed codec picture's colour key becomes
  a §8.9.6.3 stencil. A matted picture is cleared to the matte (Table 144 at α = 0). The row stays
  partial for its other named refusals (Type 3, non-Identity-H, `sh`, soft-mask group, strokes).
- **§8.9.6.2 partial → implemented; §8.9.6 partial → implemented** (every child implemented).
  §8.9.6.1's stale sentence about the refusal was replaced by a one-line pointer.
- **Corpus witnesses.** Neither file carries a `/Redact`, so one was added in scratch.
  `issue19326.pdf` (16-bit JPX): no pixel moved outside the region.
  `issue13931.pdf` (DCT, `/Matte [0 0 0]`): about 1.9k pixels moved by one level outside the region.
  The cause is that the DCT route and the Flate route undo a matte with different rounding
  (`Prematte::restore_samples` vs `restore`). That belongs to §11.6.5.2 and is left open.
- Refusal found stale on the way: §12.5.6.23's note said an encrypted document is "refused outright";
  ADR 1162 made that conditional, and the note now says so.

Files: `crates/pdf-model/src/{image.rs,content/image.rs}`, `crates/pdf-model/tests/image_masks.rs`,
`crates/pdf-sandbox/src/{decode,lib,lockdown,lockdown_linux,protocol,worker}.rs`,
`crates/pdf-sandbox/tests/confinement.rs`, `crates/pdf-transform/src/{redact.rs,bin/quorra-transform.rs}`,
`crates/pdf-transform/tests/redact.rs`, `crates/render-raster/tests/stencil_through_a_pattern.rs`,
`doc/conformance/ledger.toml`, `doc/todo/64`, `doc/todo/65`, `doc/state-of-play.md`, ADRs 1333, 1334.
