# 1873 — The read is divided where `Q` restores the state, and the next build reads one page on many threads

Session 1518, the step back of batch eighty-two on the project owner's target of 2026-10-10,
"improve performance" — the first frame, zooming in and out, the processor and the device, with a
step back every batch by first principles and by finding out how others do it. Status:
**accepted**; it picks the next build and builds nothing. Context: ADRs 1831, 1833, 1845, 1849,
1855, 1860, 1861; `doc/todo/69` §6. Instrument: `crates/pdf-model/examples/read_split_price.rs`
(new). Witness: `/tmp/demo_heavy_engineering_A1.pdf`. Clauses: §8.4.2, §8.5.2.1, §9.4.1, §7.5.6
(the instrument's pieces); no ledger row moves.

## 1. Where the seconds go, after ADRs 1855–1866

HEAD `c6fb18fb`, built from an export in its own target directory (`quorra` `1baece0c…`), one
`--clock` hold at load 1.1–2.7, the quietest this batch has had. **Headless**, the witness's one
page (278 579 416 bytes inflated, 78 175 395 tokens, 4 408 405 commands of 120 bytes each):

| | pinned to the eight fast logical processors | unpinned |
|---|---|---|
| the read, `pdf_model::interpret`, five and three rounds | 1 739–1 929 ms | 1 828–1 898 ms |
| of it, the inflate (`read_to_end` through the page's window) | 173–212 ms | 175–200 ms |
| of it, the lexing alone (`with_token` over the page's reader less the inflate, ADR 1833's stage method on the clock) | 335–342 ms | 341 ms |

**In the windows** (`quorra --trace`, three runs a display; ten `plus` notches 300 ms apart; a page
turn is `Right` on a two-page file whose second page is the witness's), seconds from process start
unless said:

| | the read ends | first look | first frame | last notch → its frame | turn: key → frame (its read) |
|---|---|---|---|---|---|
| owner's display | 2.40, 3.36, 3.42 | 0.54, 0.54, 0.57 | **2.48, 3.47, 3.52** | 0.62, 1.12, 1.14 (the device) | **2.60, 2.60, 2.63** (2.03–2.13) |
| Xvfb | 2.62, 2.76, 2.84 | 0.56, 0.56, 0.74 | **2.72, 2.85, 2.93** | 0.12, 0.12, 0.13 (the processor) | 2.58, 2.58, 3.35 (1.99–2.74) |

**The read is now the first frame**: ADR 1855 draws while it reads, so the frame lands 0.08–0.11 s
after the read ends — 96–97 % of it is the read, where ADR 1845 measured 76–78 %. The window's read
is 0.6–1.6 s longer than the headless one because the drawer runs beside it (ADR 1855 §3) and the
read lands on whichever core the opening thread has. A page turn is the same read on the toolkit
thread, then the frame. The first look holds round 1509's 0.54 s.

## 2. How others do it — evidence about approaches (principle 5)

- **No viewer examined divides one content stream.** pdf.js's `PartialEvaluator.getOperatorList`
  reads operations one at a time from `EvaluatorPreprocessor.read` in one loop
  (`src/core/evaluator.js`), on the one worker a document gets (`PDFWorker`, `src/display/api.js`).
  mupdf's own multi-threaded example builds a page's display list on one thread and renders bands of
  it on many; Ghostscript's clist is written by one interpreter and banded by
  `-dNumRenderingThreads`; hayro re-interprets per tile (ADR 1845 §2). The parallelism the field has
  is in the drawing, which this tree has too (ADR 1859); the read is everyone's serial step.
- **Outside PDF, a stream with nested state is divided by summarising each piece's effect on the
  state and composing the summaries.** simdjson finds every structural byte in a cheap first pass
  and parses in a second; rapidgzip decompresses one deflate stream on many threads by finding block
  boundaries speculatively and resolving the back references afterwards; the data-parallel
  finite-state machines of Mytkowicz, Musuvathi and Schulte (ASPLOS 2014) run a lexer's automaton
  from every state at once and compose. What makes a content stream easier than any of those is
  stated by the standard itself (below): its state has a stack with restore points the producer
  wrote.

## 3. The candidates, by first principles

- **A, the brief's: the tokens on many threads, the operators on one.** A cheap scan cuts the
  inflated stream at operator boundaries, N threads lex, one thread dispatches in order. Its ceiling
  is the lexing: the dispatcher still runs every operator, so the read cannot fall below the read
  less its lexing.
- **B: the operators on many threads too.** ADR 1845 §3 capped any division at 1.8× because "the
  operators' meaning is a function of every operator before them". That is true of the *state* they
  read, and §8.4.2 bounds what the state can be:

  > The Q operator shall restore the entire graphics state to its former value by popping it from the stack.

  So at a point where the stream has come back to a depth it was at before, the state is what it
  was there, changed only by the operators stated at that depth since; and since §8.5.2.1 keeps the
  current path out of the graphics state ("the current path is not part of the graphics state and
  is not saved and restored along with the other graphics state parameters"), a cut after a `Q`
  has no path open. A piece is interpretable on its own from a prefix that states its depth's state
  again, and the pieces' lists, joined in order with their clip tables renumbered, are the page's.
  §9.4.1 says `BT q … Q ET` is valid, so a cut is also kept out of a text object, a
  marked-content sequence and an inline image — the instrument refuses each by name.
- **C: a content-stream index or the list kept across opens** saves the read only when the same
  file is opened again; the owner's target is the first frame of a file. Not priced.
- **D: the device's own read.** Nothing in a content stream's meaning is the device's to compute
  — the read produces the list the device consumes — so the device's first frame waits for the same
  read as the processor's. Not a candidate; B is the device's read too.

## 4. The two priced, one measurement each

`read_split_price` cuts the witness's stream after `Q`s that return to depth one (inside the page's
outermost `q`, where its own state is stated), restates that depth's `cm`, width, cap and colours
before each piece, makes each piece a page of its own by an incremental update (§7.5.6) naming it as
the `/Contents`, interprets the pieces at once and joins them. **In every round of every count the
joined list equals the page's own list by `DisplayList`'s equality** (commands, clips by value
through the renumbering, the page's settings).

| | pinned, fast cores | unpinned |
|---|---|---|
| A's ceiling: the read less its lexing | 1 397–1 439 ms: **1.24×** | 1 487 ms: **1.23×** |
| the pieces lexed at once (what A's lexers, and B's summarising pass, cost) | 94–107 ms (4 and 8 pieces) | 53–96 ms (8, 12, 16) |
| B: 4 pieces interpreted at once | 474–521 ms: **3.7×** | |
| B: 8 pieces | 412–446 ms (two to a core) | 323–441 ms: 4.1–5.7× |
| B: 12 / 16 pieces | | **236–267 / 229–253 ms: 6.8–8.0×** |
| B: the pieces' lists joined | 117–149 ms (4 and 8) | 136–148 ms (8, 12, 16) |

**B's read, summed serially** — the inflate, the summarising pass on the pieces, the pieces at
once, the join — is 175 + 53 + 236 + 144 = **608 ms at twelve pieces unpinned against 1 828 ms:
3.0×**; without the join (the pieces handed to the drawer in order, as ADR 1855's parts already
are) 464 ms, **3.9×**; and the inflate, now the largest serial term, can run ahead of the pieces,
since a piece needs only the bytes up to its end. On the window's figures that projects the first
frame at about 0.6–0.9 s where it is 2.5–3.5 s — a projection, which the build measures.

## 5. Decision

**The next build is B: one page is read on many threads, divided where `Q` restores the state.**
A is dominated: its ceiling, 1.24×, is below what B gives on four cores. What B's build must hold:

1. **The state at a cut is composed, never assumed.** A first pass on the pieces at once records
   each piece's effect on the state stack — its pops below its start, and the operators that set
   lasting state at each depth it ends at (every Table 51/52 parameter, `gs` included, a clip
   chain's extension, the text state) — and the effects are composed in order; a piece is then
   interpreted from the composed state handed to the interpreter, not from restated bytes.
2. **A cut is only where nothing is open**: no path (§8.5.2.1), no text object (§9.4.1), no
   marked-content sequence (whose `/OC` decides what is drawn, §8.11.3.2), no compatibility section,
   no inline image; a stream with no such point is read as today.
3. **What is the page's and not the piece's stays one**: the operator budget and the list-byte
   budget (ADR 1507) charged across the pieces, the reports in the page's order, font and resource caches
   shared, ADR 1855's parts handed over in order, a take-back's spoiling unchanged.
4. **The list is the page's byte for byte** on every corpus first page that divides (the sweep
   `drawn_while_read` runs for ADR 1855, run for this), `raster_golden` holding all, and the
   witness's `callgrind_interpret --digest` `07eb7445…68e3`.
5. **The pool is the window's pool**, and the drawer beside the read is re-measured: B uses the
   slower cores by design (trap 154 is about one thread on the critical path, not about many).

## 6. Found beside it, not acted on here

- **A zoom notch on the owner's display is the device's frame now**: from the seventh notch on the
  window asked the device, and the one frame that landed (scene 127–156 ms, device 308–527 ms,
  encoded) came 0.62–1.14 s after the last notch, where round 1511 read 0.14–0.16 s on the processor; on Xvfb, whose adapter is
  a processor, 0.12–0.13 s. ADRs 1831 and 1860 choose the lane; ADR 1845's candidate B, this
  batch's slot 6, is what makes that frame cheap.
- The join is 117–149 ms for 529 MB of commands, a copy a build that keeps the pieces apart until
  the drawer has them does not make.
