# 917 — The encrypted page was a font, and the classes are swept

2026-09-04. Argued in
[ADR 0876](../adr/0876-the-encrypted-page-that-died-was-a-font-lookup-and-encryption-was-the-accident.md)
and [ADR 0877](../adr/0877-the-awkward-classes-are-enumerated-and-swept-rather-than-waited-for.md).
On its own branch, carrying round 916's stream and round 914's fix, which is the round's first
result: the two were already the same defect.

One thing was owed: `doc/todo/58` §4's "**a page of an *encrypted* document kills the confined
worker with `SIGSYS` when it is rendered**", recorded by round 916 and not chased there.

Touched: `crates/pdf-vfs/tests/awkward_classes.rs` (new); `doc/todo/58-…`,
`doc/todo/02-every-round.md`, `doc/habits.md` (*Measuring*); two ADRs, this file. **No crate source changed at all**, and that is
the finding rather than an omission.

## 1. It was `openat`, and it was the fonts

Reproduced on the round-916 tree, one confined worker, three questions of `bug1815476.pdf`: the
page count and the extracted page answered, `RenderPage` died. `dmesg` names the call —

```
audit: type=1326 … comm="pdf-vfs-worker" exe="…/pdfv-r917/gates/pdf-vfs-worker"
  sig=31 arch=c000003e syscall=257 code=0x80000000
```

— and `strace -ff` names the path, which is what settles it. `/proc/sys/kernel/yama/ptrace_scope`
is 1 on this machine, which restricts `ptrace` to descendants and so does *not* stop `strace` from
following a process it starts itself; both instruments `doc/todo/58` §4 named work here.

```
openat(AT_FDCWD, "/proc/self/status",                                O_RDONLY|O_CLOEXEC) = 3
openat(AT_FDCWD, "/usr/share/fonts", O_RDONLY|O_NONBLOCK|O_CLOEXEC|O_DIRECTORY)
+++ killed by SIGSYS (core dumped) +++
```

`Query::RenderPage` → `pdf_transform::apply(Plan::Render)` → `pdf_model::interpret` → a stand-in
for a face the document names and does not embed → `pdf_font::substitute::catalogue()` →
`std::fs::read_dir` → `openat`. **That is ADR 0870 exactly**, which round 914 had already found and
fixed on a neighbouring branch, five sessions of shape apart and one day apart in time. Merging
`round-914` and rebuilding the worker, the same run answers with a PNG.

So this round wrote **no fix**. A second fix at a second layer for one defect is two things to keep
in agreement, and there is nothing about encryption to fix. No regression test named after the
accident either: `bug1815476.pdf` is a `doc/pdf.js` document and round 914's `read_corpus.rs` reads
every file of its layout through the confinement already.

**What is worth keeping is why the misattribution happened**, because it was not careless. "Four
committed documents render and this one does not" names a difference between populations of four
and one, and the difference it names is whichever property the reader already has a word for —
round 916 was working on §7.6.4.2's permission bits at the time. ADR 0876 §Consequences has it.

## 2. The classes, swept rather than waited for

`crates/pdf-vfs/tests/awkward_classes.rs`. Ten classes — encrypted, locked, an encryption this
reader does not implement, pageless, damaged, unopenable, huge, JBIG2, JPEG 2000, and plain as the
control — each filled by *classifying* a stride-sample of every corpus root on the disk, and the
whole layout of a few of each walked from `/` through the **confined** transport. The question is
survival, not agreement, which is what makes it cheap enough to reach eight roots.

```
vfs-awkward: 8 root(s), 3916 document(s) classified, 258 chosen
vfs-awkward:   encrypted                33 document(s), 306 answered (32.8 MiB), 0 refused, 0 killed
vfs-awkward:   locked                   12 document(s),   0 answered,            12 refused, 0 killed
vfs-awkward:   encryption unimplemented  2 document(s),   0 answered,             2 refused, 0 killed
vfs-awkward:   pageless                  9 document(s),  45 answered,             0 refused, 0 killed
vfs-awkward:   damaged                  39 document(s), 360 answered (33.2 MiB),  0 refused, 0 killed
vfs-awkward:   unopenable                8 document(s),   0 answered,             8 refused, 0 killed
vfs-awkward:   huge                     33 document(s), 354 answered (89.7 MiB),  6 refused, 0 killed
vfs-awkward:   jbig2                    27 document(s), 258 answered (38.0 MiB),  3 refused, 0 killed
vfs-awkward:   jpeg 2000                32 document(s), 284 answered (74.6 MiB), 11 refused, 0 killed
vfs-awkward:   plain (control)          63 document(s), 567 answered (58.8 MiB),  3 refused, 0 killed
vfs-awkward: killed: 0, did not recover: 0, in 25.0s
```

Nothing dies. The 44 refusals are twelve passwords, nine documents that are not PDFs or have no
usable cross-reference table, two encryptions §7.6 states and this tree does not implement, and
twenty-one pages past the walk's pixel ceiling — every one a sentence a face can show, which is
`doc/todo/58` §4's fourth requirement met rather than owed.

## 3. Trap 13, and the sentence the calibration produced

`no_machine_fonts()` commented out, the worker rebuilt, the same 258 documents:

```
vfs-awkward: killed: 76, did not recover: 0, in 25.9s
```

in **six of the ten classes**: huge 26, damaged 16, plain 14, jbig2 8, encrypted 6, jpeg 2000 6.
**The control class has more kills than the encrypted one.** That is round 916's misattribution
reproduced at corpus scale and is the round's best argument for the instrument existing: one
document cannot say which of its properties killed the worker, and a population wide enough for the
classes to disagree can.

`did not recover: 0` beside 76 deaths is session 902's recovery measured for the first time.

## 4. Two things got fixed in the instrument, both trap 11

- The recovery check first asked for `Vfs::pages()` to answer `Ok`, and reported eleven failures
  that were a locked document, an unopenable one and an encryption this reader does not implement —
  each of which answers `Err` for ever and is right to. It asks now that the answer is not a corpse.
- The first run failed on trap 10's *third* copy: `pdf-sandbox-worker` in this worktree's `gates`
  directory was the pre-merge build, and the sweep says so by name rather than sweeping the two
  codec classes with their images missing.

## 5. The policy did not change

No system call was admitted, `doc/todo/34` is untouched, and `pdf_sandbox::lockdown`'s allow-list is
what it was. The honest fix for this class is never to widen the filter — ADR 0870 argues that at
length — and this round found nothing that needed it.

## 6. Gates

The whole `doc/todo/02` §2 sequence, one line at a time, one corpus walk on the machine at a time —
the branch merges three others and §2's merge rule admits no map. **Run twice, before and after
merging `main`, and all thirty lines exit 0 both times.** The figures below are the second run's.

- `cargo nextest run --workspace`: 3296 tests run, 3296 passed, 30 skipped, 69.1 s.
- The corpus, oracle, text, both censuses, dates, xmp, jpeg2000 and fixed-documents gates green;
  quorra 958 pages compared in 30.4 s, 929 agree, 22 differ, 7 refused; the transform gate 205.5
  pages/s against a floor of 40.
- All six transform walks green, `foreign_corpus` included — and **that is round 914's one open
  line answered on this branch**: it prints `bookmarks: §14.7 faults: 0`, where that round's first
  run saw one fault under a neighbour's walk at a load average of 32. `merge` 2 and `pages` 1 are
  the gate's own accepted figures.
- Both `pdf-vfs` walks green: `vfs-write` 974 documents in 45.5 s, `vfs-read` 974 in 278.5 s
  through the confined transport, 12 743 files read and 0 disagreements — the second corpus-scale
  confirmation that the font kill is gone.
- `awkward_classes` as printed above, and `cargo test -p conformance` green with the new ADRs,
  todo edits and habit in the tree.

The three lint findings on the new file were fixed rather than allowed: a division and two
additions under `arithmetic_side_effects` (`checked_div`, `saturating_add`, `split_once`), a
`match` that is a `let … else`, and a test function 163 lines long, split into `choose`, `sweep`
and `report`. A fourth was an *unfulfilled* `clippy::panic` expectation, which is its own small
lesson: this workspace permits a panic inside a `#[test]` function, so the expectation is only
fulfilled when the panic is in a helper — which is where `read_corpus.rs` already puts its own, and
where this one went.

## 7. What is left

- **Two instruments ask half a question each.** `read_corpus.rs` compares bytes over `doc/pdf.js`;
  `awkward_classes.rs` asks about survival over all eight roots. `doc/todo/58` §4 says how they
  merge, and the merge is the walk's: widen its population and delete this file.
- **`viewer-confined` has the probe and not the sweep.** The same ten classes through
  `pdf-view-worker` is the same instrument against the other confined program, and nothing has run
  it.
- **The fidelity ADR 0870 traded for a live worker** is still owed, unchanged: a confined mount
  draws a page naming an uninstalled face from the compiled-in Latin faces, and the broker is the
  place that can hand a font across.
