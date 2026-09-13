# 1030 — A password the mount lends to every generation

Session 1011, general improvement. Status: **accepted**. 1029 was free and 1030 is the number the
round was given; both are recorded here so a later round does not read the gap as a deletion.
Amends `crates/pdf-vfs/src/{lib,worker,confined}.rs`, `crates/pdf-fuse/src/main.rs`,
`crates/pdf-vfs/tests/{a_face,a_write,confined,read_corpus,write_corpus}.rs`,
`crates/pdf-vfs/examples/faces_on_the_port.rs` and `doc/todo/58-the-file-system-faces.md`.

`§N` is ISO 32000-2 and nothing else; quotation marks mean verbatim. Every figure below was
produced in this session by the command beside it.

## 1. What the round asked first, and what a number eliminated

The brief named the population question — the shape that has paid six rounds running — and asked
it of the viewer side, which nobody had. Three candidates, three answers, and two of them are
*nothing to do*, which is worth writing down because a round that reports only what it changed
teaches the next one to skip the elimination.

**`viewer-core`'s vocabulary against what the four hosts reach.** 27 `Command` variants and 32
`Query` variants, counted by parsing the two enums out of `command.rs` and `query.rs` rather than
by grepping for a word, then matched against every construction site in each of the seven crates
above the boundary:

| | reached by `viewer-ui` | by `viewer-ffi` | by no host |
|---|---|---|---|
| `Command`, 27 variants | 27 | 27 | **0** |
| `Query`, 32 variants | 29 | 32 | **0** |

The three `viewer-ui` does not issue by name are `PageLabel`, `Dirty` and `Readback`, and none is
a gap: `PageLabel` reaches all four windows through `viewer_host::page_entry`, which every panel
calls (`viewer-ui/src/bin/quorra/sidebar.rs:139`, `viewer-gtk/src/host.rs:1394`,
`viewer-qt/src/host.rs:1293`); `Dirty` is the poll form of `Event::Dirty`, which all three native
hosts act on; `Readback` is the C ABI's and `viewer-accessibility`'s. `viewer-confined`'s protocol
carries all 32 with an exhaustive `match` the compiler holds, and
`viewer-ffi/tests/every_query_reaches_the_abi.rs` holds the ABI's half. **Nothing to do**, and the
count is the evidence rather than a reading.

**Dead public API across the fourteen crates in this round's lane.** Every `pub fn` name matched
against every use in `crates/`, `tools/`, `raster/` and `kio/`: **two** hits out of hundreds, and
one of them (`StagedId::from_u64`) is the reverse of a conversion that is used, which is a defensible
symmetry. The other is Finding 2 below.

**`raster/`'s conformance corpus against a denominator it produces.** `raster-function-conformance`
holds §7.10.5 cases with an expectation each and a reference evaluator written from the clause, and
its own test compares the two — which would be a crate marking its own homework if that were all.
It is not: `raster/crates/raster-gpu/tests/function_conformance.rs` runs the same cases **on the
device**. **Nothing to do.**

## 2. The finding: every consumer in this tree can be given §7.6.4.1's password except the mount

`pdf-vfs`'s `Vfs::shortfalls` — the list a face prints so that a person is told what the design does
not do rather than discovering it (trap 5) — carried this entry, and it was accurate:

> an encrypted document opens only under §7.6.4.1's default user password: a worker is created per
> generation and `viewer_core::Secret` is deliberately not Clone, so a mount that survived a change
> of the file would need the password re-supplied, and nothing here asks for one yet

`doc/todo/58` §5 carried the same sentence and named the two designs that would close it: "a lending
`Secret`, or a `SecretSource` a face implements". The consequence, measured rather than inferred —
a `Vfs` over `doc/pdf.js/test/pdfs/issue6010_1.pdf` with `InProcessWorkers`, listing `/`:

```
Err(Worker(PasswordRequired("source 0: a password is required")))
```

Every *other* consumer in this tree can be given one. The four windows prompt (§7.6.4.1's own
second step, `viewer_host::password`); `quorra-transform` takes `--password-fd <n>`; the worker
protocol carries one (`pdf-vfs/src/wire.rs:756`, `serve.rs:440`); `ConfinedWorkers::start` already
took `Option<&Secret>`. The whole plumbing existed and stopped one call short of the top:
`Vfs::current_in` passed a literal `None` at the one place it spawns a worker. This is the fourth of
`doc/todo/02` §1's six shapes — **a capability that reached the crate and never reached the
program** — and the evidence that it was felt rather than theoretical is that *both* of this crate's
corpus walks had grown a factory of their own to work around it: `KeyedWorkers(password, …)` in
`read_corpus.rs` and `write_corpus.rs`, the second with a doc comment saying in as many words that
it is "that source, at the seam where it would go".

## 3. The decision: the password is the mount's, lent to every generation's worker

Of the two designs `doc/todo/58` named, the **lending `Secret`** is taken, and it needs no new trait.

- `Workers::spawn` takes `password: Option<&Secret>` rather than `Option<Secret>`.
- `Vfs` holds `Option<Secret>` for its life; `Vfs::with_password` is where a face hands one over and
  `Vfs::new` is unchanged.
- `ConfinedWorkers` passes the borrow straight to the wire (`Secret::reveal`, which it already did).
  `InProcessWorkers` builds a `Secret` of the generation's own through `Secret::new` +
  `push_str`, because `pdf_transform::Source` owns the password it is given.

**Why the lifetime is the mount's and not the worker's**, which is the half the clause decides
rather than the architecture. §7.6.4.1 says "Correctly supplying either password ( owner or user
password) should enable the user to gain access to the document" — access to the document, not to
one reading of it — while RFC 0003 section 5.4 makes every change to the file a *new* worker. A
password held per worker would have to be supplied again every time somebody else saved the file,
and a mount has nobody to ask: the person who could answer is not present at the `readdir` that
needs the answer. That is also why the clause's own remedy is not available here. Its second step
is "the interactive PDF processor should prompt for a password" — a `should`, addressed to an
*interactive* processor — and a FUSE mount is neither at the moment it matters, so the password
goes in when the mount is built, from the face that *was* in a position to ask.

**Why `Secret::new` + `push_str` rather than `Secret::from(String)`** where a copy is unavoidable:
`Secret`'s `Drop` clears the buffer it owns, and `From<String>` can only clear the one it is handed
— "what this cannot do is clear whatever the *caller's* copy was in", says the type itself. Building
through the type's own buffer means every copy of the password in this crate is one a `Drop`
reaches. The `Clone` the type refuses is still refused, and this is not a way around it: the copy is
a `Source`'s and dies with the generation.

## 4. The first face that asks: `pdffs --password-fd <n>`

No interface was invented. The transform suite settled this one and its sentence is the reason:

> **Passwords never appear on the command line.** An argv password is visible in `/proc` and in
> every shell history, so there is no `--password` flag

`pdffs` takes `--password-fd <n>` in both spellings (`--password-fd 3` and `--password-fd=3`, which
is what every valued flag of `quorra-transform` accepts), reads one line from `/dev/fd/<n>`, strips
the line ending, and hands the `Secret` to `Vfs::with_password` before the mount exists. `--password`
is refused **by name**, with a sentence saying why and what to use instead — a person who typed it
has a password in hand and needs to be told where to put it.

**The descriptor number and not the password is what the parsed arguments hold.** `Arguments`
derives `Debug`, `PartialEq` and `Eq`; a `Secret` in it would be a password inside whatever a test
prints. `run` reads the line at the one moment it is needed.

**What this does not reach is the KIO face**, and that is a statement rather than an omission:
`quorra_vfs_mount_open` has no password parameter, so giving the KDE worker its `openPasswordDialog`
is one C entry point and an ABI version bump. `doc/todo/58` §5 now says so.

## 5. What the shortfall says now, and the assertion that had to move with it

The entry is rewritten to what is still true, which is narrower by exactly what landed:

> §7.6.4.1's password is supplied when the mount is built and cannot be supplied later: a document
> that is encrypted after the mount opened, or replaced under it by one wanting a different
> password, is refused rather than asked about, because a readdir has nobody to prompt

`tests/a_face.rs` asserted on the substring `"default user password"`, which the **new** sentence
would also have contained had it been worded a hair differently — trap 27, where an assertion on a
substring passes for every answer that shares it. It names `"cannot be supplied later"` now, which
is the narrowed claim and nothing else.

## 6. The test, and what it fails against

`a_write.rs::a_mount_is_given_the_documents_password_and_keeps_it_across_a_generation`, over
`issue6010_1.pdf` and the password `doc/pdf.js` records for it, which three other test files in this
tree already carry. Three claims, and the third is why the password is the mount's:

1. with no password the document is **refused by name** — `WorkerError::PasswordRequired` — rather
   than served as an empty tree;
2. with the *wrong* password it is still refused, which is what makes the third arm evidence that
   the password is used rather than merely stored;
3. the document changes under the mount (RFC 0003 section 5.4, so a second worker), and `/pages`
   lists the same length without the password being supplied again.

**Calibrated against the defect it looks for** (trap 13): with `self.password.as_ref()` put back to
`None` at the spawn, it fails at claim 3 —
`the password opens the document: Worker(PasswordRequired("source 0: a password is required"))` —
and passes with the line restored.

### 6a. And on the kernel, which is where it has to be true

The unit test drives the core; this drives `pdffs` on a real mount, which is the only thing that
proves the descriptor, the read, the `Secret` and the confined worker are all on one path:

```sh
printf 'abc\n' > pw
quorrafs --password-fd 7 doc/pdf.js/test/pdfs/issue6010_1.pdf mnt 7<pw &
ls mnt              # attachments images meta pages renders text
ls mnt/pages        # 0001.pdf
head -c 120 mnt/text/0001.txt   # Issue 6010
```

and the same document with no `--password-fd`:

```
ls: cannot access 'mnt': Permission denied
pdffs: …/issue6010_1.pdf: stat: source 0: a password is required [EACCES]
```

`EACCES` is FUSE's poverty and the sentence beside it is what this crate insists exists — the
refusal names what is wrong rather than showing an empty tree.

And the two corpus walks now reach the eight documents through the path a face takes rather than
through a factory of their own: `read_corpus.rs`'s `KeyedWorkers` is `TransportWorkers(Transport)`
and knows nothing about any document, `write_corpus.rs`'s is `OneStripWorkers` and exists for the
strip count alone.

## 7. Gates

Run in this worktree with five siblings live in it, so a failure in a file this round does not own
is a neighbour mid-edit and is named as one.

| | |
|---|---|
| `cargo fmt --all --check` | clean |
| `RUSTFLAGS="-D warnings" cargo clippy -p pdf-vfs -p pdf-fuse --all-targets` | clean (one `unused_qualifications` of this round's own, fixed) |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | clean |
| `cargo nextest run --workspace` | **4484 tests run, 4484 passed, 36 skipped** — including `pdf-vfs-ffi::the_kio_worker`, so the C++ face is unmoved |
| `cargo test --workspace --doc` | ok |
| `cargo fmt --manifest-path fuzz/Cargo.toml --check`, `clippy` on it | ok |
| `cargo build --profile gates -p pdf-vfs --bins` | ok (trap 10) |
| `tools/bounded.sh -- … -p pdf-vfs --test write_corpus -- --ignored` | ok in 128 s; 4955 pages bit-identical, every failure column 0, 16 217 questions and 0 repeats the cache cannot explain; peak 3.01 GiB |
| `tools/bounded.sh -- … -p pdf-vfs --test read_corpus -- --ignored` | ok in 152 s; every failure column 0, **45 encrypted documents, 490 files read, 3 refused, 0 killed**; peak 7.74 GiB |
| `cargo run -p conformance --bin quotations` | the three §7.6.4.1 sentences this round quotes are each verbatim in `doc/md/ISO_32000-2_sponsored_EC3.md`, and none appears among the 49 divergences |
| `cargo test -p conformance` | ok, 238 + the file gates |
| `cargo run -p conformance --bin pointers` | no finding names a file this round touched |

**Three of those were red on the first pass and each was a sibling mid-edit**, which is the reason
this table says how it was run: `cargo fmt --all --check` showed four diffs in `pdf-transform`
(1006) and `tools/conformance` (1010), the workspace lint stopped on an untracked
`pdf-font/examples/to_unicode_kind_census.rs` (1008), and `conformance`'s
`workspaces::every_workspace_member_is_scanned` failed naming `conformance::roots::source_roots`
(1010's `roots.rs`, then untracked). All three were re-run after those landed and the figures above
are the second run. A round that had stopped at the first would have reported three failures none
of which were its own — and one that had *only* run its own crates would have reported nothing at
all.

§7.6.4.1's ledger row is `partial` and stays so — this round adds a place the clause is answered
and closes none of what that row still names. The row is **not** edited to list `pdf-vfs`, and
deliberately: `doc/conformance/ledger.toml` was already modified by a sibling when this round
reached it, and a second hand in one file is how a merge loses a sentence.
