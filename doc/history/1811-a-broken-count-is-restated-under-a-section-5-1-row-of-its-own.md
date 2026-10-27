# 1811 — A broken `/Count` is restated under a section 5.1 row of its own, and two verbs check the tree

Session 1811, Slot 6 of batch one hundred and twenty-one, an archive round; 2026-10-27. ADRs 2456
and 2457; Q677 unused. I worked in `scratchpad/r1811/`. No ledger status moves; §7.7.3.2's row
(implemented) names the new code and two tests.
**Read first.** §7.7.3.2's Table 30 (ISO 32000-1's Table 29 for part 2) makes `/Kids` a required
array and `/Count` a required integer, the number of descendant leaves; the NOTE puts the count on
the writer. Section 5.1 was `Unchecked`, so ADR 2444 refused a broken count by ADR 0947's rule.
**Built (ADR 2456).** Two implemented 5.1 rows in `pdf-archive`, carried by both parts' 5.1
reading: `file-structure/page-tree-node-count-is-an-integer` and `…-kids-is-an-array`. The count
row is `Mechanical` (`Rewrite::PageCountRestated`): `archive::page_tree::restated` walks the tree as
`pdf_model::Pages`' leaf count does and writes the leaves beneath each failing node; a node written
inside its parent, one reached by two routes with two counts, or one past the bounds stays refused.
The kids row is refused by name (`the-fence`); the pre-stage check now holds `/Kids` alone.
**Built (ADR 2457).** `optimize` and `redact` ask `refuse_a_source_a_copy_would_carry_broken`:
the recovery refusal and ADR 2444's two-entry check. `optimize` on `poppler-91414-0-53` now
refuses (exit 4) instead of writing a file veraPDF cannot parse.
**Hypothesis held** for `T02-02_008` and both `91414` documents; `PDFBOX-6040-nodeloop`'s count
is answered and it stays refused by `file-structure/file-header` (header `%PM-5F-1.4`); the
missing-`/Kids` pair stays refused.
**Census** (converted/conforming/refused/unreadable), HEAD → this, `doc/pdf.js` then `doc/corpora`:
`as-if-printed` 2b 789/2/162/11 → 791, 350/6/115/16 → 351/6/116/16; 4 792 → 794, 352/0/120 →
353/0/121; `as-shown` 2b 764 → 766, 303/6/163 → 304/6/164; 4 768 → 770, 303/0/169 → 304/0/170.
Every line that moved is one of the four documents; no other list changed.
**Proof.** veraPDF finds all twelve freed outputs (three documents, two profiles, 2b and 4)
compliant. As the reader draws them, `T02-02_008`'s outputs equal the source; `91414`'s differ by
14 edge pixels (≤ 9 levels), and a plant with `/Count 1` written in place differs by the same 14,
so they are the stroked Helvetica's substitution, not the restatement. Unit tests plant each
shape, a cycle and a two-route node; the root's restated count equals `Pages::len`.
**Gates.** In an export of HEAD and my hunks (a sibling's pdf-model edit broke the worktree build):
`cargo fmt --check -p pdf-archive -p pdf-transform` exit 0; `rustfmt --check` on my files exit 0;
clippy `-D warnings --all-targets` on both exit 0; nextest on both 1025 passed, 0 failed; rustdoc
`-D warnings --document-private-items` on both exit 0; census `--tree 6` exit 0 before and after;
`tools/state.sh --round 1811 archive` exit 0 (validator missed 0, over 0 at six targets; every
conforming input still conforming; cross-check 13 560 compared, 0 disagreements); `cargo tree`
reaches neither crate from pdf-model's corpus binary, so against HEAD's six arms nothing can move.
`cargo test -p conformance --no-fail-fast` exit 101, 67 suites ok, two reds neither mine (`names` at
a pdf-model doc comment, `records` at round 1813's record). 7152 s.
