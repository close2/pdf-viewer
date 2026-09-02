# Bounds that cap size rather than guard against a bomb

Status: **open, and the choice is made** — asked for by the project owner, with a witness they
supplied. §3's three defects are carried out (ADR 0306) and **the witness now draws whole**; the
owner has ordered §5's roads **D → B → C**, each of which now has its own file
([`14`](14-stream-the-decompression.md), [`15`](15-ship-the-confinement.md),
[`16`](16-resumable-interpretation.md)). What stays here is the comparison, road A, and the
residue §3 names. **§5 carries each road's price in current
numbers** as of ADR 0354, and the sentence to read before choosing is that one road's price fell
by a third while nobody was looking at it and another's prize shrank by the same third.
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
`mutool`'s 97 MB and `pdftoppm`'s 20 MB — on a document whose content stream is **147 972 263
bytes**, 141 MiB, decoded in one part. (This sentence said 66 MB for nine rounds; ADR 0362
measured it twice from either arm of a spike, and `lexer.rs`'s own comment had said 141 MiB since
ADR 0341.) And
the earlier measurement in this section, taken by splitting the file into seven chunks, recorded
**1.30–1.33 s and 215 MB**, so interpreting it whole costs about 20% more time and 75% more memory
than interpreting it in pieces. Both are questions for §5's road D, which is the entry that changes
the *kind* of the quantity.

**"Nobody has attributed that" stood here until the five-hundred-and-nineteenth session, and
`massif` attributes it in one command** — `valgrind --tool=massif --time-unit=B`, whose peak
snapshot names the two blocks alive at it rather than a total. On §2's Bomb A they are
`filter::flate`'s buffer and `Arc<[u8]>::copy_from_slice`, and the sum is the whole of the peak to
within the program's own eight megabytes. **The instrument was never the difficulty**, which is
worth writing down beside a residue that survived four rounds: `ru_maxrss` says how much, and
`massif` says which two allocations, and the second question is the one that turns a residue into a
defect. ADR 0354 took a third of the witness's peak off on the strength of it — 429 MB → 381 MB
through `pdf-retrieve`, and the same through `render_at` for a byte-identical raster.

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

| | before | after ADR 0306 | after ADR 0354 |
|---|---|---|---|
| **Bomb A** | 0.81 s, **831 MB**, `MAX_OPERATIONS` | 0.71 s, **831 MB**, `MAX_OPERATIONS` | 0.77–0.79 s, **768 MB**, `MAX_OPERATIONS` |
| **Bomb B** | 3.26 s, **3694 MB**, `MAX_OPERATIONS` | 1.18 s, **1095 MB**, `TooLarge { part: Some(0), limit: 1073741824 }` | 1.16–1.22 s, **1031 MB**, the same report |

Bomb A is unchanged between the first two columns and should be: 200 million operators is 200
million operators however they are counted. Bomb B loses 70% of its peak in the second, because
`max_stream_len` is now a gibibyte and reaching it is a refusal rather than a clamp.

**The third column is not a continuation of the second and the two bombs' figures moved for one
reason: `Vec::reserve` is amortised.** By the time it was measured again, Bomb B had gone back up
to **1811 MB** — the loop that replaced `read_to_end` in the five-hundred-and-eighth session
computed the right growth step and handed it to a method documented to take
`max(2 × capacity, len + additional)`, so the last step before a gibibyte ceiling granted 1804 MiB.
The measurement and the multiplication agree. Bomb B now costs **exactly the bound**, and Bomb A —
which never reaches the bound — loses a third of its peak to the second half of the same finding:
a whole decode ends in a buffer of up to 2*L* and `Arc<[u8]>` is a copy beside it, so the peak was
capacity + length. ADR 0354.

**It was still a gibibyte commanded by 1.85 MB of file, and D took it back**: since ADR 0365 the
page's `/Contents` is read through a 64 KiB window, and Bomb B costs **8.4 MB** and reports
`MAX_OPERATIONS`. The row above is what the bound used to buy; §5's table carries what replaced
it. **And since ADR 0427 the same bomb hidden in a form XObject costs 10.7 MB where it cost 1032**
— the sentence here read "a bomb in a *form* still costs the gibibyte" until then. A bomb in a
**tiling pattern's cell** still does, on a measurement that round's fuzzing produced rather than on
an omission. `doc/todo/14` is closed: the filter family was ADR 0429's and §8.7.3.1's cell
ADR 0430's.

The clean statement, and the test to apply to every bound in the tree:

> **Every bound that is genuinely load-bearing guards a *cycle*, a *decode*, or an *allocation* —
> a small input that commands unbounded work. `MAX_OPERATIONS` and `MAX_STATE_DEPTH` guard none of
> those. They cap size.**

**And the sentence has a fourth member the table below did not have a column for, found in the
five-hundred-and-sixty-fourth session: a *memo*.** ADR 0399's two documents took 2 m 13 s and 35.6 s
not because any bound was too small or too large, but because `image::RasterCache`'s probe is linear
in its entries and §8.9.7's inline image added one entry per *draw* that nothing could ever find —
so a page's cost was quadratic in its own image count while every bound in the table below refused
nothing. **A memo is a bound's opposite and belongs to the same audit**: a bound that is too small
draws less than the file says and says so, and a memo whose key cannot be hit draws exactly what the
file says and takes minutes doing it. The question to ask of one is *what population does its
lookup walk, and can a document grow that population without limit?*

| bound | if removed, a *small malicious* input can… | verdict |
|---|---|---|
| `MAX_FORM_DEPTH` 64 (16 until ADR 0793, which found a tiling cell outside it) | recurse until the **stack** aborts the process — which the address-space ceiling cannot see, and which Rust turns into an abort rather than a report | **load-bearing, and a stack figure rather than a habit since ADR 0793** |
| `max_stream_len` 1 GiB + the Flate/LZW guards | turned 1.85 MB into 3.7 GB (measured); 1095 MB since ADR 0306 lowered the bound to fit the ceiling and made reaching it a refusal, and **exactly the bound** since ADR 0354 stopped the buffer doubling past it | **load-bearing, and still the weakest link** |
| `MAX_TILES` 4096 | state `/XStep 0.001` over 600 units — 3.6×10¹¹ empty cells, about four days; an empty cell executes no operator, so nothing else sees it (ADR 0271) | **load-bearing**, but bounds a *count* where it means to bound *work* |
| `pdf-sandbox`'s `MAX_PIXELS`/`MAX_SAMPLES`, `RLIMIT_AS`, seccomp, Landlock | unbounded decode in the historically worst attack surface | **load-bearing** |
| `xmp` ×5, `der`/`cms`/`x509`/`pkcs1`, `function.rs`'s `MAX_STITCH_DEPTH` (a 720-byte file overflowed every stack until session 425), `icc`, `mesh`, `image::MAX_SAMPLES`, every cycle guard | each turns a tiny file into unbounded work | **load-bearing** |
| §8.9.6.3's and §11.6.5.2's mask chains — `explicit_entry`, `soft_mask_entry` | until ADR 0399, **nothing at all**: an image whose `/Mask` names an image mask stating a `/Mask` of its own recursed `decode_parts` → `apply_explicit_mask` → `decode` until the stack aborted the process, and Table 143's `/Mask` row was unread while its `/SMask` row was guarded | **load-bearing, and it is not a constant** — Table 87 and Table 143 both say the entry "shall not be present", so the standard's depth is one and the guard is a refusal rather than a number |
| **`MAX_OPERATIONS` 4 M** | nothing a bomb needs: the memory is already spent, and the time is unbounded either way because one `sh` can paint the whole page | **caps an honest document** — and capped it seven times harder than it said, until ADR 0306 |
| **`MAX_STATE_DEPTH` 256** | nothing — the cost is per saved state and the ceiling sees it (1 document of 65 944 wants 337; Table C.1's own figure is 28) | **caps an honest document** |
| `readback::BUDGET`, `MASK_BUDGET`, quorra's device budget, `MAX_PIXELS`, the zoom range | LRU clamps and refusals sized to a device, not refusals of content | **neither — good citizens** |
| **`MAX_GROUP_BLIT_PIXELS` 2^35** | state 73 047 page-spanning transparency groups — 300 billion blitted pixels, ~640 s, and no interpretation budget sees it because 298 379 commands is not many (ADR 0780) | **load-bearing**, and the third of these that bounds *work* rather than a count: it reads the demand off the display list and refuses before a pixel is spent |

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

- **A decode costs about twice its output, and until ADR 0354 it cost three times.** The loop
  doubles because it cannot know where the stream ends, and `Arc<[u8]>` is then a copy of the
  result, so the peak is *capacity plus length* — up to 3*L* where ADR 0306's derivation of the
  gibibyte assumed 2*L*. `shrink_to_fit` before the hand-over makes it the 2*L* the ADR assumed,
  which is why **no constant moved**: the code now costs what the number was derived from. The
  remaining 2*L* is the copy, and it is `Arc<[u8]>`'s own — `impl From<Vec<T>> for Arc<[T]>` copies
  and forgets, because an `Arc` needs a header the `Vec` has no room for, so it cannot be taken
  back by an allocation trick. §5's road D is the only entry that removes it, and only for the
  content-stream route; the resource paths want the whole buffer and are why the copy exists.
- **The image path drops the reason.** `Document::image_stream` still calls the `Option`-returning
  `decode_with_parms`, so an image whose decode passes the bound is refused loudly as an image this
  reader could not decode rather than as one it declined to. One call site and a variant of
  `ImageStream`'s error; nothing about it is hard and no document exercises it.
- ~~**A ceiling breach in the confined worker is still `WorkerDied { detail: "killed by signal 6" }`**,
  indistinguishable from a crash.~~ **Carried out in §5 B's item** (ADR 0597), and it was worse than
  this line said: `RLIMIT_FSIZE` is 0 in the confinement and the worker's standard error was the
  host's own, so a host logging to a *file* got `killed by signal 25` — `SIGXFSZ`, the wrong cause —
  and not one word of the worker's own explanation. A document the ceiling cannot hold is now
  refused by name before an allocation is attempted, and a breach that still kills arrives with the
  worker's last line attached.
- **`image::RasterCache`'s probe is still linear in its entries**, and after ADR 0399 those entries
  are the *distinct* images a page draws rather than the draws — which is the property a resource
  image always had, and it is what took the two witnesses from 330.5 G and 71.9 G instructions to
  10.7 G and 6.3 G. A page stating tens of thousands of **distinct** inline images would still be
  quadratic in them, three orders of magnitude below the rate measured there: the sharper of the two
  witnesses draws 12 092 inline images of which **five** are distinct. Nothing in reach exercises it,
  so this is written down rather than built, and what would build it is `DisplayList::add_clip`'s own
  construction one level up — bucket the entries by the digest the key already carries. **The
  measurement to take first is the population**, not the probe: a census of distinct images per page
  over the corpus would say whether any document has more than a handful.

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

## 5. Four roads. The choice is the owner's, and here is what each costs today

**The prices below were re-taken in the five-hundred-and-nineteenth session against the code as it
now stands**, because two rounds had changed the ground under them since they were written: the
five-hundred-and-eighth replaced the inflate path with a pump (ADR 0343), and this one capped its
buffer and removed a copy (ADR 0354). Read the table before the four sections; the sections are the
argument and the table is the arithmetic.

| road | what it removes | what it costs, in today's code | moved by 508 / 519? |
|---|---|---|---|
| **A** deadline + callback | unbounded *time* | one parameter on `interpret`, one check at `run.rs`'s existing increment site, two boundary messages, and a rule pinning the gates | **no** — the check point is where 471 left it |
| **B** ship the confinement | unbounded *anything*, by killing | **a tier change (`doc/todo/34` §2) and nothing else**: the `try_reserve`/`Refused` path is built and the restart it needed turned out not to be owed (ADR 0597), and the tier change is now *decided* — display lists cross with the raster kept as a per-page fall-back, and a device inside the confinement dies on its first `ioctl` (ADR 0607). What is left of it is a codec. Linux-only | **cheaper twice** — see below, and `doc/todo/15` |
| **C** resumable interpretation | unbounded *latency* | a state-machine rewrite of `Interpreter::run`, against an oracle of 1794 pages | **no** |
| **D** stream the decompression | the *allocation* | **shipped, all five of the content streams §7.8.2 names** — a page's `/Contents` in 530 (ADR 0365), the three beside it in 592 (ADR 0427), the LZW pump in 594 (ADR 0429) and §8.7.3.1's tiling cell in 595, once the cell was drawn once and its marks copied (ADR 0430) | **done and measured: Bomb B costs 8.4 MB against 1032 in `/Contents`, 10.7 against 1032 in a form and 9.4 against 1055 in a pattern cell; the witness 194 MB against 381; every gate identical, +5.74% then +0.089% instructions on an ordinary page and −94% on a tiling one** |

**D is half-built and nobody set out to build it.** §5 D below says "`filter::flate` already holds a
*streaming* decoder — `flate2::read::ZlibDecoder`, an `io::Read` — and then calls `read_to_end`".
**That sentence is now wrong in the direction of D**: the adapter is gone. `filter::inflate_buffer`
holds a `flate2::Decompress` across iterations, keeps its own input cursor, writes through
`decompress_vec`, and terminates on three named conditions — `Stopped::Whole`,
`Stopped::Damaged(_)`, `Stopped::PastTheBound`. That *is* a pump. A window-fed decoder is that loop
with a fixed buffer in place of a growing one and a consumer between the two, and the vocabulary it
must report in already exists, because D's own caveat — "a streaming rewrite that does not separate
[damage from the bound] is the same bug with better memory behaviour" — was separated in 471 and
made reliable in 508.

**And D's prize shrank in the same measurement — then grew again when somebody built it.** What it
removes is now:

| | before ADR 0354 | after | what D left, **shipped and measured** |
|---|---|---|---|
| Bomb B, 1.85 MB of file | 1811 MB | **1031 MB** (the bound, exactly) | **8.4 MB**, and `MAX_OPERATIONS` four million operators in |
| the same bomb inside a form `XObject` | — | 1032 MB, `undecodable form /Fx` | **10.7 MB**, and `MAX_OPERATIONS` (ADR 0427) |
| `Entwurf.pdf`, 141 MiB content stream | 429 MB | **381 MB** | **193.7 MB**, display list included (ADR 0365) |

So D is still the only road that changes the *kind* of the quantity, and the last column is a
spike's `VmHWM` rather than an estimate (ADR 0362, `examples/window_lexer_spike`). **The two
right-hand figures in the old table — "a window" and "about 315 MB" — were a prediction, and the
second was wrong for a reason worth keeping**: it assumed the witness's peak was the display list
and the raster, and `massif` says the peak is *two copies of the decoded content stream*, with the
display list arriving at about 99 MB after they are freed. The 141 MiB in the row is measured too;
this table said 66 MB.

**B is cheaper for a reason that is not about B.** ADR 0306 derived `max_stream_len` = 1 GiB partly
from "a decode costs about twice its output […] 2L has to fit in the 3 GiB the raster leaves". The
code did not obey that: a whole decode cost up to 3*L* and a bomb up to twice the bound, so a single
stream could still command more of `INTERPRETER_ADDRESS_SPACE_LIMIT` than the arithmetic allowed.
It obeys it now — measured `VmPeak` for Bomb B is **1041 MB against a 4 GiB ceiling**, where it was
1821 MB. B's standing objection that "the 4 GiB ceiling is currently *smaller than* what one 2 GiB
stream can demand" is answered by the bound rather than by the ceiling, and B's remaining costs are
the three in the table above, unchanged.

**A and C are untouched**, and that is worth stating rather than leaving to inference: nothing in
471, 508 or 519 went near `Interpreter::run`'s shape or put a clock anywhere near `pdf-model`.

**The order in the last paragraph of this section still holds and is now three steps further
along**: §3's defects, then D's measurement, then D. **D is shipped for the stream this file is
about** — ADR 0365, the five-hundred-and-thirtieth session — and the two bombs are the row above:
Bomb B commands 8.4 MB where it commanded a gibibyte, and the witness draws whole from 194 MB.
The gibibyte in §2's sentence "**It is still a gibibyte commanded by 1.85 MB of file**" is no
longer commanded by that file, and `max_stream_len` is no longer what refuses it. **And three of the four nested content streams followed in the
five-hundred-and-ninety-second**, on the decoded-stream memo's own condition rather than on a new
number (ADR 0427); the fourth, §8.7.3.1's cell, is an exception that round's own fuzzing measured.
What is left of D is in [`14`](14-stream-the-decompression.md) §"What is still owed" — that cell
and one filter family — and the owner's order is at **B** next.

## 5.1 The four roads themselves

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
- **Does not fix**: memory. Bomb B's whole peak is spent before the first check — 3.7 GB when this
  was written, a gibibyte since ADR 0354, and every byte of it still before A's deadline can fire.

### B, C and D have files of their own, and the owner has ordered them

**The project owner chose, in the five-hundred-and-nineteenth session's aftermath: D, then B, then
C.** Each road's argument moved out of this file and into its own, so that the evidence lives with
the item the way every other todo does; what stays here is §5's table, which is the *comparison*,
and A, which nobody chose.

| road | file | one line |
|---|---|---|
| **D** | [`14-stream-the-decompression.md`](14-stream-the-decompression.md) | removes the allocation; shipped for four of the five content streams §7.8.2 names (ADRs 0365, 0427), and what is left is §8.7.3.1's cell and a pump for the four filters that are not Flate |
| **B** | [`15-ship-the-confinement.md`](15-ship-the-confinement.md) | hands the bound to the kernel; a tier change and two defects, and its arithmetic objection is answered |
| **C** | [`16-resumable-interpretation.md`](16-resumable-interpretation.md) | always interruptible; a state-machine rewrite against an oracle of 1794 pages, and it contains A |

**They are not exclusive**, which is why an order rather than a choice was the right answer. A is
a subset of C's requirements; B is orthogonal to both; **D is underneath all three** and is the
only one that removes the allocation rather than surviving it. §3's defects came first and are
carried out; the owner's order takes the rest from underneath upwards.


## 6. What a round taking this owes

- **Nothing arbitrary may be replaced by something equally arbitrary.** A new default is a
  measurement or it is the same mistake with a bigger number.
- **The gates must stay reproducible.** Whatever lands, `interpret` under the corpus and the
  oracle stays a pure function of the document and the view state, and something in the tree
  asserts it rather than a comment claiming it.
- **A count that reports must say what it counted.** The whole of §1 is one comment that named
  operators and one loop that counted tokens.
- **A bound on an allocation must be measured on the allocation.** ADR 0354's addition, and it is
  the same sentence one layer down: `tests/stream_length_bound.rs` checked what a bomb is *told*
  and was right for two rounds while the buffer behind it was twice its stated size, because a
  refusal looks the same either way. Whatever lands, something reads `capacity` — or `ru_maxrss`,
  or `massif`'s peak snapshot, which names the two blocks alive at it rather than a total.
