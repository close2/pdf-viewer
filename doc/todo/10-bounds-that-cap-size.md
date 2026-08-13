# Bounds that cap size rather than guard against a bomb

Status: **open** — asked for by the project owner, with a witness they supplied. §3's three
defects are carried out (ADR 0306) and **the witness now draws whole**; what is open is §5, whose
choice is the owner's, and the residue §3 now names.
Priority: 10 (a defect: a document this program can draw, and does not).
Witness: `tmp/Entwurf.pdf` — **not in the repository and not addable to it**, so everything
below is either a reproducible measurement or a general rule, and no test may name that path.
Instrument: `cargo run --release -p pdf-model --example content_budget_census -- <dir>…`, which
counts a page's operators and its lexer tokens in one pass and prints the largest decoded stream
and the largest `/Contents` total beside them.
Clauses: §7.8.2 (content stream syntax), §8.7.3.1 (tiling patterns), §7.4 (filters), §C.2
Table C.1 (the standard's own architectural minima).
Code: `crates/pdf-model/src/content.rs`, `crates/pdf-syntax/src/{parser,filter}.rs`,
`crates/pdf-sandbox/src/lockdown_linux.rs`, `crates/viewer-confined/src/lib.rs`,
`crates/viewer-core/src/{event,viewer}.rs`.

## The owner's brief, which is the frame for all of it

> It doesn't make sense to have some arbitrary hard limits. If we are the fastest PDF viewer on
> the planet (and that's clearly a goal) people will want to use it for complicated documents.
> When I mentioned protecting against input I had the gif-bomb in mind, which made PCs unusable
> as all memory was consumed. It's possible that we don't need such protection any longer (maybe
> that's now up to the OS), it's also possible that we should have time limits, but again, why
> should we prevent people from using our viewer for very complicated PDFs. Maybe we should
> always be interruptible? Maybe we need different behaviours depending on command-line usage and
> when having a UI (the UI could provide a callback warning the user and allowing the user to
> abort) — however don't block and wait for the user; it's possible the user knows, goes to get a
> coffee in the meantime and comes back to an unfinished viewer.

Two sentences of it are the design constraints and are easy to lose: **the callback may not
block**, and **a bound that stops a bomb is a different object from a bound that stops a big
document**. The whole plan below is the second sentence applied one bound at a time.

## 1. The witness, and why it was a defect rather than a policy — **fixed, ADR 0306**

`tmp/Entwurf.pdf` is 49 679 512 bytes, **one page**, Inkscape 1.4.3 through cairo 1.18.4: a
hand-drawn geological cross-section traced to Bézier vectors. No text, no images, no fonts.
2 868 970 `c`, 127 295 `m`, 58 003 `f` — **3 185 295 operators**, in **20 834 587 lexer tokens**.

**It draws whole now**, and the constant did not move: `MAX_OPERATIONS` said "[m]ost operators
executed for one page" and counted lexer *tokens*, from one increment site at the top of
`content.rs`'s `while let Some(token)` — before the operand/operator distinction is made, so a `c`
cost seven. §7.8.2 puts an operator after its operands and the counter now increments where the
interpreter knows it has one.

| | before | after |
|---|---|---|
| `pdf-retrieve page … 0` | `LimitReached { limit: "MAX_OPERATIONS" }`, 0.54 s, 380 MB | `complete: true`, `unsupported: []`, 1.36 s, 380 MB |
| `render_at … 1 1.0` | 0.62 s, 381 MB, **7.99%** of the raster inked | 1.54–1.59 s over five samples, 381 MB, **34.64%** inked |
| `mutool draw -r 72` | 2.08–2.31 s, 97 MB, 34.88% inked | |
| `pdftoppm -r 72` | 3.38–3.53 s, 19–20 MB, 34.38% inked | |

The three renders agree about the page's ink to within a quarter of a point, which is what says it
draws *whole* rather than merely *more*. `Document::open` on the 49.6 MB file is **13.3 ms**.

**Two residues from that table, and they are this file's now rather than ADR 0306's.** We are the
fastest of the three and **the least frugal by a factor of four to twenty** — 381 MB against
`mutool`'s 97 MB and `pdftoppm`'s 20 MB — on a document whose content stream is 66 MB decoded;
nobody has attributed that. And the earlier measurement in this section, taken by splitting the
file into seven chunks, recorded **1.30–1.33 s and 215 MB**, so interpreting it whole costs about
20% more time and 75% more memory than interpreting it in pieces. Both are questions for §5's
road D, which is the entry that changes the *kind* of the quantity.

**Why no test saw it.** `pdf-model/tests/hostile_budgets.rs` built its fixture from
`"n\n".repeat(4_000_002)` — deliberately a zero-operand operator, "so this measures the bound
rather than the operator". That is the one input shape where tokens and operators are the same
number. Every fixture there now states operands, and
`a_stream_of_many_tokens_and_few_operators_still_draws` is the control that discriminates.

## 2. The line the owner drew, measured rather than argued

Two bombs were built to settle it.

- **Bomb A** — 0.39 MB, deflate 1029:1, 200 M `n` operators: 0.72 s, 831 MB peak, reports
  `MAX_OPERATIONS`.
- **Bomb B** — 1.85 MB inflating to 1.77 GiB: 3.10 s, **3695 MB peak**, reports `MAX_OPERATIONS`.

**A 1.85 MB file commands 3.7 GB of resident memory, and `MAX_OPERATIONS` did not stop one byte
of it**: the stream is fully materialised by `decoded_stream_data` before the interpreter sees a
token. `MAX_OPERATIONS` bounds time *after* decompression and bounds memory not at all. **The
bound the owner is thinking of is not the bound that caps their document** — they are different
objects, in different crates, and the one that is load-bearing is the weaker of the two.

**Both were rebuilt from this description in the four-hundred-and-seventy-first session and came
out the same sizes to the byte** — 389 317 and 1 847 467, both 1029:1 — which is what makes the
comparison below a measurement rather than a memory. `bomb.py`-shaped generators are not committed
because the description above is enough to rebuild them, and that is the point of writing it down.

| | before | after ADR 0306 |
|---|---|---|
| **Bomb A** | 0.81 s, **831 MB**, `MAX_OPERATIONS` | 0.71 s, **831 MB**, `MAX_OPERATIONS` |
| **Bomb B** | 3.26 s, **3694 MB**, `MAX_OPERATIONS` | 1.18 s, **1095 MB**, `TooLarge { part: Some(0), limit: 1073741824 }` |

Bomb A is unchanged and should be: 200 million operators is 200 million operators however they are
counted. Bomb B loses 70% of its peak, because `max_stream_len` is now a gibibyte and reaching it
is a refusal rather than a clamp. **It is still a gibibyte commanded by 1.85 MB of file**, and no
entry in §5 but D takes that back.

The clean statement, and the test to apply to every bound in the tree:

> **Every bound that is genuinely load-bearing guards a *cycle*, a *decode*, or an *allocation* —
> a small input that commands unbounded work. `MAX_OPERATIONS` and `MAX_STATE_DEPTH` guard none of
> those. They cap size.**

| bound | if removed, a *small malicious* input can… | verdict |
|---|---|---|
| `MAX_FORM_DEPTH` 16 | recurse until the **stack** aborts the process — which the address-space ceiling cannot see, and which Rust turns into an abort rather than a report | **load-bearing, do not touch** |
| `max_stream_len` 1 GiB + the Flate/LZW guards | turned 1.85 MB into 3.7 GB (measured); 1095 MB since ADR 0306 lowered the bound to fit the ceiling and made reaching it a refusal | **load-bearing, and still the weakest link** |
| `MAX_TILES` 4096 | state `/XStep 0.001` over 600 units — 3.6×10¹¹ empty cells, about four days; an empty cell executes no operator, so nothing else sees it (ADR 0271) | **load-bearing**, but bounds a *count* where it means to bound *work* |
| `pdf-sandbox`'s `MAX_PIXELS`/`MAX_SAMPLES`, `RLIMIT_AS`, seccomp, Landlock | unbounded decode in the historically worst attack surface | **load-bearing** |
| `xmp` ×5, `der`/`cms`/`x509`/`pkcs1`, `function.rs`'s `MAX_STITCH_DEPTH` (a 720-byte file overflowed every stack until session 425), `icc`, `mesh`, `image::MAX_SAMPLES`, every cycle guard | each turns a tiny file into unbounded work | **load-bearing** |
| **`MAX_OPERATIONS` 4 M** | nothing a bomb needs: the memory is already spent, and the time is unbounded either way because one `sh` can paint the whole page | **caps an honest document** — and capped it seven times harder than it said, until ADR 0306 |
| **`MAX_STATE_DEPTH` 256** | nothing — the cost is per saved state and the ceiling sees it (1 document of 65 944 wants 337; Table C.1's own figure is 28) | **caps an honest document** |
| `readback::BUDGET`, `MASK_BUDGET`, quorra's device budget, `MAX_PIXELS`, the zoom range | LRU clamps and refusals sized to a device, not refusals of content | **neither — good citizens** |

## 3. Three defects that were owed on any road — **all three carried out, ADR 0306**

They were not architecture and did not wait for a decision, which is why they were taken first.

1. ~~**The token/operator conflation**: the counter, its comment, `hostile_budgets.rs`'s
   zero-operand fixture, ADR 0271, `doc/todo/49` and ledger §7.8.2.~~ **Done.** The counter counts
   operators, the value stays at four million, and six documents were corrected — the two named
   above plus `doc/performance.md`, `doc/todo/03` and §7.7.3.3's ledger row. Re-measured over
   926 680 pages of 65 967 crawled documents: 48 pages pass four million tokens, **8** pass four
   million operators.
2. ~~**The Flate and LZW length guard is a silent clamp that keeps its partial output.**~~
   **Done.** `filter::FilterRefusal` separates `Unsupported`, `Corrupt` and `TooLarge { limit }`;
   the salvage of a *truncated* stream is kept and the guard refuses; and a third hole turned up
   in `ascii85`, whose `z` arm reached the check by way of a `continue` — eight `z` under a bound
   of eight produced thirty-two bytes and reported nothing. Both new tests were confirmed to fail
   with the defect put back.
3. ~~**`max_stream_len` and the confined ceiling contradict each other**, and there is no aggregate
   budget.~~ **Done.** `max_stream_len` is `1 << 30`, bounded from above by the ceiling
   (4 GiB less the raster's gibibyte, over a decode that costs about twice its output) and from
   below by the largest decoded stream in 5 047 187 streams, which is 483.84 MiB. The aggregate is
   Table 31's own sentence rather than a new number — the array of parts "form[s] a single stream",
   so the bound one stream gets is the bound the array gets — reported as
   `ContentIssue::TooLarge { part: None, limit }`.

**What is left of this section, and it is residue rather than a defect:**

- **A decode still costs about twice its output.** `read_to_end` grows a `Vec` by doubling and
  `Arc<[u8]>` is then a copy of it, so a gibibyte stream commands two, and the ceiling has three to
  give after the raster. That is the arithmetic the new bound was derived *from* rather than a
  contradiction, but it is the reason a bomb still costs 1095 MB. §5's road D is the only entry
  that removes the allocation.
- **The image path drops the reason.** `Document::image_stream` still calls the `Option`-returning
  `decode_with_parms`, so an image whose decode passes the bound is refused loudly as an image this
  reader could not decode rather than as one it declined to. One call site and a variant of
  `ImageStream`'s error; nothing about it is hard and no document exercises it.
- **A ceiling breach in the confined worker is still `WorkerDied { detail: "killed by signal 6" }`**,
  indistinguishable from a crash. That is §5 B's item and not this one's.

## 4. What exists to build on

- **A real deadline with a clean error, already shipping**: `pdf-sandbox`'s `REQUEST_TIMEOUT`,
  30 s, enforced by the *parent* with `poll` → `SandboxError::TimedOut`. For image codecs only.
- **A budget as a constructor argument**: `TargetSpec::for_page(…, max_pixels)`,
  `Readbacks::with_budget`, `MaskCache::new`. As a user-facing flag: `safedocs --budget-mb`, the
  only one in the tree. As a *report* rather than a kill: `safedocs`'s `PER_DOCUMENT_BUDGET` →
  `over_budget()`.
- **A `Duration` threaded through an API**: `Reference::render_within(…, budget)`, with
  deliberately no unbounded variant.
- **Pump-shaped resumable work**: `Command::Find` → one page per turn → `Event::Searched
  { remaining }` → `Find::Stop`. The only progress message in the whole boundary, and the only
  "abandon a long operation" command.
- **Kill-based cancel, measured**: `viewer-confined`'s `Canceller` — `AtomicBool` plus
  `Child::kill()`, 0.83–1.97 ms against 44.2 s for the same document allowed to finish, because
  "a cancel it has to agree to is a cancel the document can decline" (ADR 0241).
- **Ask-the-host-and-continue**: `Event::PasswordRequired` + `Command::Supply`.
- **A refusal designed to become a question**: `Event::Refused { operation }` plus
  `Command::Restrict`, ADR 0212 — and `doc/todo/38`'s rule that **a level enum must not ship with
  one caller**, because "a variant nothing produces and nothing answers is a level that silently
  behaves like another one".

And two facts that constrain the design more than anything else:

- **The one cancel and the one memory ceiling in this tree ship in no host.** `grep
  viewer-confined crates/*/Cargo.toml` returns no dependent. Not `viewer-ui`, not GTK, not Qt, not
  the FFI. Of the six `viewer-core` consumers, **none can cancel a render**: no host rasterises off
  the event loop, and a `RenderRequest` is deliberately self-contained with no back-channel.
- **`Interpreter::run` is not suspendable cheaply.** It is a recursive `&mut self` method whose
  per-stream state lives on the Rust call stack and recurses through forms, groups and patterns.
  Making it *cancellable* is trivial — the check point already exists. Making it *resumable* is a
  state-machine rewrite of the tree's hottest and most-tested code.

## 5. Three roads. The choice is the owner's

### A — a deadline and a host callback, in process

`interpret` grows a fourth input: a deadline, a cancel flag and a "how am I doing" callback,
checked at the existing check point. `MAX_OPERATIONS` and `MAX_STATE_DEPTH` stop being counts and
become reports. `Event::Working { … }` and `Command::Stop` cross the boundary; a host that ignores
them gets today's behaviour with a far larger default.

- **For**: cheapest by a wide margin — the check point exists. Four arbitrary bounds collapse into
  one honest instrument. `Entwurf.pdf` draws. It reuses the `Event::Refused`/`Command::Restrict`
  shape `doc/todo/38` already established for exactly this "off / on / ask / warn" ladder.
- **Against**: it puts a **clock** in `pdf-model`, and `viewer-core`'s rule 3 says no clock — the
  honest form is the host supplying the deadline as a value, the way it already supplies `Tick`.
  Worse, **a wall-clock deadline is not reproducible**, and the oracle's whole comparison rests on
  `interpret` being a pure function of the bytes and the view state. A deadline that fires under
  load would silently change a display list. That needs a hard rule — **off in every gate, on in
  every host** — and an assertion that holds it.
- **Does not fix**: memory. Bomb B still costs 3.7 GB before the first check.

### B — ship the confinement and let the OS hold the bounds

Make `viewer-confined` the viewer's actual path (`doc/todo/34` is written for it), then relax the
counting bounds hard: keep the cycle guards and the decode bounds, drop the size caps. The ceiling
becomes the memory answer, the `Canceller` the time answer, and the host offers "this is taking a
while — stop?" backed by a kill the document cannot decline.

- **For**: it is the owner's "maybe that's now up to the OS", answered *by* the OS. Already built,
  already verified against the kernel rather than the source. Cancel measured at about a
  millisecond. Bomb B is genuinely stopped — measured, not argued. Principle 3's other half
  finally reaches the program.
- **Against**: it is a **tier change**, and `doc/ui-boundary.md` calls putting `viewer-ui` on this
  boundary "a decision with a number attached rather than a switch" — page one would go through a
  pipe. Today a ceiling breach arrives as `WorkerDied { detail: "killed by signal 6" }`,
  **indistinguishable from a crash**, and the document dies with it; both need fixing (a
  `try_reserve`/`Refused` path, and worker restart plus document re-open). Linux-only
  (`doc/todo/35`). And the 4 GiB ceiling is currently *smaller than* what one 2 GiB stream can
  demand.

### C — make the unit of work small and let the host schedule everything

Generalise the search pump: interpretation becomes a resumable job the host advances one chunk per
turn of its event loop, emitting progress and taking `Stop`. Bounds become budgets *per chunk*,
and "very complicated document" becomes "many chunks" rather than a refusal.

- **For**: the only one of the three that is genuinely *always interruptible*, in the owner's
  words. The UI stays live throughout, no clock enters the core (the host decides when to pump),
  and it matches an architecture this tree has already shipped once and proved on six consumers.
  It yields **partial rendering** naturally — draw what you have and keep going — which is what a
  person actually wants from a 50 MB drawing.
- **Against**: much the largest change, for the reason in §4 — a state-machine rewrite of
  `Interpreter::run`, against an oracle that compares 1794 pages. The cost of one chunk is still
  unbounded (one `sh` paints the page), so it needs A's deadline anyway for the pathological
  operator. And it does nothing about the 3.7 GB spent before interpretation starts.

### D — stream the decompression, so the bomb never becomes an allocation

Raised by the project owner, who observed that nobody here had considered it:

> We might be able for instance to prevent gif-bombs by streaming the decompression. There are
> possibly reasons it doesn't fit, but I have the impression that we haven't even considered it.

**They are right that it was never considered, and the code is much closer to it than the other
three roads are to theirs.** `filter::flate` already holds a *streaming* decoder —
`flate2::read::ZlibDecoder`, an `io::Read` — and then calls `read_to_end` into a `Vec`. The
decompressor streams; the consumer does not. Bomb B's 3.7 GB is that one call.

**What it changes is the *kind* of the quantity, and that is the whole argument.** A window-fed
lexer turns a decompression bomb from an unbounded *allocation* into unbounded *time* — and time
is exactly what roads A and C make interruptible, while memory is what none of them can take
back. A 1.85 MB file inflating to 1.77 GiB would cost a fixed buffer and run until somebody stops
it, instead of taking the machine down before anybody is asked. That is also the only answer in
this file that needs **no number at all**: a window is a buffer size, not a policy, and the owner's
objection is to policies stated as constants.

**So D is best read as a precondition rather than a fourth alternative.** A and C bound time and
leave §2's measured 3.7 GB untouched; B answers memory by killing the process, which is the
blunt version of the same answer. D is the one that removes the allocation, after which the
counting bounds have nothing left to justify them.

**Where it fits, and where it does not** — this is the part that has to be measured rather than
assumed, and the split is not even:

- **Content streams are the good case, and they are the case that matters.** The interpreter reads
  a content stream once, forwards, one token at a time, and never seeks back. §7.8.2 even blesses
  the shape: where `/Contents` is an array, "the division between streams may occur only at the
  boundaries between lexical tokens", so several parts chain into one reader instead of being
  concatenated into one `Vec` — which is where today's *missing aggregate budget* (§3.3) also
  lives. Every filter that appears on a content stream — Flate, LZW, ASCII85, ASCIIHex,
  RunLength — is inherently streaming.
- **`Lexer::new` takes `&'a [u8]`**, and that is the real work. A reader-fed lexer needs a window
  that can hold the largest single lexical object, and `max_string_len` is 2²⁶, so either the
  window grows for one token or a string gets its own bound. Neither is hard; both are decisions.
- **Inline images are the sharp edge.** `inline_image::scan` searches forward from `ID` for `EI`
  over data whose length the dictionary does not state, which is a lookahead of unbounded size
  inside a bounded window.
- **The image and font paths want the whole thing anyway.** An embedded font program is parsed
  with random access; image sample data is indexed; an ICC profile, an xref stream and JBIG2
  globals are all read as a unit. `decoded_stream_data` returning `Arc<[u8]>` is right for those
  and streaming buys them nothing — so this is an *added* route rather than a replacement, and the
  refusals for those paths (`image::MAX_SAMPLES`, `icc::MAX_PROFILE`, the codec bounds) stay
  exactly as they are.
- **It cuts across `doc/todo/41`'s decoded-stream cache and `doc/todo/47`'s search**, which want to
  *keep* a decoded stream rather than stream past it. This entry read "41 is priced and refused
  today, so there is no conflict yet"; **41 was taken in the four-hundred-and-eighty-second
  session** (ADR 0317) and here is the sentence it owes, **as evidence rather than as a decision** —
  the choice between these four roads is the owner's and that round did not take one.

  **The measurement says the two designs disagree about content streams and agree about everything
  else.** What repeats over a document-wide sweep of ISO 32000-2 is not the content streams — those
  are read once each, forwards, which is exactly D's good case — but the *resources*: 8 798 of
  12 586 filtered decodes are a second decode of something already decoded, 830 MB of re-inflation
  against 46 MB of first decodes, and the three largest are font programs inflated 1993, 1486 and
  808 times. A font program is a random-access parse, and D's own list above already says streaming
  buys it nothing. So the cache's value and D's value come from different streams: a streaming lexer
  over content streams would leave 23.4% of a sweep exactly where ADR 0317 found it, and the memo
  removes nothing D was going to remove.

  What stays real is narrower than "cuts across": a round doing D must not route font, image and
  profile streams through the window, and the memo is now one more reason those paths stay whole.
- **One behaviour must survive it.** `flate` deliberately keeps partial output from a truncated
  stream, because "a partially-inflated content stream still renders most of a page". Streaming
  makes that the natural case rather than a special one — but §3.2's defect is that the same code
  keeps partial output *silently* when it hits the length guard, and a streaming rewrite that does
  not separate those two is the same bug with better memory behaviour.

**What a round taking D owes first**: the measurement, not the rewrite. Feed `Lexer` from an
`io::Read` behind a fixed window, run Bomb B and `tmp/Entwurf.pdf` through it, and report peak
resident and wall clock for both against §1's and §2's figures. If a 64 KiB window draws
`Entwurf.pdf` at 1.3 s and holds Bomb B at a few megabytes, the rest of this file's arithmetic
changes shape.

**They are not exclusive.** A is a subset of C's requirements; B is orthogonal to both; **D is
underneath all three** and is the only one that removes the allocation rather than surviving it.
The plausible order is §3's defects, then D's measurement, then A behind an off-by-default switch
with the gates pinned, then B or C as a separate decision with its own number attached.

## 6. What a round taking this owes

- **Nothing arbitrary may be replaced by something equally arbitrary.** A new default is a
  measurement or it is the same mistake with a bigger number.
- **The gates must stay reproducible.** Whatever lands, `interpret` under the corpus and the
  oracle stays a pure function of the document and the view state, and something in the tree
  asserts it rather than a comment claiming it.
- **A count that reports must say what it counted.** The whole of §1 is one comment that named
  operators and one loop that counted tokens.
