# Bounds that cap size rather than guard against a bomb

Status: **open** — asked for by the project owner, with a witness they supplied.
Priority: 10 (a defect: a document this program can draw, and does not).
Witness: `tmp/Entwurf.pdf` — **not in the repository and not addable to it**, so everything
below is either a reproducible measurement or a general rule, and no test may name that path.
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

## 1. The witness, and why it is a defect rather than a policy

`tmp/Entwurf.pdf` is 49 679 512 bytes, **one page**, Inkscape 1.4.3 through cairo 1.18.4: a
hand-drawn geological cross-section traced to Bézier vectors. No text, no images, no fonts.
2 868 970 `c`, 127 295 `m`, 58 003 `f` — **3 185 295 operators**, in **20 834 587 lexer tokens**.

`target/pdf-retrieve page tmp/Entwurf.pdf 0` reports `LimitReached { limit: "MAX_OPERATIONS" }`
and the viewer draws **19%** of the artwork, correctly and loudly, and stops.

**`MAX_OPERATIONS` is 4 000 000, its doc comment says "[m]ost operators executed for one page",
and it counts tokens.** One increment site, `content.rs`'s `while let Some(token)`, before the
operand/operator distinction is made — so a `c` costs 7 against it. Two independent numbers agree
to two decimals: display-list commands drawn / total fills = 11 128 / 58 003 = 19.19%, and
4 000 000 / 20 834 587 = 19.20%. **This document states 814 705 fewer operators than the
advertised bound and is truncated anyway**, because for curve-heavy vector art the effective
operator budget is about 6.5× tighter than the constant claims.

**Why no test saw it.** `pdf-model/tests/hostile_budgets.rs` builds its fixture from
`"n\n".repeat(4_000_002)` — deliberately a zero-operand operator, "so this measures the bound
rather than the operator". That is the one input shape where tokens and operators are the same
number. The conflation propagated: ADR 0271, `doc/todo/49` and ledger §7.8.2 all say documents
"want 4.1 to 53.6 million **operators**", and those are token counts. **Correcting the four
documents is owed whatever else is decided**, and it is the cheapest item in this file.

**And the document is not hard.** Split at fill boundaries into seven chunks of ~3M tokens, every
chunk interprets with an empty `unsupported` list — nothing else refuses: not `MAX_OPERANDS`, not
`MAX_STATE_DEPTH`, not `TargetSpec::for_page`, not the rasteriser. Whole artwork in one process:
**1.30–1.33 s, 215 MB peak resident**, against `mutool draw` 3.83 s and `pdftoppm` 6.72 s.
`Document::open` on the 49.6 MB file is **13.3 ms**, which is incremental parsing working exactly
as `CLAUDE.md` requires. **We would be roughly 3× faster than the fastest reference if we drew
it, and one constant is the only reason we do not.**

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

The clean statement, and the test to apply to every bound in the tree:

> **Every bound that is genuinely load-bearing guards a *cycle*, a *decode*, or an *allocation* —
> a small input that commands unbounded work. `MAX_OPERATIONS` and `MAX_STATE_DEPTH` guard none of
> those. They cap size.**

| bound | if removed, a *small malicious* input can… | verdict |
|---|---|---|
| `MAX_FORM_DEPTH` 16 | recurse until the **stack** aborts the process — which the address-space ceiling cannot see, and which Rust turns into an abort rather than a report | **load-bearing, do not touch** |
| `max_stream_len` 2 GiB + the Flate/LZW guards | turn 1.85 MB into 3.7 GB (measured) | **load-bearing, and the weakest link** |
| `MAX_TILES` 4096 | state `/XStep 0.001` over 600 units — 3.6×10¹¹ empty cells, about four days; an empty cell executes no operator, so nothing else sees it (ADR 0271) | **load-bearing**, but bounds a *count* where it means to bound *work* |
| `pdf-sandbox`'s `MAX_PIXELS`/`MAX_SAMPLES`, `RLIMIT_AS`, seccomp, Landlock | unbounded decode in the historically worst attack surface | **load-bearing** |
| `xmp` ×5, `der`/`cms`/`x509`/`pkcs1`, `function.rs`'s `MAX_STITCH_DEPTH` (a 720-byte file overflowed every stack until session 425), `icc`, `mesh`, `image::MAX_SAMPLES`, every cycle guard | each turns a tiny file into unbounded work | **load-bearing** |
| **`MAX_OPERATIONS` 4 M** | nothing a bomb needs: the memory is already spent, and the time is unbounded either way because one `sh` can paint the whole page | **caps an honest document** |
| **`MAX_STATE_DEPTH` 256** | nothing — the cost is per saved state and the ceiling sees it (1 document of 65 944 wants 337; Table C.1's own figure is 28) | **caps an honest document** |
| `readback::BUDGET`, `MASK_BUDGET`, quorra's device budget, `MAX_PIXELS`, the zoom range | LRU clamps and refusals sized to a device, not refusals of content | **neither — good citizens** |

## 3. Three defects that are owed on any road

These are not architecture and do not wait for a decision.

1. **The token/operator conflation**, above: the counter, its comment, `hostile_budgets.rs`'s
   zero-operand fixture, ADR 0271, `doc/todo/49` and ledger §7.8.2.
2. **The Flate and LZW length guard is a silent clamp that keeps its partial output**, so a
   truncated bomb is indistinguishable from a complete decode. ASCII85 and RunLength refuse
   properly. **Trap 5 says unsupported input stays loud, and this is the one guard that does
   not.**
3. **`max_stream_len` and the confined ceiling contradict each other.** 2 GiB per stream, and
   `read_to_end`'s growth doubles it — the worker's abort reads `memory allocation of 3800000000
   bytes failed`, about twice the stream — against a 4 GiB `RLIMIT_AS`. One stream can therefore
   command the whole ceiling and leave nothing for the raster. And there is **no aggregate
   budget**: `Page::content_with_report` concatenates every `/Contents` part with no total, and
   `/Contents` may hold `max_array_len` = 2²⁰ entries.

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

**They are not exclusive.** A is a subset of C's requirements; B is orthogonal to both and is the
only one that answers memory. The plausible order is §3's defects, then A behind an off-by-default
switch with the gates pinned, then B or C as a separate decision with its own number attached.

## 6. What a round taking this owes

- **Nothing arbitrary may be replaced by something equally arbitrary.** A new default is a
  measurement or it is the same mistake with a bigger number.
- **The gates must stay reproducible.** Whatever lands, `interpret` under the corpus and the
  oracle stays a pure function of the document and the view state, and something in the tree
  asserts it rather than a comment claiming it.
- **A count that reports must say what it counted.** The whole of §1 is one comment that named
  operators and one loop that counted tokens.
