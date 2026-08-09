# The todo directory

One file per piece of owed work. `doc/HANDOVER.md` says what the project *is*; these say what
it still owes, and they hold the detail that used to make that file long — the evidence, the
clause reading, the measurement and the cost of each item live with the item.

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

| | item | corpus |
|---|---|---|
| [00](00-ambiguous-bucket.md) | Empty the oracle's ambiguous bucket, page by page | 5 names |
| [01](01-ledger-partial-rows.md) | Read the ledger's 240 `partial` rows against the code — and the seven sweeps | — |
| [02](02-every-round.md) | **What every round does** — the gates, the sweeps, the binaries | — |
| [11](11-shapes-that-still-disappear.md) | **The sub-pixel fill and the sub-pixel stroke are closed** (ADR 0226); what is left is a sub-pixel rule that is *diagonal*, and two marks abutting across a cell's box edge with no witness | 1 |
| [13](13-the-transfer-function.md) | §10.5's `/TR` decides what a screen shows, and this tree ignores it — the ledger's first `silent` row since session 35, and a question for the owner | 1 draws wrong |
| [21](21-font-substitution.md) | `Identity` orderings; a font reported as a whole; per-character fallback, with no witness left | 40 |
| [22](22-variable-text-edges.md) | §12.7.4.3's remaining edges: a `/DA` font `/DR` lacks and cannot spell, a composite `/DA` font with no witness. The baseline's guard and the list box's cost both closed in the four-hundred-and-third (ADR 0240), the value a host could not read back closed twice over in the four-hundred-and-eleventh (ADR 0247) — the note had been stale for sixty sessions, and the *password* half of it was a live bug — and the list box's *host* half closed in the four-hundred-and-twelfth (ADR 0248), leaving the page's refusal exactly where the argument put it | 1 |
| [23](23-transparency-departures.md) | §11.4 and §11.6.6, each reported where it can change a pixel — **§11.5.3's population is closed** (ADRs 0217, 0220), **§11.4.6's shape** (ADR 0234) and **§11.4.4's non-isolated group** (ADR 0237), leaving §11.6.6's blending space for a painted group and, inside §11.4.6, a knockout group whose elements blend | 6 |
| [24](24-image-sampling-intent.md) | Carry an image *and its sampling intent* to the backends — **the vocabulary is built and the mask is on it** (ADR 0210); JPEG 2000's resolution level is written, measured and waiting on one push to `close2/hayro` (ADR 0233), and `render-gpu`'s sampled shading is the rest | 1 |
| [25](25-view-dependent-annotations.md) | `/FixedPrint`, which waits on a printing path | 15 |
| [26](26-icons-a-clause-only-recommends.md) | `Stamp`'s icon, whose standard names are legends rather than symbols | 1 |
| [27](27-the-interfaces-own-font.md) | The interface's own font: a character it cannot set is a box now; what is open is *coverage* | 74 |
| [28](28-a-catalogue-that-draws-nothing.md) | A catalogue that drew nothing: the YCCK image is decoded, §11.5.3's masks are finished (ADRs 0217, 0220), §11.4.6's knockout draws (ADR 0234) and §11.4.4's non-isolated groups do (ADR 0237); **§11.4.7's blending space is all that is left**, and one run over the file is a check rather than a question | 0, and that is the point |
| [30](30-a-native-host.md) | **All three hosts are built** — GTK4 (ADR 0244), Qt 6 through a C++ bridge (ADR 0246) and **the C ABI** (`crates/viewer-ffi`, ADR 0247), whose three amendments were taken first and one of which was a live bug rather than the doc sentence this file predicted. **No host added a message, three running.** What is left is *surface*: 39 entry points is not the whole vocabulary, and every missing one is a symbol that costs a compiled C caller nothing. **§12.7.5.4's list box — the one gap three hosts found — closed in the four-hundred-and-twelfth** (ADR 0248): `Edit::SetField` carries a set of Table 234 `/Opt` indices, both native hosts offer real multiple selection, and no *message* was added. **43 entry points since the four-hundred-and-fourteenth**, and `PDFV_EVENT_KIND_COUNT` moved 15 → 16 for the first time: a find bar in all three hosts needed `Command::Find` and `Event::Searched` (ADR 0250). What is left is the scale a form host draws at and `RadiosInUnison` | 102 of 964 documents have a delegable widget on page one |
| [31](31-accessibility-host.md) | AccessKit over `Query::AccessibilityTree` — **built and read back off a real AT-SPI bus** (ADR 0214); a `TH`'s axis, a `Form`'s control role, the `Text` interface and actions are what is left | — |
| [32](32-presentation-player.md) | **Seven of Table 164's twelve styles are drawn** (ADR 0230); `Blinds`, `Glitter`, `Dissolve` and `Fly` are reported by name, and §12.4.4.2's states have no control | — |
| [33](33-annotation-editing.md) | Editing a free text annotation the *file* states, and Table 177's `/CL` callout line — §12.5.6.10's markup, §14.8.2.5's logical order, the caret, the click that places it, the selection inside a value and §12.5.6.6's free text a person draws and types into are done | — |
| [34](34-sandbox-the-interpreter.md) | Confine the interpreter and rasteriser — **built, drawing real pages behind seccomp, answering all twenty-five questions and now stoppable** (ADRs 0218, 0223, 0241): a cancel is a kill, because a cancel a hostile document can decline is not one, and a 1567-byte document that draws for 44 s stops in 1.5 ms. What is left is the window, which is tier 2; items 4 and 5 are repriced with measurements and both need a decision this round refused to take blind | — |
| [35](35-confinement-off-linux.md) | Confinement on macOS and Windows — what the snapshot release cannot ship, and the three ways out | — |
| [38](38-a-documents-restrictions-have-levels.md) | A document's restrictions are the reader's to set: **the shape is built and Table 22 is consulted at last** (ADR 0212); *ask* and *warn* wait for a host that can answer, and bit 5's copy for a host that can name it | 7 assert something |
| [39](39-a-fragment-that-says-where-to-open.md) | Annex O's fragment identifiers: **eight of eleven carried out**, three reported by name. **`search` came off that list in the four-hundred-and-fourteenth** — `Command::Find` is a document-wide search, started as the document opens and walked one page per step by the host (ADR 0250) — and what is left is `fdf`, a fetch; `ef`, todo 38's *ask* level, the one part of it ADR 0212 deliberately did not build; and `highlight`, a concept the vocabulary lacks | 0, and no file could ever have one |
| [40](40-mask-chain-crop.md) | A clip chain as one crop and one intersect — **unblocked and re-priced** (ADR 0236): worth 42% of `MaskCache::get` rather than most of it, and the memory that blocked it was 27.9 MB in session 113 and is 12.31 today. The worst page halved this round for a different reason — 3490 `sh` operators drawn page-wide and kept in 24 pixels each — and what is left to settle is whether a parent's mask rows are *bit*-exact in a child's band, which ADR 0219 says they are not | 1 |
| [41](41-decoded-stream-cache.md) | 0.7% of interpretation, priced and not taken | — |
| [42](42-the-launch-path.md) | The launch path: 145 ms to 110, four of five items closed, the fifth is quorra's | all |
| [43](43-the-projects-own-turnaround.md) | **The *project's* performance rather than the program's**: a round was 608 s and is 268; §5's fat link is what is left, and caching our own renders is priced and refused | — |
| [45](45-where-a-frame-goes.md) | **What the instrument found once it could attribute a frame** (ADR 0227 closed todo 44). Two of its four items are closed by ADR 0228 — the 2 ms page-turn publication was two X11 round trips for a window position, and the bimodal translation was one image reduction paid per source sample. What is left: quorra's `encode`, 45% of a page turn and reported upstream as `QUORRA_FEEDBACK.md` §13; a reduced raster still recomputed on every redraw, wanting a witness; and no run on the owner's own machine | — |
| [46](46-a-region-of-a-huge-image.md) | Reduced resolution answers *zoomed out*; the witness is one tile with no precinct partition, so *zoomed in* needs code-block skipping and a bounded IDWT — census first | 1 |
| [51](51-signatures-and-public-keys.md) | A signature's three questions: **the first two are answered** — has the document changed since it was signed (ADR 0215), and does the signature verify under the RSA key in the certificate the file carries (ADR 0229). What is left is Table 260's DSA and ECDSA, both named at runtime, and the signer's *trustworthiness*, which is a certificate store and a network; public-key handlers; `/R` 5 | 10 signatures, 4 changed, **10 verify** |
| [52](52-zlib-rs-deallocates-through-the-wrong-pointer.md) | `zlib-rs` fails both of Miri's aliasing models — an upstream report to write | — |
| — | [`_scan-conversion.md`](_scan-conversion.md) | shared: §10.7.4, what this tree departs from and why |
| — | [`_image-codecs-and-the-sandbox.md`](_image-codecs-and-the-sandbox.md) | shared: the three sandboxed codecs are already pure safe Rust — what the sandbox is really for, corpus demand measured, and what a subset would and would not buy |

**Closed by decision rather than by work** — recorded in `doc/conformance/ledger.toml` and not
here: `/ColorTransform` (Table 13, whose one corpus witness contradicts the clause), a stream
whose data is in an external file (§7.3.8.1 — the renderer has no filesystem, principle 3),
§12.7.6.2's submit and §12.6.4's remote, launch, sound and movie actions (a network, a second
file, a media engine), a filled degenerate subpath's device pixel (§8.5.3.3.1, which the clause
itself calls "device-dependent and not generally useful"), and grid-fitting a stroke's
coordinates under `/SA` (see `_scan-conversion.md`).
