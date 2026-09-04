# 913 — The second face, thirty-five functions, and an error mapping the RFC got wrong

2026-09-04. Argued in [ADR 0868](../adr/0868-a-boundary-of-thirty-five-functions-and-a-refusal-that-is-an-object.md)
and [ADR 0869](../adr/0869-the-rfcs-error-mapping-does-not-survive-kio-and-the-ask-has-a-channel-and-no-wire.md).
The **sixth** implementation round of [RFC 0003](../rfc/0003-file-system-faces.md), on round 911's
branch because it continues that stream. Both of the RFC's faces now exist.

Touched: **`crates/pdf-vfs-ffi/`** (new crate — `lib.rs`, `abi.rs`, `tree.rs`, `refusal.rs`,
`status.rs`, `include/pdf_vfs.h`, `c/browse_a_document.c`, four tests);
**`kio/`** (new, and outside the cargo workspace — `CMakeLists.txt`, `src/pdfworker.{h,cpp}`,
`src/pdf.json`, `test/drive_the_worker.cpp`);
`Cargo.lock`; `doc/conformance/ledger.toml` (one sentence on §7.5.6);
`doc/todo/02-every-round.md`, `doc/todo/58-…`, `doc/crate-map.md`, `doc/stack.md`,
`doc/state-of-play.md`; two ADRs, this file.

Nothing in `crates/` outside the new crate changed. That is the round's own evidence for RFC §7's
"the faces contain *no* layout knowledge": a second face was built and the core did not move.

## 1. Which branch, and what it merged

`round-911`, which had committed by the time this round opened its worktree — its own tip is round
909's plus the mount by hand. `main` was merged afterwards and brought rounds 910, 912 and the
round-867 merge with it. Two conflicts, both the shape a parallel stream produces: `doc/todo/58`,
where round 911 and this round each rewrote the same three paragraphs about a face, and
§12.4.2's ledger row, where round 906 added `update.rs` to its `code` list and round 910 added
`split.rs` — resolved as a union of the lists and both sessions' sentences in session order.

## 2. The four places this ABI differs from `viewer-ffi`, and why each is not a preference

`viewer-ffi` is the instruction rather than an analogy: RFC §7 names it. So the shape is copied
without variation — a verb is a function, an answer is an owned handle, bytes cross by copy, the
`unsafe` is one module and a test counts its tokens — and the differences are the round's actual
work. ADR 0868 has them in full; in one line each:

- **A refusal is an object.** Both halves — `pdf_vfs::Errno` and §5.3's sentence — have to survive
  the call, and the two candidates that are not an object each fail for a stated reason: a wider
  status enumeration folds the caller's own mistakes into the document's answers, and a
  last-error slot is a global that calls which did not fail also write.
- **The counted enumeration is `errno`.** `refusal::tag` is an exhaustive `match` whose only
  purpose is that it cannot be written incompletely, so a fourteenth kind in the core is a
  compile error here and a startup refusal in a compiled C caller.
- **The staged four are absent.** RFC §5.4 says a KIO `put` "is already transactional"; the
  kernel's four verbs are the FUSE face's, and four entry points nobody calls would be four more
  shapes to keep frozen.
- **`pdfvfs_split` is on this boundary rather than in the C++**, because a machine with no KDE can
  still test it — and it answers a *length* rather than two strings, so the commonest call in the
  face allocates nothing.

## 3. What KIO decided, and the one thing the RFC got wrong

**RFC §5.3 says refusals are reported "as `ERR_UNSUPPORTED_ACTION` / `ERR_WRITE_ACCESS_DENIED`
with the sentence", and KIO does not work that way.** For almost every code in `KIO::Error` the
string a worker returns is a *parameter* substituted into KIO's own canned message, so
`ERR_WRITE_ACCESS_DENIED` with our sentence renders as "Could not write to «two lines about why a
page's text is not a byte stream»". The sentence is the entire point of §5.3 — it is what a mount
cannot carry and this face can — so a refusal that carries its reason is `ERR_WORKER_DEFINED`,
whose string KIO shows verbatim, with the `errno` name appended for anything reading the string.
Four errnos keep a canned code because KIO's words for them are true and shorter than ours.

Two more, both performance decisions that become correctness ones once they are wrong: a `listDir`
states **no** size, because §5.5 makes a `stat` generate and a listing that stated sizes would
rasterise every page of `renders/300dpi/`; and `mimetype` is answered from the name, because KIO's
documented fallback for a worker that does not implement it is to issue a whole `get`.

## 4. The *ask* level: a claim this round found false

`doc/todo/58` ended with "[n]othing in the core has to change for it" — a KIO worker can put a
question, so the *ask* level could be implemented. **It is false**, and building the face is what
showed it: the restriction decision is taken inside the confined generator, and RFC §6 gives that
process no channel to a person **by construction**. `messageBox` is necessary and not sufficient.

So the *ask* is not wired, and ADR 0869 §3 costs the two ways out — make the wire a dialogue,
which changes the protocol both confined workers in this tree speak, or ask first in two round
trips with a `Query` that answers *would this be restricted, and why* — and recommends the second.
What **is** wired is the *warn* level, through KIO's non-modal `warning()`, and the harness
observes it arriving at a job as `KJob::warning` carrying §7.5.6's own sentence to the person
deleting the page. That is the first place in this tree where that clause's consequence is shown
rather than logged.

`warning()` rather than `messageBox()` is itself a decision: a deletion *always* produces that
sentence, so a modal dialogue per `rm` is a face nobody keeps installed.

## 5. What was actually driven, and what was not

Four instruments, and they answer different questions:

| | asks | skips when |
|---|---|---|
| `header_and_library_agree.rs` | do the header and `src/abi.rs` state the same names and numbers | never |
| `a_c_program_drives_the_abi.rs` | does a C compiler accept the header and a linker find the symbols | no `cc` |
| `unsafe_position.rs` | is the `unsafe` where the crate says it is | never |
| `the_kio_worker.rs` | does **KIO** load the plugin and answer | no `cmake`, ECM, Qt 6 or KF6 |

The last is the one that matters and the one that could not be assumed. `kio/test/
drive_the_worker.cpp` is a KIO **client** — a `QCoreApplication` running the jobs Dolphin runs —
so KIO reads the plugin's embedded metadata, decides `pdf:` is served there, forks `kioworker`,
loads `pdf.so`, and every command crosses a socket. It listed the root and `pages/`, `stat`ed a
page and `get` exactly the bytes the `stat` promised, took all three §5.3 refusals as the core's
own sentences, watched the warn channel arrive, and deleted and inserted a page through KIO's own
`del` and `put` while the listing renumbered 5 → 4 → 5.

**It is not Dolphin**, and that is the round's honest limit: nothing here sees a session, so
nothing says how a listing renders, whether the `archiveMimetype` association makes a click on a
PDF *enter* it rather than open it, what drag and drop out of `pages/` does, or what a refusal's
dialogue looks like. `doc/todo/58` §3 carries it as owed, in the same shape the mount by hand was
owed before round 911 did it.

One thing the harness found that would otherwise read as a defect: **KIO appends a "." entry to
every listing itself.** `pdfvfs_list` never answers one, so the harness drops it; a count that did
not would be one larger than the document's on every directory.

## 6. How the build stays optional, and trap 13

`kio/` has no `Cargo.toml`, is named by no manifest and reached by no build script, so
`members = ["crates/*", "tools/*"]` cannot see it and `cargo` on a machine with no KDE is
untouched. What builds it is a Rust test that **skips, printing what CMake could not find**, where
the toolchain is absent — and *fails* on any other configure error, because a plugin that will not
compile against a toolchain that is there is a defect rather than an absence.

That predicate was run against the defect before it was believed: a copy of `kio/` whose
`find_package` names a package that does not exist produces `Could not find a package
configuration file provided by "ECMNOTHERE"`, which is what the skip matches. Two synthetic
absences (`-DCMAKE_DISABLE_FIND_PACKAGE_KF6`) produce a *different* message and correctly fail
rather than skip.

## 7. What the gates found, and it was three things

**A tree-wide population this round joined.** `crates/viewer-qt/tests/unsafe_position.rs` walks
every crate in `crates/` and asserts that exactly two lift `#![deny(unsafe_code)]`. `pdf-vfs-ffi`
is a third, and it is the *same* kind of third the second was — a C boundary, because a C caller
cannot be handed a `Result`. The list, the test's name and its argument are updated rather than
the crate exempted, and the doc comment above it says what the rule still is: no crate that
touches PDF bytes lifts the denial, and this one parses nothing at all because RFC §6 puts every
byte of that in a confined process. It is `doc/todo/02` §4's `parts` sweep at the smallest scale —
a cardinal counting this tree's own parts, in a test rather than in prose.

**A figure that was another instrument's.** `the_kio_worker.rs` pinned an extracted page at
36 265 bytes, and the merge that brought rounds 910 and 911 in moved it to 36 997 with nothing
about this face changed. A derived file's length is the *writer's* number, so pinning it here
makes a face's gate fail for the transform suite's reasons. What binds is the property RFC §5.5 is
about — the `get` returns exactly as many bytes as the `stat` promised — and both numbers are now
read off the harness's own lines and compared, with a floor so that two zeroes cannot satisfy the
equality.

**A lockfile a fresh worktree does not have.** `doc/todo/02` §2's fuzz `clippy` line failed on
`tinyvec`, which does not compile on this toolchain — round 909's finding exactly, and the fix it
made to `tools/worktree.sh` copies `fuzz/Cargo.lock` at `open` time. This worktree was opened
*before* that fix was merged into it, so it had none and `cargo` resolved the fuzz workspace from
scratch. Copied in from the main checkout, which is what the script now does. The lesson is the
one ADR 0742 already states, one round later: a thing that is gitignored is a thing a fresh
worktree does not have, and a fix to `tools/worktree.sh` only helps worktrees opened after it.

## 8. One lint finding worth keeping

`clippy.toml` sets `allow-expect-in-tests`, so `clippy::expect_used` fires only **outside** a
`#[test]` function. An `#![expect(clippy::expect_used)]` in a file whose every `.expect` is inside
the test function is therefore *unfulfilled*, and an unfulfilled expectation is itself an error
under `RUSTFLAGS="-D warnings"`. `the_kio_worker.rs` had that shape for one revision — the
attribute copied from a neighbour whose helpers are ordinary functions — and failed the build with
it. It carries the attribute now, because the round's second revision gave it a helper.

Trap 7's rule is unharmed and is worth restating with the qualification: `#[expect]` over
`#[allow]` is right, and an `#[expect]` for a lint that *cannot fire* where it is written is not
an `#[allow]` — it is a build failure, which is the direction to fail in.

## 9. Gates

The full `doc/todo/02` §2 sequence, on the merged tree, on a quiet machine — a merge runs
everything, and this round merged twice. The figures are in the round's report and not here.
