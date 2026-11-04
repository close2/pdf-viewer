# 1930 — A control character is left off the packet page, and stated

Session 1930, a single archive round in the rounds worktree. ADR 2682 (2683 unused); no question. No
trap or habit.

**Why.** The owner's A778 chose option (b) for Q778. Under `state-every-loss`, a C0 control
character in a packet laid out on the appended page is left off that page and stated.
`no-data-loss` keeps the refusal. Option (a), the control pictures, waits for a shipped face.

**The premise, measured.** `balloon_a1b_jp2k` was run under all eight shipped profiles at 2b and 4.
Six of them refused it with `NO_FACE` at both targets: `as-if-printed`, `as-shown`,
`only-metadata-loss`, `keep-everything`, `no-data-loss` and `state-every-loss`. The other two,
`refuse-any-loss` and `derive-attachments`, refused because nobody authorised the packet's
replacement. The census holds eight `NO_FACE` documents. A scan of their decoded packets found a C0
character in one only, this one: U+0000 at byte 17 of object 2.

**Built.** ADR 2682. The step ADR 2678 admits through `keeps_per_object` now acts one character
at a time. Where it is admitted, a UTF-8 packet is laid out without its C0 controls (LF, CR and tab
excepted). Each code point is listed on the statement page with the loss
`metadata-packet-character`, its object, `U+XXXX`, a count and byte offsets, never the character
itself. The report (`replaced_metadata_packets[].omitted`, an `omitted:` line) and `xmpMM:History`
say the same. U+007F and the C1 controls keep `NO_FACE`. No face under `data/` covers
U+2400–U+243F (`fc-query`), so option (a) has no face yet.

**Tests.** `tests/archive/packet_characters.rs`, three. First, U+0000 and U+000B are left off and
stated at 2b and 4, while a CRLF still ends a line. Second, `no-data-loss` refuses with the face
sentence. Third, U+007F keeps that refusal. With the gate inverted, the first two failed.

**Census.** ADR 2669's command was compared with round 1929's run. There are 2 new outputs,
`balloon_a1b_jp2k` under `state-every-loss` at 2b and at 4, and none are gone. All 4498 shared
outputs match once masked. Both new outputs pass veraPDF 1.30.3. Ledger: §14.3.2 stays
`implemented`; its note names ADR 2682 and its tests name the new test.

**Gates.** `cargo fmt --check` (`pdf-transform`): exit 0 after one `cargo fmt` of the test file.
`clippy -D warnings --all-targets` (`pdf-transform`): exit 0. `nextest` of `pdf-transform` and
`pdf-archive`: 1361 passed, exit 0. Archive tier (`corpus`, `archive_corpus`, `witnesses`): exit 0
each. `cargo test -p conformance`: exit 0. `state.sh docs`: exit 0. veraPDF 1.30.3 over the 2 new
outputs: both pass.
