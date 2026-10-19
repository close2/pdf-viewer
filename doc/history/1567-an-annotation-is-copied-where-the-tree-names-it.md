# 1567 — An annotation is copied where the tree names it, a soft hyphen's division is undone in a copy, and a press on what nothing reaches says why

Slot 4 of batch ninety, 2026-10-15, a clause round. ADRs 1970, 1971; no question.

**The premise did not hold, and so neither did the hypothesis (58 → 60).** ADR 1948 §4 named
`160F-2019.pdf` and `bug1868759.pdf` as pages held in page order by an `OBJR` annotation. The first
has no `OBJR`, no `/StructParent` and no `/MarkInfo`; its unaccounted bytes are an untagged reset
button's caption. The second puts all 1949 characters outside every `/MCID` sequence. Both are ADR
1948's text nothing claims, and the measured 58 stands.

**(1) Built, because §14.8.2.5.2 states it.** `Interpretation::objects` (`ObjectSpan`) carries each
claimed annotation's object and range. `Tree::logical_range` takes `ContentSpans` and places an
`OBJR`'s text where the traversal reaches it: once (Table 358 NOTE 2), less any bytes a sequence
inside the appearance covers (§14.7.5.3 NOTE 1). `annotation-text-widget.pdf` and
`annotation-choice-widget.pdf`, whose only content items are fields, now copy in the tree's order,
and the control with the annotations withheld does not. **(2)** `select::rejoined` takes the break
after U+00AD out of both copy orders and keeps the character (`Cf`, UAX #14 `BA`). `find` skips the
same break after a soft hyphen the needle states, so a pasted word finds its page (ADR 1971).
**(3)** `ViewState::report_unreachable_press` says the open's Table 31 or §12.7.3 / Table 224
sentence for the annotation under a press, once a document, at every level. It shares
`script_sites::Unreachable` with `report_unreachable`.

**Rows read anew, statuses unchanged** (`implemented`): §14.7.5.3, §14.8.2.5, §14.8.2.5.2 and
§14.8.2.3 gained their tests, and two stale reasons were corrected (§14.8.2.5.2's "rather than a
range of the page's readback", §14.8.2.3's "a copy gets what §9.10.2 produced").

**Files.** `pdf-model/src/{structure,content,content/report,content/annotations,content/text,
view/script_sites}.rs`, `pdf-model/tests/{logical_order,marked_content_scope,replacement}.rs`,
`pdf-model/examples/replacement_census.rs`; `viewer-core/src/{select,interact,open,viewer}.rs`,
`viewer-core/tests/{tagged_copy,script_triggers}.rs`; four ledger rows; `doc/state-of-play.md`; ADRs.

**Gates.** rustfmt: 0 diffs over 16 files. clippy `-D warnings --all-targets`: pdf-model and
viewer-core exit 0. nextest over both: 2501 passed, 23 skipped. `cargo test -p conformance`: exit 0.
t3-text_extract (`--clock`), t3-selection and t3-accessibility: exit 0 each. `tools/state.sh docs`:
exit 0. Against HEAD's arms, the export (HEAD plus this round's hunks) moved 0 of 968
cpu-1x page digests and 0 of 965 cpu-4x; its own tests exit 0. Each new test fails with its change
undone (trap 13). Drive (export's binaries, own Xvfb, `a` then `c`): `quorra` copies the fixture's
82 characters in logical order, field value between the paragraphs, `Transparen\u{ad}cy` joined, and
the widget page's 639; `quorra-gtk` and `quorra-qt` copy 69, because a field is a native control
there, with the same join. 7444 s.
