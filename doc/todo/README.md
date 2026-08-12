# The todo directory

One file per piece of owed work. `doc/HANDOVER.md` says what the project *is*; these say what
it still owes, and they hold the detail that used to make that file long — the evidence, the
clause reading, the measurement and the cost of each item live with the item.

**The index below is one line per item, and that is deliberate.** Each file's own header block
carries its status, its corpus witnesses, its clauses and its code, and *that* is the authority:
a summary here that restates them is a second copy to keep in sync, which is how this index came
to say a clause was ignored eighty-seven rounds after it was implemented. Read the line to choose
an item; open the file to take it.

## Sorting

The number prefix **is** the priority, and `ls` sorts by it:

| band | means |
|---|---|
| `00`–`09` | **standing** — take some of this every round, whatever else is on |
| `10`–`19` | **defects**: wrong pixels or wrong output, with a diagnosis and usually a fix already argued |
| `20`–`29` | **owed features with corpus demand** — a real document asks and we refuse |
| `30`–`39` | **capability** — things the program cannot do at all, mostly hosts and interfaces |
| `40`–`49` | **performance**, each already measured and priced |
| `50`–`59` | **blocked** on a dependency, a decision or an infrastructure this program does not have |
| `_`-prefixed | not a todo: shared background several items refer to |

Within a band the number is a rough order, not a queue. `CLAUDE.md`'s two tracks still decide:
take from the *demand-driven* side (10–29, what the corpus and oracle name) **and** the
*spec-driven* side (00–09, what the ledger and the standard name) in every round.

## What a file holds

A header block — status, priority, corpus witnesses, the clauses, the code — then the argument.
A todo file is a place to *think*: what the clause determines, what has been measured, what the
fix would cost, and what has to be settled before it can be taken. When an item is done the file
is **deleted** and its argument lives on in an ADR, which is where a decision belongs.

## The index

| | item |
|---|---|
| [00](00-ambiguous-bucket.md) | Empty the oracle's ambiguous bucket, page by page — emptied, so the item is now the ratchet plus step 7's ink sweep, which a round that changes pixels re-runs |
| [01](01-ledger-partial-rows.md) | Read the ledger's `partial` rows against the code — and the sweeps, the last of which reads quotation marks (ADRs 0249, 0254, 0255) |
| [02](02-every-round.md) | **What every round does** — the gates, the sweeps, the binaries, the commit |
| [03](03-more-corpora.md) | More corpora, the fetcher and the survey: three submodules under `doc/corpora/`, plus `tools/safedocs` over the SafeDocs crawl. What is left is *taking* a chunk a round, a licence reading for `openpreserve/format-corpus`, and the oracle over the new corpora (ADRs 0258, 0261, 0264, 0266, 0269, 0271) |
| [11](11-shapes-that-still-disappear.md) | Shapes that still disappear: the sub-pixel fill, stroke and diagonal are closed (ADRs 0226, 0268) and §10.7.4's clip chain is half paid (ADR 0280). What is left is the same sentence one step along — `tiny-skia` multiplies the finished mask into the mark's own coverage — plus the rule exactly one device pixel wide, the cap a substitute does not draw, and two marks abutting across a cell's box edge |
| [12](12-one-bound-two-jobs.md) | One bound doing two jobs: the differing fraction on text pages sits below the spread of the implementations that set it, and it also decides whether two references form a consensus — measured and left where it is (ADR 0243) |
| [13](13-the-transfer-function.md) | §10.5's `/TR` — **implemented**; the file is kept for the argument, which is what the round cost, and because `01`'s sweeps read it |
| [21](21-font-substitution.md) | `Identity` orderings; per-character fallback, with no witness left. A substituted face is replaced by one of its family whose code table over the declared range is a strict superset, and ADR 0152's silence is kept with numbers (ADRs 0269, 0270) |
| [22](22-variable-text-edges.md) | §12.7.4.3's remaining edges: a `/DA` font `/DR` lacks and cannot spell, a composite `/DA` font with no witness (ADRs 0240, 0247, 0248) |
| [23](23-transparency-departures.md) | §11.4, §11.4.7 and §11.6.6, each reported where it can change a pixel. Six rounds closed five constructions somebody had priced (ADRs 0217, 0220, 0234, 0237, 0262, 0263, 0272, 0274, 0275, 0276, 0277); what stands wants a *second* colour space rather than a second direction, plus a knockout group whose elements blend |
| [24](24-image-sampling-intent.md) | Carry an image *and its sampling intent* to the backends — the vocabulary is built and the mask is on it (ADR 0210); JPEG 2000's resolution level waits on one push to `close2/hayro` (ADR 0233), and `render-gpu`'s sampled shading is the rest |
| [25](25-view-dependent-annotations.md) | `/FixedPrint`, which waits on a printing path |
| [26](26-icons-a-clause-only-recommends.md) | `Stamp`'s icon, whose standard names are legends rather than symbols |
| [27](27-the-interfaces-own-font.md) | The interface's own font: a character it cannot set is a box now; what is open is *coverage* |
| [28](28-a-catalogue-that-draws-nothing.md) | A catalogue that drew nothing — **nothing is left to build and the whole file is one run**: every departure it printed is expected to be gone, and only the owner has the document |
| [30](30-a-native-host.md) | All three native hosts are built (ADRs 0244, 0246, 0247) and none needed a new message. What is left is *surface*: the C ABI's entry points are not the whole vocabulary, plus the scale a form host draws at and `RadiosInUnison` |
| [31](31-accessibility-host.md) | AccessKit over `Query::AccessibilityTree` — built and read back off a real AT-SPI bus (ADR 0214); a `TH`'s axis, a `Form`'s control role, the `Text` interface and actions are what is left |
| [32](32-presentation-player.md) | Table 164's transition styles: the ones the table's own words determine are drawn (ADR 0230), the rest are reported by name, and §12.4.4.2's states have no control |
| [33](33-annotation-editing.md) | Editing a free text annotation the *file* states, and Table 177's `/CL` callout line |
| [34](34-sandbox-the-interpreter.md) | Confine the interpreter and rasteriser — built, drawing real pages behind seccomp, answering every question and stoppable (ADRs 0218, 0223, 0241). What is left is the window, which is tier 2 |
| [35](35-confinement-off-linux.md) | Confinement on macOS and Windows — what the snapshot release cannot ship, and the three ways out |
| [36](36-a-retrieval-api.md) | Retrieve the standard from the standard. The CLI is built and all three joins are closed (ADR 0257); what is left is one `Query` variant for a page's text on the confined pipe, and the substitution itself |
| [38](38-a-documents-restrictions-have-levels.md) | A document's restrictions are the reader's to set: the shape is built and Table 22 is consulted (ADR 0212); *ask* and *warn* wait for a host that can answer |
| [39](39-a-fragment-that-says-where-to-open.md) | Annex O's fragment identifiers — `tools/state.sh annex-o` says which are carried out and which are reported, and this file says what each of the rest needs: a fetch, todo 38's *ask* level, and a concept the vocabulary lacks (ADRs 0209, 0250) |
| [40](40-mask-chain-crop.md) | A clip chain as one crop and one intersect — unblocked and re-priced (ADR 0236), with a soft mask's buffer and a group's target-sized where a clip mask's is band-sized. The arithmetic over them is gone (ADR 0271); the copying is not |
| [41](41-decoded-stream-cache.md) | A decoded-stream cache: a fraction of a percent of interpretation, priced and not taken |
| [42](42-the-launch-path.md) | The launch path: four of five items closed, the fifth is quorra's |
| [43](43-the-projects-own-turnaround.md) | The *project's* performance rather than the program's: §5's fat link is what is left, caching our own renders is priced and refused, and the three gates that rasterise all 974 first pages roughly doubled with nothing else moving and nobody has bisected it |
| [45](45-where-a-frame-goes.md) | Where a frame goes, once the instrument could attribute one (ADRs 0227, 0228): quorra's `encode`, a reduced raster recomputed on every redraw, and no run on the owner's own machine |
| [46](46-a-region-of-a-huge-image.md) | A region of a huge image: reduced resolution answers *zoomed out*, so *zoomed in* needs code-block skipping and a bounded IDWT — census first |
| [47](47-search-performance.md) | A cold document-wide search, half answered (ADRs 0256, 0260): a repeated search is nearly free, a cold one is unchanged, parallelism is measured and declined on memory, and nothing memoises a decoded stream |
| [48](48-the-specification-we-check-against.md) | The specification we check against: `doc/md/` dropped every annotation and the annotations are the **errata** (ADRs 0252, 0253, 0254, 0255). What is left: §8.9.5.4, §14.8.6.3's enclosure requirement, single-quoted ledger spans, and every quotation of the standard in `doc/*.md`, `doc/todo/` and the ADRs, which nothing reads at all |
| [49](49-restrictions-worth-re-examining.md) | Which restrictions are load-bearing and which are habit — four of five settled (ADRs 0256, 0257, 0260, 0271); item 3, the API that hands a thread pool in, is what is left |
| [51](51-signatures-and-public-keys.md) | A signature's three questions: the first two are answered (ADRs 0215, 0229). What is left is Table 260's DSA and ECDSA, the signer's *trustworthiness* — a certificate store and a network — public-key handlers, and `/R` 5 |
| [52](52-zlib-rs-deallocates-through-the-wrong-pointer.md) | `zlib-rs` fails both of Miri's aliasing models — an upstream report to write |
| — | [`_scan-conversion.md`](_scan-conversion.md) — shared: §10.7.4, what this tree departs from and why |
| — | [`_image-codecs-and-the-sandbox.md`](_image-codecs-and-the-sandbox.md) — shared: the three sandboxed codecs are already pure safe Rust, what the sandbox is really for, and what a subset would and would not buy |

**Closed by decision rather than by work** — recorded in `doc/conformance/ledger.toml` and not
here: `/ColorTransform` (Table 13, whose one corpus witness contradicts the clause), a stream
whose data is in an external file (§7.3.8.1 — the renderer has no filesystem, principle 3),
§12.7.6.2's submit and §12.6.4's remote, launch, sound and movie actions (a network, a second
file, a media engine), a filled degenerate subpath's device pixel (§8.5.3.3.1, which the clause
itself calls "device-dependent and not generally useful"), and grid-fitting a stroke's
coordinates under `/SA` (see `_scan-conversion.md`).
