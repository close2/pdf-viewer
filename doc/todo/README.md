# The todo directory

One file per piece of owed work. `doc/state-of-play.md` says what the program *is*; these say what
it still owes, and they hold the detail that used to make that file long — the evidence, the
clause reading, the measurement and the cost of each item live with the item.

**The index below is one line per item, and that is deliberate.** Each file's own header block
carries its status, its corpus witnesses, its clauses and its code, and *that* is the authority:
a summary here that restates them is a second copy to keep in sync, which is how this index came
to say a clause was ignored eighty-seven rounds after it was implemented. Read the line to choose
an item; open the file to take it.

## Sorting

The number prefix **is** the priority wherever the band has room, and `ls` sorts by it:

| band | means |
|---|---|
| `00`–`09` | **standing** — take some of this every round, whatever else is on |
| `10`–`19` | **defects**: wrong pixels or wrong output, with a diagnosis and usually a fix already argued |
| `20`–`29` | **owed features with corpus demand** — a real document asks and we refuse |
| `30`–`39` | **capability** — things the program cannot do at all, mostly hosts and interfaces |
| `40`–`49` | **performance**, each already measured and priced |
| `50`–`59` | **blocked** on a dependency, a decision or an infrastructure this program does not have |
| `_`-prefixed | not a todo: shared background several items refer to |

**A band holds ten numbers and can fill, and then the prefix stops being the priority** — the
header block's `Priority:` line takes over, and `ls` sorts those items last rather than by
priority. `60`, `61` and `62` are 50-band items and `63` is a 30-band one, each because its band
was full when the file was written. The sentence above carried no such qualification and had been
wrong since the first of the four (ADR 0983).

**A number a deleted item used to have is not free.** A todo file is deleted when its item is
done, and the ADRs that argued it go on citing `doc/todo/NN` — so writing a new file at that
number makes every one of those citations resolve, silently, to an item about something else.
`--bin pointers` sees it only as *findings disappearing*, which is the shape nobody reads as a
regression. Grep `doc/` and `crates/` for the number before naming the file; the
seven-hundred-and-ninety-sixth session took `17` and had to give it back (ADR 0730).

**A number two live files share is the same defect without a deletion in front of it**, and it is
worse: both files exist, so every pointer is *live* and `--bin pointers` cannot see it at all —
where a deleted number at least moves the count of findings. `ls doc/todo/ | grep -oE '^[0-9]+' |
sort | uniq -d` is the sweep, and what it prints today is the number to trust: the round that found
these found two of the three by reading and the third only once that command was run (ADR 0974's
postscript), which is why the command is written here rather than its answer.

**`36` was resolved and `46` and `47` were not** (ADR 0983). `36` was both a retrieval API and a
frame cadence; the grep across `doc/` and `crates/` settled it, because nearly every citation of
`doc/todo/36` — and every one written from `crates/` — is about the cadence. The frame item keeps
the number and the retrieval API is [`63`](63-a-retrieval-api.md), which says in its own header what
it used to be called.

**`46` and `47` keep their duplicates, because the rename costs more than the ambiguity.** All four
files are cited **by full filename** from `doc/adr/` and `doc/history/`, and those are not edited to
follow a file that moved underneath them (ADR 0232 §2) — so moving either half turns live pointers
into absent ones in files nothing may repair. What is available instead is a disambiguation by
subject, and the subjects are disjoint:

| a citation of | about | means |
|---|---|---|
| `doc/todo/46` | the compute kernels, the zoom step's floor, flatten-from-quadratics | [`46-the-kernel-floor.md`](46-the-kernel-floor.md) |
| `doc/todo/46` | §12.5.3, the annotation pass, the seam in `content.rs` | [`46-a-wheel-tick-that-interprets.md`](46-a-wheel-tick-that-interprets.md) |
| `doc/todo/47` | `encode`, the record-replay seam, host work per zoom step | [`47-the-encode-term.md`](47-the-encode-term.md) |
| `doc/todo/47` | a window drag, `WindowEvent::Resized`, the two arms of a resize | [`47-the-resize-frames.md`](47-the-resize-frames.md) |

**And each of the three numbers has a *third* referent that no longer exists**, which is the
paragraph above happening rather than this one: a `doc/todo/47` written before ADR 0335 is the cold
document-wide search that ADR deleted, a `doc/todo/46` in ADR 0373 is the region item it deleted,
and a `doc/todo/36` in ADRs 0200 and 0202 is an item about a collection's ordering.
`grep -rn 'todo/36\|todo/4[67]' doc/ crates/` is the population; the sentence around the citation is
what resolves it.

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
| [00](00-ambiguous-bucket.md) | Empty the oracle's ambiguous bucket — emptied, so the item is the ratchet plus step 7's ink sweep, which a round that changes pixels re-runs. All seven of the gate's verdicts are held by name |
| [01](01-ledger-partial-rows.md) | Read the ledger's `partial` rows against the code — and the sweeps, which that file catalogues and counts, each with an ordinal and none with a superlative |
| [02](02-every-round.md) | **What every round does** — the gates, the sweeps, the binaries, the commit |
| [03](03-more-corpora.md) | More corpora, the fetcher and the survey. What is left is *taking* a chunk a round and the oracle over the new corpora; the rule for adding one is the owner's — include unless a licence clearly forbids |
| [05](05-an-instrument-for-the-interactive-surface.md) | An instrument for the interactive surface — all three are built and two of them gate; what is left is the save round-trip's ratchet and each item's named remainder |
| [10](10-bounds-that-cap-size.md) | Bounds that cap size rather than guard against a bomb, asked for by the project owner. The three defects owed on any road are carried out; what is open is which of the four roads to take, with each road's price, and that choice is the owner's |
| [11](11-shapes-that-still-disappear.md) | Shapes that still disappear. Most of the file is closed, item 8 — a mark under a matrix with no inverse — with it; what is left is `tiny-skia` multiplying the finished mask into the mark's own coverage, what an eight-bit raster does to ink under one of its levels, and two marks abutting anywhere |
| [12](12-one-bound-two-jobs.md) | One bound doing two jobs — **answered in both halves and nothing here is owed**: the floor is derived, implemented and priced, and the two knobs turn out to be one knob, which the gate re-runs as a counterfactual every run |
| [13](13-the-transfer-function.md) | §10.5 — applied to solid colours, image samples and every colour a shading makes, and read from a halftone dictionary's `TransferFunction` besides, so the clause is `implemented`. What is left is §11.7.5.2's per-region model, reported and priced in the file |
| [14](14-stream-the-decompression.md) | **Road D — done**, first of the three the owner ordered out of `10`: all five of §7.8.2's content streams are read through a window and all five of §7.4's byte filters pump in a chain. The file is kept because fourteen comments point a reader at it and `01`'s sweeps read it |
| [15](15-ship-the-confinement.md) | **Road B**, second: ship the confinement and let the kernel hold the bound. The tier change, the stoppable draw, the abandonment policy and the first host on the boundary are built; what is left is moving the three established windows onto it, and the owner's warn-then-abort as an input they share |
| [16](16-resumable-interpretation.md) | **Road C**, third: interpretation as a resumable job the host pumps — the only always-interruptible road, a state-machine rewrite against the oracle's pages, and it contains road A |
| [21](21-font-substitution.md) | `Identity` orderings; per-character fallback, with no witness left |
| [22](22-variable-text-edges.md) | §12.7.4.3's remaining edges: the Arabic free text value, refused whole — read, priced and pinned |
| [23](23-transparency-departures.md) | §11.4, §11.4.7 and §11.6.6, each reported where it can change a pixel. What stands wants a conversion between **two presses** per pixel at a group boundary, which is a function rather than a quantity and which no corpus document asks for |
| [25](25-view-dependent-annotations.md) | `/FixedPrint`, reported and owed — a `shall` on rendering, not on printing |
| [26](26-icons-a-clause-only-recommends.md) | `Stamp`'s icon, whose standard names are legends rather than symbols |
| [27](27-the-interfaces-own-font.md) | The interface's own font: the face reaches every script it carries now; what is open is a script it does not |
| [28](28-a-catalogue-that-draws-nothing.md) | A catalogue that drew nothing — **nothing is left to build and the whole file is one run**: every departure it printed is expected to be gone, and only the owner has the document |
| [30](30-a-native-host.md) | All three native hosts are built and none needed a new message; the C ABI's entry points are the whole vocabulary. What is left is `viewer-gtk` obeying Table 234's `/TI`, on a binding floor rather than a decision, and the colour of the gap between two pages |
| [31](31-accessibility-host.md) | AccessKit over `Query::AccessibilityTree` — built and read back off a real AT-SPI bus, with the actions a client may request, §14.8.3.3's content rectangle and §14.8.5.4.5's other two derivations closed; what is left is four questions for the platform |
| [32](32-presentation-player.md) | Table 164's transition styles: the ones the table's own words determine are drawn, the rest are reported by name, and §12.4.4.2's states are walked; what is left is a window with no chrome |
| [33](33-annotation-editing.md) | Editing a free text annotation the *file* states — built, with `LockedContents` read through the restriction policy, Table 177's `/CL` callout line drawn and §12.5.4's border with it. What is left is bit 8's `Locked`, which waits on deleting or moving an annotation |
| [34](34-sandbox-the-interpreter.md) | Confine the interpreter and rasteriser — built, drawing real pages behind seccomp, answering every question, stoppable, and shipping marks to a host that holds the device. What the marks arm leaves a host is in [15](15-ship-the-confinement.md) |
| [35](35-confinement-off-linux.md) | Confinement on macOS and Windows — what the snapshot release cannot ship, and the three ways out |
| [36](36-a-frame-every-refresh.md) | A frame every refresh — the owner wants 60 Hz as the floor and 120 Hz as the target, reprojecting when a correct frame is missed and re-basing on a late one; the unsettled half is where the pixels come from in 8.3 ms, which is probably an ask to quorra |
| [37](37-a-frame-that-says-it-is-stale.md) | A frame that says it is stale — built for both windows and the same arrangement on both, with each of the five rules enforced by a test, a type or the structure. Nothing this item names is owed |
| [38](38-a-documents-restrictions-have-levels.md) | A document's restrictions are the reader's to set: every Table 22 bit named, five operations, the four levels and the verdict in one module asked once, with the events a window receives for *ask* and *warn*; what is left is a way for a person to choose a level and a dialogue to answer the question with |
| [39](39-a-fragment-that-says-where-to-open.md) | Annex O's fragment identifiers — `tools/state.sh annex-o` says which are carried out and which are reported, and nothing here restates its answer. The file is kept as the reading beside that command, and the two limits it names are not this annex's |
| [40](40-mask-chain-crop.md) | A clip chain as one crop and one intersect — unblocked and re-priced, with the copying and the chain step that admits every pixel of its band both taken. What is left is the chain itself, now a priced choice rather than an open question |
| [41](41-decoded-stream-cache.md) | A decoded-stream cache — taken on the population a reader is in, with the memoised *refusal* and `image_stream` beside it. What is left is a refusal whose **encoded** bytes exceed the budget, and `NestedContent::damage`, which still pumps to the end once per read |
| [42](42-the-launch-path.md) | The launch path: four of five items closed, the fifth is quorra's |
| [43](43-the-projects-own-turnaround.md) | The *project's* performance rather than the program's: what a round reads and which gates it chooses, both taken; §5's fat link measured against what it buys and kept |
| [44](44-a-draft-that-takes-ten-seconds.md) | A draft the owner supplied that **took** ten seconds to appear and now takes one and a half, every phase of it named. What is left is one upstream ask and §3.1's page-space scene, which this document does not exercise |
| [45](45-where-a-frame-goes.md) | Where a frame goes, once the instrument could attribute one: quorra's `encode` (theirs), the other two backends' copy of the reduced-raster cache — which wants a measurement in the *confined host* first — the owner's own windowed run, and §5's fit against covered pixels rather than commands |
| [46](46-the-kernel-floor.md) | The kernel floor: the zoom step *is* the compute kernels, and after three experiments declined by number the floor is the serial per-tile arena walk and the edge traffic, not register occupancy. What remains with the step's magnitude are quorra-side designs, each owing its own argument |
| [46](46-a-wheel-tick-that-interprets.md) | §12.5.3's re-interpretation on the event thread, on a `NoZoom` document. The shape was chosen by measurement and the seam is built — the annotation pass is a tail on every accumulator, so the state at that seam is kept and the pass re-run instead of the page. What is left is the clause's own unattributed pass, and whether it is worth taking |
| [47](47-the-encode-term.md) | The encode term: host work per zoom step, deliberately deferred with a stated revisit condition — take it up when a kernel-floor round makes encode the largest term, **measured on the compute lane the window actually takes for the gesture**. Successor to `45`'s encode item |
| [47](47-the-resize-frames.md) | The resize frames, **attributed**: interpretation is not on the path, and only the arm where the page's raster follows the window has a cost — which is the compute kernels under [`46`](46-the-kernel-floor.md) with the encode term under [`47`](47-the-encode-term.md). What is left is one decision about §14.7's tree, republished on every step of a drag |
| [48](48-the-specification-we-check-against.md) | The specification we check against: `doc/md/` dropped every annotation and the annotations are the **errata**. The sixth population — every quotation of the standard in the Markdown this project wrote — is read; what the file still owes is its own steps 4 and 5, the disagreement sweep and the substrate question |
| [49](49-restrictions-worth-re-examining.md) | Which restrictions are load-bearing and which are habit — four of five settled, and `MAX_FORM_DEPTH` decided as a stack figure; item 3, the API that hands a thread pool in, is what is left, plus `MAX_PRESSES`, now per-interpretation |
| [50](50-the-windows-dx12-retest.md) | The Windows DX12 retest — the owner's first traces arrived and half the file is answered. The file says exactly what to run next, including the `--coverage cpu` A/B only that machine can take |
| [51](51-signatures-and-public-keys.md) | A signature's three questions: the first two are answered for **every algorithm family the standard names**, and every digest either table names is computed. What is left of the second is four **curves**, each refused by package availability and named by its own identifier; then the signer's *trustworthiness* — a certificate store and a network — and §7.6.5's public-key handlers |
| [52](52-zlib-rs-deallocates-through-the-wrong-pointer.md) | `zlib-rs` fails both of Miri's aliasing models — an upstream report to write |
| [53](53-what-hayros-tracker-asked.md) | Two residues left of reading hayro's tracker against this tree: a digit run that swallows an operator and reports nothing, and a Type 1 program whose unassigned codes claim glyph 0. Neither is witnessed by a corpus document, and each says what would change that |
| [55](55-a-filter-that-mixes-in-black.md) | The **shipped** backend's image filter blends straight-alpha texels, so every partly covered edge of an image carries the black its transparent samples are stored with — §8.9.6.2's interpolation `shall` said in as many words, and the two backends that meet it are gated. Nothing here can fix it and the ask is written (`doc/QUORRA_FEEDBACK.md` §39) |
| [56](56-a-script-engine-that-is-memory-safe.md) | §12.6.4.17's two `shall`s, and whether the exclusion that closes them has outlived its reason. Researched and measured on both denominators, with the engine grading, the placement argument, the smallest first step and the amendment the owner would have to ratify — **blocked on that decision and on nothing else** |
| [57](57-the-transform-suite.md) | **The transform suite** — RFC 0002's stream. The serializer, **all five writing verbs** (`attach`, `split`, `merge`, `pages`, `optimize`) and §14.7's carried structure tree are done, each with its own corpus walk in `doc/todo/02` §2. What is left is three things and two of them are the owner's; the file says which |
| [58](58-the-file-system-faces.md) | **The file-system faces** — RFC 0003's stream: the shared core, the confined worker, the write side, the `pdffs` mount, the KIO face with its question channel and both corpus walks have all landed. What is left is a decision the owner has not been asked, the KIO face's own first use, and the one departure from the approved §4 the owner should overrule or ratify |
| [59](59-the-resource-port.md) | **The resource port** — what a confined worker may be *given*, and by whom. Built for its first resource: the worker asks by description, the broker matches and answers, no path crosses either way, and every host withholds it by default. What is left is the other resources (ICC profiles first), a way for a person to choose, and `CLAUDE.md` principle 3's amendment, which is `doc/questions/Q24` |
| [60](60-paths-a-document-names.md) | **Paths a document names** — §7.11.3's `/F` and §7.3.8.2's external streams, which are the *other* half of the resource question and are the half that must **ask**: a resource the document names is not a resource the reader's machine offers, so [59](59-the-resource-port.md)'s port is the wrong shape for it and `doc/todo/38`'s four levels are the right one. Accepted, not started |
| [61](61-what-a-library-asks-the-machine.md) | **What a library asks the machine** — confined-worker deaths on a system call no document caused, and the rule that keeps them from becoming permissions: a *probe* is answered before the lockdown, while a **precondition** the standard library checks on a resource the worker was *given* is not a probe. `doc/questions/Q26` asks whether the second answer was a round's to give |
| [62](62-the-exemption-no-row-states.md) | **The exemption no row states** — both target parts of ISO 19005 exempt a named resource whose name the associated content stream never references, and `pdf-archive` states neither. Measured rather than estimated, and the exemption moves the corpus nothing. Blocked on a reachability answer `Examination` does not have |
| [63](63-a-retrieval-api.md) | Retrieve the standard from the standard — a 30-band item, and `doc/todo/36` until ADR 0983. The CLI is built and all three joins are closed; what is left is one `Query` variant for a page's text on the confined pipe, and the substitution itself |
| — | [`_scan-conversion.md`](_scan-conversion.md) — shared: §10.7.4, what this tree departs from and why |
| — | [`_image-codecs-and-the-sandbox.md`](_image-codecs-and-the-sandbox.md) — shared: the three sandboxed codecs are already pure safe Rust, what the sandbox is really for, and what a subset would and would not buy |

**What is not implemented has a file, and that is what this index is.** Every one of them is
*reported* at runtime rather than silently skipped; the corpus witnesses, the clause and what it
would cost live with the item rather than here.

**Closed by decision rather than by work** — recorded in `doc/conformance/ledger.toml` and not
here: `/ColorTransform` (Table 13, whose one corpus witness contradicts the clause), a stream
whose data is in an external file (§7.3.8.1 — the renderer has no filesystem, principle 3),
§12.7.6.2's submit and §12.6.4's remote, launch, sound and movie actions (a network, a second
file, a media engine), a filled degenerate subpath's device pixel (§8.5.3.3.1, which the clause
itself calls "device-dependent and not generally useful"), grid-fitting a stroke's
coordinates under `/SA` (see `_scan-conversion.md`), rendering intents beyond
`AbsoluteColorimetric`, and **a glyph a document's own embedded subset does not contain**, which
was traced to the end of every route the standard states: §9.7.4.2's and §9.6.5.4's rows carry
the evidence, and `poppler` draws such glyphs from a face this machine has, which is a fallback
rather than a reading. **The two halves of that last one are two different refusals** (ADR
0270): a `loca` entry that is empty *is* the program's statement that the code makes no mark, and
`Interpretation::codes_reaching_a_blank_glyph` counts it apart from a code that reaches nothing
at all.
