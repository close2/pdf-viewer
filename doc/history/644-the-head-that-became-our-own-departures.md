# 644 — The head that became our own departures

Sixth merge round, four branches, one conflict — two rounds each adding an item 7 to
`doc/todo/11`, both kept, 640's renumbered to 8. And the batch that produced it contains the first
sign that the crawl is running out of the thing it was opened for.

## The sequence, whole, on a quiet machine

| | |
|---|---|
| `fmt`, `clippy --workspace --all-targets` under `-D warnings` | clean, silent |
| **`cargo check --manifest-path fuzz/Cargo.toml --bins`** under `-D warnings` | **clean** — §2's new line |
| `nextest --workspace` | **2364 passed, 17 skipped** |
| doctests, `-p conformance` | clean (157 + 5 + 1) |
| corpus | 974 documents, 68 incomplete |
| oracle | 1794 pages — 907 agrees, 66 contradicted, 786 ambiguous |
| `render-quorra` | 957 pages — 932 agree, 23 differ, 2 refused |
| **`fixed_documents`** | **33 checked, 0 absent** |
| text extraction, both censuses, dates, XMP, JPEG 2000 | clean |
| `cargo deny check` | all four ok |

Ledger unchanged at 875 rows, 436 implemented, 222 partial, 18 reported, **no `silent` row**.

## The crawl is shallowing, and 640 said what that means

Eight chunk rounds have ranked **62 000 of 65 944** documents; **3944 remain**. The deepest negative
row per chunk, in order taken: **−20.3, −84.2, −112.6, −43.5, −10.2, −8.9.**

640's finding is not the number but what is *in* the head: **this tree's own documented
scan-conversion departures rather than a misread clause** — ADR 0308's conflation of two abutting
marks, §10.7.4's anti-aliasing departure, `Image::area_averaged` — with a witness for the first that
nobody had to construct. **The first chunk of eight where that is true.** When the head stops being
defects and starts being decisions this project has already argued, the instrument has done what it
was opened for and the next question is which departure to close, not which document to read.

640 also did the right thing about it: **because ink had gone quiet it ran a second instrument** —
reports over all 10 000 — and found 101 documents that report anything, two of them holding nine
tenths. A quiet instrument is a reason to bring another, not a reason to stop.

## Four rounds, and the two shapes they added

- **640** closed `doc/todo/25`'s own open lead: a `/Text` annotation with no `/AP` and a
  `/Rect [0 542 400 792]` was drawn as a 250-unit sticky note over a quarter of a book cover. **The
  defect was a condition, not a reading** — `anchored_icon` had already derived "a fixed size, which
  is by definition not `/Rect`'s" and then written it under `|| !is_empty(rect)`. Seven documents in
  67 193 can reach it and **the curated corpora carry not one**.
- **641** found the fifth failure shape **with its sign reversed**: §12.11's parent row claims to
  read Table 276's handlers while both children say `/RH` is read by nobody. A parent *overstating*
  — and **no sweep can print it**, because the sweep that hunts for a lacking thing reads only
  `inapplicable` rows. That is a gap in the twelve, and it is a round.
- **642** gave both native hosts §12.4.4.1's clock with **no new boundary message — the third round
  running**, and `viewer-ui` *lost* code to the shared `Clock`.
- **643** made the contradicted-group tally **twelve for twelve** by writing out `colors.pdf`'s
  closed form and ranking all five renderers against the geometry rather than against each other.

## Two errata shapes, both new

- **640's**: §12.5.3's Issue #34 is a **pure addition** — *"When an appearance dictionary is not
  present, the rendered appearance will be implementation dependent."* — missed for 220 sessions
  **because the same issue's struck half already had a verdict**. A round reading the strikeout
  recorded the issue as handled and never looked at the caret.
- **642's**: two annotations `spec-errata emit` prints under **§12.5.1** are not that clause's — the
  heading merely opens the same page, and their icons sit in §12.4.4.2's margin. They *decide* a
  question `viewer_core::presentation` had recorded under the heading *"there is no current node,
  and that is a decision"* — the other way, in both halves.

**Both say the same thing about the instrument**: `emit` prints an annotation's text, and *where it
points* is a separate question that has now changed an answer twice.

## The fuzz targets, fixed between batches

`fuzz/` is not a workspace member, so no local gate built it, and `confined_wire` had not compiled
since **606** — it matched `Reply::Frame { raster, .. }` and read `node.parent` off a flat list,
both shapes 606 and 610 left behind. Found because the owner's push finally made CI's other jobs
green and left this one visible. §2 has a `cargo check --manifest-path fuzz/Cargo.toml --bins` line
now; all fourteen targets build under `-D warnings`. Principle 3 makes it worse than a compile
error — between 606 and 644 this tree had no fuzzing at all.

## CI, first time green

The owner's push of `95830400` produced run `32455496378`: `test`, `check`, Windows, macOS, `deny`
and — **for the first time, because earlier failures had always skipped them** — the three snapshot
jobs and publish. 630's `-u` linker argument works on a real runner, and Miri went from exceeding a
one-hour ceiling to **257 tests in twelve minutes**. `nightly`'s remaining red was the fuzz build
above.

## Owed

- **3944 crawled documents**, two thousand-document archives and eighty-one small ones.
- **`doc/todo/11` items 7 and 8**, both opened by this batch and both priced: the quarter-quantised
  edge coverage, and a singular transform costing the page rather than the mark.
- **The sweep that cannot see an overstating parent.**
- **The owner's session**: `tmp/pi.pdf`, for 628.
