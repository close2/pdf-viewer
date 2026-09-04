# 0895 — What the counter found, and the seven walks it does not reach yet

Session 927. Status: **accepted**. The second of this round's two records:
[ADR 0894](0894-a-cost-floor-that-counts-because-a-clock-here-is-a-lottery.md) is the instrument
and the floors; this is what they found the first time they were pointed at the corpus, and what a
count would be worth on each of the other gates.

## 1. It found a defect in the row nobody had looked at since session 899

`meta/xmp.xml` is the one file in RFC 0003 §4's layout whose **existence the document states**
rather than the layout, so it is the one row whose path validation cannot derive its answer and has
to ask the worker. `locate_in`'s check was:

```rust
if route.generator == Generator::MetadataStream
    && !matches!(current.worker.ask(&Query::MetadataStream)?, Answer::Bytes(_))
```

`Query::MetadataStream` fetches §14.3.2's whole decoded stream. So **every listing of `/meta`, every
`stat` and every `open` under it fetched the stream again to decide whether a name exists** — and
`Vfs::entries_of`'s `MetaNames` arm did the same to decide whether to list `xmp.xml` at all.

It is ADR 0886's defect one row along, in a function neither face can see, and every instrument in
front of it was blind for the same reason: the bytes were produced once, so `Vfs::generated` was
right and said nothing. On the five-page annex, nine questions about `/meta` cost **eleven** fetches
of the stream; over the corpus the whole-layout floor named the subject `"metadata"` on the first
document it walked.

**The fix is the one ADR 0886 used, and it is the third note of its kind.** `metadata_stream` is now
the only place that asks: it answers from the cache under the path the bytes belong to, remembers
in the generation that a document states no stream at all, and is what `locate_in`, `entries_of`
and `generate` all call. A validation therefore *warms* the read, exactly as a listing of
`images/NNNN/` does since 0886. One question a generation, and `tests/a_face.rs` holds it there.

## 2. And a shortfall, which is a different thing and is recorded as one

The first corpus run failed on 36 documents, all with one to four repeats out of eleven to nineteen
questions, and the cause was the same everywhere: **a refusal produces no bytes, so there is
nothing for the cache to hold, and the next question about that subject runs the generator again.**
A page whose codec this reader does not have costs its refusal on the `stat` and again on the
`open`.

That is not the defect the floor is for, and counting it as one would make the gate fire on every
document with an unsupported image on page one — a refusal working exactly as trap 5 asks it to. So
`Questions` counts the two separately: `repeated`, which is the floor, and
`reasked_after_a_refusal`, which is printed and recorded in `doc/todo/58` §5 as a shortfall of the
cache rather than a fault of the walk. **Fixing it would mean remembering refusals**, which is a
design question with a real edge — a refusal that depends on a budget is not obviously the same
refusal next time — and this round did not take it on a first sighting.

## 3. The seven other walks, and what a count would be worth on each

The round was asked to say what it left. This is a survey of `doc/todo/02` §2's remaining
corpus-scale gates, read against the same question: **is there an expensive call that could be
counted, and what would the invariant say?** Four of the seven already have a counter in the
library they drive and need no library change at all.

| walk | the counted invariant | what it needs |
|---|---|---|
| `viewer-core` `accessibility_census` | **one interpretation per page visited** — every page is asked `Query::AccessibilityTree` and then `Query::Reports`, and both derive from one interpretation | a counter at `viewer_core::open`'s one interpretation funnel, and an accessor beside `Viewer::readback_cache` |
| `viewer-core` `selection_census` | one page interpretation per document, however many carets are walked — `Viewer::readback_cache` already reports `hits`/`misses`/`evicted` and this test reads neither | nothing: the accessor is public already |
| `pdf-transform` `gate` | the thing its **40 pages a second** is proxying is stated in `render.rs`'s own comment — "one font cache, shared by every page" — so the count is `FontCache::report().misses == the distinct font dictionaries those pages reach`, with `rebound == 0` | a report accessor on the renderer; the subprocess run stays as the pixel gate |
| `pdf-model` `corpus` | one decode per distinct filter chain (`Document::decoded_streams()` — `hits`, `misses`, `evicted`) and one font load per distinct font dictionary | test-only: `interpret_with_fonts` with a locally owned cache makes both readable |
| `oracle`, `text_extraction` | `pdfref`'s caches already report `Statistics { hits, misses, remembered_timeouts }`, and neither gate asserts on them: `misses == 0` warm, `hits + misses == pages × readers` | test-only |
| the six `pdf-transform` walks | one `Document::open` per distinct byte string per operation — every `apply` re-opens every source, and these walks call `apply` several times a document | one global counter in `pdf_syntax::Document::open_with_password`, which would serve seven gates at once |
| `render-quorra` `corpus` | one device rasterise per page — the harness itself draws a differing page twice | a counter inside the backend; the least tractable of the seven |

Two general findings from that survey are worth more than the table:

- **The instrumentation already exists more often than not.** Three of the seven need no library
  change, and a fourth needs one accessor. What is missing is not counters but *assertions on
  them*: `pdfref`'s cache statistics are computed, printed and never held to anything.
- **`Document::open` is the single highest-leverage counter this tree does not have.** The oracle
  re-opens a document once per page (1794 opens over about 988 documents), and each transform walk
  re-opens each source per plan. A process-global count in one function would let seven gates state
  a relation they currently cannot.

None of this is done here, deliberately: the round was told to floor what it could defend rather
than eight walks at once, and each row above is a round's worth of work with a defect to check it
against (trap 13). `doc/questions/Q27` puts the ranking to the owner, because which of them is
worth a round is a priority rather than a technical question.

## 4. What is still unfloored inside `pdf-vfs` itself

The floor is about repeats, and three of this crate's costs are not repeats. They are the wall-clock
shortfalls `doc/todo/58` §5 already carries, and they stay: `ls -l pages/` on a 1023-page document
generates every page once and is minutes; `text/document.txt` builds whole rather than streaming;
and the cache has no disk half. Each is a **duration**, and ADR 0884 is what a duration costs to
make believable on this machine — nine processes, a pinned core list, two calibration probes, and
bands from forty-four runs. A corpus walk cannot be pinned, so those three want an instrument of
their own shaped like `launch_path.rs` rather than a line in the walk.
