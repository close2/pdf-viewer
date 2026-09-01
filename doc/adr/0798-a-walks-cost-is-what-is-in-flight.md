# 0798 — A walk's cost is what is in flight, not what is walked: the corpus bound, and why it is a walk's

Session 866. Status: **accepted**.

## Context

On 2026-09-01 the rounds' corpus campaign ran as this account: eight survey shards over
`batch2/GHOSTSCRIPT`, `standing_count_census` over five batches, gates and builds beside them. At
19:41 anonymous memory stood at about 43 GB; by 19:44 the working set was about 90 GB against 63 GB
of RAM with 47 GB of swap in use; the machine went into a soft lockup and was powered off by hand
at 22:01. No process was under a limit and `systemd-oomd` did not act. The owner asked two things:
whether this is a leak in the viewer, and — if it was bad luck — for the memory to be limited "a
little bit".

`doc/todo/03` §1's method was the campaign's shape: one process per archive or shard, each running
its documents through one rayon `par_iter` over the global pool, which is one thread per core.
Twenty-four cores, eight shards: **192 documents in flight**.

## What was measured

Everything below is one process at a time under `nice -n 19`, the same `safedocs survey` binary
§1's method uses, built in this worktree's `gates` profile, with resident memory sampled once a
second over the whole process tree and the leader's `VmHWM` read beside it. The lists are fixed
and sorted; a shard is a symlink farm under `tmp/`.

**A slice of `batch1/PDFBOX`** (every seventh document, 541 of them), already surveyed by an earlier
round:

| run | documents | peak resident |
|---|---|---|
| whole | 541 | 5.67 GiB |
| first half | 270 | 4.08 GiB |
| second half | 271 | 2.76 GiB |
| every fifth, one rayon thread | 108 | 0.42 GiB |

**A slice of `batch2/GHOSTSCRIPT`** (the first 680 in sorted order — one of the eight shards the
campaign ran), which is where the discriminating numbers are:

| run | documents | threads | peak resident |
|---|---|---|---|
| whole | 680 | 24 | 12.88 GiB |
| first half | 340 | 24 | 1.32 GiB |
| second half | 340 | 24 | 12.58 GiB |
| second half | 340 | 8 | 11.62 GiB |
| second half | 340 | 4 | 11.38 GiB |
| second half | 340 | **1** | **11.16 GiB** |
| every third | 226 | 1 | 0.65 GiB |
| first half of every third (a prefix of the row above) | 113 | 1 | 0.56 GiB |
| whole, `MALLOC_ARENA_MAX=2` | 680 | 24 | 11.72 GiB |

**`standing_count_census` over `batch1`** (3792 documents): 2.05 GiB, in one second.

## What the numbers say

**It is not a leak.** A leak is memory that does not come back between documents, and its
signature is a whole-run peak near the *sum* of the halves' and a single-threaded series that
climbs from the first document to the last. Neither is here. The whole GHOSTSCRIPT slice peaks
where its second half does (12.88 against 12.58 GiB) while its first half peaks at 1.32; doubling a
single-threaded walk from 113 to 226 documents moves the peak from 0.56 to 0.65 GiB; and the
single-threaded series over 340 documents is flat at about 90 MB for five seconds, climbs to
11.16 GiB over the next five, and settles at 2.4 GB for the rest of the run — the last figure being
the allocator holding freed memory below a live allocation, which `MALLOC_ARENA_MAX` moves by a
gibibyte and no more.

**It is a document.** The second half of the slice was walked one document at a time under
`RLIMIT_DATA` of 2 GiB (`tools/bounded.sh --data 2 -- render_at …`), and exactly one of its 340
documents ran out: `GHOSTSCRIPT-688117-0.zip-0.pdf`, 3.2 MB, a Letter-size page, 11 562 objects,
of which **10 260 are image XObjects one sample tall and two to nine samples wide**, each
Flate-encoded at eight bits per component — a producer that wrote a picture as strips. Drawing its
first page costs **10.59 GiB at every scale** — 1.0, 0.5 and 0.25 alike — and interpreting it
without rasterising (`display_list_digest`) costs the same 10.59 GiB. So the cost is the
interpreter's, it is about a mebibyte per image XObject, and it is held for the duration of the
page rather than per image. The survey's verdict on the document is `complete`. The rest of the
slice is under 2 GiB apiece; the PDFBOX slice's 5.67 GiB is the same shape at a smaller size, and
the thread-count series (11.16 → 12.58 GiB from one thread to twenty-four) is this one document
plus twenty-three ordinary ones beside it.

**What multiplied it was the method.** One document at 10.6 GiB is survivable. Eight shards each
holding a pool of 24 threads is 192 documents in flight, and a directory of fuzzed files from a
renderer's bug tracker is exactly the population in which several such documents are in flight at
once; the census beside them read five batches into memory through the same pool; the working set
followed. The 90 GB was the sum of what was open, and nothing bounded what could be open.

**Two things were not the cause and are recorded so nobody looks there again.** The process-wide
caches — `pdf-font`'s predefined CMaps and substitute catalogue, `pdf-model`'s eight-entry press
cache, the sandbox handle — are bounded by what the machine or the standard holds, and the only
unbounded one (`substitute::COVERING`, a memo keyed on the characters asked for) holds a handful of
bytes per distinct request. And the build is not it: a `gates`-profile build of the four binaries
this round needed peaked at 2.31 GiB over the tree, with `sccache` warm; `nextest` over the
workspace at 7.28 GiB is the largest thing a core-gate round runs.

## Decision

**A corpus walk is put under a bound, and the bound is the walk's rather than the process's.**
`tools/bounded.sh --shards N -- <command>` gives each of N side-by-side processes `nproc/N` rayon
threads and `32 GiB / N` of `RLIMIT_DATA`, runs it at nice 19, samples the process tree's resident
memory once a second, and ends with one line saying what the run cost — or that the bound stopped
it. `--data` overrides the share, `--tree GiB` adds a ceiling on the whole tree's resident sum for a
`cargo build`, whose memory is spread over `rustc` processes no single rlimit sees. Four shards is
the cap. `doc/environment.md` carries the agreement and `doc/todo/03` §1 the method.

**Why `RLIMIT_DATA` and not `RLIMIT_AS`.** Since Linux 4.7 `RLIMIT_DATA` counts private anonymous
mappings, which is everything the allocator hands out; `RLIMIT_AS` counts the file mappings and
thread stacks as well, and a rasteriser with a rayon pool has 24 of the latter before it has read a
byte. The tree already uses `RLIMIT_AS` inside the sandbox worker, where the process is
single-threaded and the address space is the point.

**Why not a cgroup.** The agent's processes live in the owner's own session — `/proc/self/cgroup`
puts them under `user-1000.slice/…/app-org.kde.konsole-*.scope` — there is no `user-1001.slice`,
and `systemd-run --user` cannot reach a bus from this account. A cgroup limit is the right
instrument and it is the owner's to set; the recommendation is below.

**Why 32 GiB.** The machine has 61 GiB. Sixteen are the owner's desktop and browser; a parallel
round's gates and a build are about twelve on the figures above; what is left is one walk's.
**One walk at a time** follows, and it is the agreement `doc/todo/02` §2 already makes about a
loaded machine for a different reason. Swap is not in the sum on purpose: 94 GB of swap is what
turned an out-of-memory kill into a soft lockup, and a bound that lets a walk reach it is not one.

**What it costs**, which is trap 18's question. `RLIMIT_DATA` touches no descriptor, so a program
that runs out says so on its own standard error — Rust's allocator prints one line and aborts — and
the wrapper pipes that channel through `tee`, keeps a copy, and reads it back afterwards so that its
last line names the bound rather than the document. A status of 134 *without* that line is a panic
under `panic = "abort"` and the wrapper says so and refuses to name a cause: the first version of
it read every abort as the bound, and the first thing it reported was an 82-byte pageless file
(trap 11). The tree ceiling is the wrapper's own kill and is reported as such. What is genuinely
lost is a sub-second spike between samples; the leader's `VmHWM` is read beside the sum so that
one process's peak is exact. And a shard the bound stops has surveyed *nothing* — it is re-run
with more shards, never recorded as a line.

**And the document is a defect, not a limit to tolerate.** A page of ten thousand two-byte images
costing ten gibibytes to interpret is the case `CLAUDE.md`'s principle 3 names — pathological
content Rust does not protect against — and the mebibyte-per-image cost is a fixed charge that
scales with a count rather than with bytes, which is exactly the shape a budget on bytes cannot
see. `valgrind --tool=massif` names the site: 82 % of the peak is `BTreeMap<Name, Object>::clone`
under `RasterCache::parts` — the cache's miss path clones the resource dictionary the image was
drawn from into every entry and charges `RASTER_BUDGET` the samples alone, so ten thousand
eight-byte rasters hold ten thousand mebibyte dictionaries. `doc/todo/17` carries it, priced three
ways; it is not fixed here because
a change to the interpreter can move a pixel and owes the whole `doc/todo/02` §2 sequence, which a
measuring round beside a gating one may not run.

## Verification

- `tools/bounded.sh --data 1 -- python3 -c 'bytearray(3*2**30)'` ends in `MemoryError` and the
  wrapper's line names the limit; `--tree 1` on a process that touches two gibibytes ends with
  `KILLED BY THE TREE CEILING`; the ordinary case ends with the cost line and status 0.
- `--data 2` on the document above: `memory allocation of 544 bytes failed`, status 134, and the
  wrapper's last line names the data limit.
- The core gates and `cargo test -p conformance` ran through the wrapper under `--tree 28`, which
  is how the build and test figures above were taken.

## What the owner can do, and only the owner

Verified against `systemctl(1)`, `systemd-run(1)`, `systemd.resource-control(5)` and
`oomd.conf(5)` on this machine; not attempted from this account.

- **Give the agent's session a cgroup limit at launch**, which bounds every process it will ever
  start, however many and whatever they are:

  ```sh
  systemd-run --user --scope -p MemoryHigh=36G -p MemoryMax=40G -p MemorySwapMax=4G \
      sudo -u AI bash -lc 'cd /home/cl/projects/pdf-viewer && claude'
  ```

  `--scope` makes a transient scope of the command, `-p` takes the same assignments as
  `set-property`, `MemoryHigh` throttles and `MemoryMax` invokes the kernel's killer *inside the
  unit*. This is the one mechanism that reaches every process regardless of how it was started.
- **Or bound the running konsole scope** without restarting anything:
  `systemctl --user set-property --runtime app-org.kde.konsole-<pid>.scope MemoryMax=40G MemorySwapMax=4G`
  — `set-property` applies resource-control settings at runtime, and `--runtime` keeps a transient
  scope's setting from being written to disk.
- **Why `systemd-oomd` did nothing**: `oomd.conf` here is empty and every unit reads
  `ManagedOOMSwap=auto` / `ManagedOOMMemoryPressure=auto`, and a unit becomes a candidate for
  monitoring only when one of those is `kill`. `systemctl set-property user-1000.slice
  ManagedOOMSwap=kill` (as root) would have had it act at 90 % of swap on the cgroup using the
  most, which yesterday was the right one.
