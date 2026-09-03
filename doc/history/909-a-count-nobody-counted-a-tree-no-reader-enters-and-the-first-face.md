# 909 — A `/Count` nobody counted, a tree no reader enters, and the first face

2026-09-04. Argued in [ADR 0860](../adr/0860-the-write-side-has-a-corpus-walk-and-it-found-a-count-nobody-counted.md)
and [ADR 0861](../adr/0861-an-inode-is-a-name-not-a-page-and-the-first-face-is-a-mount.md).
The **fourth** implementation round of [RFC 0003](../rfc/0003-file-system-faces.md), on round 906's
branch because it continues that landing.

Two things were owed and both landed: the walk `doc/todo/58` §5 called "the next round of this
stream's strongest candidate", and the face RFC §7 recommends building first.

Touched: **`crates/pdf-vfs/tests/write_corpus.rs`** (new — the walk);
**`crates/pdf-fuse/`** (new crate — `lib.rs`, `kernel.rs`, `main.rs`, `tests/a_face.rs`);
`crates/pdf-transform/src/update.rs` (`count_of`, `leaves_under`, `the_catalog_reaches`),
`crates/pdf-transform/src/attachments.rs` (one warning);
`crates/pdf-transform/tests/writer.rs` (two removals that now warn);
`tools/worktree.sh` (one copy — see below);
`Cargo.toml`, `Cargo.lock`; `doc/conformance/ledger.toml` (four rows);
`doc/todo/02-every-round.md`, `doc/todo/58-…`, `doc/crate-map.md`, `doc/stack.md`,
`doc/state-of-play.md`; two ADRs, this file.

## 1. What the walk is, and why it is through the core

Every other writer in this tree has a corpus walk; the write side of RFC 0003 had thirteen
assertions about five files against a population of 974. And the five verbs go through a writer no
`pdf-transform` walk touches at all — `Plan::Update`, the in-place editor round 906 had to build
first — so the population was not merely small, it was empty for that writer.

The walk drives `Vfs::write` and `Vfs::remove` rather than `pdf_transform::apply`, because that is
what a face does. Insert a one-page document at `pages/0001.pdf`, delete `pages/0001.pdf`, copy a
file into `attachments/` and take it out again, overwrite `meta/info.json` and write it straight
back — each on its own in-memory backing, each held to §7.5.6's prefix property read off the
*file* after the commit, to the document re-opening at the page count the edit stated, to the
renumbered listing, to §14.7.5.4's `/StructParents` stripped from the carried page and untouched on
every page that was already there, and to **every surviving page drawing bit-identically to the
page it was**.

That last one does double duty and it is why the walk is worth its wall clock. An insertion moves
every ordinal down by one and a deletion of the first moves every ordinal up by one, so the raster
comparison is *between different ordinals* — which makes it a check of RFC §5.2's "[o]rdinal names
are **positions, not identities**" and not only of the writer.

## 2. Three defects, and one thing that is a clause rather than a defect

- **A node with no `/Count` counted as zero.** `count_of`'s doc comment promised "the leaves under
  it counted" and its body was `unwrap_or_default()`. An insertion under such a node wrote
  `/Count 1` over a node that now held two pages, and the two-page document read back as one.
  Trap 28 at the smallest scale it comes in: the comment above the fallback was a claim the code
  had never made. It now walks, reading a node exactly as `pdf_model::count_leaves` does, so the
  number written is the number a reader that disbelieves it will count.
- **An edit into a tree no reader enters.** `issue9418.pdf`'s catalog states no `/Pages` at all,
  so `pdf_model::Pages` recovers it by scanning — and a scan has object numbers where the tree had
  positions, so an insertion "before page 1" came back *after* it. `issue21436.pdf`'s `/Pages`
  names a `/Type /Page` whose `/Parent` is an orphan node, and the splice changed nothing.
  §7.7.2's Table 29 settles both: a page whose `/Parent` chain does not pass through the object the
  catalog names is a page the catalog does not reach. Refused by name on insert and delete alike.
  Repairing the catalog was considered and declined — where it names a *different* tree, choosing
  is guesswork.
- **§7.5.6's own consequence was said on one deletion verb and not the other.** A deleted page
  warned that its bytes stay in the file; a deleted embedded file did not. Found by writing the
  face rather than by the walk: a mount's only channel for such a sentence is a log line, and the
  test that asserts the line found nothing to assert on.
- **Sixteen documents cannot be written twice the same way, and that is §7.6.3.1.** All sixteen
  are AES-encrypted, and `pdf_syntax::write::identify` already had the sentence: a fresh random
  initialisation vector in front of every AES string makes an update differ from one save to the
  next by construction. So RFC 0002 §9's first layer does not bind there. What still does is the
  *length* — the same plaintext under the same crypt filter is the same number of bytes — and the
  walk also counts the encrypted documents whose two updates *agree*, because that count is the
  discriminator between "encrypted" and "AES".

One fact about the instrument that its own numbers need: **883 of the 974 documents have one
page**, and `update` refuses to delete a document's last one. The delete verb is therefore
measured on a tenth of the corpus, which is a property of the pdf.js corpus rather than of the
verb.

## 3. The face, and the one thing a kernel forces

`pdffs <file.pdf> <mountpoint>`, on `fuser` pinned `=0.18.0` — its pure-Rust `/dev/fuse` path,
which on Linux with no `libfuse` feature never calls `pkg_config` at all, so there is no C linkage
and no header to have. Four new locked packages, MIT, and every one of `fuser`'s other
dependencies was already here.

The decision a face cannot avoid is **what an inode is**, and RFC §5.2 has already answered it by
making an ordinal a position: there is no page-shaped thing for an inode to be the identity *of*.
So an inode is a *name* — one number per path, never reused, kept for the life of the mount — the
timeouts are zero, and the `lookup` generation is always 0 because a generation exists to
distinguish files after a number has been *reused*.

The rest is RFC §7's prohibition, kept: no path pattern, no directory name, no generator. The
place that is easiest to break is `ls -l`, where a face wants to know which files are writable, so
the mode bits come from `Vfs::write_meaning` — the core's own layout table answering.

## 4. Where the line between a gate and a person is

**A mount in a gate is a different question from a mount by hand, and this round did the first
only.** `fuser`'s pure-Rust path asks the kernel for `/dev/fuse` and runs `fusermount3`, so a gate
that mounted would be measuring the machine's kernel configuration, its `fuse` group membership
and `/etc/fuse.conf` — none of which is a property of this tree, and any of which turns a gate
into a coin toss. That is `viewer-ffi`'s C-compiler argument and it lands the same way.

So the face is tested where it can be: the inode table, `lookup`/`getattr`/`readdir` over §4's
tree, `open`/`read` and the generation an open file keeps, `create`/`write`/`flush`/`release` with
§7.5.6's prefix read off the file, a write released without a flush, an insertion and a deletion,
and every §5.3 refusal as its `errno` *and* its sentence. What no gate reaches is `fuser`'s wire
format, and the one part of that which is checkable anyway — the `pdf_vfs::Errno` to
`fuser::Errno` table — is held exhaustively against the numbers the core states, so a mapping that
wired `EACCES` to `EPERM` fails. `doc/todo/58` §3 carries the mount by hand as owed work.

## 5. Gates, and trap 13

The change→gate map's core plus everything `pdf-transform` and `pdf-vfs` are under, one corpus
walk at a time on the machine. The poller is `doc/todo/02` §2's own answer — `readlink
/proc/PID/exe`, which is what a process *is* rather than what it was asked to be — and it earned
its place twice in this round: a neighbour's SafeDocs survey and a neighbour's `pages_corpus` were
each running when the walk wanted the machine, and one of the round's own runs was stopped and
re-started because it had begun beside a survey.

**One gate failed for a reason no change of this round could explain, and the fix is in the
worktree script.** `fuzz/Cargo.lock` is gitignored, so a fresh worktree has none and `cargo`
resolves the fuzz workspace from scratch — a *different dependency set* from the one the main
tree, CI and every neighbouring round lint. That resolution took a `tinyvec` that does not compile
on this toolchain, and `doc/todo/02` §2's fuzz `clippy` line failed on a crate no round here has
ever touched. `tools/worktree.sh` now copies the lockfile in beside the corpora, and the line
passes. It is ADR 0742's finding one file over: a thing that is gitignored is a thing a fresh
worktree does not have, and the gate that reads it becomes a different gate without saying so.

**The walk was shown to fail before it was believed.** Its first run failed on real conditions
nobody had planted — three renumbering failures, two rasters that drew differently, sixteen
nondeterministic insertions — which is the strongest form of trap 13's calibration. The
load-bearing raster assertion has no witness left after the fixes, so it was calibrated
deliberately as well: `splice_into_tree`'s `before` was inverted so that every insertion lands one
position late, and the walk was re-run. The figures are in the round's report and not here.

## 6. What the next round of this stream does first

The **read** side has no corpus walk either — `renders/`, `images/`, `text/` and `meta/` are
measured against four committed documents — and it is the same shape and a larger bill.
`Plan::Update`'s output has been read by nobody but us, so RFC 0002 §9's fourth layer wants
extending to it. And somebody has to mount the thing.
