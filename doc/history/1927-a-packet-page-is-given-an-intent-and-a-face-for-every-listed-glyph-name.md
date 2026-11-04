# 1927 — A packet page is given an intent, and a face for every listed glyph name

Session 1927, a single archive round in the rounds worktree. ADRs 2676 and 2677; Q778. No trap or habit.

**Why.** R14 of `doc/pdfa-blocking-issues.md`: six documents whose appended page could not be composed.

**The premise, measured** (each under `state-every-loss` at 2b and 4, one by one). Three causes held
in shape, none in remedy. `breast+cancer+immunotherapy` was `NO_OUTPUT_INTENT`, but the source states
no intent at all, so the brief's "colour in the intent's family" answers nothing: no intent was
*prepared*, because only a failed device-colour row prepares one. `PDFBOX-3148-2-fuzzed` was the
defect: its catalog packet is `/A85` data that will not decode, the keep skipped it, and an empty
composition answered `NOT_ASKED_FOR`. `issue20453` lacked one character, `ё`, which Liberation Sans
draws: the limit was `/WinAnsiEncoding`, not the face, and the CJK pack is not in the tree.
`balloon_a1b_jp2k`'s missing character is U+0000 in the packet header. `241573_ftp`'s packet is not
"not UTF-8" text but Flate data whose stream states no filter. `JPXTestGrey`'s refusal (2b only) is
the extension-schema container, not the page.

**Built.** ADR 2676: a packet row or the property row answered by page, in a source stating no
intent, prepares the intent as the statement page does; a replaced packet with no bytes owes no page
(`preserve`, `FreshMetadataPacket`, `NO_BYTES_TO_PRESERVE`); and a packet the replacement takes
answers the property row (`Cleaned::only_replaced`), as ADR 2442 holds for the schemas. ADR 2677: the
shipped face's encoding becomes a `/Differences` array over `/WinAnsiEncoding`, by listed Adobe
Glyph List names, only where WinAnsi cannot name a character. Kept refused: a computed `uniXXXX`
name, U+0000 (Q778), and `241573_ftp` (the profile's second step at the packet rows is unbuilt).

**Tests.** `tests/archive/packet_pages.rs`, five tests at 2b (two at 4 too): the intent, the
`/Differences` with text extracting as the packet's, a CJK character keeping `NO_FACE`, the property
row taking the replacement's decision, and an undecodable packet appending no page. Each of the four
remedies planted back made its test fail. The Cyrillic page was rendered and looked at.

**Census.** ADR 2669's command against round 1925's census: 16 new outputs, none gone —
`breast+cancer+immunotherapy` and `PDFBOX-3148-2-fuzzed` under both profiles at 2b and 4,
`issue20453` under `state-every-loss` at 2b and 4, and `wp592`, `bug1057544` and `bug1919513` under
both at 2b (the same missing intent). Of 4330 shared outputs 4328 match once masked; `bug1997343` and
`bug2009627` differ only by `NO_FACE`'s restated sentence on their statement pages. All 18 pass
veraPDF 1.30.3. Ledger: §9.6.5.4 stays `implemented`, its note naming ADR 2677.

**Gates.** `cargo fmt --check`, `clippy -D warnings` (`pdf-transform`): exit 0. `nextest` of `pdf-transform`
and `pdf-archive`, bare (locked, its build passed the 6 GiB bound): 1350 passed, exit 0. Archive tier
(`corpus`, `archive_corpus`, `witnesses`): exit 0 each. `cargo test -p conformance`: exit 0.
