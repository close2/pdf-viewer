# 1125 — The other two required entries had no witness (`batch-1123-1128`, §8.9.5.1)

## The contract, and what round 1042 had left
§8.9.5.1's row was `partial`; the note's closing sentence read that `image_dimensions.rs` holds
"the four sentences" — the four refusal variants of `positive_integer` — and reads `issue4575.pdf`'s
dictionary out of the file. That file is the *test* `tests/image_dimensions.rs`, and it exists and
passes; round 1042 did the `/Width` and `/Height` half. Table 87 makes four entries required, each
on its own condition. This session read the other two — `/BitsPerComponent` and `/ColorSpace` —
against the code and against the corpus.

## Census (trap 8)
`examples/required_entry_census` (new, keepable) walks every image dictionary in a corpus, conditioned
on `/ImageMask` and the last codec exactly as Table 87 conditions each requirement. Over 963 opened
pdf.js documents, **2997 image dictionaries** (199 masks, 28 `JPXDecode`): 1 malformed `/Width`, 1
`/Height` (both `issue4575.pdf`), 2 masks that state a forbidden `/ColorSpace` (ignored), and **0**
missing/out-of-range `/BitsPerComponent`, **0** missing `/ColorSpace` where required. Every refusal but
the dimensions' has no corpus witness — the fixtures must be synthetic, and are.

## Four sentences resolved
- **`/Width`, `/Height`** — always required; refused by name in `positive_integer`; `image_dimensions.rs`.
- **`/BitsPerComponent`** — required except mask/JPX. A *value* outside `1 2 4 8 16` is refused in
  `unpack` (`bit_depths.rs`, now `3` as well as `12`). Its *absence* on an unfiltered image was a
  silent `.unwrap_or(8)`; now a documented repair — `RunLengthDecode` delivers 8-bit by Table 87 and
  the census finds no document omits it.
- **`/ColorSpace`** — required except JPX, forbidden on masks. Absent-where-required refused by name
  (`colour space absent is not supported`); correct in `colour_space` for sessions, **untested until
  now** — pinned in `bit_depths.rs`.

## Four fixtures (trap 13)
No `/Height` (`image_dimensions.rs`); `/BitsPerComponent 3` (new); mask with no `/ColorSpace` = valid
(`image_masks.rs` stencils); `JPXDecode` with no `/BitsPerComponent` = valid (`jpx_channels.rs` CMYK).

## Row: stays `partial`
Every required entry is read and every violation refused-by-name or repaired-by-choice. `/AF` keeps it
partial: `attachment::associated` reads it, nothing hands it an image's dictionary, so an image's
associated files are reachable by nobody (§14.13.7's). No pixel changed — behaviour is unchanged; only
a silence became a comment and two refusals gained a test.

## Gates
(recorded in the report)
