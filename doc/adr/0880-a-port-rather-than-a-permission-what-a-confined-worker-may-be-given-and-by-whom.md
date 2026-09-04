# 0880 — A port rather than a permission: what a confined worker may be given, and by whom

Session 920. Status: **accepted**. The first of this round's two records: the layer `doc/todo/59`
asked for — what crosses it, why it is not a widening of anything, why fonts specifically need no
prompt, and the trade that is real and is written down here rather than left implied. ADR 0881 is
the hosts on top of it and the measurement.

## Context

The project owner, on 2026-09-04:

> I think we need to rethink our 'no access to the filesystem' policy. what do you think about a
> clean layer, which every implementation must (can?) overwrite. the cli would wrap the access with
> a flag. GUIs could either have a setting, or ask the user. access to fonts might be reasonable
> without user intervention.

The demand underneath it is ADR 0870's, and it is a fidelity cost this tree took deliberately and
wrote down. `pdf_sandbox::lockdown::Profile::Interpreter` is an allow-list of system-call numbers
whose action is `SECCOMP_RET_KILL_PROCESS`; `openat` is not on it; and
`pdf_font::substitute::catalogue` walks `/usr/share/fonts` with `read_dir` the first time a
document names a face it did not embed. So a confined worker asked for a substitute was **killed**
rather than told no — four documents in the first sixty of `doc/pdf.js` did it, and session 914's
fix was `pdf_font::substitute::no_machine_fonts()` before the lockdown: a live worker that
substitutes from the fourteen compiled-in faces and reports the shortfall under §9.10.2. A page
rather than a glyph, in the mount and in the confined viewer both.

ADR 0870 named the fix it did not make: "the broker is unconfined, it already opens the document
and passes the descriptor across with `SCM_RIGHTS` (ADR 0812), and a face that can hand over a
document can hand over a font file."

## Decision

### 1. The layer is a port, and the difference from a permission is the whole design

**Nothing about the worker changes.** Its system-call set is byte-for-byte what
`PERMITTED_INTERPRETER_EXTRA` already held — `git diff crates/pdf-sandbox/` for this round is empty
— it still has no filesystem, no network, `RLIMIT_FSIZE` 0 and `RLIMIT_NOFILE` 8, and **it still
cannot name a path**. What it gains is that it may *ask*:

- the worker sends a **description** — `pdf_font::substitute::Request`'s family, weight and slope,
  the characters a script needs, and how many of the matcher's own answers to pass over. A
  `/BaseFont` out of an untrusted file therefore never becomes a path lookup inside the process
  that parses untrusted bytes, and session 917's ten-class matrix keeps meaning what it means;
- the **broker** — unconfined, and the process that already opens the document — matches that
  description, reads the file, and answers with the face's program and the file's own *name*;
- the worker parses the program it was handed, exactly as it parses an embedded one.

The name rather than the path is a decision and not an economy: a worker has no use for a path, and
sending one would tell a process that parses untrusted bytes where this machine keeps its files, for
nothing.

### 2. `can`, not `must` — the owner's own parenthetical, answered

`confined_transport::Host` answers a resource request with *nothing offered* unless a host has
called `Host::offer`. `pdf_font::provider::offered` answers `None` unless a worker has called
`faces_come_from`. Both defaults are nothing, so **a host that ignores this layer is exactly the
host that shipped before it**: `substitute::find` still never fails, the compiled-in faces still
draw the text, and §9.10.2's coverage note still says what was lost. Nothing here can turn a
substitution into a failure — every path in `provider` answers `None` where anything at all goes
wrong, and `None` is the posture ADR 0870 left.

What it costs a host that declines is one round trip per *distinct description* — memoised in
`provider::OFFERS`, so a page with three missing fonts asks three times and a hundred pages of them
ask no more. That is measured in microseconds and it is why there is no negotiation at start-up: a
capability flag in the greeting would be a second thing that can disagree with the first.

### 3. Laziness is a requirement and it is inherited rather than re-argued

`CLAUDE.md` principle 2 forbids system font enumeration on the launch path. The broker's matcher is
`substitute::machine_face`, which is `catalogue()` behind its existing `OnceLock` — so the index is
built on the **first miss**, in the broker, and a host that opens a document whose fonts are all
embedded never builds one at all. The port added no eager work anywhere: `Host::offer` stores a
closure, and `faces_come_from` stores a function pointer.

### 4. One matcher, not two

`doc/todo/59`: "the broker's matcher, lazy and cached, and where it lives so that both `pdf-vfs` and
`viewer-confined` use one implementation rather than two."

It lives in `pdf_font::substitute` and it is the walk that was already there.
`preferred_paths` is the families-by-endings order `installed_accepted` used to inline;
`covering_path` is `installed_covering`'s search answering a path instead of bytes; and
`machine_face` is the two of them behind one signature. The in-process callers go through the same
two functions, so there is no confined reading of Table 120's descriptor and a separate unconfined
one. `pdf_font::provider::open_a_face` is a decode, that call, and a `std::fs::read`.

### 5. It is a *transport* facility, so neither protocol gains an arm

ADR 0874 declined to make the wire a dialogue for the *ask* level, and the argument was about cost
landing in the wrong place: "a question arriving *instead of* an answer means every `ask` call site
in both brokers has to be able to be re-entered, and the seccomp-confined side has to be able to
block on a reply while holding a half-computed render."

**Both halves of that cost are avoided here by putting the exchange one layer down.**
`confined-transport` reserves two kind bytes — `frame::RESOURCE_REQUEST` (0xF0) and
`RESOURCE_ANSWER` (0xF1), past both protocols' ranges, which number their frames from 1 — and
`Host::read_frame` answers one and goes straight back to reading the frame that *is* the answer. So:

- `viewer_core::Command`, `Event`, `Query`, `Answer` and `pdf_vfs::worker::Query`, `Answer` are
  untouched. Neither vocabulary knows this exists, and `confined_wire`'s fuzz target sees no new
  shape.
- No broker call site is re-entered: `Host::exchange` still writes one frame and returns one, and
  every caller of it is unchanged.
- The confined side *does* block on a reply while holding a half-computed render, and that is fine
  rather than costly: it is a synchronous call inside `pdf_font::substitute`, on whichever thread
  wanted the face, and the host is already blocked reading.

This is the one exception to `confined-transport`'s "no opinion about what crosses it", and it is
argued rather than assumed: the pair is not either protocol's discriminant space, it is a request in
the *other direction*, and the crate still reads none of the payload — the description and the
identity are `pdf-font`'s bytes and this crate hands them across untouched.

### 6. The resource crosses as bytes, and the descriptor that `doc/todo/59` asked for cannot

**This is the round's finding, and it was found by building the thing the item specified.**
`doc/todo/59` said the broker "passes a **descriptor** over the channel §7.5.6's document already
crosses (`SCM_RIGHTS`, ADR 0812), which the worker reads positionally with the `pread64` it already
has." That is what was written first. It worked — and it killed every debug build.

A descriptor arrives as a `std::os::fd::OwnedFd`. The worker reads the font file once and then
**drops** it, and `OwnedFd::drop` asks `fcntl(fd, F_GETFD)` before `close`, under
`core::ub_checks::check_library_ub()`, to catch a double close. `fcntl` is not on the allow-list.
From the worker's own `strace`, on `XiaoBiaoSong.pdf` through `pdf-view-worker`:

```text
write(1, "\360\0\0\0\0\0\0\0\v", 9)       = 9        # the description going out
recvmsg(0, …, cmsg_type=SCM_RIGHTS, cmsg_data=[3] …) = 9
recvmsg(0, {… "\0\0\0\0\0\1AX\1NimbusSans-Regular.otf" …}) = 31
pread64(3, "OTTO\0\f\0\200\0\3\0@CFF …", 82264, 0)   = 82264   # the face, read
fcntl(3, F_GETFD)                         = 0x48
+++ killed by SIGSYS (core dumped) +++
```

The release build survives it and the debug build does not, which is the worst shape a defect can
take: every gate in this tree runs debug binaries.

There is **no way to close a descriptor from safe Rust without that check** — every std type that
owns one closes through `OwnedFd` — and the two ways round it are both refused. Widening the
allow-list is exactly what `doc/todo/61` exists to forbid ("the moment an environment probe is
allowed to widen the policy, every future crash has a cheap fix that costs the boundary a little,
and the boundary is the product"). Leaking the descriptor spends one of the eight `RLIMIT_NOFILE`
leaves, and the ceiling already budgets five for open documents.

So the answer frame carries the identity and the bytes. What that costs is one copy of a file that
is tens of megabytes at worst, in a broker that has the memory, and it is the same arm
`Command::Open` already has for a document held in memory. What is lost against the descriptor
route is nothing this port needed: the worker reads the whole face into memory either way, and the
security property — *the broker opens, the worker never does* — is identical.

**The finding is larger than this port and is recorded rather than fixed here.** ADR 0812's document
descriptor is dropped when a *document* is closed, so the same `fcntl` sits under every confined
close in a build with library-UB checks on. `doc/todo/61` carries it as the fifth instance of its
own class, which is what that item asked for: "find the fifth instance before it finds us."

### 7. Fonts without asking, and why that is defensible rather than merely convenient

The owner's sentence was "access to fonts might be reasonable without user intervention", and it is
— for a reason this program can state rather than assume.

The usual objection to letting a document reach the font set is **fingerprinting**, and
fingerprinting needs a channel back. There is none here. This program has no script engine
(`CLAUDE.md`'s JavaScript exclusion is closed and argued), and the worker has no network — `socket`,
`connect` and `bind` are off the allow-list and Landlock denies every network access besides. So a
document cannot observe which face matched, cannot count how long the match took in any way it can
report, and cannot say anything about this machine to anybody. The port also answers by
*description*, so a document cannot aim it: there is no path in the request and no path in the
answer.

That is why the four levels of `CLAUDE.md` principle 3 are **not** what this layer uses. Those are
about a document's assertions over its reader; this is about a reader's own machine, and the
question a prompt would ask ("may this program use your fonts to draw your document?") has no
adverse party in it.

**What is real, and is the trade this record exists to write down**: with the port on, **the broker
parses the user's own font files in an unconfined process**. That moves attack surface in the wrong
direction. It is smaller than it sounds and it is not nothing:

- the input is the *user's* rather than the *document's*. A document cannot choose which file is
  read; it can only describe a family, and the matcher answers from a fixed catalogue of the
  machine's own installed faces;
- but "smaller" is not "absent". `skrifa` and this tree's own `sfnt`, `cff` and `type1` readers run
  over those bytes in the broker, which is the process with the filesystem. A malformed font
  installed on the machine is then read by an unconfined process where before it was read by a
  confined one — or, before ADR 0870, by an unconfined one anyway, which is the honest baseline: the
  *unconfined* viewer has always done this;
- the direction that is genuinely new is the **confined** faces. `pdf-viewer-confined` and `pdffs`
  were, since session 914, the only configurations in this tree that never parsed a machine font at
  all. Turning the port on gives that property up, deliberately, in exchange for the page.

That is why it is off by default in every host, and why the flag and the setting exist at all rather
than the port simply always being on.

## Consequences

- `crates/pdf-font/src/provider.rs` is new: the description's encoding, the identity's, the
  `MachineFaces` setting, the arming function, the memo, and `open_a_face`. It is the only place in
  the port where a path appears, and it is in the unconfined process by construction.
- `crates/pdf-font/src/substitute.rs` gains `preferred_paths`, `covering_path` and the public
  `machine_face`, and `installed_accepted`/`installed_covering` consult the provider where
  `machine_fonts()` is false. `COVERING`'s memo now holds a path rather than bytes, which is what
  made one matcher possible.
- `confined-transport` gains `frame::RESOURCE_REQUEST`, `RESOURCE_ANSWER`, `MAX_RESOURCE_REQUEST`,
  `MAX_RESOURCE`, `Provided`, `Broker`, `Host::offer` and `link::ask_the_host`. `Host` loses its
  derived `Debug` for a written one, because a closure has nothing to print.
- **`ask_the_host` writes with `rustix::io::write` on `stdout`'s descriptor rather than through
  `std::io::Stdout`, and that is a deadlock avoided rather than a style.** Both workers hold a
  `StdoutLock` for the whole of their serve loop and are *inside* an answer when a face is wanted, so
  a second `stdout().lock()` from the rasterising thread would wait for a lock the answering thread
  does not release until the answer is written. Serialisation is the port's own `Mutex` instead.
- Both `confine()` functions arm the port in the paragraph that already states what the process
  cannot reach — beside `no_machine_fonts()`, `set_isolation` and `available_parallelism` — because
  that is the last moment anything can be decided.
- `pdf_vfs::ConfinedWorkers` is no longer a unit struct: it carries `faces`, and every
  `Box::new(ConfinedWorkers)` is now `ConfinedWorkers::default()`. `ConfinedWorkers::start` takes the
  setting as its fifth argument rather than reading it from anywhere, so a caller cannot get it by
  accident.
