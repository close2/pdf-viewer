# 920 — A port rather than a permission, and a face that crosses by description

2026-09-04. Argued in
[ADR 0880](../adr/0880-a-port-rather-than-a-permission-what-a-confined-worker-may-be-given-and-by-whom.md)
and
[ADR 0881](../adr/0881-four-hosts-that-can-offer-a-face-and-what-the-port-is-worth-over-the-corpus.md).
`doc/todo/59`, the item the owner accepted on 2026-09-04, built for its first resource.

Touched: `crates/pdf-font/src/provider.rs` (new), `crates/pdf-font/src/substitute.rs`,
`crates/pdf-font/src/lib.rs`; `crates/confined-transport/src/{frame,host,link,lib}.rs`;
`crates/pdf-vfs/src/{confined,serve,lib}.rs`, `crates/pdf-vfs/Cargo.toml`,
`crates/pdf-vfs/examples/faces_on_the_port.rs` (new), `crates/pdf-vfs/examples/vfs_cost.rs`,
`crates/pdf-vfs/tests/{confined,read_corpus,awkward_classes}.rs`;
`crates/viewer-confined/src/{lib,worker}.rs`, `crates/viewer-confined/tests/confined.rs`;
`crates/viewer-host/src/{policy,lib}.rs`; `crates/viewer-ui/src/bin/pdf-viewer-confined.rs`;
`crates/pdf-fuse/src/main.rs`; `crates/pdf-vfs-ffi/src/tree.rs`;
`doc/conformance/ledger.toml` (§9.8.1, §9.10.2), `doc/todo/59-…`, `doc/todo/61-…`,
`doc/traps/instruments-and-reports.md` (trap 32), `doc/HANDOVER.md`,
`doc/questions/Q24-…`; two ADRs, this file.

**`crates/pdf-sandbox/` is not in that list and that is the point of the round.** `git diff` on it
is empty: the allow-list did not move, and the worker's traced system calls after the filter is
installed are a subset of what it already made.

## 1. What the port is

The worker asks by **description** — `substitute::Request`'s family, weight and slope, the
characters a script needs, and how many of the matcher's answers to pass over — and the broker
matches, reads and answers with the face's program and the file's own *name*. No path in either
direction, so a `/BaseFont` out of an untrusted file never becomes a path lookup inside the process
that parses untrusted bytes.

It sits in the **transport** rather than in either protocol: `confined_transport::frame`'s
`RESOURCE_REQUEST` and `RESOURCE_ANSWER`, answered inside `Host::read_frame`, which then goes
straight back to reading the frame that *is* the answer. So `viewer_core`'s vocabulary and
`pdf_vfs::worker`'s are untouched and no broker call site is re-entered — the two costs ADR 0874
declined to pay for the *ask* level are avoided rather than paid.

Nothing is offered by default at either end: `Host::offer` is what a host calls, `faces_come_from`
is what a worker calls, and a host that does neither gets the worker session 914 left.

## 2. The descriptor that `doc/todo/59` specified, and why it is bytes

The item said a descriptor beside the frame, as ADR 0812's document crosses. That was written
first, it worked, and it **killed every debug build**:

```text
write(1, "\360\0\0\0\0\0\0\0\v", 9)       = 9
recvmsg(0, …, cmsg_type=SCM_RIGHTS, cmsg_data=[3] …) = 9
recvmsg(0, {… "\0\0\0\0\0\1AX\1NimbusSans-Regular.otf" …}) = 31
pread64(3, "OTTO\0\f\0\200\0\3\0@CFF …", 82264, 0)   = 82264
fcntl(3, F_GETFD)                         = 0x48
+++ killed by SIGSYS (core dumped) +++
```

`OwnedFd::drop` asks `fcntl(fd, F_GETFD)` before `close`, under
`core::ub_checks::check_library_ub()`. `fcntl` is not on the allow-list. There is no way to close a
descriptor from safe Rust without that check, widening the filter is what `doc/todo/61` exists to
forbid, and leaking it spends one of eight `RLIMIT_NOFILE`. So the resource crosses as bytes — one
copy of a file that is tens of megabytes at worst, in the process that has the memory, and the
security property is identical because the broker is what opens either way.

**Trap 32**, and it is not a repeat of 31: no filesystem call appears in the code, and the *release*
build survives while every debug one dies — which is "the thing I ran by hand works and the gate is
red".

## 3. The instance that is still open, and it is the document's own descriptor

`doc/todo/61` §1 asks for the fifth instance of its class before it finds us. This is it, and the
claim that nothing in this tree closes a document in a confined worker was one nobody had run. So it
was run:

```text
cargo nextest run -p viewer-confined -E 'test(a_document_closed_in_the_confined)'
    -> killed by signal 31 (SIGSYS: a system call the confinement forbids)
PDF_VIEW_WORKER=…/release/pdf-view-worker   (the same command)
    -> PASS
```

`viewer-confined`'s `a_document_closed_in_the_confined_process_leaves_a_worker_that_still_answers`
is that witness, `#[ignore]`d with the reason on it. **It is not fixed here**: the only fix that
leaks nothing is a seccomp rule for `fcntl` narrowed by argument to `F_GETFD` — which
`lockdown_linux.rs` already anticipates the mechanism for — and widening the allow-list, even by a
read-only query about a descriptor the process already holds, is `doc/todo/61`'s decision to make
deliberately rather than a side effect of a round about fonts.

## 4. The hosts

| face | how | default |
|---|---|---|
| `pdffs` | `--machine-fonts` | withheld |
| `pdf-viewer-confined` | `--machine-fonts`, or `PDF_VIEWER_MACHINE_FONTS=on` | withheld |
| the KIO worker, and any C host of `pdf-vfs-ffi` | `PDF_VFS_MACHINE_FONTS=on` | withheld |

The environment variables are a poor interface and are what a window started from a desktop entry
and a KIO worker started by `kioworker` have; `doc/todo/38`'s rule that no user interface is built
until it is asked for binds here as it did in ADR 0875. The three in-process windows and
`pdf-transform` are not confined and have the fonts already.

## 5. What it is worth

`crates/pdf-vfs/examples/faces_on_the_port.rs`: page one at 150 dpi, three columns — this process
unconfined with the machine's fonts, a confined worker offered nothing, a confined worker with the
port on — compared byte for byte against the first.

The four documents ADR 0870 named:

```text
document                         here   withheld    offered   verdict
XiaoBiaoSong.pdf                10.02       9.67      10.02   the port pays ADR 0870's cost back in full
SimFang-variant.pdf              2.55       2.54       2.55   the port pays ADR 0870's cost back in full
90ms_rksj_h_sample.pdf           0.25       0.11       0.25   the port pays ADR 0870's cost back in full
ThuluthFeatures.pdf             16.48      16.48      16.48   nothing was owed: withheld already matched
```

The fourth is the one to read: it was *killed*, not degraded, and it turns out this machine offers
nothing better for that Arabic face than the compiled-in one. A record that had said "four
documents lost their page" would have been wrong about one of them.

Over all 974 documents of `doc/pdf.js`, in 332 s under `tools/bounded.sh --data 12 --tree 12` at a
2.59 GiB peak: **40** differed and are now byte-identical to the unconfined answer, 918 owed
nothing, 16 refused in every column, and **0** are offered and still different. **Twelve of the
forty went from a blank page to a drawn one** — `issue2840.pdf` 0.00 → 21.20 ink, `issue5244.pdf`
0.00 → 15.33, `issue9084.pdf` 0.00 → 14.17, and nine more — which is §9.7.4.2's case, where a
composite font's substitute is reachable only by character and `None` is a page with no glyphs at
all.

**The first run of that measurement was wrong and said so plausibly**: 124 documents "still short",
of which 119 had a reference page with no ink. `pdf-sandbox-worker` was not beside the example's
binary, so the unconfined column decoded no JBIG2 and the confined ones did (ADR 0218 — a confined
process cannot spawn). Trap 10 wearing trap 16's clothes. The example refuses to run without the
decoder now, and the same command then reports 0.

## 6. Gates

The full `doc/todo/02` §2 sequence, in a worktree branched from `main`'s merge of round 917, one
line at a time and waiting on `/proc/PID/exe` for a neighbour's walk before every bounded line. The
change reaches `pdf-font`, which is the map's first row, so everything was owed — and this is a
fifth round, so everything was owed twice over. **All twenty-four lines exit 0.**

`cargo fmt --all --check`, `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"`, and
both `fuzz/` lines: silent. `nextest` is **3303 tests in 69.2 s**, one slow, 31 skipped — one of the
31 being this round's own `#[ignore]`d witness. The doctest line is green.

| line | |
|---|---|
| corpus | 974 documents in 10.8 s: 0 unopenable, 9 locked, 1 encrypted beyond us, 5 pageless, 64 incomplete, 0 slow |
| oracle | 979 agree, 61 contradicted, 836 ambiguous, 47 not comparable |
| quorra | 958 pages in 34.3 s: 929 agree, 22 differ, 7 refused, 16 not comparable |
| fixed documents | 71 checked, 0 absent, 71 rows |
| transform gate | 101.2 pages/s against RFC 0002 §12's floor of 40 |
| the five transform walks | writer 8.1 s, split 60.6 s, merge 134.5 s, pages 166.3 s, optimize 30.2 s, foreign 203 of 974 in 84.5 s — every fault count 0 |
| `vfs-write` | 974 documents in 43.7 s |
| `vfs-read` | 974 documents in **1503.7 s**, 12 743 files read (552.5 MiB), *not the generator's bytes: 0*, *the two transports disagree: 0* |
| `vfs-awkward` | 8 roots, 3916 documents classified, 258 chosen, **0 killed** in every one of the ten classes |
| conformance | 218 tests |

**Then `main` moved and the sequence was owed again on the merged result.** Round 919 landed
between this round's branch point and its commit: `crates/corpus-classes` is new, `read_corpus`
walks every corpus root on the disk rather than `doc/pdf.js` alone, and `pdf-vfs`'s
`awkward_classes.rs` is gone into it (ADRs 0878, 0879). Three of the merge's four files conflicted
and each was resolved rather than taken — `pdf-vfs`'s manifest keeps both dev-dependencies,
`doc/todo/59` keeps round 919's answer to its own item 4 beside this round's, and `doc/todo/61`'s
list keeps 919's first two items and gains this round's third.

On the merged tree the core is green again — `fmt`, `clippy --workspace` and both `fuzz/` lines
silent, **3303 tests**, the doctests, conformance — and the widened read walk runs:

```
vfs-read: 1132 documents in 329.8s, 24 threads, confined transport, 16 pages a doc/pdf.js
          document and 2 of every other root's
vfs-read:   documents walked: 1117, files read: 14274 (784.8 MiB)
vfs-read:   not the generator's bytes: 0 … the two transports disagree: 0 … panicked: 0
vfs-read:   0 killed, in every one of the ten classes
```

That last line is the one this round's change most needed: the port arms both confined workers
unconditionally, so **every** document in that population now sends a resource request on its first
missing face, to a broker that offers nothing — and nothing died, nothing changed, and the two
transports still agree byte for byte.

`vfs-read` took 1503.7 s against session 914's 324.6 s, and the reason is the machine rather than
the tree: a neighbouring round's own `read_corpus` was walking the corpus beside it for most of it.
`doc/todo/02` §2's rule about a loaded machine is about *clocks*, and this line has no assertion on
one — every count it holds is exact and every one of them is the same as 914's.

**§5's binaries were rebuilt and installed**, which this round owed twice: it is a fifth round, and
the measurement in §5 above is a measurement.

## 7. What the item still owes

`doc/todo/59` carries it: **the second resource** (ICC profiles, §14.11.5 and RFC 0006 §5.3, for
which the transport already carries opaque bytes and knows nothing about fonts); **a way for a
person to choose** rather than a flag and two environment variables, which is `doc/todo/38`'s
sentence and not this round's to break; and **`CLAUDE.md` principle 3's amendment**, which is the
owner's own sentence — `doc/questions/Q24` proposes the exact wording and this round deliberately
did not touch the file.

And `doc/todo/61` §3 is owed by somebody: the document's descriptor, dropped on close, kills a debug
worker today.
