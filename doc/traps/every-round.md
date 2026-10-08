# The ten traps every round reads

Status: **standing** — the every-round half of the trap index (ADR 1639).

**This is the short list, and it is chosen by a count.** `tools/state.sh traps` counts which traps
the rounds' records cite; the ten below carry about four-fifths of every citation ever made, and
the rest share the last fifth, most of them cited a few times or never. So a round reads these
ten rows before it starts, and opens [`README.md`](README.md) — the full index, one row per trap,
which is what any citation by number resolves through — only when its contract names a trap, or
when the position column of a group file it was told to open bites.

**A row here is a lookup key and not the trap.** The incident, the evidence and the argument are in
the group file the last column names; nothing is decided from a row alone (ADR 1036). The rows are
the full index's rows, verbatim: a change to one is made in `README.md`, and
`cargo test -p conformance --test traps` keeps that file whole.

**Two of the ten are not optional for the round they are about.** If this round can change a pixel,
**trap 1** is the one that has paid every session since the tenth; if it adds a report, **trap 11**
stops it firing on a condition the clause does not state.

| # | you are in a position to spring it when | and the rule is | group |
|---|---|---|---|
| 13 | you sweep, grep or census for a class of defect and it comes back clean, **or lift a bound to see who reaches it** (trap 29 is inside this one) | plant the defect back and confirm the sweep names it, and give a lifting a control that must stop; an uncalibrated instrument's clean answer is a sentence about the instrument, not about the tree | instruments |
| 8 | you conclude something from what the corpus does or does not contain, **or from a hand-built fragment** (trap 4 is inside this one) | a corpus finds what documents contain, not what the standard says, and the fragment is not the tree either; measure unreachability by *breaking the rule* and watching a gate move, never with the instrument under test | parsers |
| 9 | two references agree and you are about to call that evidence | they can agree because they share code, or because they share a *gap*; read the list of ways it fails rather than the count of them | oracle |
| 1 | a change of yours can put a mark on a page — **any** such change, however small the diff looked | the metrics lie; render the page and look at it, because no count can see a font that loaded and drew garbage, a page upside down, or a gradient that came out opaque | pixels |
| 11 | you add a report, or a census, or decide when one fires | a report is only as good as the condition it fires on; derive the condition from the clause, print what it matched, and cost it in gated pages | instruments |
| 5 | you implement part of a feature, or handle an input you cannot fully support | unsupported input must stay loud; a silent fallback that renders something plausible is the failure mode that reports nothing, and it hides best *inside* a partly-implemented feature | parsers |
| 10 | you run a `--profile gates --test` line, or any test that decodes JBIG2, CCITT or JPX | the sandbox worker is a separate binary and Cargo will not rebuild it for you, so a walk builds it as its wrapper's own `--build '--profile gates -p pdf-sandbox --bins'` inside its hold (trap 109); a missing worker and a stale one look nothing alike — and each profile keeps its own, so a `dev` nextest after a `gates` build fails 38 decoders until `cargo build -p pdf-sandbox --bins` has run for `dev` too, which is no walk and takes no lock (round 1414) | instruments |
| 15 | you run a sweep binary by its path rather than through cargo, in a worktree | the binary carries the tree it was **built from**; take the path from `cargo metadata`, and the tell is that nothing moves when you re-run after an edit | instruments |
| 25 | an instrument's population is a hand-written list of names | it can name a thing that never existed, and finding nothing there prints a tick; derive the population from a manifest, cargo, or the tree | instruments |
| 2 | you compose a transform into a paint, a gradient, an image or a stroke width | a paint is positioned in the *path's* space and both backends apply the drawing transform to it already, so composing it yourself applies it twice | pixels |

The shared-machine rules — one heavy walk at a time, `ulimit -u 8192` and `tools/bounded.sh` on
every heavy command, no `git stash`, scratch under `scratchpad/r<round>/` — are not traps but
agreements, and [`doc/environment.md`](../environment.md)'s opening block states them in one line
each; traps 109 and 116 are their incidents.
