# ADR 0597 — The third copy, and the diagnostic the confinement destroyed

Status: accepted, 2026-08-25. Session 719. Takes `doc/todo/15` — road B, second of the three the
project owner ordered D → B → C — and carries out its second owed item, *a ceiling breach that is
not a crash*, plus the number its first item asked for. Cites no clause: this is `CLAUDE.md`
principle 3, and the ledger is untouched.

## What the confinement is now for, which is the first thing this round had to settle

`doc/todo/15` was written when the confined worker's four-gibibyte ceiling was the thing standing
between a 1.85 MB file and the machine. **Road D removed that job.** Measured this round, with
`VmPeak` from `/proc` — the counter `RLIMIT_AS` is actually compared against — sampled off the
worker while it is alive:

| in the confined worker | `VmPeak` | of the 4 GiB ceiling |
|---|---|---|
| started, before any document | 147 568 KB | 3.5% |
| `doc/PDF20_AN001-BPC.pdf`, five pages | 147 568 KB | 3.5% |
| **Bomb A**, 0.39 MB → 400 MB, 200 M `n` | 147 568 KB | 3.5% |
| **Bomb B**, 1.85 MB → 1.9 GB | 147 568 KB | 3.5% |

Both bombs were rebuilt from `doc/todo/10` §2's description for the fourth time and came out
**389 317 and 1 847 467 bytes, both 1029:1** — the sizes that file records, to the byte. Neither
moves the worker's peak by a single kilobyte: the peak is reached during start-up and no document
in this round's set ever passes it. `MAX_OPERATIONS` reports, and the ceiling refuses nothing.

**So the honest statement of what the ceiling is for has changed, and it is still a good one.** It
was *the* defence against a decompression bomb; it is now a backstop for the paths road D does not
cover — a font, an image, a colour profile, all read whole by design — and for whatever nobody has
thought of. What it is not any longer is the thing that catches the witness it was sized against.

**And that reframing is what made this round's real finding findable**, because it forced the
question *what does still reach the ceiling?* The answer is the ordinary thing rather than the
hostile one: **a large document.**

## Finding 1 — the open path held three copies of the document

`VmPeak` for the confined worker, opening a valid one-page document padded to a stated size (the
shape `examples/confined_page` already builds in memory, so that what is measured is the transport
and not an interpretation):

| document | `VmPeak` before | start-up + 3 × length | after | start-up + 2 × length |
|---|---|---|---|---|
| 999 996 130 B | 3 011 720 KB | 3 011 714 | **2 035 164 KB** | 2 035 160 |
| 1 299 996 132 B | 3 890 624 KB | 3 890 621 | — | — |
| 1 399 996 132 B | 4 183 592 KB (**99.7% of the ceiling**) | 4 183 592 | **2 816 412 KB** | 2 816 412 |
| 1 499 996 132 B | **the worker died** | 4 476 564 (over) | **3 011 728 KB** | 3 011 724 |

The left column is arithmetic, not a rule of thumb: **`VmPeak` was the worker's start-up size plus
exactly three times the document's length, to the kilobyte, at every size measured.** The three are
the frame buffer `read_frame` fills, the copy `decode_command` makes into `Command::Open`, and the
`Arc<[u8]>` `pdf_syntax` copies that into.

**One of the three was free.** `answer` took the payload by reference and `serve` held it across
the whole of the work — and everything a decoded message owns is already its own, so the buffer was
dead the moment the decode returned. Taking it by value and dropping it there takes the live set
from three to two, and the right-hand column is the same measurement afterwards, again exact.

The largest document a confined viewer could open was therefore about **1.31 GiB**, against a
`MAX_MESSAGE` of two gibibytes and nothing anywhere saying so. It is now about 1.43 GiB and it is
*said*.

## Finding 2 — the confinement destroyed the worker's own explanation

The 1.9 GB document, before this round, through `examples/confined_peak`:

| the host's standard error is | the host is told | the worker said |
|---|---|---|
| a **pipe** | `the confined viewer stopped without answering (killed by signal 6)` | `memory allocation of 1899996152 bytes failed`, on the operator's terminal |
| a **file** | `killed by signal 25` | **nothing at all** |

Signal 25 is `SIGXFSZ`. **`RLIMIT_FSIZE` is 0 in the confinement**, and the worker's standard error
was the host's own, *inherited* — so on any host whose diagnostics go to a file, which is every
logged deployment, the worker's first attempt to explain itself is a write that exceeds a file-size
limit of zero and kills it before a character reaches the disk. The host is then told the wrong
cause, in a signal number, about a failure that had a sentence attached to it.

`doc/todo/15` recorded this defect as "indistinguishable from a crash". It was worse than that: it
was *misattributed*, and the comment above the inherited descriptor — "so that a worker that dies
says so where the operator can see it" — was false exactly where an operator would be looking.

`tests/confined.rs::a_confined_worker_cannot_write_a_diagnostic_to_a_file` pins the mechanism on
one write, needing no bomb: the same probe is killed by `SIGXFSZ` with a file and returns normally
with a pipe.

## What was built

### 1. A message budget the worker derives from its own ceiling

The last of the two remaining copies is `impl From<Vec<T>> for Arc<[T]>`, which copies and forgets
because an `Arc` needs a header the `Vec` has no room for (ADR 0354 found the same thing one layer
down). There is no stable fallible form of it, so **the last allocation cannot be a `try_reserve`**
— which is what decides the shape: a message the ceiling cannot hold twice is refused *before the
first byte is read*, rather than aborted at the last.

Every term of the budget is read or measured:

```
budget = (ceiling − already − settling − MAX_PIXELS × 4) / copies
```

- `ceiling` is `pdf_sandbox`'s `INTERPRETER_ADDRESS_SPACE_LIMIT` as the kernel installed it, which
  the worker already reports in its greeting. Zero — every platform `doc/todo/35` covers — means no
  budget.
- `already` is `VmSize` from `/proc/self/status`, read **before** the confinement, because
  afterwards there is no filesystem: `openat` is not on the interpreter's allow-list, and this is
  the only moment the question can be asked at all.
- `settling` is what the process still grows by after that moment — `rayon`'s pool, a thread's
  stack, and on `glibc` a 64 MiB arena for it. Measured: `VmSize` is 14.1 MB at the baseline and
  82.0 MB once a page has been drawn, so the baseline misses 68 MB; the constant is that rounded up
  to 128 MiB, which is 3% of the ceiling. **Rounding up is the direction to be wrong in**, and the
  first draft of this round did not have the term at all — which would have left the arithmetic
  90 MB optimistic in its own worst corner.
- `MAX_PIXELS × 4` is a page's pixels in RGBA, subtracted because the document is still held when
  the raster is allocated. It is the same arithmetic `INTERPRETER_ADDRESS_SPACE_LIMIT` was derived
  from, run backwards — so `viewer_core::MAX_PIXELS` is now public rather than copied, on the rule
  that a copy of a number is a number that can drift.
- `copies` is 2, and it is the measurement in Finding 1 rather than a guess.

On this machine that is **1 536 108 544 bytes**, and the ladder behaves as the arithmetic says:

| document | before | after |
|---|---|---|
| 1 499 996 132 B | worker killed, signal 6 | opens, `VmPeak` 3 011 728 KB, 71.8% of the ceiling |
| 1 599 996 132 B | worker killed | **refused by name**, `VmPeak` unmoved at 147 568 KB |
| 1 899 996 132 B | worker killed, signal 6 or 25 | **refused by name**, and the worker went on to open two more documents |

The refusal is a `FRAME_REFUSAL` — the frame the worker already had for "I will not do this, here is
why" — so it reaches a host as `ConfinedError::Refused` carrying a sentence that names the size
asked for, the ceiling, the factor and the budget. **The worker's payload is read past rather than
left in the pipe**, which is what makes it a refusal rather than the end of the conversation, and
`a_frame_over_the_budget_is_read_past_rather_than_allocated` is the discriminating test: two frames
back to back and a budget admitting only the second.

**`doc/todo/15` asked for "worker restart plus document re-open" beside this, and for the population
the budget covers it is not needed** — the worker never dies, so the document it already had open is
still open. That is a better answer than a restart and it is the one to prefer wherever a refusal can
be reached before an allocation.

### 2. `try_reserve` where a length the other side stated becomes an allocation

One shape, stated as a rule: **an allocation whose size the *other side* of this pipe chose is a
`try_reserve`; an allocation whose size the work chose is not.** `Reader::owned_bytes` is where the
first kind now goes — a document's bytes, a supplied file, a saved file, an attachment, a raster, a
thumbnail's samples and every string — and it answers `ProtocolError::NoRoom { what, bytes }`, which
the worker already turns into a refusal frame.

**The host has the same hazard in the other direction and had no guard at all.** `protocol`'s list
reader already reasons that "a subverted worker sending nine bytes of header and a count of 2^31
would have the host ask its allocator for tens of gibibytes of `String` headers and abort" — and the
frame *payload* on the host's side was `vec![0u8; length]` against a two-gibibyte `MAX_MESSAGE`,
believing the claim outright. It is a `try_reserve` now, with `ConfinedError::NoRoom` beside it and
the same read-past so that the worker survives being refused.

### 3. The worker's last words reach the host

`Stdio::piped()` in place of `Stdio::inherit()`, a thread that reads it to end of input, and the
tail of what it said appended to `ConfinedError::WorkerDied`'s detail. A pipe is not a file, so
`RLIMIT_FSIZE` cannot apply to it and Finding 2's silence is structurally gone; everything read is
still written on to the host's own standard error, so an operator sees what they saw before.

It is a thread because the alternative is a deadlock: diagnostics read only after the worker has
stopped are diagnostics sitting in a pipe, and a worker blocked writing to a full pipe never answers
the frame the host is blocked reading.

**A thread on the launch path is a cost `CLAUDE.md` asks to be measured rather than assumed, and it
is not measurable here.** Two binaries built from the same tree differing only in this, run
interleaved, twelve samples each, load average 9–28 — `Confined::start`'s own figure for spawning
and confining a worker:

| | median | min | max |
|---|---|---|---|
| inherited descriptor, no thread | **1.086 ms** | 0.753 | 6.882 |
| a pipe and a thread reading it | **1.085 ms** | 1.069 | 1.115 |

A process spawn is a millisecond and a thread spawn is tens of microseconds, so this is the
arithmetic behaving; what the table adds is that the *spread* of the arm with the thread is
narrower, which is the two outliers being the machine rather than either arm.

`a_worker_that_dies_saying_something_says_it_to_the_host` pins it, and was run against the unfixed
arrangement first (trap 13): with the inherited descriptor the host prints exactly *the confined
viewer stopped without answering (exited with status 3)* and the worker's sentence is nowhere.

## The number the tier change costs, which `doc/todo/15`'s first item asked for

`examples/confined_page`, both arms in one sitting, three runs each, release binaries installed by
§5 immediately beforehand. **Load average 13–27 from three parallel rounds** — the peaks above are
load-immune and these are not, so they are reported as ranges and the comparison is within-run.

| | `PDF20_AN001-BPC.pdf`, 173 kB | ISO 32000-2, 19.2 MB |
|---|---|---|
| worker started and confined | 1.13–1.74 ms | 1.16–1.74 ms |
| opened, interpreted and drawn, confined | 6.07–7.47 ms | 65.0–74.5 ms |
| the raster back across the pipe (849×1200) | 3.59–4.81 ms | 3.82–4.81 ms |
| the same document's bytes crossing, drawn blank | 1.68–1.94 ms | 40.6–52.6 ms |
| unconfined here, one strip, as the worker draws | 5.65–6.47 ms | 26.7–47.6 ms |
| unconfined here, every core | 4.77–4.99 ms | 25.9–37.8 ms |

So page one through a pipe is, on a small document, **about 1.1 ms of spawn and confinement, about
1.8 ms of document, and about 4 ms of raster on top of a 5 ms page** — roughly double, and it is the
*raster* that dominates rather than the confinement. On the specification it is 65–75 ms against
26–48 ms, of which 41–53 ms is the document crossing, which is ADR 0241 §5's five passes and is
unchanged by this round.

**That is a cost to weigh, not a decision this round makes.** `doc/todo/34` §2 still holds: putting
`viewer-ui` on this boundary is a change of *tier*, `CLAUDE.md` says page one goes to the graphics
device, and neither way out of that — shipping display lists instead of pixels, or putting `wgpu`
inside the confinement — has been argued. The tier change stays unbuilt and now has its number.

## What this round did not do, said plainly

- **The tier change itself.** Above, with the number.
- **Off Linux there is no budget and no ceiling**, and the code says so: `message_budget` answers
  `u64::MAX` where the confinement reports no address-space limit, so a platform without one reads
  what the protocol carries and nothing narrower. `Confinement::shortfall` already tells a host that
  it has no ceiling; `doc/todo/35` is where the rest of that lives.
- **The two copies that remain.** One is the `Arc`, which cannot be removed without an allocation
  trick that `#![forbid(unsafe_code)]` rules out; the other is `decode_command`'s, which could go if
  the frame buffer could be handed over rather than read out of, and that is a restructuring rather
  than a fix.
