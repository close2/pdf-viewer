# 1818 — a deep JPEG 2000 redaction is written back by a lossless writer of the tree's own

Slot 3 of batch one hundred and twenty-two, 2026-10-27, a "clause" round (Opus); ADR 2470; ADR
2471 and Q684 unused. I worked in `scratchpad/r1818/`. **§12.5.6.23 stays `departed`, narrowed.**
**Premise.** The brief's finish line was `departed` → `implemented`; it did not hold, because the row
departs on two refusals (ADR 2439's table: "holds; one half the owner's") and ADR 1363's
calculator function is the other half, untouched by A668 item 3. The hypothesis held: no admissible
crate fits, and a zero-level reversible encoder fits one slot.
**Survey (ADR 2470 §2).** Eight safe or nearly safe crates read from their own sources (executing
them was refused by this session's permission policy): four stop at sixteen bits or have no
encoder, `justjp2` is one release line in `i32`, and `j2k-native` writes to 38 bits but its current
releases need rustc 1.99 against the workspace's 1.97. The binding bound is the decoder's anyway:
`hayro-jpeg2000` computes in `f32`, exact to 24 bits.
**Built.** `crates/pdf-jpx-write`: ISO/IEC 15444-1's smallest lossless codestream (one tile, zero
levels, 5-3, no quantisation, one layer, 64 × 64 blocks of style 0), Annex D's passes, Annex C's MQ
coder, Annex B's packets and tag trees, Annex I's JP2 boxes with a `cdef` box for an opacity
channel; no dependencies, `forbid(unsafe_code)`. `pdf_sandbox::MAX_PRECISION` (24) and
`Raster::field_bytes` carry a writer's whole decode in a three-byte field; a drawing decode is
unchanged, and the archive's transcode refuses on the widest channel, opacity included. The redaction clears a 17–24-bit `JPXDecode` image in that field and writes it back
(`redact/deep.rs`) as `JPXDecode` at each component's own depth, `/Decode` at those depths, no
`/BitsPerComponent`, `/SMaskInData` and its channel kept inside the codestream. Refused by name: a
component past 24 bits (the decoder's words), a deep image of other than one or three colour
components (a JP2 file enumerates greyscale and sRGB; T.801's JPX file with its M.11.1 reader
requirements box is the step left).
**Proof.** Round trip at every depth 1–24, grey and RGB, mixed depths, both opacity kinds, through
`pdf_sandbox::decode_here`; two planted defects (a Table D.3 XOR bit, a run position's high bit)
each turned three of the four round-trip tests red. Redaction tests: a 20/18/24-bit RGB image's
left half reads back zero and draws black, its right half reads back integer for integer and draws
as before; a 21-bit grey image's 17-bit opacity channel is cleared inside the codestream; a 25-bit
component is refused naming the bound. Row note, `doc/todo/65`, state of play, crate map: kept true.
**Gates.** `rustfmt --check` on every file of mine 0; `cargo fmt --check -p pdf-jpx-write` 0;
clippy `-D warnings --all-targets` on the four crates 101 on siblings' files alone (`archive/fonts/bystanders.rs`, `media.rs`,
`tests/archive/rekeyed.rs`), 0 with their four lints allowed; `cargo nextest run` `-p pdf-jpx-write`
4/4, `-p pdf-sandbox` 64/64, `-p pdf-model` 2312 passed 21 skipped, `-p pdf-transform` 738 passed 21
skipped, each exit 0; fuzz `jpx_write` 601 s, 147 212 runs, no crash, exit 0; `--bin cited` 0,
`--bin quotations` 0; `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` on the four 0;
`cargo test -p conformance --no-fail-fast` 101, one red, a sibling's (an unattributed quotation in
`colour_transform.rs` and `bystanders.rs`). No arm was run: the drawing decode's
integers at sixteen bits or fewer come from the same arithmetic, and `cargo deny` has no new
crate to read (a path dependency only).
