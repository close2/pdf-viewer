# 999 — `apply` asks, and one file runs the program

Date: 2026-09-12. Branch: `batch-999-1004`, worktree `/home/AI/pdf-viewer-rounds`, shared with
five sibling rounds (1000–1004). ADR: [1019](../adr/1019-apply-asks-and-one-file-runs-the-program.md).

## What was asked, and what was built

RFC 0007's external-tool request and the one shared executor — the owner's `A54`, `A55` and `A56` —
on top of session 992's configuration format.

Six things, in the order the slice named them:

1. **The request type.** `crates/pdf-transform/src/tool.rs`: `ToolRequest` (program, arguments,
   input bytes, expected media type, bounds, delivery, and a stable identifier), `ToolResult`,
   `ToolOutcome`, `ToolOutputs`, `Tool`, `Bounds`, `Delivery`. Every field is data — nothing is a
   handle or a closure — which is the property the replay rests on.
2. **The shared executor.** `crates/pdf-transform/src/executor.rs`, one public function, the only
   code in this tree that starts a process. It lives in `pdf-transform` because `cargo metadata`
   says so: the three consumers `A54` names (the command-line program, the KIO worker, the FUSE
   filesystem) all reach the converter through that crate, and `viewer-core` is a dependency of
   none of them. The command-line program's loop is built; the other two reach `apply` through
   `pdf-vfs`, so their loop is one loop in `crates/pdf-vfs`'s commit path, and `A54`'s `Owes:` line
   now names exactly that.
3. **Replay.** A recorded `ToolResult` in the plan makes a pass start nothing at all.
4. **`derive` per `A55`**, at the catalogue's flagship site, with its four terms as code.
5. **`supply`**, at the first of the catalogue's twelve.
6. **The warning per `A56`**, in three places that cannot drift because they are one constant.

## Three things worth keeping

**A closure would have been shorter and would have cost the determinism claim.** The reason two
passes beat a callback is that a closure is not data: it cannot be recorded, compared or replayed,
and RFC 0002 §9's claim would have come to rest on whatever the closure did. The two-pass shape
means a recorded tool output *is* part of the plan, so the second pass is the same pure function it
always was. The replay test records one output, deletes the program, and converts twice.

**A child with piped output cannot be waited on under a timeout without somebody draining the
pipe.** The executor hands the child files for all three descriptors, in a directory it made and
removes with a `Drop` guard. The wait loop then does no I/O, the timeout is a poll and a kill, and
the output limit is applied to a file's stated length *before* a byte is read.

**Two rules of `doc/rfc/0007` §4.1 disagree at the one site that runs a tool.** The subclause lists
`{media-type}` among the placeholders and also says nothing document-derived reaches `args`. At an
embedded file the only media type available is the attachment's own `/Subtype`, which is the
document's. A configuration naming it is refused with a sentence saying so, rather than filled from
a place the RFC forbids. ADR 1019 records it as a departure from the RFC's list.

## A citation this round found wrong on the way past

Session 992's configuration reader cited `§7.11.4.2` for the embedded file stream's `/Subtype`. That
subclause is *Related files arrays*; Tables 44 and 45 are `§7.11.4.1`'s. Four sites carried the
error and now do not. The conformance checker does not catch this — it checks that a cited clause
exists and that a quotation is verbatim, not that a table belongs to the clause beside it — so it is
worth saying that the finding came from reading the clause rather than from a gate.

## The stand-in, and why the round says so out loud

Nothing on this machine converts a spreadsheet to PDF/A, so the program exercised end to end is a
shell script the test writes: it reads the attachment on standard input and writes the PDF that
attachment becomes. That proves the seam — request built from the document, caller runs it, bytes
come back, promised media type checked, attachment replaced, output held to the target again, report
and packet both saying what happened — and it proves nothing about `soffice`, which is not this
project's to prove. `doc/profiles/derive-attachments.toml` is the declaration a real deployment
edits.

## Files touched

`crates/pdf-transform/src/{tool,executor,lib}.rs`,
`crates/pdf-transform/src/archive/{config,remedies,decision,rewrite,prepare,report,mod}.rs`,
`crates/pdf-transform/src/bin/quorra-transform.rs`, `crates/pdf-transform/Cargo.toml`,
`crates/pdf-transform/tests/{archive,archive_corpus,profiles}.rs`,
`doc/profiles/{derive-attachments,keep-everything}.toml`, `doc/pdf-a-mitigations.md`,
`doc/pdf-a-conversion-limits.md`, `doc/questions/{A54,A55,A56}` (their `Owes:` lines only),
`doc/adr/1019-apply-asks-and-one-file-runs-the-program.md`, this file.

## The worktree, shared

Five siblings edited the same checkout at the same time. Three consequences worth recording for the
next batch run.

**A failing test in a shared worktree is a question about whose file it is before it is a defect.**
`crates/pdf-archive` was mid-refactor while this round's tests ran, so twelve `tests/archive.rs`
cases unrelated to this work failed and passed again between one of that round's commits and the
next. `cargo fmt -p <crate>` is the only safe spelling for the same reason: the unscoped form would
have written into five other rounds' files mid-edit.

**`foreign_corpus` cannot be run beside another round's copy of itself, and the failure looks like a
defect.** Two gate sequences in one build root both write
`$CARGO_TARGET/tmp/foreign-readback/<document>/`, and the loser's `merge.pdf` is gone before poppler
and mupdf are asked to draw it — which the walk reports as *a foreign reader drew the source page
and could not draw ours*, over documents this round never touched. Run alone it passes with zero in
every lane. The walk's scratch directory is a per-run one waiting to be written; until it is, "never
beside a sibling's" is the whole of the protection, and this is the walk it protects.

**A watcher's own command line is in `pgrep`'s haystack.** `until ! pgrep -f "foreign_corpus-"` never
exits, because the `bash -c` running it contains that string. The instruction's rule about `pkill -f`
is the same trap one tool over; matching the binary's path under `gates/deps/` and waiting on a pid
is what works.
