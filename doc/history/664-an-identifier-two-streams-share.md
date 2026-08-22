# 664 — An identifier two streams share

Tenth merge round, four branches, **no conflicts** — the third clean four-way merge running. The
batch's centre is 661, which turned an unmeasured worry into a measured defect and then found two
more beside it.

## What was merged

`round-660`, `round-661`, `round-662`, `round-663`, branched from `f92a48b7` and its two successors.

## The sequence, whole, on a quiet machine

`fmt`, `clippy --workspace --all-targets` under `-D warnings`, `cargo check --manifest-path
fuzz/Cargo.toml --bins` — all silent · `nextest` **2413 passed, 17 skipped** · doctests, conformance
(163 + 5 + 1) · corpus **974 documents, 68 incomplete** · oracle **908 agrees, 65 contradicted, 786
ambiguous** · `render-quorra` **933 agree, 22 differ** · `fixed_documents` **40 checked, 0 absent** ·
accessibility census **1336 with no place** · text, selection, dates, XMP, JPEG 2000 · `cargo deny`
all four ok. Ledger unchanged at 875 rows, 224 `partial`, no `silent` row.

## 661: the collision is real, and two more defects were beside it

§14.7.5.2 says it three ways — a `/MCID` "uniquely identifies the marked-content sequence **within
its content stream**", a form XObject's stream may carry its own sequences, and Example 5 writes the
collision out with page and form both numbering from zero. §14.7.5.4 makes the route back per stream
by construction, and Errata Collection 3 Issue #308's NOTE states the consequence outright. So the
clause answers cleanly and **this tree was flattening two streams' numbering**.

Measured, with the census calibrated against a planted collision before it was believed:

| | curated (1245) | crawl (65 703) |
|---|---|---|
| pages marked by ≥ 2 streams | 1 document | **701 documents** |
| two streams sharing an identifier | **1** | **42** — 163 pages, 7334 identifiers |
| stating Table 357's `/Stm` | 0 | 545 |

**17 of the 42 also state `/Stm`** — conforming files, read wrong outright. Both things keyed on the
identifier were affected: ADR 0134's text range and ADR 0486's rectangle. And going in found two more
that nobody had named: §14.9's `/Alt` was indexing the **page's** parent tree with a **form's**
identifier, and `elements_on_page` asked a form only for `/StructParent`, so an element tagged inside
a form was pruned as belonging to another page.

**One recovery, with its cost written down.** The strict reading cost 61 elements their place in two
corpus documents that put every sequence in one form and name each with a bare integer — **the
accessibility ratchet caught it**, which is the gate doing exactly what it is for. So where the
page's own stream holds no such identifier and exactly one other stream does, that one is answered;
two carrying it answers nothing rather than both. The recovery is an *inference a caller cannot
distinguish from a stated attribution*, and `doc/todo/31` says so rather than hiding it.

## 660: a warning answered by a signature

655 left this with a warning — a rebuild that reached for the graphics state *at the paint* would
read the very parameters §11.6.7 says a pattern "shall not inherit … at the time it is evaluated".
**`mark_colouring` and `build_shading` take a `&ShadingDefinition` and no `&GraphicsState`**, so the
wrong version does not compile. A type rather than a rule, on this tree's precedent (`HeldContent`,
`Ungrounded`).

Its price re-derivation is an honest draw and still paid: 655 said a hundred lines, the actual is net
185, and **five of the six pieces were already in the tree** — the missing one was the definition
itself, since `pattern()` read the `/Shading` object, resources and matrix and dropped all three at
the end of the function. What the re-derivation bought was knowing the cache turns a rebuild-per-mark
into a lookup, so no memo was needed. Byte-identical rasters over 974 corpus pages **and** 221 named
crawled documents.

And reading §10.5 whole found a **different** debt: its second bullet requires a halftone
dictionary's `TransferFunction` to override the graphics state's, and `/HT` is read by nobody on
§10.6's inapplicability. Closing one half of a row exposed another.

## 662 and 663: two criteria, and a claim in eight homes

**662 invented the sharpest selection criterion this pool has had**, and it is a principle-5 question:
*a contradicted verdict claims the standard rather than the consensus decides the page — so how many
clauses does the group's note cite?* Seventeen for one group, eighteen for another, and **zero for
`CONTRADICTED_TIGHT_CONSENSUS`**, whose name is itself a statement about the references. All three of
its pages are §10.7.4 in three different paragraphs, none cited; ours is the arithmetic at 0.166 of
255 where the others are 2.8 to 6.7; and **all five renderers are nearer the area average than the
point sample**, so every one departs from "there shall not be averaging over the pixel area". It also
found a minimal witness that had eluded nineteen sessions: seven rungs give an edge its own coverage
and the eighth — transparency group plus soft mask — **squares it**, reached only because §11.4.4's
NOTE 5 flattens a group away unless a mask is in force.

**663 swept for negatives measured before the crawl existed** and found four, plus the decay running
**the other way**: a *positive* count stale in the curated direction, because a second witness arrived
with a submodule. §12.6.4.11's row says "this one changes what is drawn" and claimed no witness; the
crawl holds **2165 `/S /Hide` actions over 8 documents**, all performed. Its defect is a retired claim
**living in eight homes**, one of them user-visible: a host was being told "five of Table 164's twelve
transition styles are reported by name" when four are and `R` is a cut. Session 553 fixed two ledger
rows and left the other eight.

## A claim from 660 that does not reproduce, and is recorded as not reproducing

660 reported that `cargo nextest run -p pdf-model` **alone** fails six CCITT tests on `HEAD` with
black and white exchanged, passing only under `--workspace`, and attributed it to Cargo's feature
unification resolving `hayro-ccitt` differently when the package is scoped. It cost that round twenty
minutes.

**On the merged tree it does not reproduce**: `cargo nextest run -p pdf-model` is **1099 passed, 14
skipped**. Either it was true of that branch's base and something in this batch changed it, or it was
particular to that worktree. Recorded here rather than in the traps, because a trap that cannot be
reproduced is a trap that will waste the next round's time in the other direction — and the round
that meets it again now has both observations to work from.

## Owed

- **An appearance stream no `/Stm` can name**, closable only by carrying the `/AP` reference through
  `Appearance` with Table 357's `/StmOwn` — the same item from the structure side.
- **§10.5's halftone `TransferFunction`**, new from 660.
- **`doc/todo/11` item 4's group blit**, which now has a ladder *and* a corpus witness and still needs
  a shape channel beside a group's raster.
- **No gate links an oracle group's note to the code it describes** (662): trap 1 states the habit and
  nothing enforces it, which is how ADR 0476 left a note stale within three sessions.
- **The owner's session** for `tmp/pi.pdf`, and a push — CI's `test` job failed on the last one and is
  fixed on `main` (`7d8695af`); the next run should judge it.
