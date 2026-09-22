# 1217 — The photograph on the way into a page turn

Date: 2026-09-22. Branch: `batch-1213-1218`, worktree `/home/AI/pdf-viewer-rounds`, shared with five
siblings. ADRs [1271](../adr/1271-a-photograph-on-the-way-into-a-page-turn.md) and
[1272](../adr/1272-a-page-turn-that-does-not-wait-for-the-photograph.md); question [Q121](../questions/Q121-may-a-page-turn-present-a-page-whose-photograph-has-not-arrived.md).

## What was asked, and what was taken

`doc/todo/36`'s last item that is this tree's: what a five-megapixel photograph costs a page turn
(ADR 1260 section 5). Two of its parts were this crate's own passes over bytes and both are gone:
the decoder is asked for the four-channel raster it was about to be widened into, and the `DNL`
walk reads the codestream a word at a time. Three callgrind arms, a wall-clock A/B in one sitting
and `tools/state.sh frame` are in ADR 1271; `raster_golden` held 974, moved 0.

## Three things worth keeping

**The economy the contract named is one ISO/IEC 10918-1 does not permit.** Conditioning the `DNL`
scan on a frame header of `Y = 0` looks free and is not: section B.2.5 says the segment defines *or
redefines* the number of lines and is mandatory *if* `Y` is zero, and mandatory-if is not only-if.
The conditional walk was built and `dct_components`'s third case failed with the image refused; the
cost came out of the walk instead, a word at a time rather than a byte.

**A library that takes an option after it has already decided something answers half of it,
faster.** `zune-jpeg` picks its colour conversion while reading the headers; asking the same
decoder for `RGBA` afterwards reshaped the buffer and left the three-lane conversion filling it —
−15.3%, with the pixels wrong. The profile caught it, the function asked for not being the one that
ran; one-component fixtures mean the crate's own JPEG tests could not have. A habit, in the report.

**The deferral was priced and not taken.** A page turn could present the page with the photograph's
rectangle empty, but the hole cannot resemble the picture: a baseline JPEG admits no
reduced-resolution decode, so most of the cost is paid before anything can be shown. ADR 1272 has
the design and the structural objection — a display list that depends on whether a decode finished
is no longer a pure function of the bytes — and Q121 puts the owner's sentence back to them, a page
turn having, unlike ADR 0386's gesture, nothing correct on the screen to keep.

## Files touched

`crates/pdf-model/src/image.rs` (`next_ff_byte`, `defined_number_of_lines`'s inner walk,
`decode_jpeg`, the new `widened`), `doc/todo/36-a-frame-every-refresh.md`, `doc/performance.md`,
the two ADRs, Q121, this file. `doc/todo/41` needed nothing: a second `Do` is already `RasterCache`'s.
