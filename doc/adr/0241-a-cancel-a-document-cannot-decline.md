# ADR 0241 — A cancel a document cannot decline

Status: accepted, session 404.

## What was decided

**`viewer-confined` grows a cancel, and the cancel is a kill.** `Canceller` is a cheap, cloneable,
`Send + Sync` handle another thread holds; `Canceller::cancel` ends the worker with a signal;
whichever call the host thread was blocked in returns `ConfinedError::Cancelled`, and so does every
call after it. `Confined::canceller` takes one from a running viewer and `Confined::start_with`
takes one made *before* there is a worker, because `start` blocks too.

Nothing else in the confinement moved. `doc/todo/34`'s items 4 and 5 were priced rather than taken,
and the prices are §§5 and 6 — both of them different from what that file said.

## Why a cancel is a kill

`doc/todo/34` item 3 states the problem and ADR 0218 states the non-answer: a decode has a
thirty-second budget because one image's cost is bounded by its own dimensions, while a page's is
bounded by the document *and* the magnification, so any fixed number refuses work a viewer permits.
What was missing was a cancel, and the shape of one was left open.

**The shape follows from who is on the other end.** The confined process is interpreting a hostile
document — that is the entire reason it is confined — so a cancel it has to *agree* to is a cancel
the document can decline. A flag the interpreter polls bounds only the work that reaches a poll,
and when it reaches one is the attacker's choice, not ours: a content stream that expands into a
hundred million marks, a filter chain that inflates for a minute, a mesh whose subdivision does not
terminate. Cooperative cancellation is a *convenience* for well-formed documents and it is not a
security property.

So the cancel is the one the kernel enforces. `SIGKILL` cannot be polled for, deferred or declined,
and it is delivered by a process the document has no reach into.

Two consequences, and both are in the type rather than in a comment:

- **The worker's state goes with it.** The document, the edits and the frame are in the process
  that was killed. `ConfinedError::Cancelled` is what every later call returns, and a host that
  wants to carry on starts another `Confined`. That is the price, it is not small, and it is why
  `Cancelled` is a distinct variant from `WorkerDied` — one is a fault to report, the other is the
  host getting what it asked for.
- **A cancel is not a deadline, and this crate still sets none.** What is offered is the *ability*
  to decide, on whatever grounds a host has: a person pressing escape, a wall clock the host owns,
  another document becoming the one in front. `Confined` imposes none of them, for the reason ADR
  0218 gives and this decision does not change.

### The three mechanical questions, and how each is answered without `unsafe`

- **How does a thread that is not the owner kill the child?** `Child::kill` needs `&mut`, and the
  owning thread is the one blocked in a read. The child therefore lives in an
  `Arc<Mutex<Option<Child>>>` shared by the `Confined` and every `Canceller`. The alternative — a
  raw process identifier and a signal — is `unsafe` or a dependency, and this crate is
  `#![forbid(unsafe_code)]`.
- **How does a kill unblock a blocking read?** It does not have to: the worker's stdout closes when
  the worker dies, so `read_exact` returns `UnexpectedEof` on its own. There is no race to lose,
  because a cancel always kills and a kill always closes the pipe — the flag decides only what the
  error is *called*.
- **What about a cancel that arrives before the worker exists?** `start_with` publishes the child
  into the shared slot *before* it blocks on the greeting, and re-checks the flag afterwards, so a
  cancel between the check and the spawn is not lost. A cancel before the spawn returns
  `Cancelled` and starts nothing at all.

The lock is held across `wait` in exactly two places, and both are after the worker's output has
closed or it has been signalled — so what is held for is a dead process being reaped, not the
length of a render.

## What was demonstrated

`tests/confined.rs`, three tests, and `examples/confined_cancel` for a person.

**The document is generated rather than committed**, in `tests/support/amplification.rs`, and the
generation is the point. §8.10.1's form XObject may draw another form XObject; `pdf_model`'s
`MAX_FORM_DEPTH` bounds how *deeply* that nests, because depth is where a cycle lives, and nothing
bounds the **breadth** — nor sensibly could, since a page legitimately draws a form a thousand
times. Four levels branching ten ways is ten thousand page-covering fills:

| | |
|---|---|
| the document | **1567 bytes** |
| allowed to finish, release, 900×1200 | **44.2 s**, and 44.3 s on a second run |
| blocked before the cancel | 251, 251, 252, 251, 252, 252 ms — the sleep, by construction |
| **from `cancel()` to the host having its thread back** | **0.830, 1.198, 1.377, 1.522, 1.716, 1.974 ms**, six runs in two sittings |

So the thing a host could not stop at all now stops in about a millisecond and a half, and the
ratio between the two columns is about twenty-five thousand. Afterwards the viewer says
`is_cancelled` and answers a question with the cancel rather than with a broken pipe.

The test asserts the three separate claims rather than the one: that the work **had not finished**
after two seconds, which is what makes it a cancel of something rather than a race with a fast
page; that the host thread **came back** with `Cancelled`; and that the viewer is **finished**,
with a later question refused without going near the pipe.

## The fifth sweep, over the API this round added

`doc/todo/01`'s fifth sweep asks whether anything one side can do the other cannot ask for. One
layer down, on this crate's own surface, it is: *does every operation that can block for an
unbounded time have a way for a host to stop it?*

| operation | blocks for | cancellable |
|---|---|---|
| `Confined::start` | a spawn and a greeting | **it was not** — hence `start_with` |
| `Confined::start_with` | the same | yes, and before the spawn |
| `Confined::handle` | a document, a page, a magnification | yes |
| `Confined::query` | the same | yes |
| `Confined::drop` | a kill and a reap | bounded by construction |
| `Canceller::{new, cancel, is_cancelled}` | nothing | — |
| `wire::{command, query, events, answer}` | nothing beyond their input | — |

`start` is the finding, and it is the one a `canceller()` method alone could not have closed: a
handle taken from the value `start` returns does not exist while `start` is blocked. That is why
`Canceller::new` exists as a separate constructor rather than the type being reachable only from a
running viewer.

## 5. Item 5 repriced: the pipe is a tenth of what the pipe was blamed for

`doc/todo/34` item 5 says 19.2 MB of ISO 32000-2 down a pipe is "most of the 67 ms that document
takes to reach its first page", and proposes a `memfd` or an `SCM_RIGHTS` descriptor. Three things
were measured before deciding anything, and each moved the answer.

### The instrument

`examples/confined_page` now sends a **ballast** document after the real one: a valid one-page
catalogue plus a single stream nothing refers to, padded to exactly the real document's length. The
confined side reads all of it, parses it in microseconds and draws a blank page — so what is timed
is the *transport* and nothing else. A zero-byte ballast beside it gives the fixed cost.

| | measured |
|---|---|
| ISO 32000-2 opened, interpreted and drawn, confined | 65–108 ms |
| **19.2 MB of ballast, crossed and drawn blank** | **41–66 ms** |
| 0 bytes of ballast, crossed and drawn blank | 1.2–2.3 ms |
| the same 19.2 MB through a bare pipe (`dd | dd`, three runs) | **3.7, 4.9, 5.5 ms** |

**The pipe moves those bytes in about four milliseconds and this transport spends forty to sixty.**
Item 5's sentence was right that the document dominates and wrong about why: nine tenths of it is
allocation, copying and page faults on our side of the two file descriptors, not the kernel's.

### What was taken, because it needed no decision at all

One of those copies was a nine-byte header being put in front of a payload by building a third
buffer the size of the payload. Both ends now write the header and the payload in two calls;
`protocol::frame` survives for the tests, which want a whole frame as a value. Measured nine runs each way, alternating builds:

| | before, min / median | after, min / median |
|---|---|---|
| 4.1 MB raster crossing (`Query::Frame`) | 4.32 / 5.64 ms | **3.23 / 3.74 ms** |
| 19.2 MB ballast crossing (`Command::Open`) | 43.81 / 57.01 ms | 41.42 / 54.58 ms |

The raster is the honest claim: seven of the nine "after" samples are below the *minimum* of the
nine "before" samples, and it is the page-turn path, which is the interactive one. **The document
line moved by less than its spread and is therefore not claimed** — the machine's noise on a 19.2 MB
open is ±15 ms and the copy removed is worth about four.

### What was not taken, and why it is a decision rather than an omission

The remaining forty-odd milliseconds are four more passes over 19.2 MB: the host's encoder builds
the payload, the pipe copies in and out, the worker allocates the frame and `decode_command` copies
the document out of it into `Command::Open`. A read-only mapping removes all four at once, which is
the prize item 5 named — and `pdf_syntax::Document` already takes bytes whose lifetime it does not
own, so the receiving end is ready for it.

**Two things stop it, and the first is decisive.**

- **Every mapping API in the ecosystem is `unsafe` at the call site.** `memmap2::Mmap::map`,
  `memmap2::MmapOptions::map_copy_read_only` and `rustix::mm::mmap` are all `pub unsafe fn`, and
  they are so for a real reason: a mapping's bytes can change or vanish under the reader if
  another process writes to or truncates the descriptor. `rustix::fs::memfd_create` is safe —
  making and *writing* a `memfd` needs no `unsafe` — but mapping it does. So the crate that maps
  must contain an `unsafe` block, and `viewer-confined` is `#![forbid(unsafe_code)]` and holds a
  whole document, which is `CLAUDE.md` principle 3's compiler-enforced rule and not a convention.
  A dependency does not help here the way ADR 0186's and ADR 0214's do, because what is needed is
  not a crate that *contains* `unsafe` but a crate that *hides* it behind a safe signature, and
  none does. The construction that would justify one is a **sealed** `memfd` — `F_SEAL_WRITE |
  F_SEAL_SHRINK | F_SEAL_GROW` makes the contents immutable and therefore makes the mapping sound —
  and it would be a new crate in this workspace whose whole job is "seal, pass, map, hand out
  `&[u8]`" and which never parses anything. That is a decision to take deliberately, with the
  question "does a crate that holds PDF bytes without reading them fall under the rule?" answered
  out loud rather than assumed.
- **Getting the descriptor across costs the seccomp filter.** The document arrives in
  `Command::Open`, *after* the worker was spawned, so an inherited descriptor means one worker per
  document — a restructuring, not an optimisation. The runtime alternative is `SCM_RIGHTS` over a
  unix socket, and the interpreter profile has no `socketpair`, no `sendmsg` and no `recvmsg`, by
  design: `tests/confined.rs` asserts that a confined interpreter cannot open a socket. Adding
  three system calls so that a document arrives faster is trading the load-bearing layer for
  latency, which is the wrong direction in a file whose principle 3 outranks its principle 2.

`doc/todo/34` records both, with the numbers.

## 6. Item 4 repriced: the old price was taken on the wrong page

`doc/todo/34` item 4 says one rasterising thread "costs about 1 ms of the 7 ms this page takes".
`pdf-model`'s `strip_spans` example — which exists for exactly this question and predates the
confinement — says the number depends on the page by an order of magnitude:

| page | 1 strip | best | strips the geometry *grants* |
|---|---|---|---|
| `PDF20_AN001-BPC.pdf` p1 at 1× (160 commands) | 2.2 ms | 1.3 ms | **2**, whatever is asked for |
| the same at 2× | 8.2 ms | 5.8 ms | **2** |
| ISO 32000-2 p101 at 1× (3007 commands) | **19.9 ms** | **7.2 ms** | 8 asked, 8 granted; 11 at 16 |
| the same at 2× | **31.0 ms** | **12.7 ms** | 15 at 16 asked |

So on a sparse page one thread costs about a millisecond and no arrangement can recover more,
because ADR 0139's constrained split grants only two strips there — 23% of that page's rows are
legal cut rows. On a dense text page it costs **twelve of twenty milliseconds**, and at 200% it
costs eighteen of thirty-one. Trap 12b's shape one layer out: a price taken on one page is a price
for one page.

That raises item 4 above item 5 for interactivity, since a page turn pays it every time and an open
pays the transport once.

### And the claim item 4 rests on is now measured rather than written down

`doc/todo/34` offers "per-thread Landlock plus an allocator warm-up" and says of it: "[i]t rests on
the allocator not asking again later, which is a claim about `glibc` internals — write it down as
one, or measure it."

**Measured.** `tests/confined.rs` gained a probe and a test:
`an_allocator_warmed_before_the_filter_does_not_ask_the_kernel_again`. It builds a 24-thread `rayon`
pool *before* the confinement with a `start_handler` that allocates a mebibyte, broadcasts another
allocation to every thread so that none is merely declared, applies the interpreter profile, draws a
real page on 24 strips, and then broadcasts twenty rounds of four-mebibyte allocations to all of
them — because `arena_get2` asks `__get_nprocs` only once `narenas > mp_.arena_test`, so a page that
happened not to cross that threshold would prove nothing.

It draws. Three runs, exit 19 (`DREW`) every time, and `strace -f` counts **25 `clone3` before the
seccomp filter, none after it, and no `openat` at all after it**. So on this `glibc`, the arena
question is asked once, and asking it early answers it for good.

**That is a precondition and not an adoption.** A pool warmed before the confinement has the seccomp
filter — it is installed with `TSYNC`, so it reaches threads that already exist — and does **not**
have the Landlock domain, because `landlock_restrict_self` binds the calling thread and its future
children only. `pdf-sandbox` has no entry point that applies Landlock alone to a thread, so the
`start_handler` cannot put its worker in the domain, and shipping this today would put the confined
interpreter's rasterising threads outside the depth layer. ADR 0218 rejected the arrangement on
exactly that ground and the rejection stands; what has changed is that the *other* half of it is no
longer a guess. The test is what will say so if `glibc` ever changes its mind.

## What this does not weaken

- **The confinement is untouched.** Both profiles are the same system calls they were; nothing was
  added to either allow-list, and the reason `SCM_RIGHTS` was not adopted is that it would have
  needed three.
- **The sandbox is still a flag whose default is the safe one.** There is no path in
  `viewer-confined` that interprets a document in the calling process, and a cancel is not one: it
  ends the work, it does not move it.
- **`viewer-confined` is still `#![forbid(unsafe_code)]`**, which is most of §5's content.
- **The wire format did not change.** The header is written in a separate call and is the same nine
  bytes in the same order; `fuzz/fuzz_targets/confined_wire.rs` reads exactly what it read before.
