# 767 — The third backend a two-backend sentence never counted

The ledger's `partial` rows read as a family, on the older question rather than on the errata
ranking — §8.9.6, masked images, off the blame ordering's head. Two findings, failing in opposite
directions: a row owing **more** than the tree owes, and a row owing **less** than it says.

Date: 2026-08-25.
ADR: [0697](../adr/0697-the-third-backend-a-two-backend-sentence-never-counted.md).

Touched: `doc/conformance/ledger.toml` (§8.9.6, §8.9.6.1, §8.9.6.2),
`crates/pdf-model/tests/image_masks.rs` (one test),
`crates/render-gpu/tests/headless_gpu.rs` (one test),
`crates/render-quorra/examples/filtered_edge_colour.rs` (new),
`doc/QUORRA_FEEDBACK.md` (section 39), `doc/todo/55-a-filter-that-mixes-in-black.md` (new),
`doc/todo/README.md`, `doc/verify.md`, the ADR and this file. **No library source is touched at
all**: everything added is a test, an example or a document, so no pixel this tree draws can have
moved.

## Why this family

The pair ranking has no head left — 730 spent the last of its three strongest pairs and fell to a
tie at rank 4, and 734 put its score at chance over individual rows. So the ordering used here is
the one session 442 read thirty-two rows off and nothing has re-derived since: every `partial` row
by when its own `note` line was last written, which `git blame` answers. Its head is the batch 442
itself wrote, which is the ordering's one known flaw and is exactly what makes the head worth taking
now: those notes have stood unread while the tree moved under them, and a claim's *age* is the one
property no sweep in `doc/todo/01` measures. §8.9.6 and §8.9.6.2 are in that batch.

## The findings

- **A `shall` met by two backends out of three, in a sentence that said "both".** §8.9.6.2's last
  requirement — interpolation during stencil masking smooths the edges of the mask and does not
  interpolate the painted colour values — was answered in two rows by naming the raster rather than
  a branch, and the answer is right: a stencil's cleared samples are `[0, 0, 0, 0]`, so filtering
  premultiplied gives the painted colour at partial coverage exactly and filtering straight mixes
  in the black those samples are stored with. The rows then wrote **both backends**, and there are
  three. The third filters straight: `crates/quorra-gpu/src/shaders/image.wgsl` samples and
  premultiplies afterwards, so a tap is `mean(rgb) * mean(a)` where the clause asks for
  `mean(rgb * a)`. Measured by the new example on one magnified stencil — the CPU backend and vello
  give `[255, 0, 0, alpha]` at every partly covered pixel and quorra gives `[126, 0, 0, alpha]`, a
  departure of **131 of 255** on the painted channel. **It is what the shipped viewer draws with**,
  and `Image::is_smoothed` turns the filter on for every reduced image as well as for every
  `/Interpolate`.
- **A refusal restated one condition too wide.** §8.9.6, §8.9.6.1 and §8.9.6.2 each named "a stencil
  under a graphics-state soft mask" among the family's residue. The refusal in `content::image` is a
  stencil painted through a **pattern** under one — where the recomposition needs the mask slot the
  state is already using — and §8.9.6.2's own note has the two paragraphs that bound it. The parents
  carried the sentence without them. An ordinary stencil under a soft mask is drawn through both,
  which §11.6.4.3 requires rather than permits, and nothing here had ever asked.

## What would have predicted them

- The first: **a cardinal that counts this tree's own parts**. `--bin counts`, the tenth sweep,
  reads a cardinal only where it governs one of the ledger's own words for a *row*; "both backends"
  governs a noun this tree has a countable population of, and no instrument reads that population at
  all. The same two words are still in `pdf-render`'s own `Image::is_smoothed` doc comment and in
  the comment above it, both written before quorra existed. A sweep whose left-hand side is a
  numeral governing `backend`, `rasteriser`, `crate`, `worker`, `host` or `submodule`, and whose
  right-hand side is the workspace's own membership, would have printed all three.
- The second: **an understating parent**, the mirror of the eighteenth sweep. `--bin overstated`
  reads a parent asserting something *is read* against a child denying it; this is a parent
  restating a child's *refusal* and losing the condition the child stated it under. Both sides are
  again this project's own sentences, so a program could read it.

Neither is in any existing sweep's population, and `--bin overstated` was byte-identical across this
round.

## Gates and sweeps

`tools/round.sh` says this is **not** a fifth round, and the change→gate map's crates are reached
only through test targets and one example, so §2's core ran and the two gates the touched crates
name ran beside it: `fmt`, `clippy -D warnings`, `nextest --workspace` (2686 tests, 18 skipped), the
doctests, the fuzz `check`, the sandbox worker, `pdf-model`'s corpus gate, `render-quorra`'s corpus
gate and `cargo test -p conformance`, which was the last thing run. All green. §5 was not owed and
no measurement was taken against an installed binary. The machine's one-minute load was 10 to 23
through the run and no line that spawns a reference renderer was among them.

**The conformance gate failed twice and both failures were this round's**, which is what it is for:
an unescaped pair of quotation marks inside a TOML note, and a `§39` written after
`doc/QUORRA_FEEDBACK.md`, which `the_ledgers_own_prose_names_clauses_and_tables_that_exist` reads as
a clause of ISO 32000-2. Both corrected; the second is why the ledger says "section 39".

**Clippy caught a paraphrase wearing quotation marks** — a `doc_markdown` lint on a sentence this
round had invented and attributed to §11.6.4.3. It is a rustdoc blockquote of the clause's own words
now.

Thirteen sweeps run before the edits and after them, with `spec-errata`'s `applied` and `check`
beside them. `tables`, `entries`, `unread`, `blockers`, `callers`, `overstated` and `ledger` are
**byte-identical**. **Not one defect bucket moved:**

- `pointers` 8544 ← 8524 with **absent 134 ← 131**, and the three are one path —
  `crates/quorra-gpu/src/shaders/image.wgsl`, named by the feedback section, the ADR and the todo
  file. That is the sweep's fourth rung, a path this tree deliberately does not carry, and eight
  quorra paths were already in that bucket for the same reason. Symbol pointers 146 ← 140, **13
  undefined unchanged**.
- `counts` 8313 ← 8294 sentences with 431 ← 430 attributed counts, **149 the family agrees with, 58
  "no such way" and 4 places counting one family twice, all three unchanged**; the new count is
  attributed to a clause with no rows below it, 224 ← 223.
- `owed` 3924 ← 3913 terms with **180 named by no source over 112 rows, both unchanged**.
- `quotations` 6412 ← 6404 document quotations over 999 ← 997 documents with **diverging unchanged
  at 38**, and 1952 ← 1951 ledger quotations with **diverging unchanged at 2**.
- `capabilities` 193 ← 192 sentences, 159 ← 158 witnessed; `inapplicable` unchanged in every bucket,
  with one vocabulary count gaining a naming file; `overtaken` 587 ← 586 decision records with **47
  overtaken unchanged**.
- `spec-errata applied`: 772 places naming an erratum over 57 445 read, with **the read-first list
  at 10, the corrections quoting retired wording at 90 and the places inside `errata-read.md` at
  72** — the same four figures 760 recorded, so this round's prose added no erratum claim.

`quoted` and `unpriced` were not run: this round touches no page-list note and both take the
oracle's log as their right-hand side.

**The numbering of the new todo file was chosen by the pointer sweep rather than by the band.** It
was written as `50`, which resolved a dead pointer in ADR 0186 onto an unrelated file; moved to
`54`, which resolved five more in ADRs 0412, 0413 and 0435; and is `55`, which nothing in the tree
cites. A deleted todo file leaves its number behind, and reusing one silently repoints every ADR
that referred to it — visible only because `--bin pointers` counts *live* pointers as well as absent
ones, and the live count went **up** while the file was still called 54.

**Both of the round's tests are calibrated against a failure rather than a plant where one was
available** (trap 13). `a_stencil_under_the_graphics_states_soft_mask_is_drawn_through_both` was run
against two plants, one for each thing the rows described: the state's mask dropped for a stencil —
it fails on the cell that should have been cut — and the refusal the rows describe, added by name —
it fails on the report. `cpu_and_gpu_smooth_a_stencils_edges_without_darkening_its_colour` needs no
plant: the third backend in this tree fails it today, which the example prints.
