# 1828 — Every PDF/A blocking reason counted, with a remedy or an exception for each

Slot 6 of batch one hundred and twenty-three, 2026-10-27, a review round of the archive kind (Opus),
A681 item 1, run by two agents: the first handed over at 226k context after the document was
written, and the second took the handover and wrote ADR 2490 and Q694; ADR 2491 unused. I worked in
`scratchpad/r1828/`. No ledger status moves (ISO 19005 has no ledger rows).
**Census** (HEAD, real producers' files, both profiles, 2b and 4, without and with a copy of the CJK
pack): pdf.js `as-if-printed` 2b 793/2/160/11 → 804/2/149/11 with the pack, `doc/corpora` unchanged,
`as-shown` unchanged — round 1821's figures. 289 documents refused or declined at 2b and 292 at
part 4 under `as-if-printed`, 262 at both parts. **Hypothesis** (under twenty reasons carry nearly
every refused document; the exceptions a few file types) held in shape: 105 sentences over 19
remedy families, and 24 documents outside them — dynamic XFA 4, no page drawn 5 and the
converter's own defects 15 (17 at 4); the other exceptions are the 27 unreadable (20 want a
password) and an embedded office file at 2b and plain 4 (answered at 4f). **Premise** held, with one correction: 23
documents the verb declines as a source sit outside the census's refused count, and 18 of them this
reader draws (render verb, one looked at).
**Written**: `doc/pdfa-blocking-issues.md` (every reason, its documents, a remedy with its loss and
profile or a named exception, the ranking for batch 124) and its instrument
`doc/pdfa-blocking-issues/rank.py`; the attachment census (22 documents, 64 files) by a Sonnet
helper. ADR 2490 (proposed) reopens every block behind the remedy profiles with its loss and its
need, decides R02 `image-smoothing` (both profiles `discard`, §8.9.5.3's hint) and the rows that
change no earlier answer, names four new losses, and sets batch 124's order (R02, R08, R03, R06b,
R05, R03a, R11, R04, …; R09 first under `as-shown`). Q694 asks the owner the three that change one:
A570's photograph gate (R03, R08), ADR 2114 section 4's press (R05), A48's line (R11, R03a) — each
with a recommendation; meanwhile the tree refuses them as today. **Built**: nothing in a crate.
**Gates.** Census `--tree 6` without the pack exit 0 (282 s), with the pack exit 0 (126 s);
`rank.py` on both logs exit 0, 0 reasons unmapped; attachment census exit 0 (15 s);
`tools/state.sh --round 1828 archive` exit 0 (3 701 s, most of it waiting for the walk lock);
`tools/state.sh --round 1828 remedies` exit 0 (1 290 s; 0 answers at a site the listing does not
name, every profile and target); `cargo test -p conformance --test documents --test evidence --test
records --test adr_status --test questions` exit 0 (5 targets, after ADR 2490's `Status:` was made
to name Q694); `cargo run -p conformance --bin pointers` exit 0; `cargo test -p conformance
--no-fail-fast` exit 101, 68 targets green and 1 red, `fuzz_workspace` on the `formcalc` fuzz target's
missing `doc/verify.md` line and `fuzz/seeds.sh` arm — slot 2's crate, not mine.
