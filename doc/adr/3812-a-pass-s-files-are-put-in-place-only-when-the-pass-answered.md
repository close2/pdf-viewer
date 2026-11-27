# 3812 — A pass's files are put in place only when the pass answered

Session 2277. Status: **accepted**. Context: RFC 0002 section 12, ADR 2140 (one program, two
worlds), `doc/questions/A834`.

## Context

`pdf_syntax::serialize` writes a document to its writer as it walks it, and the CLI's file sinks
created each output at its name. A verb that failed after its first byte — a stream that did not
encode, a budget passed, a full disk — left a truncated file under the name the person asked for,
over whatever file stood there. For a redaction that is the worst case: the file replaced may be
the one the person meant to keep, and the part written is a redaction of nothing.

Buffering a whole output in memory before writing was rejected: a merge or an archive conversion
writes outputs far larger than the memory a run should hold, and the serializer streams for that
reason.

## Decision

- `Sinks` gains a provided method, `commit`, called by the program once a pass's `apply` answered
  without a refusal. Its default does nothing — standard output and memory cannot take a byte back,
  and a test reading memory needs no staging — so no other implementor changes.
- The CLI's `FileSinks` (`bin/quorra-transform/files.rs`) writes each output to a hidden sibling
  (`.<name>.<pid>-<n>.partial`, `create_new`), renames each onto its name at `commit`, and removes
  what it staged when dropped uncommitted. A sibling because `rename(2)` within one file system
  replaces a file whole: a reader of the name sees the old file or the new one.
- The confined route carries the word: the worker's `ForwardedSinks::commit` sends
  `SINKS_COMMIT`, the broker commits its own file sinks and answers. A worker that fails or dies
  mid-pass never sends it, so the broker's sinks, dropped with the pass, keep nothing.
- A pass is all or nothing: a verb writing several outputs that fails at the third leaves none of
  them. The report of the failure is the whole of what the run produced, which is what a refusal
  means.

## Consequences

`files::tests::a_pass_that_fails_after_writing_leaves_the_file_that_was_there` writes part of an
output, drops the sinks as a failed verb does, and finds the earlier file whole and nothing
beside it. Standard output keeps what it was given; a pipe cannot be taken back, and `-o -` says
so by being a pipe.
