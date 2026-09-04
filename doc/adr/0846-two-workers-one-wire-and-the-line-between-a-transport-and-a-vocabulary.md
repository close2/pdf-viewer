# 0846 — Two workers, one wire, and the line between a transport and a vocabulary

Session 902. Status: **accepted**. The first of this round's two records: what `viewer-confined`
and `pdf-vfs` share, what they cannot share, and where the line between the two runs.

## Context

Two programs in this tree now hand a document to a process that has no filesystem and ask it
questions:

- **`pdf-view-worker`** (ADR 0713, ADR 0812): the viewer's document, interpretation and
  rasterisation, confined, with the document crossing as a descriptor sent with `SCM_RIGHTS`.
- **`pdf-vfs-worker`** (ADR 0847, this round): RFC 0003 §6's generator, which produces the files a
  document-as-a-directory offers.

Round 899 built the second one's *seam* and deliberately not the second one's *process*:
`pdf_vfs::worker` states `Query`, `Answer`, `Worker` and `Workers` as plain data and two traits,
with `InProcess` the unconfined implementation, and ADR 0841 §2 records what makes the confined
one a transport change rather than a redesign. `doc/todo/58` §4 then says the thing that makes this
round's order non-negotiable: **no face ships before the confined worker exists**, because a mount
is entered by anything that touches a folder and a file manager will open a document nobody chose
to open.

So the question is not whether to confine. It is what the second confined worker is allowed to
copy from the first. `viewer-confined` is 7 500 lines, and about 600 of them are the wire: a frame
header, a greeting, a socket that passes a descriptor, the supervision of a child that can be
killed at any moment, and the arithmetic that turns an address-space ceiling into a message
budget. Copying those 600 lines would be the shape of defect this project has already paid for
twice — two copies of one rule, drifting.

## Decision

### 1. The transport is shared, and it is a crate

`crates/confined-transport` holds exactly what has no opinion about what crosses it:

| module | what |
|---|---|
| `frame` | one kind byte, one big-endian length, and `MAX_MESSAGE` |
| `greeting` | the caller's eight-byte magic and `pdf_sandbox::lockdown::Confinement` |
| `channel_unix` / `channel_pipe` | the host's end of the worker's standard input, and `SCM_RIGHTS` |
| `link` | the worker's end, read with `recvmsg` so a descriptor arrives with its header |
| `supervision` | `Canceller`, the child, its last words, and how it ended |
| `host` | `Host::start` — spawn, greet, and refuse — and `Host::exchange` |
| `ceiling` | `message_budget`, and the one moment `/proc/self/status` can be read |

`viewer-confined` was moved onto it in the same round rather than left beside it, because a
shared thing nobody has moved onto is a third copy. Its whole test suite — thirty-three tests
including every confinement probe — passes unchanged, which is the evidence that the extraction
changed nothing.

### 2. The vocabulary is not shared, and the line is *the kind byte*

`viewer-confined` carries `viewer_core::Command`, `Event`, `Query` and `Answer`; `pdf-vfs` carries
`pdf_vfs::worker::Query` and `Answer`. Nothing about the two populations is the same, and no
future refactoring will make them so: one is a window's vocabulary and the other is a file
system's.

The line is drawn at one function. **`frame::parse_header` answers a length and hands the kind
back untouched**; each protocol matches the byte against its own set and refuses what it does not
define. A transport that validated the kind would have to hold both protocols' discriminants,
which is precisely the thing that would stop it being shareable — and the temptation is real,
because the version this crate was extracted from *did* validate it.

The same line explains the magic. It is the caller's eight bytes rather than the crate's, so
`PDFVCF05` and `PDFVFS01` are two protocols on one wire and a host that got the wrong program back
refuses at nine bytes rather than at the first answer it misreads. A test in `greeting` holds
that: a greeting under another magic is not a greeting.

### 3. What each side keeps

Three things stayed in `viewer-confined` that could have moved, each for a reason:

- **`ConfinedError`.** It is public API with sentences a host prints, and `Resuming::after` matches
  over it without a wildcard. `TransportError` is the transport's failure population and
  `From<TransportError> for ConfinedError` is a **wildcard-free match**, so a variant added to the
  shared crate stops `viewer-confined`'s build until somebody has said which refusal it is. That is
  `doc/ui-boundary.md`'s rule applied to a boundary the rule did not previously reach.
- **`WORKER_PROGRAM`'s sentence.** `program_beside_executable` is shared; the message a missing
  worker produces is not, because it names *this crate's* build command.
- **The two numbers under `message_budget`.** The arithmetic is shared and its terms are not: the
  viewer subtracts `viewer_core::MAX_PIXELS × 4` for a page's raster and the vfs worker subtracts
  its own `Budget::max_pixels × 4`, and both state how many copies of a message live at the peak.
  `copies` is a `NonZeroU64` rather than a `u64`, because a message that lives in no copies is not
  a message and the division by it is the one arithmetic in the module that could go wrong.

## Consequences

- One frame format, one greeting format, one descriptor route, one supervision. A finding about
  any of them — and ADR 0597's two are both in there, `RLIMIT_FSIZE` killing a worker that writes
  its diagnostic to a file, and the settling allowance a baseline misses — now reaches both
  workers.
- `viewer-confined` loses its `rustix` dependency, which moves to the transport. No new
  third-party crate enters the tree.
- A third confined worker is a `Host::start` and a codec, which is the point.
- The cost is one more crate in a workspace that has twenty-odd, and one more hop when reading
  `viewer-confined`'s wire. Against that: the wire is now readable in one place at 700 lines
  rather than found in three files of a 7 500-line crate.
