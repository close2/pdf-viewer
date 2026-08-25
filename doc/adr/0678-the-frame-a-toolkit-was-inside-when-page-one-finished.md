# ADR 0678 — The frame a toolkit was inside when page one finished

Status: accepted, 2026-08-25. Session 759. The quiet-machine launch A/B ADR 0668 §7 said was owed,
taken — and it found a **44 ms regression on `viewer-gtk`'s launch path**, which is the poll waiting
for a main loop that is inside its own first frame. `viewer_host::Drawing::settle` is the fix: a
window with nothing on the screen yet *waits* for page one out of a one-refresh budget instead of
polling for it. Clauses: none; `CLAUDE.md` principle 2 against principle 3, as ADR 0668 was.

## 1. What was owed, and why it could not have been read off 754's table

ADR 0668 §7 refused to average over a launch measurement taken at load 25 to 40 and recorded the
refusal instead: before 9–100 ms, after 62–448 ms, both arms drifting an order of magnitude across
one alternating sequence. What it *claimed* was the in-run number — one extra trip through the
toolkit's main loop, 2.16 ms in GTK and 2.13 ms in Qt with the loop otherwise idle — and it said
plainly that this did not license "the regression is 53 ms" or "the regression is 2 ms".

Taken on a quiet machine the answer is neither, and the reason is the four words ADR 0668 §7 could
not test: **with the loop otherwise idle.** At launch the loop is not idle. It is inside the
toolkit's own first frame, which is the expensive one.

## 2. The measurement

`opened` → `first frame on the screen`, both stamps from `viewer_host::Trace` inside one process,
which is 749's rule taken seriously: a launch measured from process start would be the machine's
number, and the difference of two stamps in one run is not. Twenty alternating pairs per host,
`doc/PDF20_AN001-BPC.pdf` under `Xvfb`, **load average 1.5 to 2.8 throughout** against 754's 25 to
40. Two release builds differing only in `d1ecef4d` — 754's own commit against its parent — so the
column is that change and nothing else.

| `opened` → first frame, ms | n | mean | min | max |
|---|---|---|---|---|
| `viewer-gtk`, before `d1ecef4d` | 20 | **9.5** | 8 | 12 |
| `viewer-gtk`, after | 20 | **53.4** | 49 | 66 |
| `viewer-qt`, before | 20 | **10.6** | 9 | 13 |
| `viewer-qt`, after | 20 | **7.9** | 6 | 11 |

The GTK ranges do not overlap: every one of twenty after-runs is slower than every one of twenty
before-runs. **On a 110 ms launch that is 40% of it**, and `CLAUDE.md` section 2 makes it a
regression rather than a cost — "a parallel path that improves throughput while worsening
time-to-first-page is a regression", stated for exactly this.

Repeated on ISO 32000-2's 1023 pages, where the document half of the launch is three times the
size: **18.0 ms before, 56.9 after, 17.8 fixed.** The regression is the same size on both
documents, which is what says it is not the document's.

## 3. Where the 44 ms is, in one line of the trace

```text
trace:  0.106  About: 14 row(s)
trace:  0.169  page 1 rasterised 509x720 in 3.252476ms, waited 61.528955ms
```

The page drew in **3.3 ms** and the window waited **61.5 ms** for it. Nothing was slow; the answer
was ready in three milliseconds and could not be collected for sixty.

`Drawing::POLL` is a millisecond and `viewer-gtk` arms a `glib::timeout_add_local_once` for it. A
timeout is dispatched when the main loop comes back round, and at launch the main loop does not:
`open_document` runs inside the first `size_allocate`, so the moment the pump returns, GTK begins
its own first frame — bringing up GSK's renderer, which under `Xvfb`'s software Vulkan holds the
loop for the better part of sixty milliseconds. `doc/todo/42` §5 already had that number from the
other direction, as `viewer-ui`'s first present under lavapipe: 54 to 68 ms.

**The control is one environment variable.** `GSK_RENDERER=cairo`, which never touches a device:

| `viewer-gtk`, after `d1ecef4d`, page 1 | drew | waited |
|---|---|---|
| default renderer (Vulkan on llvmpipe) | 3.4–3.6 ms | **57–61 ms** |
| `GSK_RENDERER=cairo` | 3.2–3.6 ms | **11.3–12.5 ms** |

So the wait *is* the toolkit's first frame, and it is not a defect in `viewer_host::drawing`: the
arrangement asks the loop for a turn, and the loop has not got one to give until it has finished
doing the most expensive thing it does all launch. Before 754 the page was rasterised *inside* the
allocation, so it was already in the texture when that frame began; after 754 the toolkit's most
expensive frame landed **in front of** page one instead of beside it.

## 4. What `viewer-qt` said, and why its faster number is the worse picture

Qt's column shows no regression at all — 7.9 ms after against 10.6 before, the after arm *faster*.
That was the finding that located the fault, because two hosts sharing one arrangement and
disagreeing means the disagreement is the toolkit's. `viewer-qt` paints a `QImage` with no device
bring-up in the way, so its loop is free and the poll is dispatched.

**And its faster number is trap 1 exactly.** The document states `/OneColumn`, so Table 29's
arrangement shows two pages and the window asks for both. What the frame lines say:

```text
after   0.091 page 1 rasterised ..., waited 5.2ms
        0.092 2168976 bytes into a QImage      <- first frame on the screen
        0.099 page 2 rasterised ..., waited 7.8ms
        0.101 4337952 bytes into a QImage
before  0.071 page 1 ...   0.074 page 2 ...
        0.077 4337952 bytes into a QImage      <- first frame on the screen
```

**2 168 976 bytes is exactly half of 4 337 952.** Qt's "first frame on the screen" in the 754 arm is
a frame carrying *one* of the two pages the arrangement shows; the second arrived nine milliseconds
later in a second paint. So Qt regressed too — not in the number its own instrument prints, but in
what the frame that number names actually contained. A metric improved and the picture got worse,
which is the oldest sentence in `doc/traps/`.

## 5. The fix: the launch waits, and nothing else does

`viewer_host::Drawing::settle(budget)` is `collect` with a `recv_timeout` in front of it. A host
calls it **while it has put no frame on the screen**, and `collect` everywhere else:

```rust
let drawn = if self.presented {
    self.drawing.collect()
} else {
    self.drawing.settle(viewer_host::Drawing::SETTLE)
};
```

Three lines, the same three in both hosts, because the rule is `viewer-host`'s and not a toolkit's.
Four things make it small enough to defend:

- **It is only ever the launch.** A window that has presented something owes a person a live window
  and must not block its loop at all. A window that has presented nothing has no frame to spoil and
  no input to lose, and the thing it is waiting for is the only thing it exists to show.
- **It is not a deadline and takes no thread back.** Nothing is interrupted, nothing is abandoned: a
  page still unfinished when the budget runs out stays in flight and arrives through the poll
  exactly as it did before, one toolkit frame later. The two conditions in `drawing.rs`'s head are
  still the only two that raise an interrupt, so ADR 0657's refusal of an automatic deadline stands
  untouched — and `a_page_that_outlasts_the_budget_is_not_taken_back` asserts it.
- **The budget is the launch's, not the call's**, and the accounting is inside `Drawing` rather than
  in two hosts. Table 29's arrangement asks for every page it shows, so a column asks two or three
  times before its first frame and a per-call bound would multiply by however many pages a document
  chose to open in. `Drawing::spent` accumulates **time actually blocked** — so a thousand-page
  document whose cross-reference table takes thirty milliseconds to read has spent none of it, there
  having been nothing in flight to wait for. That distinction is not decoration: with the clock
  started at the first idle call instead, ISO 32000-2's open would have eaten the budget before page
  one was asked for.
- **The bound is one 60 Hz refresh**, and it is the corpus's rather than a feel. ADR 0657's census
  says **93.9%** of `doc/pdf.js`'s first pages draw inside one such period at twice device scale, so
  this admits nearly all of them to the toolkit's first frame and gives up on the rest rather than
  growing to fit them — the slowest of the 957 takes 252 ms and the amplification fixture takes
  27 600, and no bound reaches those without becoming the freeze 754 removed.

## 6. What it bought, measured the same way

| `opened` → first frame, ms | n | mean | min | max |
|---|---|---|---|---|
| `viewer-gtk` before `d1ecef4d` | 15 | 9.8 | 9 | 11 |
| `viewer-gtk` after | 15 | 52.9 | 48 | 63 |
| `viewer-gtk` **settled** | 15 | **9.9** | 9 | 11 |
| `viewer-qt` before | 15 | 10.6 | 9 | 13 |
| `viewer-qt` after | 15 | 7.1 | 6 | 10 |
| `viewer-qt` **settled** | 15 | **11.7** | 10 | 13 |

GTK's launch is back where it was, to within a tenth of a millisecond of the pre-754 arm. Qt's rises
from 7.1 to 11.7 and that is the right direction: §4's byte counts say the 7.1 was half a picture,
and the settled arm's first frame carries `4337952` bytes again — both pages, as before 754.

The frame line says the same thing from inside:

```text
trace:  0.105  page 1 rasterised 509x720 in 3.823736ms, waited 3.935297ms
trace:  0.108  page 2 rasterised 509x720 in 3.028794ms, waited 3.056436ms
trace:  0.109  first frame on the screen at 108.566219ms
```

**0.11 ms of wait over the drawing**, against 58 ms. That is what ADR 0668 §7's "a thread and a
channel" predicted all along; what it could not predict was that the prediction only held once the
loop was asked at a moment it could answer.

And on the 1023-page document, where the open is thirty milliseconds rather than three: page 1 waits
**6.78 ms for a 6.70 ms draw**, so the budget survived the open intact.

## 7. What still works, checked rather than assumed

754's headline behaviour is what a fix on this path could quietly cost, so it was re-run: the
1567-byte amplification fixture in `viewer-gtk`, three zoom-ins 700 ms apart through XTEST while the
first draw was still running.

```text
trace:  0.107  opened, 1 page(s)
trace:  1.196  page 1 abandoned after 509x659 in 1.086421245s, waited 1.087020738s
trace:  1.900  page 1 abandoned after 637x824 in 700.352335ms, waited 703.870697ms
trace:  2.601  page 1 abandoned after 796x1030 in 700.055002ms, waited 701.265060ms
```

The window took every key while a page was drawing and each superseded draw stopped rather than
running to a picture nobody would see — the same three lines ADR 0668 §8 printed. The settle expires
after one refresh and the loop is live from that moment; what a hostile page one now costs a launch
is 16.7 ms, once, bounded by a constant rather than by the document.

## 8. What this ADR does not claim

- **The 44 ms is llvmpipe's magnitude, not the shape's.** What is structural is that page one's
  answer has to come back through a main loop that is inside its first frame; what that frame costs
  is the driver's, and on a real adapter it is smaller. `doc/todo/42` §5's closing paragraph still
  says the launch on the user's own GPU is the user's to measure, and this does not change it.
- **Nothing was measured about `viewer-ui`**, which drives its own event loop and presents itself,
  and therefore has neither the poll nor the toolkit frame this is about.
- **The C ABI is untouched**, and `doc/todo/30` still carries it: a C caller is told to move the
  request to a thread of its own, so there is no main loop of ours for it to wait on.
