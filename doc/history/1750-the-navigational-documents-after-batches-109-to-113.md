# 1750 — The navigational documents after batches 109 to 113

Session 1750, slot 6 of batch 114, a docs round; 2026-10-25. No ADR (2334 and 2335 unused), no `Q`.
Census first: `git diff --diff-filter=A e883ed13..7f1f3f15` and `ee365e4f..e883ed13` against each
crate-map row and the five documents, and ADRs 2242–2320 against `doc/state-of-play.md`.

**Premise, in part.** Rounds had added sentences as they went (45 lines to the state of play), but
the archive slots had cited none of thirteen ADRs there (2246, 2262, 2263, 2278, 2279, 2284, 2293,
2306, 2307, 2318, 2319 among them), and two sentences had gone wrong: §7.4.9's "three whose
defining texts are not held" after ADR 2292 drew e-sRGB and e-sYCC, and "no dialogue a document
raises interrupts a presentation", which ADR 2309 section 3 limits to the toolkit and C windows —
`quorra`'s and `quorra-confined`'s cards are not held. The handover's section 1 still called the
launch gate red; five of its section 6 threads were closed (ADRs 2253, 2262, 2263, 2276, 2279).

**Hypothesis held**: about forty sentences across the five; the text walk's two diagrams redrawn.

**Done.** State of play: e-sRGB and e-sYCC, the function lane's level, the device lane's searched
pattern, GTK's fractional scale and Qt's first Wayland size, the KWin probe, eleven archive and
redaction remedies, the presentation's dialogues narrowed. Crate map: `extended_srgb`, the operator
loop's `Machine`, `ShowString`, `font_files`, `placeholders`, `fonts/pack`, `area/union`,
`winding/`, `LaunchInstance`, `wheel`, `native_window::placing`, `window/placement`. HANDOVER: the two
KWin probes, `tools/cjk-pack.py`, `doc/reviews/`, the `adr_status` and `evidence` gates.
Environment: the loop's uses by the owner's words of 2026-10-10 and 2026-10-23, the nested KWin.
Running the viewer: the CJK pack as a second download, the parked presentation, `--touchpad`, the
install list instead of a count. Orchestrator handover: section 1 by commands, the 2026-10-25
direction, section 6 rewritten. `doc/todo/74`: entry 14 (ADRs 2288, 2304, 2320), the functions
re-read against `tools/state.sh gods`, section 17 the census's next blocks. Teaching: the text
walk's dispatch is `run_operator` → `run_text_showing_op`, its `BDC` arm `run_marked_content_op`'s
and the replacement's take-back `end_marked_content` (ADR 2304); `run_reader` named none of them.

**Gates.** `cargo test -p conformance --no-fail-fast` (177 s): exit 101, every target ok but
`conformance` (`archive/font_files.rs`'s quotation and two ledger tests a sibling is editing) and
`round_numbers` (round 1744's `window_against_page.rs` example); `documents`, `commands`,
`teaching`, `cited`, `spelled_ordinals` and `doc_md_absences` ok. `--bin quotations` exit 0, no
divergence in the files touched; `--bin pointers` exit 0, none of its absences in them;
`tools/state.sh navigation`'s history grep 0 in all four. `tools/state.sh --round 1750 docs`: exit
101 (2595 s queued, 592 s held), rustdoc refusing `pdf-colour`'s `rgb_alpha` link to the private
`to_rgb_at` in a sibling's uncommitted edit of `colour/to_rgb.rs`; no crate is this round's, so not
re-run. `--test evidence --test adr_status`: ok; `--test records`: ok for this record, exit 101 at
the last run on sibling 1748's record, then mid-write. No crate touched, so no tier 1.
