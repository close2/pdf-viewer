# 893 — A page turned by an integer, and a raster that cannot be asked

2026-09-03. Argued in [ADR 0830](../adr/0830-a-page-edit-is-an-integer-a-permutation-and-a-second-page-object.md)
and [ADR 0831](../adr/0831-a-rotated-page-is-not-compared-pixel-for-pixel-and-the-structure-tree-is-not-carried.md).
The eighth implementation round of [RFC 0002](../rfc/0002-the-transform-suite.md), on the
long-lived branch `round-867`. `main` had moved — sessions 887 and 889 — so it was merged in
first, in a commit of its own.

`pages` is RFC §6.2's other half and the third verb on session 886's serializer. Almost none of
it is machinery: `merge`'s engine already writes a document out of a list of pages, so the round
is three decisions and one honest refusal to assert.

Touched: **`crates/pdf-transform/src/pages.rs`** (new), `src/merge.rs` (the engine
parameterised — `Placement`, `Duplicates`, `merge::write`, the annotation duplication, the
`/Rotate` override), `src/lib.rs` (`Plan::Pages`, three refusals, `Origin::Edited`),
`src/bin/pdf-transform.rs` (the verb, the four flags read off argv, `--help`), `Cargo.toml`
(`raster-compare` as a dev-dependency); **`crates/pdf-transform/tests/pages.rs`** and
**`tests/pages_corpus.rs`** (new), `tests/verbs.rs` (the command line), `tests/split_corpus.rs`
(a merge reconciliation); `doc/conformance/ledger.toml` (six rows),
`doc/todo/02-every-round.md` (the new gate line), `doc/rfc/0002-…` (§6.2's open question
answered), `doc/state-of-play.md`, `doc/todo/57-…`; ADRs 0830 and 0831, this file.

## 1. The merge, and one improvement it nearly cost

Four files conflicted. Three were both sides appending — the ledger's six rows unioned by test
list and by note, `viewer-confined`'s operation discriminants keeping `main`'s factoring and
gaining this branch's `Assemble` arm, `state-of-play` keeping both paragraphs. §7.6.4.2 was the
one where `main` *replaced* rather than appended (/R 5 is read since session 887, not refused), so
its note is `main`'s with this branch's bit-11 paragraph after it.

The fourth is worth recording because taking a side would have lost something silently:
`open_inputs` exists only on this branch (a merge reads several files) and `main`'s round 881 had
changed the single-input body to open through `FileBytes::on_disk`. Taking this branch's function
whole would have dropped that. Every input opens on disk now, so a merge of large files costs each
one's trailer, table and selected pages rather than its bytes.

`tests/split_corpus.rs` needed one reconciliation the merge could not see: `Document::bytes`
answers `&FileBytes` since round 881, so the walk uses the bytes it had already read.

## 2. The boundary between `pages` and `merge` is the count of files

RFC §6.2 asked whether the two are one verb. They are two, and the line is **one input against
several**, which §4.1 already draws. So `--insert` takes a range of this document — a duplicate —
and a path in the argument is a usage refusal naming `merge`, because cross-file renumbering is
the thing `merge` exists to do.

The engine is shared rather than copied: `merge::write` takes `Placement`s and both verbs build a
list of them. That is why `pages` carries §8.11's groups, §12.7's form, §7.9.6's trees, §12.3.3's
outline, §12.4.2's labels, §14.11.5's intents and §12.8.1's unsigned fields without one line of
new code — every reconciliation session 888 derived applies to a page *leaving* as it does to one
arriving.

## 3. What a relative rotation composes with

Table 31 defines `/Rotate` and §7.7.3.4 defines what a page's is when the page states none. A
signed angle composes with the value the *second* clause gives — the effective one — not with what
the page's own dictionary says, because otherwise one flag does different things to two pages a
reader shows identically. Reduced modulo a whole turn; a zero written as no entry at all, which is
what "Default value: 0" makes it; a caller's angle that is not a multiple of 90 refused by name
rather than rounded.

**§12.5.3 was read for this and decides nothing about the file.** The clause names the very edit —
"when the page is rotated (by changing the Rotate entry in the page object; see 7.7.3, "Page
tree")" — and puts the whole consequence on the viewer: the annotation's stored `/Rect` is in
default user space and does not move, and a `/NoRotate` annotation pivots at display time about
its rectangle's upper-left corner. So `pages --rotate` writes nothing to any annotation. That is a
clause read to find out whether work was owed, and the answer was that none was.

## 4. A page in two places is two page objects

Table 31's `/Parent` is one reference, so `--insert` gives the second and later placements their
own page objects; the content stream and the resources are shared, because nothing under a page
points back at it — except its annotations, and Table 172's `/P` is one page. So a duplicated page
gets its own annotation objects, numbered before any is built so that a `/Popup` or an `/IRT`
names the copy beside it.

**A page carrying a §12.7 widget is refused by name.** §12.7.4.2 makes the fully qualified field
name a field's identity: a duplicated widget is either a second field under that name — invented —
or a second representation needing an entry in the field's own `/Kids`, written into an object the
plan never named. Both are a form edited rather than a page duplicated. The corpus's
`annotation-text-widget.pdf` is the fixture; no committed document states a widget at all, which
was measured rather than assumed.

## 5. The gate that could not be written, and the three that could

RFC §9 asks for "the rotation-transformed comparison for rotate": draw the source page, turn the
raster a quarter turn, require equality. It fails on 905 of 905 rotated corpus pages, and both
reasons are the renderer's.

The grid turns with the page, so a glyph edge covering a pixel 6 % on one grid covers it 8 % on
the other — `issue15150.pdf`, a 7 × 7 raster whose one non-white pixel reads (255, 239, 239) and
then (255, 234, 234), in exactly the place the rotation puts it. And the page's size in pixels is
fractional, so the sliver between `W × s` and its ceiling sits on the right after the *page* turns
and at the top after the *raster* does: worth one whole pixel, measured on `issue2761.pdf` as a
mean absolute difference of **0.000** with one column allowed for against 19.4 without, and on
`issue4398.pdf` and `bug1146106.pdf` as 0.019 and 0.008 against 0.132 and 0.938. `CLAUDE.md` names
that case as one the standard leaves open, so a gate asserting on it would be asserting on the
renderer's documented choice.

So the walk measures that comparison and prints it, and asserts three exact things instead: the
**round trip** (`+90` then `−90` draws bit-identically to the source, on the same grid, no raster
rotated anywhere — 0 failures), the **dimension swap** (§7.7.3.3's own claim about a quarter turn
— 0 failures), and **bit identity for every page the plan did not rotate**, which is RFC §9's
"pages without rotate" class in as many words. The aligned comparison is now `doc/todo/57` work
and wants `render` to report the sub-pixel offset it placed the page at, so the whole-pixel shift
can be derived rather than searched for.

## 6. §14.7 said plainly

No verb of this suite carries the structure tree, and this round is where that had to be written
rather than implied. `/StructTreeRoot` is not carried and is warned about; a carried page keeps
the `/StructParents` integer its producer wrote, and with no parent tree in the output that
integer **names nothing at all** rather than the wrong element. The distinction is the decision:
half a tree would point one page's marked content at another page's structure element and tell an
assistive reader something false. The walk asserts the output states no `/StructTreeRoot` at all,
which is the check that nothing has begun half-carrying it by accident.

## 7. What was looked at

The two rasters of `issue15150.pdf` printed pixel by pixel, which is how the rotation direction
was confirmed before any comparison was written; a five-by-five offset search over five documents'
rotated rasters, which is what turned "the antialiasing differs" into "one column, exactly"; a 180°
rotation tried as a way to avoid the swap and found to have the same sliver; Table 31, Table 167,
Table 172 and Table 161 read whole, and §7.7.3.4, §12.3.2.2, §12.4.2, §12.5.3 and §14.7.1 read
against the code rather than cited.

## 8. Gates

`pdf-transform`, the ledger and five documents changed, and a merge from `main` came in, so the
whole `doc/todo/02` §2 sequence was run in this worktree — the walking lines under
`tools/bounded.sh`, one at a time, after waiting for a neighbouring round's safedocs batch to
finish. The results are in the round's report and not here.

Two things the sequence caught that were not the round's: the debug-profile sandbox worker was
stale (trap 10 in its other spelling — the *gates* profile had been rebuilt and the debug one had
not), and the conformance checker attributes a blockquote to the nearest preceding clause number,
so two Table 31 quotations under a heading that named two clauses were read against the wrong one.

## 9. What the next transform round does first

`doc/todo/57`'s order is now `optimize` (§7.5.7's producer half, and the reachability pruning
`split` deliberately does not do), **the structure tree** — still the largest single thing the
suite owes, and ADR 0831 §2 is why it is not being paid down a fragment at a time — `split
--at-bookmarks`, the aligned rotated comparison, and the foreign readback, which now has four
writers' output in it. RFC 0003's file-system faces are what the writing verbs were for, and
`pages` is the one their write side needs.
