# 1871 — A photographed page of a document stating optional content is drawn as this viewer shows it

Slot 1 of batch 128, 2026-10-29, an "archive" round (A717, Opus 5.5), two sessions; ADR 2576 written, ADR 2577 and
Q747 unused. I worked in `scratchpad/r1871/` and two exports of this tree built there. No ledger row moved.

**The premise that did not hold.** The three corpora documents the brief named (`050734`, `064034`, `504076`) pass the
new gate and are then held by the next one, a page whose content this reader does not draw whole
(`fonts/photographed.rs`'s `NOT_DRAWN_WHOLE`): they move line. `state-every-loss` answered the font site `stop`.

**What changed.** The loss `optional-content-of-photographed-pages` (`decision.rs`, in the font site's bundle): a page
that would be photographed in a document stating optional content (ISO 32000-2 §8.11) is drawn in the configuration
the window opens it in (§8.11.4.3's `/D`, `View` usage), at 2b and part 4; Level A and Level U keep the refusal, whose
sentence names the loss. Each photographed page names the groups its content names with the state drawn (64 groups,
200 characters a name, the rest counted) in the report, its JSON and `xmpMM:History`. `as-if-printed` carries it
through its font `discard`; `state-every-loss` now answers the font site with `try = ["append-statement", "discard"]`,
the first step passed over by name (ADR 2576 decision 6). Clippy's two findings: the gate's bools grouped as `Beyond`,
the sRGB profile write as `srgb_profile`.

**The plants.** The red square moved into the group `/D` shows: the drawn-as-shown test fails (43 056 red pixels),
reverted. The font row taken out of `state-every-loss`: the new profiles test fails at 2b, restored (`cmp`). One freed
output with `pdfaid:conformance` planted as `Z`: veraPDF fails clause 6.6.4.

**Gates.** In the export (`--profile gates`): `cargo test -p pdf-transform --test archive --test profiles` exit 0 (498
and 13 passed), and in the worktree exit 0 (501 and 13 passed, siblings' tests included); clippy `--no-deps -D
warnings -p pdf-transform -p pdf-model --all-targets` exit 0 after the two fixes above; fmt exit 0. Census against
HEAD's, both corpora: pdf.js `as-if-printed` 2b 872 → 874 converted (86 → 84 refused) without the pack and 873 → 875
with it, part 4 883 → 885 and 884 → 886, either part 71 → 69 and 70 → 68 refused at both; the corpora's "states
optional content" line 3 → 0 and "does not draw whole" 16 → 19 at both targets in both runs; every other line
unchanged; bound exit 0 (242 s, 201 s). `rank.py` exit 0 twice. veraPDF on the two freed documents (`issue19484_1`,
`issue19484_2`) at 2b and 4: four PASS; the fixture's outputs PASS at 2b and 4 and a planted one FAILs at 2b.
`getrusage` on a hostile page naming 20 000 groups: HEAD 67 MiB, refused; this change 102 MiB, converted, 64 named and
19 936 counted. `--bin cited` exit 0, `--bin quotations` exit 0; `cargo test -p conformance --no-fail-fast` exit 101,
469 passed and 1 failed: `every_record_since_the_rule_states_its_gates` on `doc/history/1876-…`, round 1876's, in
progress.

**Found, not fixed.** For batch 129: `fonts/photographed.rs` `admitted` (`NOT_DRAWN_WHOLE`) refuses a page whose
interpretation reports `Content{Damaged}` or a parser pseudo-operator ("BT without ET", an operator inside an array)
that ADR 0816's fence does not list, the three documents' next refusal (`504076` also reaches a TrueType `.notdef`,
R03a). `statement.rs` has no section for photographed pages, so `append-statement` at `fonts/font-programs-embedded`
is passed over (`config/steps.rs`).
