# 637 — The parameter a point owns, and the object that borrowed it

The ledger's one `silent` row, closed the way the status asks for: the departure it names is
reported now, on a condition derived at the point rather than at the mark. Reading for it found a
second silent gap one clause up, in a feature §10.5's row called implemented.

Date: 2026-08-21.
ADR: [0469](../adr/0469-the-parameter-a-point-owns-and-the-object-that-borrowed-it.md).

Touched: `crates/pdf-model/src/content.rs`, `content/transparency.rs`, `content/path.rs`,
`content/text.rs`, `content/image.rs`, `content/pattern.rs`, `content/report.rs`,
`crates/pdf-model/src/form.rs`, `crates/viewer-core/src/report.rs`,
`crates/pdf-model/tests/transfer_functions.rs` (new),
`doc/conformance/ledger.toml` (§10.5, §11.7.5, §11.7.5.2, §12.7.4, §12.7.4.1),
`doc/todo/01-ledger-partial-rows.md`, `doc/todo/13-the-transfer-function.md`,
`doc/todo/README.md`, the ADR and this file.

## What §11.7.5.2 requires, read whole

632 put the row at `silent` on the clause's closing sentence and priced what it owed. This round
read §11.7.5 whole — §11.7.5.1's two categories, §11.7.5.2's six conditions and its overprinting
paragraph, §11.7.5.3 beside it, §10.5 for what a transfer function is, §11.6.4.x and §11.6.6 for
how opacity reaches a point — and the condition falls out of two sentences and the one between
them:

> The halftone and transfer function to be used at any given point on the page shall be those in
> effect at the time of painting the last (topmost) elementary graphics object enclosing that
> point, but only if the object is fully opaque.

> Together, these conditions ensure that only the object itself shall contribute to the colour at
> the given point, completely obscuring the backdrop.

> For portions of the page whose topmost object is not fully opaque or that are never painted at
> all, the default halftone and transfer function for the page shall be used

The middle sentence is what makes the derivation exact rather than approximate. Where the topmost
object is fully opaque the composited colour *is* that object's colour, so applying the object's
function before compositing and applying it after are the same arithmetic. Where it is not, the
clause puts the page's default on a composite that already has each contributor's function inside
it.

**A point is drawn wrong exactly when some object covering it carried a transfer function and the
topmost object covering it is not fully opaque.** One statement, covering both the translucent
object that carries the function and the translucent object painted over an opaque one that
carried it — and the second is invisible to any rule that only looks at the mark being made, which
is how a two-condition report would have under-reported.

## What was built, and what it matched

`Interpreter::note_transfer`, called from the seven places an elementary object marks the page,
raising `Unsupported::TransferFunction`. The page-level condition is the geometric
over-approximation of the point-level one — a mark the clause does not call fully opaque, made
while some mark on the page has carried a function — because nothing in the interpreter knows which
objects overlap. It cannot under-report; it over-reports only a page whose translucent marks all
miss its transferred ones.

**The ancestry was the trap 632 named and it is carried rather than read.** §11.6.6 resets the
blend mode, both alpha constants and the soft mask before a group's content runs, and §11.6.7
starts a tiling cell from the initial state — so a flag reading the mark's own alpha would have
reported nothing at all for the case the clause spends four of its six conditions on.
`Interpreter::opaque_ancestry` is narrowed in `group_commands` from the state at the `Do`, and in
`tile` from the state that painted the pattern, and `transfer_painted` is scoped away inside a soft
mask's group beside the four flags already scoped there for §11.5.3's reason.

**What it matched, in gated pages: nothing.** The census was re-derived rather than taken —
`examples/transfer_function_census`, 964 documents opened, **13 stating a Table 57 `/TR` or `/TR2`
and exactly 1 stating anything but `/Identity` or `/Default`** — and that one, `issue6931_reduced.pdf`,
interprets with `unsupported []`. The corpus gate's incomplete count is unchanged, which is the
number that decides whether trap 11 was sprung.

The fixtures are the witness instead (trap 8): `tests/transfer_functions.rs`, three tests, each
with its mutation — a `ca 0.5` fill under a stated `/TR`, a group whose `Do` is translucent and
whose inner mark is opaque by its own state, and a `sh` under a function. Each mutation is the same
page made opaque or given `/Identity`, and reports nothing.

## §10.5, found while enumerating

Every place a transfer is applied is a short list — two calls to `GraphicsState::transferred` and
one to `transferred_image` — and the list makes visible what is not on it. `fill_paint` returns
`Paint::Shading` on the line above the one that maps a colour, and `sh` never asks. So an axial or
radial ramp's stops, a mesh's corners and a sampled shading's program reach the backend unmapped,
against a clause whose subject is the component value without qualification.

Reported, not implemented: reaching every colour a shading carries is `Shading::with_alpha`'s walk
done again with a closure, in `pdf-render`, and the population is zero. §10.5's row is `partial`
and says which half. `doc/todo/13` prices it beside the per-region model.

`spec-errata emit` over all fourteen documents before writing: **nothing touches §10.5 or any of
§11.7.5.** The nearest are §10.4.2.4, §10.6.5.4, §10.6.5.6, §10.7.2, §11.4.8 and §11.6.6.

## The second row: §12.7.4, and evidence that reached nothing

The blame band was re-derived on this base — 824 commits — and it is **twelve rows at five
commits** where 632 found sixteen at seven, with the same forty-two-commit gap above it. Rank 1 is
§12.7.4, whose stated reason is a claim about this codebase: "[a] family's parent row is not
maintained by the sessions that implement its members."

The row is right about that and was corrected in the three-hundred-and-seventy-first session for
it. **What nobody corrected was the arrays the corrected note rests on.** Its `test` array held one
entry — `variable_text.rs::quadding_moves_the_line_within_its_box`, a §12.7.4.3 test — and its
`code` array held `appearance.rs` alone, while every sentence in the note is about `view.rs` and
`form.rs`.

§12.7.4.1 cites the same test and opens with "Table 226's inheritance is implemented". That
fixture's widget is one merged dictionary: the `/Parent` chain `Field::read` walks has no links in
it, so §12.7.4.1's own rule ran zero times in the only test either row offered.
`form.rs::a_fields_type_flags_and_value_come_from_the_ancestor_that_states_them` states `/FT`,
`/Ff` and `/V` two links up, reads all three back through `form::fields`, and fails when
`MAX_FIELD_ANCESTRY` is cut to 1 — through the report the clause's forbidden bound owes, since a
half-walked ancestry is refused rather than answered.

620's newest shape, paying for the fourth round running. The rule this round leaves in
`doc/todo/01`: **when a note is corrected, the `code` and `test` arrays are corrected in the same
edit or they are not corrected at all** — and that is greppable, which makes it a thirteenth sweep.

## One mistake worth recording

The first five edits of this round went into `/home/cl/projects/pdf-viewer/` rather than into
`.claude/worktrees/r637/`, because the absolute path was typed from the main worktree. They were
moved across as a patch and reverse-applied in `main` before anything was built. **A parallel round
that edits by absolute path has two trees to be wrong about**, and the tell was `git diff --stat` in
the worktree showing only the edits made through a shell that had `cd`-ed there.

## Gates

`pdf-model` is the change→gate map's first row and `tools/round.sh` called this a fifth round, so
the whole sequence ran; every line exit 0, nothing else running beside it.

`fmt` clean. `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` exit 0 — it caught
the one thing the new `Unsupported` variant broke, `viewer_core::report::describe`'s exhaustive
match, which is the crate boundary working. `cargo nextest run --workspace` 2337 passed / 17
skipped. Doctests clean.

Corpus **974 documents, 68 incomplete** — unchanged, which is the number that says trap 11 was not
sprung. Oracle **1794 pages: 907 agree, 66 contradicted, 786 ambiguous, 2 our geometry, 2 reference
geometry, 13 not comparable, 18 no render** — identical to 632's, verdict for verdict. Text
extraction 10969/11163 matched words in bounds (98.26%) over 508 documents. `selection_census`
1000/1011 words over 453 documents, 0 readback differences; `accessibility_census` 988 documents,
104 with structure; `dates` 1545 strings, 1514 conforming; `xmp` 319 documents, 318 read;
`jpeg2000` green. `render-quorra` 957 pages: 932 agree, 23 differ, 2 refused, 17 not comparable.
`fixed_documents` 29 checked, 0 absent. `cargo test -p conformance` green — **875 rows, 0
unreviewed, 951 verbatim quotations**, and the status breakdown is 436 implemented, 222 partial,
**18 reported**, 78 inapplicable, 8 writer-side, 113 out-of-scope. **No `silent` row.**

Sweeps run because the ledger moved: `quotations` — 1674 ledger quotations, 1 diverging, and that
one is §8.9.5's and was there before; `counts`, `tables` and `pointers` printed their standing
false positives and no new hit. §5's binaries rebuilt and installed.
