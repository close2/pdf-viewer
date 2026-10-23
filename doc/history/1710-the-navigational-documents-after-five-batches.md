# 1710 — The navigational documents after five batches, and one orchestrator handover

Session 1710, slot 7 of batch one hundred and nine, a docs round; 2026-10-23. No ADR (2254 and 2255
unused), no `Q`. Census first: `git diff --diff-filter=A 5ded4916` against each crate-map row, the
five tools, the hosts' environment variables, and the sixty ADRs of `5ded4916..ee365e4f`.

**Premise, in part.** No pointer in the five documents was dead (`--bin pointers`: none of its
absences is in them). Two of the five tools were already indexed — `dotnet-check.sh` in the crate map
and both shell checks in `doc/verify.md` — and the survey and launch modules are batch 103's, already
named. What did not hold the other way: `doc/state-of-play.md` was untouched for five batches, so
none of their capabilities was in it; it said `--launched-documents` defaults to `refuse` where the
code has said `ask` since ADR 2185; the crate map called `hosts/windows/` "today `Quorra.Interop/`".
`Q566` is answered on the owner's disk (`A566`, 2026-10-22), so it is not open as the brief says.

**Hypothesis, in part.** About forty sentences, not fifteen to twenty-five: the state of play owed
every batch. The handover went from 239 + 139 lines to 208; its loop and owner sections stay.

**Done.** State of play: §11.7.5.3's paint-time pair and the sources kept until paint, a poisoned
press, Table 109's widths and the CFF scale, resident line runs and cropped alpha, a price per view,
GTK placement, the Qt sink, the zoom records, the WinUI window's cards, chrome and menu, the launch
default, six archive remedies. Crate map: twelve modules and two C# projects named. HANDOVER: the
five tools and the orchestrator handover. Environment: the zoom drive. Running the viewer: the zoom
records, the WinUI window, the platform checks, four paragraphs rewritten as what is. `doc/todo/74`:
two hand-kept counts and two "now"s gone, the protocol entry at ADR 2256. The teaching walks already
named the split modules (`--test teaching`); slot 8's protocol diagram is its own.

**The handovers.** `doc/orchestrator-handover-batch108.md` is folded in and removed (`git rm`); its
account is in records 1698–1703, `b17ea075` and `git show ee365e4f:doc/orchestrator-handover-batch108.md`,
its receipts under `/home/AI/` as section 5 lists; its three lessons go to the report as habits.

**Gates.** `cargo test -p conformance --no-fail-fast` after the swap: 65 targets ok, `documents`,
`commands`, `teaching`, `cited`, `spelled_ordinals`, `round_numbers` and `doc_md_absences` among them;
2 fail on siblings' work — `conformance` on §10.7.4's note naming ADRs 2252 and 2253 before their files,
`records` on this record's placeholder, since filled (`--test records` re-run: ok). `--bin quotations`
exit 0, no divergence in the five documents; `--bin pointers` exit 0, none of its absences in them;
`tools/state.sh --round 1710 docs` exit 0 (967 s, the workspace's rustdoc with warnings denied).
The navigation section's history grep is 0 in all four documents. No crate touched, so no tier 1.

**Unfinished.** `doc/todo/65` section 6 still says an uncoloured tiling tint keeps only a converted
colour (ADR 2212 fixed it); not this slot's file.
