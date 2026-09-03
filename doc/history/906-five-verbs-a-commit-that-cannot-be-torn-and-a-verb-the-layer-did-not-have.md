# 906 — Five verbs, a commit that cannot be torn, and a verb the layer did not have

2026-09-03. Argued in [ADR 0854](../adr/0854-a-page-tree-edited-in-place-and-a-commit-that-cannot-be-observed-half-done.md)
and [ADR 0855](../adr/0855-a-write-in-flight-is-in-the-tree-and-not-in-the-document-and-ask-has-nobody-to-ask.md).
The **third** implementation round of [RFC 0003](../rfc/0003-file-system-faces.md), on round 902's
branch because it continues that landing; `main` moved **twice** under this round — round 905's
`optimize` walk and then round 904's soft masks — and each was merged in and the sequence run
whole again after it.

`doc/todo/58` §2 was the whole scope: RFC §5.2's five write verbs, "[e]ach row of `LAYOUT` already
states what it means, so what is owed is the transform call and the transactional shape, not a
design." That sentence was right about four of the five rows and wrong about where the transform
call was going to come from.

Touched: **`crates/pdf-transform/src/update.rs`** (new — the fourth writer) and
`crates/pdf-transform/tests/update.rs` (new); `crates/pdf-transform/src/lib.rs` (`Plan::Update`,
`Origin::Amended`, `apply_borrowed`); `crates/pdf-syntax/src/write.rs`
(`incremental_update_extending`); **`crates/pdf-vfs/src/commit.rs`** (new) and
`crates/pdf-vfs/tests/a_write.rs` (new); `crates/pdf-vfs/src/{lib.rs,worker.rs,wire.rs,
generation.rs,layout.rs}`, `crates/pdf-vfs/tests/{a_face.rs,confined.rs}`;
`crates/pdf-transform/tests/{optimize.rs,optimize_corpus.rs}` (a merge artefact — round 902's
`RenderPlan::strips` beside round 905's new suite, which had never been compiled together); `doc/conformance/ledger.toml` (ten rows);
`doc/rfc/0003-…`, `doc/todo/58-…`, `doc/todo/02-every-round.md`, `doc/state-of-play.md`,
`doc/crate-map.md`; two ADRs, this file.

## 1. The sentence that turned plumbing into a round

RFC §5.2's first row says page insertion is "saved as §7.5.6 append", and RFC §3 promises it of
every write: "[e]very write below is an append … so the *mechanism* needs no new permission."
**`pdf-transform` had no verb that appends to a page tree.** `pages` and `merge` renumber every
object and write a whole new file, which is right for a command line whose output is a path the
caller named and is exactly the rewrite `CLAUDE.md`'s amended exclusion forbids over a document
somebody has open — "never a rewrite of what was there".

So the round's first act was a fourth writer, `pdf_transform::update`: §7.7.3.2's `/Kids` and
`/Count` edited on the node and every ancestor, another document's page closure copied into this
document's numbering and spliced in, §14.3.3's Table 349 entries set, each as replacement objects
and one appended cross-reference section. What a rewrite reconciles and an update cannot is
refused by name (a §12.7 widget on a carried page) or said out loud (`/StructParents` stripped, and
the incoming `/AcroForm`, `/OCProperties`, `/Outlines`, `/Names` and `/StructTreeRoot` each named).
§12.4.2's labels *are* rebuilt, because the clause's indices "shall be fixed, running consecutively
through the document starting from 0 for the first page".

## 2. What the append is actually for

The commit is **not** an append: the file is written whole beside itself, synced, and renamed over
the original. §7.5.6 makes the bare append look free and it is not — the last thirty bytes of an
update are `startxref`, an offset and `%%EOF`, which is what §7.5.5 makes a reader enter the file
by, so a `write(2)` cut short inside them leaves a file naming a cross-reference section that is
not there. A rename cannot be cut short.

What §7.5.6 buys is the **checking**, and that is the better half of the bargain: because the new
file is the old file plus a suffix, RFC §6's broker — which may not parse a PDF — can prove no byte
of the producer's was lost by comparing two byte strings.

## 3. What was decided rather than found

- **A staged write is in the tree and not in the document.** Listed, `stat`-able at the length so
  far, readable back — because every copying tool stats what it just wrote, and a directory that
  named nothing there would fail the copy it had accepted.
- **An abandoned write did not happen**, and a *killed* writer is a different case: the kernel
  flushes a dead process's descriptors, so what stops a torn copy is validation. A file that is
  not a PDF is refused; a PDF cut in thirds **is recovered by this tree's scanner** and inserted
  with the pages the scan found, warned. The round expected a refusal and was wrong: a truncated
  copy and a damaged document somebody meant to insert are the same bytes.
- **Our own commit is ours.** The generation, the writes in flight and the key our last commit left
  are under one lock and the whole commit happens inside it, so no operation sees a tree belonging
  to neither generation, and `Provenance::Ours` distinguishes our edit from somebody else's.
- **A write staged against a generation that has gone is `ESTALE`**, never a clobber.
- **RFC §9's fourth open question is answered *yes***: `meta/info.json` is writable, because the
  file *is* Table 349 and writing back what was read changes nothing.
- ***Ask* is a refusal.** A file system has no dialogue, and proceeding would do the very thing the
  person asked to be consulted about. `viewer_host::unanswerable` is the precedent.

## 4. The defect the both-ways comparison caught

`wire.rs` says "[n]othing is dropped in silence", and its exhaustiveness is on the **enums**: a
`match` over `Answer` will not compile with a variant missing. The decode side matches a *byte* and
has a catch-all by construction, so the round shipped a sixth answer that could be encoded and not
decoded, and every write across the confinement failed with "an answer's kind: 6 is not a kind this
build defines". `tests/confined.rs` asks both workers every question and compares, which is what
turned a one-sided protocol change into a failure rather than a silence. The five write questions
are in that list now.

## 5. Gates

The change→gate map's core, plus everything `pdf-transform` is under: the transform gate and its
six corpus walks, one walk at a time on the machine, the poller matching the gate **binaries**
under a build directory rather than the test names (round 899's lesson). Run whole **three
times**: once on the round's own tree, once after round 905's `optimize` walk was merged in, and
once after round 904's soft masks. A merge runs everything, so the last run is the one that
counts. The results are in the round's report and not here.

**The one gate that failed was a clock, and it was measuring the machine.**
`pdf-transform`'s gate carries RFC 0002 section 12's floor of 40 pages a second and reported 33.3
in the third run, while round 907 was running a `render_at` sweep at 120 % of a core and the
fifteen-minute load average stood at 19.69. Quiet, the same tree reports **198.3** — five times
the floor. Nothing this round touches is on that path; round 904's soft masks, merged in just
before, are not on it either, because the two hundred pages of the standard the gate draws place
no soft mask.

Two lessons, and the second is the one that is new. **The predicate a round waits on has to match
the gate's kind**: this round's poller waited for other rounds' gate *binaries*, which is exactly
right for the memory rule — one corpus walk at a time — and blind to a sweep or a build, which
are what a clock competes with. And **a poller that reads command lines can match itself**: the
replacement matched `ps -eo args=`, whose output includes the polling `grep`'s own arguments,
which contain the very path pattern being searched for, so the predicate could never go empty.
`readlink /proc/PID/exe` is what a process *is* rather than what it was asked to be. Round 899's
deadlock was the same mistake pointing the other way, and `doc/todo/02` section 2 now carries both
beside the two earlier witnesses.

One thing worth carrying out of the running rather than out of the work: the first merge of `main`
did not conflict in any Rust file and still broke the build twice, both times in a test that
neither branch had touched. Round 902 gave `RenderPlan` a `strips` field and round 905 wrote two new
suites that construct one; each side compiled, and only the merged tree has a `RenderPlan`
literal missing a field. A clean three-way merge is not a compiling tree, and the sequence's
first line is what says so.

## 6. What the next round of this stream does first

`doc/todo/58`'s order, minus this round's item: **the FUSE face**, which is the pure-Rust one and
which now has nothing left to discover about the core — `create`, `write_at`, `flush` and `release`
are the kernel's own verbs and `VfsError::errno` is a method. What §2 still owes is three things a
face's requirements should shape: an attachment replaced in place (two updates, one transaction),
what an in-place insertion cannot carry, and whether `update` should be a CLI verb. And **the write
side has no corpus walk** — every read generator is unmeasured against the corpus and so is every
write, which is the strongest candidate this stream has for a gate.
