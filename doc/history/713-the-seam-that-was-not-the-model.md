# 713 — The seam that was not the model

Seventh merge round of the block. Four branches, **no conflicts**, and a batch whose four rounds
between them overturned an ADR's central sentence, a todo item's dependency, a ledger row's
citation, and a price's *mechanism*.

## The sequence, whole, on a quiet machine (load 1.82)

Both workers built first. `fmt` · `clippy --workspace --all-targets` under `-D warnings`, exit 0 ·
the fuzz check, exit 0 · `nextest` **2548 passed, 18 skipped** · conformance 182 + 5 + 1 + 1 ·
`cargo deny` all four ok · corpus **974 documents, 67 incomplete** · oracle **1945 pages — 983
agrees, 65 contradicted, 832 ambiguous, 42 not comparable** · `render-quorra` **957 pages — 933
agree, 22 differ** · both censuses · `fixed_documents` 40/0 · text, dates, XMP, JPEG 2000. §5's
binaries rebuilt and installed. Ledger 875 rows, **445 implemented, 223 partial, 0 unreviewed**.

`tools/state.sh hosts`: **`Query` 31 of 31, 169 entry points** (was 20 of 31 at 117).

## 711 — the sentence that had stood, and the dependency that never existed

**ADR 0308's central sentence — "the seam is not a deviation from the model, it is the model" — is
wrong, and the standard says so in four steps.** §11.2 is a `shall`: shape and opacity "shall be
defined at every point in the plane". §11.6.4.2 gives the value at a point: for a path or a glyph the
shape "shall always be 1.0 inside and 0.0 outside the path". So §11.3.7.3's union is a function of
**point** values, and across a seam between two abutting marks one input is always 1.0 — the union is
1.0, and **the model states no seam anywhere**. The fraction enters only through §11.3.7.2's NOTE 1,
a `can`, in a NOTE, about what happens "when such objects are rasterized to device pixels", because
averaging does not commute with a non-linear function.

So the seam is a **licensed departure** (§10.7.1's NOTE) from a value the clause defines. And §11.2's
own NOTE 1 names the cause: the model "does not require a PDF processor … to commit to a raster
representation at any time before rendering the entire stack onto the page … since rasterization
often causes significant loss of information and precision". Which also falsifies *"not a defect of
this program"*: **every rasteriser measured has the seam because every one commits to a raster per
object**, which is the thing that NOTE says causes the loss.

**And item 7's remainder was never blocked.** The block cited §11.3.7.3, which governs two
*objects*; a path stating several rectangles is one object's **subpaths**, and §11.6.2 is a `shall`
about exactly that — *"Portions of an object shall not be composited with one another."* The
construction being weighed against a seam is **forbidden, not traded**.

So the round built it. Disjoint device rectangles are drawn at their exact closed-form area, **for
the mark and for the clipping region alike**, because measuring only one breaks `S ∩ C = S` — ADR
0476's lesson applied rather than re-learned. Census before the code (trap 14): of 223 545 fills,
12 987 are one rectangle and **3419 are several with no shared device pixel**. **Item 7's remainder
is unblocked and seven eighths paid**; the remaining 505 want one coverage buffer per mark, which is
not item 5's rasteriser.

Measured: **135 of 974 first pages move pixels**, worst channel 45–58 levels; the oracle is
**byte-identical both ways**, every ranking line the same; the cross-backend gate goes **932/23 →
933/22**, recovering the page session 690's improvement had cost; instruction counts −0.531% to
+0.074%. And a stale attribution corrected: ADR 0308's two rasterisers' figures are **exchanged**,
because ADR 0476 made the processor's rectangle exact — the ledger row had copied them.

## 709 — the boundary, and the instrument that will keep it

`Query` reaches all 31 now, and `viewer-core` was not touched: no message, no variant reshaped.
**The deliverable is the instrument.** `PDFV_EVENT_KIND_COUNT` protects a caller against a message
that *arrives* and against nothing where a *question* is concerned — which is how eleven accumulated
with no symbol and no signal. `every_query_reaches_the_abi.rs` matches exhaustively over `Query` with
**no allow-list**, which is exactly why all eleven had to land in one round, and it counts its
enumeration out of the source rather than carrying the number.

Two shapes argued rather than mechanical: Table 147's preferences are a **keyed accessor**, because a
struct by value would put that table's *size* in the ABI — and `PDFV_ABI_VERSION` accordingly did not
move, none of the 52 new entry points passing a struct by value. And **§12.3.4 was designed against a
defect**: there is no bulk thumbnail read at all, so 704's launch-path violation has **no road into a
C host**. Measured on a 233-page document: eight rows 0.81 ms / 210 672 B against every page 21.6 ms
/ 6 952 176 B.

**Trap 11 caught in the act**: the first version of its window count reported §12.3.5's collection as
reached, because a doc comment saying *"a different answer … that this host does not yet ask"*
matched as a call — and `tools/state.sh hosts` had carried that same condition since ADR 0509 without
being bitten. Both strip comments now. The new `tools/state.sh windows` reports one variant no window
reaches, `Query::Dirty`, **and says why that is not a debt** — all three learn it from the event. A
zero that needs reading rather than fixing, said so where it is printed.

## 710 — a correction that did not reach the row depending on it

Its family came from 705's ranking, run rather than read — and **the raw head is §12, an aggregate of
96 `partial` rows**, so clause-level parents have to be stripped before a head means anything. The
sharper rule: **the total ranks the family, the pairs choose the reading.**

Three findings, each following from the last:

1. **§12.5.5 claimed §12.5.2 lists three keys a reader shall ignore. It lists two** — errata strike
   the third, which §12.5.2's *own* row has recorded since session 417 and which the code applies on
   both paths. That is 697's rule **running backwards**: the correction landed in the row stating the
   mechanism and never reached the row depending on it, which is the more dangerous direction.
2. **A citation doing an argument's work, at the wrong clause.** §12.5.5 justified building no
   transparency group with "§11.6.7's NOTE 1"; §11.6.7 is *Patterns and transparency*, and the
   reduction is **§11.4.4's NOTE 5 — which states its conditions**, the second of which a non-Normal
   `/BM` breaks. No sweep in this tree could print it: a clause number beside a paraphrase is neither
   a table citation nor a quoted span.
3. **A `shall` with no reader and no report** — "the isolated and knockout values specified in the
   group dictionary shall be used" — because appearance streams ran directly and never reached the
   group path. Report added; **implementing the group declined in writing**, all four corpus
   appearance groups being the default kind, so it was pixel risk for zero requirement gain.

**Erratum #577 is a third distinct blindness in `check`: a strikeout whose text is a *value*.** `1.0`
shares no sentence with anything. Its census is committed because the row now states figures — 4
appearance groups over the 974, none isolated or knockout, against **95 isolated, 1 knockout and 8
`/Multiply`** over the crawl.

## 712 — the price that named the wrong mechanism

`image_stream` sat outside the memo on the reason "a codec's bytes are not a filter chain's" — true
about the codec and **false about everything Table 5 lets `/Filter` put in front of it**. Census:
**2420 of 2997 image XObjects run a filter there, 467.7 MB a pass.**

**The re-derivation changed what got built.** Reading the call site rather than profiling found that
**one `Do` asks `image_stream` four times** — three predicates each decoding the chain and *then*
asking which codec it was, none covered by the raster cache because it is consulted afterwards. So
the answer is a reordering *and* a memo.

Attribution stated both ways, including where the memo loses: on a 1023-page sweep, reordering alone
−2.35%, memo alone −2.06%, together **−2.26%** — the memo a net **loss of 0.10%** there, since that
document repeats none of its images and pays only displacement. On a document that repeats one image
the memo is 43 points on top, for **−60.5%**. The wall-clock attempt was discarded outright: one
binary gave 3.40 s, 4.45 s and 6.90 s as neighbour load moved.

**The declined half came with its witness and a rule.** The oversized refusal is real and reachable —
two documents differing only in gibibytes of zeros, **257 µs against 6.93 s, ≈25 000×** — but all
three constructions that would hold it give something up. *When every construction of a fix is bad,
the fix is in the wrong layer*: a pump over a chain whose every stage is pumpable removes the
gibibyte instead of remembering it, on every read. `doc/todo/14` is **reopened** with the generator
and the number.

`doc/habits.md` gains a **fourth shape of wrong price — one that names the wrong mechanism** — and
the rule that a memo's price counts the caller's calls before measuring its misses.

## What this batch says about the method

Four rounds, four overturned premises, and none of them found by a sweep:

| what was wrong | how it was found |
|---|---|
| an ADR's central sentence about the model | reading four clauses in sequence |
| a todo item's dependency | noticing the block cited a clause about *objects* for a question about *subpaths* |
| a ledger row's supporting citation | opening the NOTE the row named |
| a price's mechanism | reading the call site instead of profiling |

Every one is a *claim in this tree's own prose* checked against the standard or the code, which is
what the last two blocks have been about — and every one was cheaper than the work it guarded.

## Owed

- **505 fills sharing a device pixel** — item 7's true remainder, wanting one coverage buffer per mark.
- **`doc/todo/14` reopened**: a pump over a hex-wrapped chain, with the generator and the 25 000× figure.
- **`AccessibilityNode::lines` does not cross the ABI**, so a C caller has the tree and the extents
  and not the character offsets. Two accessors, no new decision.
- **The `#[non_exhaustive]` decision**, which quorra says is the project owner's to time.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
