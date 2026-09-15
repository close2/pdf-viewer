# 1102 — The four documents against the tree, after fourteen batches

The direction slot: the four navigational documents, `tools/state.sh` and `doc/third-party-data.md`,
verified against the tree. Nothing under `crates/`; no ADR, because nothing was decided.

| document | refs checked | dead | stale | counts owed to a command |
|---|---|---|---|---|
| `HANDOVER.md` | links, ADRs, todos, habits | 0 | 1 | 1 → `wc -l doc/traps/*.md` |
| `state-of-play.md` | links, ADRs, todos | 0 | 2 | 0 |
| `crate-map.md` | links, ADRs, module files | 0 | 4 | 27 session ordinals |
| `PLAN.md` | links, ADRs, todos | 0 | 4 | 0 |

**The population defect is the crate map's own rule turned on itself.** Its header says the
population is `cargo metadata --no-deps`'s; five members had no row — `raster/crates/*`, in this
workspace, not a submodule, with a `CLAUDE.md` and ADRs of its own. Five rows added, sets now equal.

**Stale, each rewritten as what is**: three of §12.8.1's questions answered, not two, `verdict.rs`
the only place *valid* is reachable (1039, 1076); three Brainpool curves refused became one (1063);
"evaluates no transform method" became all three (1043, 1049, 1096, 1104); "the only C++ in the
tree" and "no C in this tree at all", against ten tracked C/C++ files; "`doc/md/` is already
committed", which `PLAN.md` §5a contradicts forty lines above; `render-cpu` as "`tiny-skia`" (1082).

**Absent, one sentence each with its ADR**: the archive converter, a library verb nothing calls yet
(0947); XFDF (1108); artifact by absence (1100); optional content's audience (1106); revocation
(1067); timestamps (1071); the exact scan converter (1082); substitution at one level (1102); the
group blend at the `Do` (1107); `tools/batch.sh` and `tools/worktree.sh`, named nowhere;
`doc/todo/02` §8 as the round contract, its record budget and its scratch rule. `HANDOVER.md` gained
the licensed-texts rule — every `doc/md/` text but ISO 32000-2 cited and never quoted, ETSI's three
strictest (0187, 1085) — which `doc/third-party-data.md` now has a section for.

**Trap citations** (`traps? N`, records 1000–1097): 13 → 58, 1 → 20, 8 → 17, 37 → 9, 5 and 25 → 4, 7
and 11 → 3, 10 and 39 → 2, and 4, 6, 18, 23, 26, 31, 36, 40 → 1. **Twenty-six of the forty-four rows
were cited by nobody in a hundred rounds**: 2, 3, 9, 10a, 10b, 12, 12a–c, 14–17, 19–22, 24, 27–30,
32–35, 38. Nothing removed (owner).

**`tools/state.sh`**: every `--list` section dispatched, none unknown; `records` is already in
`all`, `ratchets` `composed` on purpose with its reason in the script. Two decays fixed —
`section_annex_o`'s comment quoted a sentence `CLAUDE.md` no longer has, and it printed a heading
over nothing. **Every gate failing here is the tree's**: eight sections on sibling compile errors,
the quotation gate on three sibling blockquotes, `batch.sh check` on two records and `cargo fmt`.
