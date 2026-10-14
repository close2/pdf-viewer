# 1904 — The script exclusion ends at this document and this viewer, and an outline item runs its script

Session 1534, a docs round of batch eighty-five. Status: **accepted** and **built**. Context:
`doc/questions/Q286` and its answer `A286`; the rulings `A100` (a requirement executed through a
control a host supplies is executed) and `A193` (RFC 0008 accepted, `off` by default); ADRs 1602,
1616, 1750, 1752. Code: `crates/pdf-model/src/view/{script_model,script_sites,scripts}.rs`,
`crates/pdf-model/src/action.rs` (comments), `crates/pdf-script/src/{wire.rs,engine/bridge.rs}`,
`crates/pdf-script-worker/src/client.rs`, `crates/viewer-core/src/{interact,scripting}.rs`.
Tests: `crates/viewer-core/tests/script_triggers.rs`, `crates/pdf-script/tests/{hook,wire}.rs`.
Clause: §12.6.4.17, with §12.3.3 the site it was missing.

## 1. The amendment, and the one phrase changed

`CLAUDE.md`'s entry *JavaScript and script-driven form behaviour* is replaced by the sentence
`Q286` proposed and `A286` ratified ("Amend the entry with the sentence above, and move §12.6.4.17
to `implemented`"), with one phrase written against the tree as `A286` asks. The proposal said
`off` is "pinned `off` in a window that cannot ask". `quorra-confined` can ask. It is pinned because
its viewer runs in a worker that a script runner cannot be handed to (`viewer_host::SCRIPTS_PINNED`,
ADR 1616 section 6). So the entry says "pinned `off` in a window that cannot supply a runner".
Everything else is the owner's words. `tools/conformance`'s `Exclusion::Script` keeps its ledger
word, `script-behaviour`. Its doc comment now states the boundary, and a note naming the exclusion
still names it in the words `ledger_notes.rs` reads, which the new entry begins with.

## 2. The premise checked, and the site it missed

`Q286` said both of the clause's requirements were executed at `on`. The name tree's are: the open
sequence runs them after the first present (ADR 1602). The first one is not, for every site. It is
"Upon invocation of an ECMAScript action, a PDF processor shall execute a script". Every invocation
site was read: the name tree, `/OpenAction`, Table 199's four, Table 197's ten (a link's `/A` among
them), Table 198's two, Table 200's five, and §12.3.3's outline item. All but the last handed their
chains to the runner. `viewer_core::interact::activate_object` performed an item's `/A`, and a
`JavaScript` in it reached `Action::Refused` at every level. A bookmark that runs a script did
nothing at `on`, and said that Tier 0 had declined it. That was a wrong diagnosis rather than
silence, but it was a requirement unexecuted.

**Built.** `ScriptSite::Outline`, and `ViewState::run_outline_scripts`. It walks the item's `/A`
chain in §12.6.2's order and hands each script over under the same budget, before-open rule,
`/CO` walk and redraw answer as an annotation's event (ADRs 1750, 1771). `activate_object` calls it
and drops the refusal once scripts were handed over (ADR 1752). On the wire the site is tag 7.
**The event is a documented choice**, read from Adobe's reference, "Event type/name combinations".
A click on a bookmark that executes a script raises `Bookmark`'s `Mouse Up`, so that is the pair.
The reference makes the target the bookmark object, which this realm does not carry. So
`event.target` is null, and `event.targetName` is the item's Table 151 `/Title`, as at
`Doc/Open` it is the tree's key. **The cost:** a script that reads a member of `event.target`
throws a `TypeError`, and the outcome logs it. The reference's bookmark object (`bookmarkRoot`
and its members) is A193's rule's to build, ranked by the census and not refused by name today.

## 3. The rows

§12.6.4.17 moves from `out-of-scope` to **`implemented`**. Its `exclusion` field goes, it gets
`code` and `test` entries, and its note states the `off` default and A193's answer 1. That answer
keeps `off` until `script_corpus` has been green for a batch, then puts the default to the owner
again. §12.6.4's summary now counts eleven of the seventeen action types as performed.
**§12.11.5 is left as it is, and is the next round's.** Its note rests on this exclusion: "This
program runs none, so the exclusion covers the clause whole". Under the amended entry, a
requirement handler is a script of this document, so the row's `out-of-scope` is a claim the
amendment decayed. Reading it against the build is a §12.11 round's job, not this one's.

**Gates.** The new test fails with `interact.rs`'s hunk reversed (trap 13), and passes with it.
Its control passes either way. `script_triggers` passed 9 of 9, `hook` 19 of 19 with the engine,
and `wire` 5 of 5, its round trip now covering `ScriptSite::Timer` and `Outline` as well. The other
statuses are in `doc/history/1534`.
