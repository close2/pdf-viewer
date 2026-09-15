# 1112 — a redaction removes the bytes, and refuses what it cannot clear

2026-09-16. ADR 1124. Contract: build redaction application (§12.5.6.23), once, clean, to green.

**The build.** `crates/pdf-transform/src/redact.rs` and the `redact` verb: applies every `/Redact`
annotation a document states and writes a **new file** through RFC 0002 section 10's serializer —
never §7.5.6's update, which appends and leaves the producer's bytes. Every non-redacted page and
its content stream cross byte for byte (a copied slot); a redacted page is *replaced* — its
`/Contents` a new stream with the region spliced out, its `/Redact` annotations dropped — and the
old content, referenced by nothing, is copied by nothing. That is what deletes the bytes.

**No font metrics (ADR 1124 §2).** Page interpreted without `/Annots` (`page_to_draw(page, None,
false)`, or `text_layer` carries §12.5.3's pass). Each placed code's §9.4.4 quad is tested against
the region mapped through `base_transform` (made public in `content.rs`); a removed glyph's advance
becomes a §9.4.3 `TJ` adjustment read from the quad itself. A second walk's code count must equal
the interpreter's, else the page is refused (trap 13). **"Within" = bounding-box intersection**, the
choice the clause leaves open (centre-in leaves half-glyphs, containment leaks).

**Departures and refusals, each priced (trap 5, principle 1).** Overlay (`/OverlayText`, `/IC`,
`/RO`) not composed — a reported per-page `Report::departures` (new field + `Departure`, parallel to
`Origin::Optimized`'s savings; `Origin::Redacted` carries the counts). Refused by name, page left as
written: Type 3, composite non-`Identity-H`, `sh`, soft mask, image/path/form meeting the region,
code-count mismatch, encrypted document. Host surface: `redact` batch verb built; whether to apply is
Table 22 bit 4 asked once via `Plan::operation` (ADR 1076). No viewer UI (owed).

**Census (trap 8).** Parsed, not byte-grep: a `/Redact` in a §7.5.7 object stream is invisible to
`grep` and resolved here — `a_redaction_hidden_in_an_object_stream_is_found_and_applied` plants one
(built via `optimize --object-streams`) and confirms. Isartor `6.5.2 -fail-h` witness opens, its
region resolves, the verb runs on it.

**Row moved:** §12.5.6.23 stays `partial` (overlay departs; image-region destruction owed), note rewritten (application built; overlay a
departure; image/complex removal a priced refusal), cites `redact.rs` + five new tests.

Gates (siblings share the tree; clippy/fmt scoped to mine). Tier 1: fmt --all --check 0; clippy
--workspace --all-targets -D warnings 0 (only the pre-existing proc-macro-error2 dep note); nextest
--workspace 4985 passed; --doc 0; fuzz fmt+clippy 0; conformance 259 passed (ledger + quotations,
unreviewed 0). Tier 2/3 under the lock, one at a time: pdf-transform `gate` ok (88.7 pp/s); pdf-retrieve
`retrieval` 10 passed; pdf-model `save_round_trip` ok (every ratchet at ceiling). tests/redact.rs 7
passed. batch.sh check exit 0.
