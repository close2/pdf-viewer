# 1940 — A bare JPEG 2000 codestream is kept in a JP2 file, and the shipped profiles take it

Session 1940, a single archive round. ADRs 2694 and 2695; no question; no trap.

**The premise.** Half held. R13's three pdf.js documents are refused by
`graphics/jpeg2000-uses-the-baseline-feature-set` alone under both shipped profiles at 2b and 4,
as the census said. But R13 was not "unbuilt": ADR 1400's transcode answers the stated-space shape,
and `no-data-loss` and `state-every-loss` already wrote all three. Only `as-if-printed` and
`as-shown` refused, because they listed `graphics/jpeg2000-*` among their `stop` sites. Each image
is a bare ISO/IEC 15444-1 codestream (`FF4F FF51`, no box) under a stated `/ColorSpace`: `bug_jpx`
is 128 × 64 grey at 8 bits, and `issue5567` and `red_stamp` are three 8-bit channels under
DeviceRGB; §7.4.9's "expect to read a full JPX file structure" is all they fail. This reader decodes
each whole; mupdf draws `issue5567` identically, and aborts on `bug_jpx` (poppler crashes), whose two
tiles code the same constant.

**The construction (ADR 2695).** It is neither Flate nor the baseline writer. `pdf_jpx_write::around`
puts JP2 boxes around the producer's codestream, byte for byte. The image header comes from the
codestream's own `SIZ`. The colour box is greyscale or sRGB, which the stated space overrides.
`preserve` tries this first and falls back to the transcode for any other shape: part 2 `Rsiz`,
colour channels of different depths, a channel count other than 1 or 3, opacity, or a file that does
not read back. Nothing is decoded into the file, so no history entry is written: a lossless
restatement records nothing (ADR 2693). **ADR 2694**: `as-if-printed`, `as-shown` and
`only-metadata-loss` answer the four JPEG 2000 sites with `preserve`; ADR 1400 §4's costs are no loss.

**Kept**: a bare codestream with no stated space stays refused by name under every profile, and
`refuse-any-loss` answers nothing. **Tests**: `tests/archive/kept_codestream.rs` (4), `around` (3);
planting a disabled wrap and an `as-shown` `stop` fails all 4.

**Census**, against round 1939's runs. Shipped: 12 NEW (the three at 2b and 4 under both profiles),
5093 identical. Stating: 12 CHANGED (the same three, Flate copy to kept codestream; `issue5567`
3.68 MB to 2.17 MB), 4549 identical. Every output holds the source codestream whole in its `jp2c`
box. All 24 draw identically to their source at 72 dpi on the CPU, with and without
`/OutputIntents`. R13 frees 3 and 3 and leaves the rank; the table was regenerated.

**Gates.** fmt, and clippy `-D warnings --all-targets`, for `pdf-jpx-write` and `pdf-transform`:
exit 0 each. nextest `-p pdf-transform -p pdf-archive -p pdf-jpx-write`: 1409 passed, exit 0.
Archive tier (`corpus`, `archive_corpus`, `witnesses`): exit 0 each. Both censuses: exit 0.
veraPDF 1.30.3: all 24 new and changed outputs pass. `cargo test -p conformance` after `git add -N`:
exit 0 once table designations named their standard; the final run's one failure, `bounded`'s
self-test timing at load 290, passed alone (exit 0). `state.sh docs`: exit 0.
