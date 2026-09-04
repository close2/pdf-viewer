# 0876 — The encrypted page that died was a font lookup, and encryption was the accident

Session 917. Status: **accepted**. The first of this round's two records: what killed the confined
worker on an encrypted document, measured rather than reasoned about, and why the answer is that
this round wrote no fix at all.

## Context

Session 916 built `CLAUDE.md` principle 3's *ask* level across the confinement and, on the way,
met a death it did not chase. `doc/todo/58` §4 records it in that round's words:

> **A page of an *encrypted* document kills the confined worker with `SIGSYS` when it is
> rendered** … `bug1815476.pdf` answers `Query::Consult`, `Query::ExtractPage`,
> `Query::ExtractImages` and the attachment questions through the confinement and dies on
> `Query::RenderPage`; every committed document in `tests/confined.rs` renders. That makes
> encryption the difference

That reading was disciplined and it was wrong, and the way it was wrong is worth more than the
defect. **"Every committed document renders and this one does not" identifies a difference between
two populations of size four and one, and the difference it names is whichever one the reader
already has a word for.** `tests/confined.rs`'s four documents differ from `bug1815476.pdf` in
their encryption, and also in their producer, their fonts, their images, their page count and their
century. Encryption was the salient one because the round was working on §7.6.4.2's permission
bits at the time.

Three deaths of this shape had already been found one at a time — session 902's
`available_parallelism` reading `/proc/self/cgroup`, session 911's allocator sizing an arena from
`/sys` on a pool thread (ADR 0865), session 914's `pdf_font::substitute` walking `/usr/share/fonts`
(ADR 0870) — so a fourth was plausible on its face.

## What was measured

The instruments are the two `doc/todo/58` §4 names, and both work on this machine.
`/proc/sys/kernel/yama/ptrace_scope` is `1`, which restricts `ptrace` to descendants and therefore
does **not** stop `strace` from following a process it starts itself; the kernel's audit line is
available to `dmesg` besides.

Reproduced with the round-916 tree, one confined worker over `bug1815476.pdf`, three questions:

```
PageCount                     -> ok Count(1)
ExtractPage { page: 1 }       -> ok Bytes([37, 80, 68, 70, …])
RenderPage { page: 1, dpi: 150 } -> ERR the confined worker stopped without answering
                                    (killed by signal 31 (SIGSYS: a system call the confinement forbids))
```

The kernel names the call:

```
audit: type=1326 audit(…) uid=1001 pid=1373433 comm="pdf-vfs-worker"
  exe="…/pdfv-r917/gates/pdf-vfs-worker" sig=31 arch=c000003e syscall=257 code=0x80000000
```

`syscall=257` is `openat`, which is where session 911's and session 914's deaths also are, and
`strace -ff -e trace=openat` on the same run names the *path*, which is what settles it. The
worker's last four calls before the corpse:

```
openat(AT_FDCWD, "/sys/fs/cgroup/user.slice/user-1000.slice/cpu.max", O_RDONLY|O_CLOEXEC) = 3
openat(AT_FDCWD, "/sys/fs/cgroup/user.slice/cpu.max",                O_RDONLY|O_CLOEXEC) = 3
openat(AT_FDCWD, "/proc/self/status",                                O_RDONLY|O_CLOEXEC) = 3
openat(AT_FDCWD, "/usr/share/fonts", O_RDONLY|O_NONBLOCK|O_CLOEXEC|O_DIRECTORY)
+++ killed by SIGSYS (core dumped) +++
```

The first three are `confine`'s own, before the lockdown, and are the two questions ADR 0847 and
ADR 0865 already put there. The fourth is `pdf_font::substitute::catalogue`'s `read_dir` over the
machine's font directories, *after* it, and it is ADR 0870's finding exactly: `bug1815476.pdf`
names a face it does not embed, `find` is asked for a stand-in, and
`SECCOMP_RET_KILL_PROCESS` does not return the `Err` that walk is written to shrug off.

**The call path**, then: `Query::RenderPage` → `pdf_transform::apply(Plan::Render)` →
`pdf_model::interpret` → `pdf_font`'s substitution for a named, unembedded face →
`substitute::catalogue()` → `std::fs::read_dir("/usr/share/fonts")` → `openat` → the filter.

## Decision

**This round writes no fix, and merges session 914's.** `round-914` was on a neighbouring branch
when this round opened; merging it and rebuilding the worker makes the same run answer:

```
RenderPage { page: 1, dpi: 150 } -> ok Bytes([137, 80, 78, 71, …])   # a PNG
```

`pdf_font::substitute::no_machine_fonts()`, stated by `pdf_vfs::confine` before the lockdown, is
the whole of it. A second fix at a second layer for one defect would be two things to keep in
agreement, and there is nothing about encryption to fix.

**No regression test is added for `bug1815476.pdf` either, and that is a decision rather than an
omission.** It is a `doc/pdf.js` document, and `tests/read_corpus.rs` — session 914's read-side
walk, on the confined transport — reads every file of the layout for every document in that
corpus. The witness exists; adding a second one named after the accident would enshrine the wrong
diagnosis in a test name.

## Consequences

- **`doc/todo/58` §4 carries one account of this class instead of two.** Session 916's bullet is
  replaced by a sentence saying that it was ADR 0870's defect met through an encrypted document,
  with a pointer here; the ADR 0870 entry stays where the fix is.
- **The count is three, not four.** Three confined-worker kills have been found in this tree, all
  three `openat`, and the fourth report was the third wearing a different document. That matters
  for the *rate* anyone reasons from: this class is not accelerating.
- **The generalisation is what is worth keeping.** A death met on one document is attributed to
  whichever property of that document the finder can name, and the finder is usually working on
  that property at the time. What settles it is the kernel's own audit line and the path in
  `strace`'s output, both of which cost a minute. ADR 0877's sweep is the other half of the
  answer: rather than classify one document, enumerate the classes and put a document of each
  through the confinement.
