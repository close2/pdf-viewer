# 1935 — A stream no reader decodes is written as the reader drew it, absent

Session 1935, a single archive round. ADR 2689; no question; ADR 2690 not used.

**The premise.** R07's eight documents hold no decoder defect: seven `UnknownFilter-*` files
state the invented `/XXXDecode`, and `poppler-742-0-fuzzed`'s image states a fuzzed `/DCTecode`,
which ADR 2679 forbids reading as `DCTDecode`. It did not hold that building R07 frees the seven:
their `%PDF-3.x` header is refused at both parts by `file-structure/file-header`, so only
`poppler-742-0-fuzzed` is freed. With the header patched to `%PDF-1.7`, the image, form, content
stream, object-stream, outline and both cross-reference files render identically before and after.
The font file converts by R08's photograph of a page whose text the reader leaves blank. The ICC
file stays refused: the reader draws its space as the `/N 4` alternate.

**The construction (ADR 2689).** `archive/undecodable.rs` finds each stream from the filter row's
findings and reads its role. An image or form becomes ADR 2115's form drawing nothing; a content
stream holds no operator; a font program is left out (`needs_a_program` agrees); an object or
cross-reference stream is not written. Each is stated by object, role, filters, length and reason
in the report (`undecodable_streams`) and `xmpMM:History`. Nothing was partly decoded, so nothing
is re-encoded. `as-if-printed` and `as-shown` discard (ADR 2490); `state-every-loss` tries
`append-statement` (no page at this site yet), then discards. `no-data-loss` refuses by name.

**Refusals kept, by name**: an ICC profile (its alternate is not written yet), an image that
another names as its mask, and any other role.

**Tests.** `archive/undecodable_streams.rs` (six tests: the profiles, image and form, content,
`no-data-loss`, mask, ICC). The old refusal test now pins `Unauthorised`. Planting the refusal back
failed four.

**Census.** Against round 1934's runs. Shipped: 2 new outputs (`poppler-742-0-fuzzed` at 2b,
both profiles), none gone, 5044 identical once masked. Stating: 1 new (the same, under
`state-every-loss`), 4528 identical; `no-data-loss` refuses it as `unauthorised undecodable-stream`.
At 4 its packet will not take the history entry, and it is refused by that sentence. The rank
(`rank.py --table`) puts R07 at 1 / 2, with `UnknownFilter-ICC` left; R06b now frees 34 / 8.

**Gates.** fmt and clippy `-D warnings --all-targets` (`pdf-transform`, `pdf-archive`): exit 0.
nextest of `pdf-transform`, `pdf-archive`, `pdf-font`: 1743 passed, exit 0. Archive tier (`corpus`,
`archive_corpus`, `witnesses`): exit 0 each; both censuses exit 0. veraPDF 1.30.3: the three new
census outputs and the nine patched outputs pass at 2b. `cargo test -p conformance` after
`git add -N`: exit 0. `state.sh docs`: exit 0.
