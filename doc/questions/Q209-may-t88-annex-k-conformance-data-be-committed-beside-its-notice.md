# Q209 — May T.88 Annex K's conformance data be committed beside its notice?

Source: round 1312, holding `pdf-sandbox`'s `JBIG2Decode` filter to ITU-T T.88 Annex K (ADR 1459).

## Where it stands

`crates/pdf-sandbox/tests/t88_conformance.rs` decodes the ten codestreams Annex K ships and holds
each page to its reference bitmap or to a named departure from T.88. The files are not in the
repository: the test reads them from `/home/AI/specs/T.88/` or `$T88_CONFORMANCE_DATA`, and passes
with a printed sentence on a machine without them — every machine but this one, CI included.

What a commit would add: ten `.jb2` streams (18 KB) and thirteen `.bmp` references (1.6 MB, three
of them the same 505 KB fax page; a one-bit format of the test's own would make them about 0.2 MB,
but then they are a derivative work rather than the files).

## What the licence says

The data comes with ICT Link's copyright notice (`Software/Copyright Notice.txt` in the zip). It
grants an irrevocable, worldwide, royalty-free, sub-licensable licence to reproduce, distribute and
prepare derivative works of the software, **for the limited purposes of** including it in a
conforming implementation of the Recommendation, evaluating it for that, and determining whether an
implementation conforms. No patent licence. A conformance test is squarely the third purpose.

The tension is the repository's own licence: it is offered under Apache-2.0 (the owner's answer of
2026-09-03), which lets anyone use what it contains for anything. Files under a narrower grant can
sit in such a repository only fenced off — their own directory, their own notice, and a line in the
licence file saying Apache-2.0 does not cover them — which is a decision about the project's
licensing, not a round's.

## The options

**1. Commit them under `crates/pdf-sandbox/tests/fixtures/t88/` with the notice beside them**, and a
line in the licence file excluding that directory. CI then runs the test everywhere; the cost is a
directory with terms different from the rest, which every downstream packager must notice.

**2. Keep them outside, as now.** No licensing question; the test runs on the owner's machine and a
round's, and skips elsewhere with a sentence.

**3. Commit only what is ours**: a SHA-256 of each reference's packed pixels instead of the bitmap,
and keep the streams outside. Halves nothing that matters — the streams are still needed to run.

## Recommendation

**Option 2, as built.** The test exists to tell the owner, the day the fork's `rev` is bumped,
which streams now pass; that happens on this machine, where the data is. What CI would gain is a
check of two streams the corpus already exercises through `raster_golden` and of a filter refusal a
unit test in `decode.rs` already holds. Option 1 is right if the owner wants Annex K in CI and is
content with a fenced directory; nothing else in the round depends on the answer.
