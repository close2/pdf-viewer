# ADR 0020 — The oracle remembers what the reference renderers said

Status: accepted, 2026-07-29.

## Context

The oracle gate (ADR 0011) renders 1794 pages with our own pipeline and with `pdftoppm`,
`mutool` and `gs`, then triangulates. It is the only instrument in this project that can tell
a page that *looks* finished from one that is right, and eleven sessions of work have been
steered by it.

It also cost 75 seconds of wall clock and **1020 seconds of processor time in the three
external renderers against 46 in ours** — a ratio above twenty to one. That is not a detail
about a test suite; it is the loop between writing a feature and finding out which pages the
feature moved, and the length of that loop decides how much gets built. The project owner
raised it directly: implementing one thing at a time felt slow because every check re-ran
everything.

The obvious observation is that the references' answers do not change. The same version of the
same renderer, given the same file and the same command line, produces the same picture; a
gate that asks again every time is doing work whose result is already known.

`doc/HANDOVER.md` had already named both the lever and its danger:

> a content-addressed cache of reference renders is the obvious lever, with the equally
> obvious risk that a cache key omitting one variable (the crop-box flag, the renderer
> version) would compare against stale renders in silence.

That risk is the whole difficulty. A cache that returns the wrong picture does not fail. It
moves the *verdict* of the one gate in this repository whose authority rests on being
independent of us, and it moves it quietly.

## Decision 1 — cache reference renders, keyed on the invocation itself

`pdfref::cache::Cache` stores each render under a SHA-256 of:

- a format tag, so an entry's meaning cannot change under a reader;
- the reference's name and its **identity** — the version string for the three external
  renderers, and for `pdfref-hayro`, which has no version flag, the digest of the executable;
- the page number and the resolution;
- the **document's own SHA-256**;
- **the argument list `Reference::build_command` is about to run**, with the document's path
  and anything under the work directory replaced by placeholders.

The last item is the answer to the handover's warning, and it is the point of the design. A
key built from a hand-maintained list of relevant variables has to be updated when `-cropbox`
is added to an invocation, and the consequence of forgetting is not a failure but a comparison
against renders made under the old flag. Deriving the key from the command line makes that
impossible by construction: **a flag that is not in the key is a flag that is not passed to
the renderer either.** `the_cache_key_carries_the_page_box_flags` pins the specific case that
already cost this project 54 documents (trap 3).

What the key cannot see is a renderer whose output changes while its version string does not.
That is a distribution's problem rather than a cache's, and `Cache::clear` and
`PDFREF_CACHE=off` are the remedies.

### Why SHA-256, written here

The document digest is what stands between the cache and handing back another file's picture,
so the argument for it should not be "collisions are unlikely on expectation". 256 bits gives
an argument; a 64-bit non-cryptographic hash gives an estimate.

Writing sixty lines of it in `pdfref::digest` rather than taking a crate is the *opposite* of
ADR 0014's decision about JBIG2, and deliberately so. That decision turned on 19 400 lines of
the most error-prone code in an image format. This is a fully specified algorithm with
published test vectors, in a tool whose only dependencies are the workspace's own crates and
`png`, and `the_published_test_vectors_are_reproduced` checks it against FIPS 180-4's own
worked examples. An unverified hash would be worse than no cache; a verified one is a file
nobody needs to think about again.

## Decision 2 — remember timeouts too, for a week

This was the second design, and the first one was rejected by measurement.

A timeout is the one outcome of a reference invocation that is *not* a function of its inputs:
how long a renderer took is decided by the machine. So the first version refused to remember
it, which is the obviously safe rule. With everything else cached, the run then spent **46 of
its 57 seconds on two pages out of 1794** — `bomb_giant.pdf` and `bug1978317.pdf`, two
decompression bombs where two renderers apiece are given thirty seconds and none returns. The
gate had become a program that waits half a minute to kill processes it killed yesterday.

So a timeout is remembered, and expires after a week. The trade is written down in
`pdfref::cache`'s module comment; the three parts of it are:

- **The gate is already non-deterministic about this.** A renderer needing 29 seconds idle and
  31 under load changes an *uncached* run's verdict too. The cache makes one observation
  sticky rather than flapping, and prints the count.
- **A wrongly-remembered timeout cannot hide a page the gate is watching.** A page whose
  reference times out leaves the comparison, and the ratchets are equality-checked in both
  directions — so a listed page that stopped being compared fails the build. What it can hide
  is a page nobody has listed, which is what the expiry bounds.
- **A week is a bound, not forever.**

`HarnessError::RendererTimedOut` exists so that this case is distinguishable in the type
rather than by reading a message, which is what makes the rule enforceable at all.

## Decision 3 — the three references render in parallel

A page's cost is now its slowest reference rather than the sum of three. Nested inside the
gate's outer `par_iter` this is free — rayon's work-stealing has no notion of nesting — and it
took the worst page from 60 seconds of pure waiting to 30 before timeouts were cached at all.

## What it bought, measured

| | before | after |
|---|---|---|
| oracle wall clock | 74.9 s | **24.5 s** |
| processor time in the three references | 1021 s | 17 s |
| every verdict | — | **identical** |

The two clocks were measured a session apart, on a machine doing other things, and the
uncached run of the *finished* tree takes 98 s rather than 75 — more pages entered the
comparison and the machine was busier. The honest statement is therefore about the ratio and
about the processor time, not about two stopwatch readings.

The cold run costs about 20 seconds more than an uncached one, once, to populate 319 MB of
PNGs under the build directory.

The identical-verdict row is the one that matters, and it was checked the only way that
settles it: the counts of agreeing, contradicted, ambiguous, geometry and not-comparable
pages from a full uncached run and a full cached run are the same numbers. `PDFREF_CACHE=off`
is how that check is repeated.

## Also: running the gate over a subset

`PDFVIEWER_ORACLE_ONLY` takes a comma-separated list of substrings and compares only the pages
whose names match — 0.2 seconds for the ten documents a new image filter touches. A filtered
run **refuses to check the ratchets**, and says so, because a list held to equality over a
subset would report every excluded page as newly fixed.

This is a convenience rather than the answer to the original problem, and the ordering matters:
the complete run is the fast one. A workflow built on "implement ten things, then work out
which files changed" would trade the gate's completeness for speed it no longer needs.

## Consequences

- The loop between a change and its verdict is 25 seconds, most of it now *our* renderer and
  our comparison rather than three other programs.
- One more thing that can be stale, with two ways to clear it and a printed hit rate that
  makes a silent change of key visible: a run over an unchanged tree that reports less than
  99% hits means the corpus or a renderer moved.
- 319 MB per build directory, which is why the location is the caller's choice and `off` is a
  supported value.
- The gate's remaining wall clock is dominated by comparison and artefact writing — roughly
  600 seconds of processor time over 24 cores. That is the next thing to look at if 25
  seconds ever becomes the constraint.
