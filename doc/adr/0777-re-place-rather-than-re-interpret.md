# ADR 0777 — Re-place rather than re-interpret, and the second reader of §12.5.6.4

Status: accepted, 2026-09-01. Session 850, a loop round on `doc/todo/46`.

ADR 0775 chose the shape — the annotation pass §12.5.3 forces is 1–6% of the interpretation it
forces, so *remove* the work rather than move it to another thread — and left the seam itself
unbuilt, with two constructions and three questions to answer first. This round answered the three
and built the seam. It also found, with the instrument built to check the seam, that this clause
had **two readers in the tree that disagreed**, and that the new one was the wrong one.

## 1. The three questions, answered

### The clone is 7 to 39 microseconds, so the list is rebuilt by copying

`doc/todo/46`'s sharpest question: a render request carries one `Arc<DisplayList>`, so a re-placed
page's list is the content prefix plus a new tail, and the prefix is copied once per notch. If that
copy is tens of microseconds the seam is a contract-preserving round; if it is not, the honest end
state is a *transform node* in `pdf_render::DisplayList` — a three-rasteriser change with an ADR of
its own.

Measured first, before anything was designed around it, with
`pdf-model/examples/replace_cost` on ISO 32000-2, best of twelve:

| page | commands | interpret | clone the display list |
|---|---|---|---|
| 10 | 2738 | 733.8 µs | **38.8 µs** |
| 187 | 1217 | 12.690 ms | **13.6 µs** |
| 407 | 658 | 2.431 ms | **7.1 µs** |
| 504 | 1836 | 3.469 ms | **22.4 µs** |
| 1001 | 2203 | 3.511 ms | **24.3 µs** |

0.1% to 5.3% of the interpretation it replaces. **So no transform node is owed**, and none was
built: the three rasterisers are untouched by this round and the display list stays
magnification-independent in the only sense that matters, which is that nothing about a *command*
changed.

### The seam is a checkpoint of the interpreter, not a merge of two interpretations

`doc/todo/46` named two constructions. The second — run the annotations into their own
`Interpretation` and merge fourteen public fields — is smaller to write and was rejected on a
reading rather than on a measurement: the per-font `glyph_coverage` report is computed over **the
whole page**, so a font whose glyphs the content half drew and whose codes the annotation half does
not would be reported as drawing nothing by an interpretation that saw only the tail. Merging also
means remapping every `ClipId` and `SoftMaskId` of the tail into the prefix's tables and shifting
every span into the prefix's readback — three separate places for a silent error to live.

The first construction — restore an `Interpreter` from an owned snapshot — was rejected in
`doc/todo/46` on the grounds that it is "about forty fields" and "a field forgotten is a report
silently lost". That is true of a snapshot written by hand. It is not true of one **the compiler
checks**, and the whole of the argument for taking it is that the check is available:

```rust
fn checkpoint(&self) -> Checkpoint {
    let Self { document: _, across: _, /* … every one of fifty-nine … */ } = self;
    Checkpoint { list: list.clone(), /* … */ }
}
```

A destructuring `let` **without `..`** is exhaustive, so a field added to `Interpreter` stops
`checkpoint` compiling until somebody has decided which of three kinds it is: accumulated (carried),
memoised (dropped and recomputed), or derived from the page (rebuilt by `for_page`). `restore`
destructures `Checkpoint` the same way and every binding must be used, which under this tree's
`-D warnings` is the same guard from the other side. **The comment above the struct says which
fields it holds; the compiler says it, and the comment says so.**

The memos are dropped on purpose and that is a claim rather than a convenience: `fonts`, `shadings`,
`resource_tables`, `icc_spaces`, `image_masks`, `image_rasters`, `stream_structures` and
`clip_extents` are memos of pure functions, so a resumed run that starts with empty ones computes
the same answers and only pays for them again — over an annotation's appearance stream, which is
small, and with `FontCache` already answering across interpretations (ADR 0710).

### §11.4.7's subtractive pair keeps nothing, and its cost is written down

A page whose blending space is `DeviceCMYK` is interpreted **twice** and the two lists merged by
geometry digest (ADR 0262), so the seam would be two seams and the merge a third thing to get
right. Such a page keeps no replacement and a zoom of it re-interprets whole — at the price it
already pays twice over. `examples/press_census` is what says how rare that is. This is
`doc/todo/46`'s own suggested answer, taken deliberately and recorded as a cost.

## 2. What it bought

`viewer-core/examples/zoom_cost` on `doc/ISO_32000-2_sponsored_EC3.pdf`, viewport 1100×1200, fit to
the page and scrolled across a boundary so the column holds two pages. **Both arms in one sitting**,
the "before" built from the same tree with the change reversed by patch (`git stash` is forbidden
here) and each crate's `lib.rs` touched on both arms (trap 10b):

| | before | after |
|---|---|---|
| worst notch over all 341 pages, 16 steps | **5.040 ms** (page 407) | **1.035 ms** (page 962) |
| page 407 | 4.993 ms | **402 µs** |
| page 1001 | 3.809 ms | **529 µs** |
| page 504 | 3.626 ms | **278 µs** |
| page 10 | 566 µs | **96 µs** |
| page 962 | 1.692 ms | **1.010 ms** |

The resize arm tracks the zoom arm within 1% throughout, which is ADR 0766's claim measured again.

**Page 962 is the point of the table, as page 1001 was ADR 0775's.** It improved by 1.7× where the
others improved by 6 to 13, and it is now the worst notch — because what is left is the clause's own
pass on a page that carries a lot of it. That is the residue this construction cannot remove, and it
is a millisecond rather than five.

## 3. The second reader, and what found it

`content::Interpreter::any_no_zoom` decides, *before* the annotation pass runs, whether this page is
worth keeping a checkpoint for — because taking one on a page the clause is not about would be a
second copy of a list nothing will ask to move. It was written to read Table 167's bit 4 out of
`/F`, which is what `ViewGeometry::adjustment` reports as `ViewAdjust::view_dependent`.

It is not what decides. Two subclauses override the file's flags **by subtype**, and neither is
visible in `/F`: §12.5.6.4 makes a `Text` annotation behave as though `NoZoom` and `NoRotate` were
always set whatever the file says, and §12.5.6.10's four markup subtypes have both cleared (ADR
0172's choice). So the new condition named a different population from the one `decide` acts on.

`pdf-model/examples/replacement_census` printed the disagreement on its first run: **five pages of
the pdf.js corpus** — `annotation-text-without-popup.pdf`, `issue13447.pdf`, `issue21126.pdf`,
`pr12564.pdf` and `rc_annotation.pdf` — where the interpretation came back view-dependent and the
seam had kept nothing. Every one of them is a `Text` annotation with no flag set at all. The effect
would have been correct and silent: those pages would have re-interpreted whole on every notch, for
ever, with no gate able to say so.

`annotation::view_flags` is the one reading now — both subtype overrides applied in one function,
asked by `decide` and by `no_zoom_in_force` — and the census reports zero disagreements over 974
documents. This is trap 11's sixth instance exactly: a second reading of one question is a second
answer.

**And the count this project had been quoting for the population is what the *files* state rather
than what the clause decides.** ISO 32000-2's "341 of 1023 pages carry `NoZoom`" is `zoom_cost`'s
`/F` scan, and most of those 341 are the 211 strike-outs §12.5.6.10 clears — which is why so many
of the `all` run's arms cost 290 ns. The example says so now; the number is still the right *driver*
for the measurement, because a superset finds the worst notch and a page outside the clause costs
the run nothing.

## 4. What checks it

- **`pdf-model/tests/replacement.rs`**, which runs on every build.
  `a_replaced_page_is_the_page_it_would_have_been_interpreted_as` takes eight view-dependent pages
  spread across ISO 32000-2, re-places each at three magnifications, and compares the result against
  a whole interpretation at the same magnification **field by field, with no `..` in the
  destructure**. Calibrated against a planted defect (trap 13): with `restore` dropping the
  readback, it fails on page 10 with the page's whole text in the message.
  `a_page_no_annotation_makes_view_dependent_keeps_nothing_to_replace_from` is the other direction,
  and both refuse to pass on an empty population.
- **`pdf-model/examples/replacement_census`**, which is the wide population and is not a gate: every
  page of every document of a corpus, both comparisons. Over `doc/pdf.js/test/pdfs`: 974 documents,
  36 comparisons, **0 disagreements, 0 pages where the seam's condition and the clause's answer
  disagree**. Over `doc/`: 14 documents, 297 comparisons, 0 and 0.

  It exists because the test's population cannot discriminate every field — the calibration found
  that a `restore` dropping `glyph_coverage` passes on ISO 32000-2's pages, whose every font draws.
  That is worth saying plainly rather than leaving implied: **the compiler's exhaustiveness is what
  guards a field the corpus does not exercise**, and the census is what would find the next one the
  corpus does.

## 5. What this does not change

- **No pixel moves.** `interpret_with_fonts` is `interpreted(…, Keep::Nothing)` and produces what it
  always did; `replace` produces what `interpret_with_fonts` would. §2's whole sequence ran green,
  including the oracle, the quorra corpus and the fixed documents, and `doc/todo/00`'s step 7 is not
  owed because nothing this round did changes what is drawn.
- **`Viewer::handle` stays synchronous** and `interpret` stays a pure function of the document, the
  view state and what the user did — which is what `CLAUDE.md`'s authoring exclusion rests on. The
  re-placement happens inside the command that changed the magnification, exactly where the
  re-interpretation did.
- **Nothing is debounced**, which `doc/todo/46` names unacceptable.
- **`Open::stale` is the one thing that discards a replacement**, and that is the precondition
  `content::interpret_replaceable` documents: the content half was resolved against a view state,
  and everything in that state but the magnification is baked into it.
