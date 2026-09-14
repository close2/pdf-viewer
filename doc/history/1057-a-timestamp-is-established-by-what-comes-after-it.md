# 1057 — A timestamp is established by what comes after it

Contract: §12.8.5 and §12.8.5.2, §12.8.4.4, and the timestamp half of §12.8.3.3.1.

1. **`pdf_signature::timestamp` reads RFC 3161 section 2.4.2's `TSTInfo` whole** — policy, imprint,
   serial, `genTime` with the fraction that clause permits and RFC 5280 forbids, accuracy under the
   grammar's `(1..999)`, ordering, nonce, `tsa`, extensions — refusing indefinite lengths by name
   on "the eContent SHALL be the DER-encoded value of TSTInfo".
2. **Four steps before a `genTime` is an instant**, each failure named. The second was *missing*:
   the signer's `message-digest` attribute binds a token's contents to the signature over it
   (RFC 5652 section 5.6), and without it a `TSTInfo` can be swapped under a signature that still
   verifies. A planted defect proves it — one octet of `genTime` moved, signature intact.
3. **`timestamp::chain`** orders a document's timestamps by what their ranges name and says what
   each covers: earlier signatures, and §12.8.4's material by object offset. Coverage is decided at
   §12.8.1's `%%EOF`, and a range ending elsewhere is refused rather than guessed at. Each link's
   own path is validated at the *next* link's instant, which is §12.8.5.3's whole argument.
4. **`viewer-core::notes`** says the chain, each token's claim in the token's own characters, what
   each covers, every refusal by name, and the paragraph saying none of it is a time.

## What it found

**A conforming timestamp authority's certificate could not validate here at all.** RFC 3161
section 2.3 requires `id-kp-timeStamping` and requires the extension *critical*; `trust` refused
every critical extension it did not recognise. `trust::Purpose` is the fix and it is an input, the
shape RFC 5280 section 4.2.1.12 asks for; `Purpose::Unstated` moves no answer this tree gave.

**§12.8.3.3.1's other timestamp is checkable with no certificate**: RFC 3161 Appendix A puts the
imprint over the `SignerInfo`'s `signature` field, a digest this program has the input to. 462 of
the crawl's signature values carry the attribute and 462 of 462 commit to the signature they sit
on; two are in `doc/pdf.js`, both with fractional genTimes — the branch RFC 5280 forbids.

Census over 90 763 documents: **139 `/DocTimeStamp` dictionaries in 98 documents**, 29 with two or
more, 88 with a DSS beside them. Witnesses at 1, 2, 3 and 5 stacked tokens are a test. A sibling's
`\uXXXX` escapes and a note with raw newlines had made `ledger.toml` unparseable for the whole
worktree; both repaired mechanically, text unchanged.

Rows: §12.8.5, §12.8.5.2, §12.8.3.3.1, §12.8.3.4.8, §12.8.4.2 and §12.8.4.4 keep `partial` with
notes that now name the anchor and nothing else; §12.8.5.3 keeps `implemented` and gains the chain.
ADR 1071.
