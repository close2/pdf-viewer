# 914 — The read side walks, and a confined worker has no fonts

2026-09-04. Argued in
[ADR 0870](../adr/0870-a-confined-worker-has-no-fonts-and-was-killed-for-looking.md) and
[ADR 0871](../adr/0871-the-read-side-has-a-corpus-walk-and-it-is-the-confined-transport-that-walks.md).
The **seventh** implementation round of [RFC 0003](../rfc/0003-file-system-faces.md), on round 911's
branch because it continues that landing.

One thing was owed and round 911 is the argument for it: `doc/todo/58` §5's "[w]hat is *still*
unmeasured is the **read** side". That round mounted the face by hand, found ten defects, and the
three deepest were reads on documents the four committed fixtures do not carry.

Touched: `crates/pdf-vfs/tests/read_corpus.rs` (new), `crates/pdf-vfs/src/serve.rs`,
`crates/pdf-vfs/tests/confined.rs`, `crates/pdf-vfs/Cargo.toml`;
`crates/pdf-font/src/substitute.rs`; `crates/viewer-confined/src/worker.rs`,
`crates/viewer-confined/tests/confined.rs`, `crates/viewer-confined/Cargo.toml`;
`doc/conformance/ledger.toml` (§9.6.2.2), `doc/todo/02-every-round.md`, `doc/todo/58-…`,
`doc/traps/instruments-and-reports.md` (trap 31), `doc/HANDOVER.md`; two ADRs, this file.

## 1. The walk

`crates/pdf-vfs/tests/read_corpus.rs`, in `write_corpus.rs`'s shape and mounted on the **confined**
worker over a `FileBacking` — the posture a face has. For every corpus document: the whole of
RFC §4's layout listed, every entry `stat`ed (which by §5.5's rule generates), every file read and
held byte for byte against the generator `crate::layout` names, then every listing and every file
read a second time.

```
vfs-read: 974 documents in 324.6s, 24 threads, confined transport, 16 pages a document
vfs-read:   refused open: 2
vfs-read:   documents walked: 972, with no page: 5
vfs-read:   directories listed: 10134, entries stat'd: 18575, files read: 12743 (552.5 MiB)
vfs-read:   /attachments/NAME: 30       /images/NNNN/NAME: 3965   /meta/NAME: 2265
vfs-read:   /pages/NNNN.pdf: 1386       /renders/DPI/NNNN.png: 2745
vfs-read:   /text/NNNN.txt: 1387        /text/document.txt: 965      files are their own generator's bytes
vfs-read:   the tree and the generator both refused: 36, pages past the ceiling: 1155
vfs-read:   refused by name: 38
vfs-read:   not the generator's bytes: 0
vfs-read:   the listing is not the layout's: 0
vfs-read:   a stat that is not the bytes: 0
vfs-read:   read twice, two answers: 0
vfs-read:   a second stat generated again: 0
vfs-read:   the two transports disagree: 0
vfs-read:   panicked: 0
```

The two documents that will not open are §7.6's, by name — one wants a password the corpus does not
record, one states an `/Encrypt` that is not a dictionary. Of the 38 refusals, 26 are a page past
the walk's own pixel ceiling at 300 dpi (`issue19517.pdf` asks for 3 678 693 350 pixels), two are
`/Kids` entries that are not indirect references (§7.7.3.2), two are codecs refusing a malformed
image by name, and **two are §7.6.6's** — `encrypted-attachment.pdf` and `auth-event-ef-open.pdf`
file an embedded stream under `/EFF /StdCF` while `/StmF` and `/StrF` are `Identity`, which is
§7.6.4.1's "[d]ocuments in which only file attachments are encrypted"; `crypt.rs` already names
both files and reads the clause, so the walk reproduced an existing reading rather than finding a
gap.

## 2. What it found, on its first sixty documents

Four of the first sixty **killed the confined generator**, `sig=31 … syscall=257`, `openat`:
`XiaoBiaoSong.pdf`, `SimFang-variant.pdf`, `90ms_rksj_h_sample.pdf`, `ThuluthFeatures.pdf`. Each
names a CJK or Arabic face it does not embed, `pdf_font::substitute` walks `/usr/share/fonts` to
stand in, and `SECCOMP_RET_KILL_PROCESS` does not hand back the `Err` the walk is written to shrug
off. The mount lost the whole generation; **the same code is `pdf-view-worker`'s**, so the confined
viewer loses the page rather than a glyph.

`pdf_font::substitute::no_machine_fonts()`, called by both `confine()`s in the block that already
asks the three other questions a confined process cannot ask afterwards. A confined worker then
behaves as a machine with no fonts installed — which `substitute::find` already guarantees never
fails — and §9.10.2's coverage note is what says a glyph is missing. ADR 0870 has the argument, the
three alternatives and the cost; trap 31 is the general form: **a fallible filesystem call is not a
safe filesystem call inside the confinement, and the population to look at is code that *opens*.**

## 3. Trap 13

Two probes, one per confined worker, each calibrated against the tree without the fix:

- `pdf-vfs`'s `a_confined_generator_can_stand_in_for_a_font_it_cannot_look_up`
- `viewer-confined`'s `a_confined_interpreter_can_stand_in_for_a_font_it_cannot_look_up`

With the `no_machine_fonts` line commented out, the first exits
`ExitStatus(unix_wait_status(159))` — signal 31 — and the assertion prints it; with the line in
place it exits `ALLOWED`. The walk itself is the corpus-scale proof: the four documents' renders
and texts went from a killed worker to bytes identical to the generator's.

## 4. One trap sprung on the way, and it was mine

`the_two_deferred_producers_reach_the_raster_arm_by_name` failed for half an hour and was diagnosed
three ways before the right one: `issue19517.pdf`'s image is JPEG 2000, the sandboxed decoder is a
separate program, and `cargo test -p viewer-confined` does not build another package's binaries.
Trap 10, in the `debug` profile this time rather than in `gates`. `cargo build -p pdf-sandbox
--bins` and it passes.

## 5. Gates

The full `doc/todo/02` §2 sequence, one line at a time and one corpus walk on the machine at a
time — the change reaches `pdf-font`, which is the map's first row, so everything was owed.
Twenty-eight of the twenty-nine lines exit 0. `nextest` is 3274 tests in 69.2 s; the corpus,
oracle, text, census, dates, xmp, jpeg2000, quorra (958 pages, 929 agree, 22 differ, 7 refused),
fixed-documents (71 rows), the transform gate (178.2 pages/s against a floor of 40) and all six
transform walks are green, and so are both `pdf-vfs` walks — `vfs-write` 974 documents in 54.4 s,
`vfs-read` 974 in 324.6 s.

**The one that failed is `foreign_corpus`, it is not this round's, and it did not fail again.**

```
transform-foreign:   bookmarks: §14.7 faults: 1
    bug1997343.pdf: §14.7.5.4: mupdf resolves the source page's parent-tree entry to
    "rrrr…" (90 characters) and ours to "rrrr…" (79)
… test result: FAILED. 0 passed; 1 failed
```

Three things about it. The `bookmarks` lane is round 910's and reached this branch through the
merge of `main` — `git show 92821fcc:crates/pdf-transform/tests/foreign_corpus.rs` names it zero
times and `main`'s names it five, so it is not a lane round 911's gates ever ran. Nothing in this
round's diff can reach it: `pdf-transform` and `pdf-model` are untouched, and the one file in the
shared graph that did change, `pdf-font/src/substitute.rs`, adds three guards on a flag no process
in that test sets. And **re-run alone on a quiet machine the same line passes with
`bookmarks: §14.7 faults: 0`** — the first run had a neighbouring round's walk beside it at a load
average of 32.

That last fact is worth more than the incident. `doc/todo/02` §2 warns that a gate spawning a
reference renderer is measuring two programs and that a loaded machine is a silent third, and it
argues the point entirely in terms of *clocks*. This is the same failure with no clock in it: what
moved was **how much structure `mutool show` resolved**, which reads as a defect in the carry
rather than as contention. §2 now says so.

One line of the sequence therefore stands as a `main` question rather than a round-914 one, and
whoever merges this branch owns it (§2's own merge rule).

## 6. What the faces still need

- **The fidelity ADR 0870 traded for a live worker.** A confined mount and the confined viewer draw
  a page naming an uninstalled face from the compiled-in Latin faces. The shape of the answer is
  ADR 0812's: the broker is unconfined and already hands the document across, so it can hand a
  face across too. `doc/todo/58` §4 carries it, and this walk is the instrument that will say when
  it is closed — the `no_machine_fonts` line comes out of the test and every page has to agree
  again.
- **`doc/todo/58` §5's older shortfalls stand**: `/Collection`'s folder schema, `text/document.txt`
  streamed rather than built whole, the cache's disk half, a `SecretSource` a face implements, a
  cached `images/` listing, and the first `stat` of a long document's `pages/`.
- **§3's next mount by hand**, which round 911 left cheaper than the first one was.
