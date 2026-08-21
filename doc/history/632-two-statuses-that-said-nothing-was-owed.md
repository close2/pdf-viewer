# 632 — Two statuses that said nothing was owed

The band 626 named at rank 10, read with the rule 620 derived. Sixteen rows rather than the twelve
626 could see, four of them read, and both defects were in the half of the ledger's vocabulary that
no sweep looks at.

Date: 2026-08-21.
ADR: [0465](../adr/0465-two-statuses-that-said-nothing-was-owed.md).

Touched: `crates/pdf-model/tests/accessibility.rs` (one test),
`doc/conformance/ledger.toml` (§8.7.3, §8.7.3.1, §11.7.5, §11.7.5.2, §14.8.4, §14.8.4.2, §14.9,
§14.9.4), `doc/todo/01-ledger-partial-rows.md`, `doc/todo/13-the-transfer-function.md`, the ADR
and this file.

## How the band was ordered, and whether blame agreed with 626

Re-derived rather than taken, which is 616's lesson and 620's and 626's: `git blame
--line-porcelain doc/conformance/ledger.toml`, each `partial` or `reported` row's own `note = `
line, ranked by where its commit falls in `git log --reverse`.

**It agreed with 626 on the seven rows 626 named, and then went on for nine more.** 626 wrote that
"rank 10 is five hundred commits later" and listed §12.7.4, §12.7.6.2, §8.7.3, §11.7.5, §12.11,
§14.8.4, §14.9 "then §12.8.2.2 and its four neighbours". On this base those are ranks 1 to 12 and
the band does not stop there: §12.8.3.4.4, .6, .7 and .8 sit at the same commit as one another and
only forty-two commits below rank 17. **Sixteen rows at seven commits**, and nine of the sixteen
were written by two signature rounds, which is why a band that had been eight or nine rows for
three rounds is suddenly long. The boundary moved exactly as 626 said it would, and quoting ranks
rather than indices is what made the comparison possible at all: this base has 816 commits and
626's numbers name different rows in it.

**620's rule chose the work for the third time.** Of the sixteen, four state a reason that is a
claim about this codebase rather than about the standard, and all four paid.

## §14.8.4.2, and a row that argued itself `implemented`

`partial` since the ledger was written, on this sentence:

> a nesting rule is a statement about whether a *document* is well formed, and nothing here
> validates documents — the same position §7.11.2.1's path rules and §7.12.4's version ordering are
> recorded in.

**Both of the rows it names are `implemented`**, each for exactly that reason — §7.12.4's says "the
rule that it be no greater than the file's own version is a *writer's*, and this reader does not
validate files" — and Annex L, which is where the nesting rules actually are, is `writer-side`,
the status whose definition is that nothing is owed. Four rows, one argument, one contradicting
status, held for two hundred and fifty sessions including a "read and kept" in the
five-hundred-and-first.

The clause states one `shall` and the tree executes it: a non-standard type nests as whatever the
role map takes it to, which is `Tree::standard_role`, and the cited test does reach it — 620's
third shape checked and passing this time. Everything else in §14.8.4.2 is a pointer and a NOTE.

§14.8.4 follows mechanically. It states no prose at all (§14.8.4.1 begins under the heading), its
one non-settled child was §14.8.4.2, and its own note gave the same Annex L sentence as the reason
for its `partial`. Both `implemented`. The sixth sweep was re-derived in the same session, which is
620's chain rule: one hit left, §O, which is the hit it was before.

## §11.7.5.2, and an `inapplicable` that described a different requirement

Reached from §11.7.5 at rank 4, whose note said "§11.7.5.2's row says why that is still
`inapplicable`". What that row says is:

> what the clause asks for is *per-region* tracking, decided by the topmost fully opaque object at
> each point, and that needs a second transfer function competing with a first inside a
> transparency group.

The first half is the clause. The second half is the row's, and it is what the status rested on.
§11.7.5.2 needs no second function, because its rule is about *opacity*: the transfer function at a
point is the topmost object's "but only if the object is fully opaque", and "[f]or portions of the
page whose topmost object is not fully opaque or that are never painted at all, the default
halftone and transfer function for the page shall be used". One stated `/TR`, one object at `ca
0.5`, and the clause bites. This tree applies the object's own transfer to its own colour before
compositing; the clause applies the topmost object's to the composited colour. The two agree
exactly on the fully opaque case and nowhere else.

`silent`, not `partial`: nothing is reported, and a `partial` row that reports nothing is the
understatement the status exists to prevent. §10.5's row carried `silent` for one round in the
three-hundred-and-fifty-seventh for the same reason, and this is the gap *inside* the feature that
round built.

**No code, because the population is measured at zero.**
`examples/transfer_function_census` over the 974 finds thirteen documents stating a Table 57 `/TR`
or `/TR2` and exactly one stating anything but `/Identity` or `/Default`; `mutool draw -F trace`
shows that one — `issue6931_reduced.pdf` — drawing its image at `alpha="1"`, Normal, no soft mask,
no `/SMask`. Nothing on this disk is drawn wrong by this today, and a report built now would fire
on nothing while under-reporting the case the clause spends four of its six conditions on: a
group's `Do` and a tiling cell's caller each decide opacity for what is inside them, so the flag
needs the ancestry and not just the mark. `doc/todo/13` prices the report and the per-region model,
in that order, and its own "what it does not need" bullet — which carried the retired sentence —
now says so.

## §14.9.4, and an erratum met by construction and executed by nothing

`spec-errata emit` over the family before writing, which is what found it. Errata Collection 3
Issue #483 (`Review`/`Accepted`) adds a third bullet to §14.9.4's list of places a replacement text
may live: the second bullet's sentence again, with an `Artifact` tag where the `Span` tag was. The
row said "[b]oth of the clause's locations are read"; the parent §14.9 said "each in both of the
places §14.9 puts it", which was not even true of `/Alt`, whose third location the same note
records four sentences later.

The interpreter already meets the erratum, because `Interpreter::accessibility` asks *every*
`BDC`'s property list for §14.9's four entries rather than a `/Span`'s alone. **Nothing in the tree
ran it** — both cited tests use `/Span` — which is 620's newest shape a fourth time.
`an_artifacts_replacement_text_replaces_what_it_encloses` draws a folio as `vii` under an
`/ActualText` of `7`, asserts the readback is the replacement, and asserts the artifact's range
covers the replacement rather than the glyphs it replaced, which is an ordering `run.rs` chose
deliberately. Mutation-checked: restricting the property list to a `/Span` tag fails it.

One detour worth recording: the erratum's own sentence cannot go in as a rustdoc blockquote,
because the conformance gate verifies blockquotes against `doc/md/` and `doc/md/` is the published
text. The neighbouring §14.8.2.2.2 test had already met this and solved it the same way — quote the
published sentence, state the erratum in prose.

## §8.7.3, a confirmation with its evidence written in

The row's reason for `partial` is a claim about this codebase — "that outline is the backends' to
compute … so there is no path here to tile" — so it was re-derived rather than repeated. It holds:
no crate that builds a display list depends on `kurbo` or on a rasteriser, and all three backends
expand a stroke themselves (`tiny_skia::PixmapMut::stroke_path`, `vello`, `kurbo::stroke`). The
clause names the stroke in the same breath as the fill, so this is a debt and its shape is now in
the row: tiling a stroke here means computing the outline a fourth time in the one crate whose
whole point is that it does not. The corpus count was re-derived off this round's own gate output —
`scorecard_reduced.pdf`, still exactly one.

Two errata stand over §8.7.3.1 and neither was recorded. Issue #428 inserts "(implementation
dependent)" at the end of "unspecified and unpredictable;", which is the ground `Cell`'s
one-cell-copied-to-every-site construction stands on — a clause that permits, in `CLAUDE.md`'s
words. Issue #294 inserts "stream" into Table 74's caption. Neither moves a requirement. Locating
them took the annotation's `/Rect` against `pdftotext -bbox`'s word boxes, because `spec-errata
emit` prints a caret's text and not where it goes; that is a gap in the tool worth knowing about
and not worth fixing for two errata.

## The sharpening `doc/todo/01` gained

620's rule ranks by what kind of claim a reason makes; 626's adds where in the clause the answer
would be. This round's is about *which statuses* decay:

> **Where a row's note cites another clause as precedent, the precedent has a status.** Either the
> two agree or one of them is wrong.

Both defects were in the settled half of the vocabulary — `implemented`, `inapplicable`,
`writer-side`, `out-of-scope` — which is the half no sweep reads, because a claim that nothing is
owed has no missing thing to grep for. The seventh sweep does read `inapplicable` rows, and it
could not have seen §11.7.5.2: it asks whether the tree names the row's vocabulary, and the tree
names `/TR`, `/HT`, `/BG` and `/UCR` all four. Only reading the clause found it. The precedent
check is a sweep somebody could write and `doc/todo/01` records it as one.

## Gates

`pdf-model` is the change→gate map's first row, so the whole sequence ran even though the diff is
one `#[cfg(test)]` block and eight ledger notes. Nothing else was running beside the gate lines,
which is `doc/todo/02` §2's rule after 626.

`fmt` clean. `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` exit 0. `cargo
nextest run --workspace` 2322 passed / 17 skipped. Doctests clean. Corpus 974 documents, 68
incomplete. Oracle 1794 pages: 907 agree, 66 contradicted, 786 ambiguous, 2 our geometry, 2
reference geometry, 13 not comparable, 18 no render. Text extraction 10969/11163 matched words in
bounds over 508 documents. `selection_census`, `accessibility_census`, `dates` (1545 strings, 1514
conforming), `xmp` and `jpeg2000` green; `render-quorra` 957 pages, 932 agree / 23 differ / 2
refused / 17 not comparable; `fixed_documents` 25 checked, 0 absent. `cargo test -p conformance`
green — 875 rows, 0 unreviewed, 934 verbatim quotations.

Sweeps run because the ledger moved: `quotations` — 1656 ledger quotations, 1 diverging, and that
one is §8.9.5's and was there before; `counts`, `tables` and `pointers` no new hits; the sixth
sweep re-derived after the §14.8.4 pair moved, still one hit.
