# 560 — A rebuild that reads an object stream's own header

**Finding.** `xref::rebuild` recovers a file by scanning for `N G obj` headers, and §7.5.7's
compressed objects have none — so a document whose cross-reference information could not be read
lost every object a modern producer packed, in silence, wearing the vocabulary §7.5.6 gives a
*deleted* object. §C.4 licenses the reconstruction and says it scans "all the objects in the file";
§7.5.7's first sentence says an object stream holds indirect objects "as an alternative to their
being stored at the outermost PDF file level", which is where the headers are. So this was one
clause implemented halfway, and the clause states the missing step outright: the `N` pairs of
object number and offset each object stream begins with. `Document::recover_compressed_objects`
reads them — **the header only**, so the objects stay unparsed until something asks — and reports
every stream it could not read.

**Date.** 2026-08-17.
**ADR.** [0395](../adr/0395-a-rebuild-that-reads-an-object-streams-own-header.md).
**Touched.** `crates/pdf-syntax/src/xref.rs` (`XrefTable::object_streams`, `enter_compressed`,
`scan_for_objects`, `is_object_stream`), `crates/pdf-syntax/src/document.rs`
(`CompressedRecovery`, `RECOVERY_DECODE_BUDGET`, `recover_compressed_objects`,
`object_stream_members`, `object_stream_pairs`), `crates/pdf-syntax/src/lib.rs`,
`crates/pdf-syntax/tests/cross_references.rs` (three tests, each failing on the tree as it was),
`crates/viewer-core/src/notes.rs` (the open-time sentence) and
`crates/viewer-core/tests/headless.rs` (the fourth test, end to end through the boundary),
`crates/pdf-model/tests/structure.rs`
and `crates/pdf-model/tests/logical_order.rs` (one document joins the tagged population),
`crates/pdf-model/examples/rebuild_census.rs` (new), `doc/conformance/ledger.toml` (§7.5.7),
`doc/todo/17-*` (deleted), `doc/todo/README.md`, `doc/adr/0395-*` (new), this file.

## The two clause questions, and what decided them

- **A number in both places.** §7.5.7: "[i]f either an object stream or a compressed object is
  deleted and the object number is freed, that object number shall be reused only for an ordinary
  (uncompressed) object other than an object stream." So the scan's entry wins and the offer is
  counted rather than dropped. Every collision on this disk — 23 — is named by a stream whose
  decode stops short, so no file here is being decided by that reading.
- **Whether the scan can find the streams at all.** It can, and by the same clause: "[t]he
  following objects shall not be stored in an object stream" heads a list whose first item is a
  stream object, so every object stream is written at the outermost level with a header of its own.

**§C.4 gets no ledger row**, and that is the annex rather than an omission: `NORMATIVE_ANNEXES` is
D, E, F, I, K, L, O and Q, and Annex C's title line says `(informative)`. The requirement side of
this work is §7.5.7's row, which now carries both readings and the population.

## The census

`pdf-model/examples/rebuild_census`, new, over the crawl, the 974 and the four corpora, reading
each stream's `N` pairs itself so that the instrument is not the code under test:

- **261 of 65 944 crawled documents reach a rebuild; 30 of those carry object streams**, 549 of
  them, holding 247 902 objects by their own `/N`.
- **223 661 object numbers were located nowhere; now they are located inside an object stream**,
  and 214 710 resolve. The rest are refused by ADR 0366's damaged-prefix rule and counted by it.
  Documents losing at least one object: **29 → 8**.
- **The affected population surveyed on both arms**, 35 documents: **7 pageless → 4, 17 incomplete
  → 17, 0 unopenable**, nothing moving backwards. The witness draws its *Hello!*.

## The one design decision

Reading the headers rather than expanding the objects, because `CLAUDE.md` forbids eager work and
an entry is made of a number. On the widest rebuilt document here — 10 MB, 316 object streams,
142 641 compressed objects — `Document::open` is 13.0–17.5 ms before, 52.3–75.6 ms as shipped, and
196.5 ms if every stream's objects are parsed at open. The bound is on decoded **bytes**
(`RECOVERY_DECODE_BUDGET`), not on the number of streams, because a kilobyte of headers can name
thousands; 64 MiB against a widest real expansion of 12.6 MiB.

## Every gate

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets`: silent.
- `cargo nextest run --workspace`: 2069 → **2073**, all passing, 15 skipped. The four are the
  recovery, the reuse rule, the loud refusal and the sentence a host is given, each verified to
  fail with the recovery removed.
- `cargo test --workspace --doc`, `cargo test -p conformance`: pass. The conformance gate caught an
  unattributed blockquote in the new code, which is the quotation rule working.
- **corpus**: `974 documents in 2.8s: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 65
  incomplete, 0 slow`, with silence lines at 5 codes over 2 documents and 57 over 9. Unchanged in
  every field.
- **oracle**: `1794 pages in 35.1s (1690 we call complete, 104 incomplete)` — `agrees` 906/862,
  `contradicted` 67/66, `ambiguous` 786/753, our geometry 1/0, reference geometry 2/2, `not
  comparable` 13/7, `no render` 19/0. **No page moves**, 99.8% reference-cache hit rate.
- **text_extraction** (two gates): pdf.js `974 documents … overall 99.2% (22836/23015 words)`, 22
  below 90%; PDFBox `40 documents … 99.8% (14257/14281)` both orders, 4 below 90%. Unmoved.
- **dates**, **xmp**, **jpeg2000**: pass. **render-quorra corpus**: `956 pages compared in 32.7s:
  931 agree, 23 differ, 2 refused, 18 not comparable`, unchanged.
- **`display_list_digest` over all 974 first pages**: **byte-identical**, both arms in one sitting
  with the same worker on disk. No pixels move, so no quorra `gpu` lane and no `doc/todo/00` step 7
  ink sweep were owed — ADR 0379's precedent for the same recovery.
- **Fuzzing**, seeded from the tree's corpora: `object` and `document` at 50 000 runs apiece, no
  crash — `document` is the target that covers §7.5's file structure and therefore this change —
  and `page` with `-fork=6`, `50 593 iterations, oom/timeout/crash: 0/0/0, cov: 31 490`.
  **`page` was run over a 1542-seed sample rather than the whole 39 851**, one seed in
  twenty-six: the first attempt spent fifty minutes in libFuzzer's per-run corpus merge without
  reaching a single iteration, which is exactly what `doc/todo/02` §2 warns of and nobody has yet
  spent the one `cargo fuzz cmin page` that would fix it. The sample is stated rather than glossed.
- **And the merge produced two `timeout-` artefacts that are not this round's.** Both are real
  documents from the seed corpus, both read their **own** cross-reference table — `rebuild_census`
  says 0 rebuilt for each — so nothing this round touched runs on them. Confirmed in a release
  build rather than dismissed as the sanitiser's, which is `doc/todo/02` §2's own instruction: one
  is an iTextSharp timetable whose single page takes **2 m 13 s** and is stopped by
  `MAX_OPERATIONS`, the other **35.6 s** and complete. A page-one interpretation of that length is
  a latency finding for `doc/todo/10` and `16`, recorded here rather than absorbed.

## The two test constants that moved, and why they are a gain

`structure.rs` and `logical_order.rs` count the corpus documents with a `/StructTreeRoot`, and the
count rose by one: `issue17147.pdf`, whose cross-reference stream cannot be decoded and whose
`/StructTreeRoot` is one of the nine objects inside its object stream. Its display list is
byte-identical; what changed is that the document's own statement about itself is reachable.
