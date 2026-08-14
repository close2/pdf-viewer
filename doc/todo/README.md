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
| [01](01-ledger-partial-rows.md) | Read the ledger's `partial` rows against the code — and the fifteen sweeps, of which the newest asks who reads an *entry the clause states* and is the first to be a committed program rather than a description (ADRs 0249, 0254, 0255, 0295, 0315, 0319) |
| [02](02-every-round.md) | **What every round does** — the gates, the sweeps, the binaries, the commit |
| [03](03-more-corpora.md) | More corpora, the fetcher and the survey: four submodules under `doc/corpora/`, plus `tools/safedocs` over the SafeDocs crawl. What is left is *taking* a chunk a round and the oracle over the new corpora; the rule for adding one is the owner's and is now *include unless a licence clearly forbids* (ADRs 0258, 0261, 0264, 0266, 0269, 0271, 0302, 0303, 0305) |
| [05](05-an-instrument-for-the-interactive-surface.md) | An instrument for the interactive surface — the gates measure a raster and the work has moved off it; the design is settled with its tolerance measured reference-against-reference (ADR 0323), and what is left is one round per instrument: selection geometry, the save round-trip, the accessibility ratchet |
| [10](10-bounds-that-cap-size.md) | Bounds that cap size rather than guard against a bomb — asked for by the project owner, with a witness they supplied. The three defects owed on any road are carried out and the witness draws whole (ADR 0306): a bound that said operators counted lexer tokens, a length guard clamped in silence, and one stream could command the confined worker's whole ceiling. What is open is the four roads — the fourth, streaming the decompression, being the only one that removes the allocation rather than surviving it — and that choice is the owner's |
| [11](11-shapes-that-still-disappear.md) | Shapes that still disappear: the sub-pixel fill, stroke, diagonal, the quantum's own boundary and the marks whose area is a *square* of the width — §8.4.3.3's caps and §8.5.3.2's dot — are closed (ADRs 0226, 0268, 0285, 0290), and §10.7.4's clip chain is half paid (ADR 0280). What is left is the same sentence one step along — `tiny-skia` multiplies the finished mask into the mark's own coverage — plus what an eight-bit raster does to a mark whose ink is under one of its levels, and two marks abutting *anywhere* — which is witnessed, measured on all three rasterisers and priced as ADR 0308's item 5, and is not a defect of this program |
| [12](12-one-bound-two-jobs.md) | One bound doing two jobs: the differing fraction on text pages sits below the spread of the implementations that set it, and it also decides whether two references form a consensus — measured and left where it is (ADR 0243) |
| [13](13-the-transfer-function.md) | §10.5's `/TR` — **implemented**; the file is kept for the argument, which is what the round cost, and because `01`'s sweeps read it |
| [21](21-font-substitution.md) | `Identity` orderings; per-character fallback, with no witness left. A substituted face is replaced by one of its family whose code table over the declared range is a strict superset, and ADR 0152's silence is kept with numbers (ADRs 0269, 0270) |
| [22](22-variable-text-edges.md) | §12.7.4.3's remaining edges: a `/DA` font `/DR` lacks and cannot spell, a composite `/DA` font with no witness (ADRs 0240, 0247, 0248) |
| [23](23-transparency-departures.md) | §11.4, §11.4.7 and §11.6.6, each reported where it can change a pixel. Six rounds closed five constructions somebody had priced (ADRs 0217, 0220, 0234, 0237, 0262, 0263, 0272, 0274, 0275, 0276, 0277) and one read §11.4.6 for *which* backdrop it hands each element (ADR 0307); what stands wants a *second* colour space rather than a second direction, plus the one knockout group whose elements blend against a backdrop that is not transparent |
| [25](25-view-dependent-annotations.md) | `/FixedPrint`, which waits on a printing path |
| [26](26-icons-a-clause-only-recommends.md) | `Stamp`'s icon, whose standard names are legends rather than symbols |
| [27](27-the-interfaces-own-font.md) | The interface's own font: the face reaches every script it carries now; what is open is a script it does not |
| [28](28-a-catalogue-that-draws-nothing.md) | A catalogue that drew nothing — **nothing is left to build and the whole file is one run**: every departure it printed is expected to be gone, and only the owner has the document |
| [30](30-a-native-host.md) | All three native hosts are built (ADRs 0244, 0246, 0247) and none needed a new message. **The remaining surface was taken in the five-hundred-and-eleventh** (ADR 0346): the C ABI's entry points are the whole vocabulary, Table 229 bit 26 is obeyed, and the scale a form host draws at is answered with the messages that existed. What is left is two tails — Qt measuring its controls on the far side of the `cxx` bridge, and §12.7.5.4's list box drawing nothing |
| [31](31-accessibility-host.md) | AccessKit over `Query::AccessibilityTree` — built and read back off a real AT-SPI bus (ADRs 0214, 0300, 0301, 0312), with a `TH`'s axis, Table 379's `/BBox`, a cell's `/Headers` and the empty answer a large tagged document's later pages got (ADR 0325) each closed on one; what is left is the elements a `/BBox` does not reach, a `Form`'s control role, the `Text` interface, actions, and what the question costs on a thousand-page document |
| [32](32-presentation-player.md) | Table 164's transition styles: the ones the table's own words determine are drawn (ADR 0230), the rest are reported by name. §12.4.4.2's states are walked (ADR 0316); what is left is a window with no chrome |
| [33](33-annotation-editing.md) | Editing a free text annotation the *file* states — built, with Table 167's `LockedContents` read through the restriction policy (ADR 0304), and Table 177's `/CL` callout line drawn on hand-built pairs because no corpus document states one (ADR 0329). What is left is `/BS`'s border, whose colour no clause states and whose *default* fires on every annotation, and bit 8's `Locked`, which waits on deleting or moving an annotation |
| [34](34-sandbox-the-interpreter.md) | Confine the interpreter and rasteriser — built, drawing real pages behind seccomp, answering every question and stoppable (ADRs 0218, 0223, 0241). What is left is the window, which is tier 2 |
| [35](35-confinement-off-linux.md) | Confinement on macOS and Windows — what the snapshot release cannot ship, and the three ways out |
| [36](36-a-retrieval-api.md) | Retrieve the standard from the standard. The CLI is built and all three joins are closed (ADR 0257); what is left is one `Query` variant for a page's text on the confined pipe, and the substitution itself |
| [38](38-a-documents-restrictions-have-levels.md) | A document's restrictions are the reader's to set: the shape is built and Table 22 is consulted (ADR 0212); *ask* and *warn* wait for a host that can answer |
| [39](39-a-fragment-that-says-where-to-open.md) | Annex O's fragment identifiers — `tools/state.sh annex-o` says which are carried out and which are reported, and this file says what each of the rest needs: a fetch, todo 38's *ask* level, and a concept the vocabulary lacks (ADRs 0209, 0250) |
| [40](40-mask-chain-crop.md) | A clip chain as one crop and one intersect — unblocked and re-priced (ADR 0236). The arithmetic over the target-sized buffers is gone (ADR 0271) and the copying went in the four-hundred-and-ninety-third (ADR 0328, byte-identical); what is left is the chain itself, whose exactness question ADR 0219 still owns |
| [41](41-decoded-stream-cache.md) | A decoded-stream cache: priced at a fraction of a percent on a corpus walked one page a document, taken on the population a reader is in (ADR 0317); a memoised *refusal* is what is left |
| [42](42-the-launch-path.md) | The launch path: four of five items closed, the fifth is quorra's |
| [43](43-the-projects-own-turnaround.md) | The *project's* performance rather than the program's: §5's fat link is what is left and is still unmeasured against what it buys, caching our own renders is priced and refused, and the "three gates that doubled" was bisected to one gate and a non-gate test sharing its binary (ADR 0282) |
| [44](44-a-draft-that-takes-ten-seconds.md) | A draft the owner supplied that takes ten seconds to appear and a third of a second per frame — evaluation owed, starting with the launch-table hole `--trace` must learn to name |
| [45](45-where-a-frame-goes.md) | Where a frame goes, once the instrument could attribute one (ADRs 0227, 0228): the reduced raster recomputed on every redraw is kept in the window's backend now (ADR 0297); quorra's `encode`, the other two backends, and no run on the owner's own machine are what is left |
| [46](46-a-region-of-a-huge-image.md) | A region of a huge image: reduced resolution answers *zoomed out*, so *zoomed in* needs code-block skipping and a bounded IDWT — census first |
| [48](48-the-specification-we-check-against.md) | The specification we check against: `doc/md/` dropped every annotation and the annotations are the **errata** (ADRs 0252, 0253, 0254, 0255). The sixth population — every quotation of the standard in the Markdown this project wrote — **is read since the four-hundred-and-seventy-fourth** and its first run produced thirteen corrections (ADR 0309). What is left: §8.9.5.4, §14.8.6.3's enclosure requirement, and single-quoted ledger spans |
| [49](49-restrictions-worth-re-examining.md) | Which restrictions are load-bearing and which are habit — four of five settled (ADRs 0256, 0257, 0260, 0271); item 3, the API that hands a thread pool in, is what is left |
| [51](51-signatures-and-public-keys.md) | A signature's three questions: the first two are answered for RSA and DSA (ADRs 0215, 0229, 0314). What is left of the second is `id-RSASSA-PSS`, Table 260's ECDSA with the EdDSA and SHA-3 the Technical Specifications add, and the four digests with them; then the signer's *trustworthiness* — a certificate store and a network — public-key handlers, and `/R` 5 |
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
