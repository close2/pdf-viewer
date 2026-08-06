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
| [00](00-ambiguous-bucket.md) | Empty the oracle's ambiguous bucket, page by page | 8 names |
| [01](01-ledger-partial-rows.md) | Read the ledger's 240 `partial` rows against the code — and the seven sweeps | — |
| [02](02-every-round.md) | **What every round does** — the gates, the sweeps, the binaries | — |
| [11](11-shapes-that-still-disappear.md) | A fill under an eighth of a pixel; a tiling cell's two halves; a hairline at the raster's edge | 4 |
| [13](13-the-transfer-function.md) | §10.5's `/TR` decides what a screen shows, and this tree ignores it — the ledger's first `silent` row since session 35, and a question for the owner | 1 draws wrong |
| [14](14-selection-over-a-bad-ocr-font.md) | Selection over a badly built OCR font: the vertical extent is invented and its guard only checks ordering — and no gate here can see it. Raised by the owner | unmeasured |
| [21](21-font-substitution.md) | `Identity` orderings; a font reported as a whole; per-character fallback, with no witness left | 40 |
| [22](22-variable-text-edges.md) | §12.7.4.3's remaining edges: a `/DA` font `/DR` lacks, a list box, and Table 231's `DoNotScroll` — a `shall` | 3 |
| [23](23-transparency-departures.md) | §11.4, §11.5.3 and §11.6.6, each reported where it can change a pixel | 19 |
| [24](24-image-sampling-intent.md) | Carry an image *and its sampling intent* to the backends | 3 |
| [25](25-view-dependent-annotations.md) | `/FixedPrint`, which waits on a printing path | 15 |
| [26](26-icons-a-clause-only-recommends.md) | `Stamp`'s icon, whose standard names are legends rather than symbols | 1 |
| [27](27-the-interfaces-own-font.md) | The interface's own font: a character it cannot set is a box now; what is open is *coverage* | 74 |
| [28](28-a-catalogue-that-draws-nothing.md) | A catalogue that drew nothing: the YCCK image is decoded; §11.6.6 and §11.4.7 are what is left, and this document is their witness | 0, and that is the point |
| [30](30-a-native-host.md) | GTK4, then Qt, then `viewer-ffi` | — |
| [31](31-accessibility-host.md) | AccessKit over `Query::AccessibilityTree` | — |
| [32](32-presentation-player.md) | Draw a transition's frames | — |
| [33](33-annotation-editing.md) | A caret, and free text — §12.5.6.10's markup and §14.8.2.5's logical order are done | — |
| [34](34-sandbox-the-interpreter.md) | Confine the interpreter and rasteriser, not only the codecs | — |
| [35](35-confinement-off-linux.md) | Confinement on macOS and Windows — what the snapshot release cannot ship, and the three ways out | — |
| [37](37-what-a-native-host-would-not-draw-itself.md) | Five of six chrome populations already cross as data; **form fields are the gap** — audited at the owner's request | — |
| [38](38-a-documents-restrictions-have-levels.md) | A document's restrictions are the reader's to set: four levels, no UI yet, and the shape they need | — |
| [39](39-a-fragment-that-says-where-to-open.md) | Annex O's fragment identifiers: **seven of eleven carried out**, four reported by name — `search` wants a document-wide search, `fdf` a fetch, `ef` todo 38's levels, `highlight` a concept the vocabulary lacks | 0, and no file could ever have one |
| [40](40-mask-chain-crop.md) | A clip chain as one crop and one intersect | 1 |
| [41](41-decoded-stream-cache.md) | 0.7% of interpretation, priced and not taken | — |
| [42](42-the-launch-path.md) | The launch path: 145 ms to 110, four of five items closed, the fifth is quorra's | all |
| [51](51-signatures-and-public-keys.md) | Validation, public-key handlers, `/R` 5 | 1 |
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
