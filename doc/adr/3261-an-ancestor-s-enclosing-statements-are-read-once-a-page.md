# 3261 — An ancestor's enclosing statements are read once a page

Session 2140, a measure round in the `lane3` worktree. Status: **accepted** and built. Context: ADR 2976
(`/ActualText` covers what its element encloses, once), ADR 2944 (the first frame is counted),
`doc/checks/launch-path.toml`; `crates/pdf-model/src/structure.rs` (`enclosing`, `EnclosingMemo`),
`crates/pdf-model/src/content/marked.rs`. Clauses: ISO 32000-2 §14.9.2.3, §14.9.4.

**The regression.** Commit 2ef6887a (round 2075, ADR 2976) replaced the `/Lang` walk up `/P`, which stopped at
the nearest element stating one, with `structure::enclosing`, which reads `/Lang` and `/ActualText` in one walk.
§14.9.4 makes the *outermost* `/ActualText` win, so that walk must reach the root on every marked-content
sequence that names an element — a correct reading, and the cost of it was not the clause but the copies:
each step is a `Document::get` of the parent, which hands back the whole dictionary with its `/K` array, and
`.as_dict().cloned()` copied it again. Callgrind, release build of 373e7eb6 against the same build with this
change, first frame (`count-first-frame`): `enclosing` 1 526 532 instructions inclusive of
`PDF20_AN001-BPC.pdf`'s 12 263 964 (92 calls), 1 241 285 of WTPDF's 42 372 846. The launch gate's
`frame_kinstructions` left its bands on both rows, and no merge ran the gate (round 2135 found it).

**The decision.** An element's answer is its own two statements laid over its parent's — its `/Lang` before the
parent's answer, the parent's `/ActualText` before its own — so an ancestor's answer is the same for every
descendant that asks. The interpreter keeps them in `EnclosingMemo`, keyed by object identity, for the page; a
walk stops at the first remembered ancestor. The parent dictionary is taken out of the `Object` rather than
copied. The bound stays exact: each answer carries the length of the chain it was read over, a walk whose total
would pass `MAX_ANCESTRY` is read again without the memo, and a chain the bound cut is not remembered. An imported
page (§8.10.4) sets the memo aside with the rest of the containing document's object-keyed state.

**Measured** (same machine, release, `tools/state.sh`'s instrument reproduced by a script of the round): AN001
first frame 12 346.8 → 11 085.5 thousand, WTPDF 42 438.0 → 41 321.3; every open unchanged to ±0.03. The walk's
answers are unchanged: `a_remembered_ancestor_answers_what_the_walk_reads` asks every element of a two-level
fixture through one memo in both orders and compares each answer with the unremembered walk's.

**Cost.** One map of small answers per page, and a second function (`enclosing_unremembered`) for the public
`language` and for the over-long chain.
