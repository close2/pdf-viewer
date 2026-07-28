# ADR 0014 — JBIG2 and JPEG 2000 decode in a sandboxed worker, through crates we do not own

Status: accepted, 2026-07-28.

## Context

Two filters were unimplemented, and they were the largest single reported gap in the tree:
161 of the 974 corpus first pages could not draw an image, nearly all of them JBIG2 or
JPEG 2000.

`PLAN.md` had said why, and the reasoning was sound when it was written. Neither format had a
memory-safe implementation. Both are historically severe attack surface — FORCEDENTRY was a
JBIG2 integer overflow in a library shipped by everyone. Wrapping the C libraries would have
undone the central argument for writing this project in Rust, so the plan deferred both
behind the sandbox and said, in as many words, that *these two decoders alone justify the
sandbox*.

That premise stopped being true in the eight months before this session. `hayro-jpeg2000`
(0.4.0, December 2025, 508 000 downloads) and `hayro-jbig2` (0.3.0, July 2026) are pure-Rust
decoders from the `hayro` PDF renderer, both `#![forbid(unsafe_code)]`, together 19 400 lines
implementing ITU-T T.800 and T.88. The JPEG 2000 decoder is tested against 20 000 images
scraped from PDFs and passes much of the OpenJPEG suite. With `default-features = false` the
`simd` feature drops out and with it `fearless_simd`, the only `unsafe` either would reach,
and the `image` and `moxcms` dependencies; what is left has no transitive dependency but
`hayro-ccitt`, which JBIG2 needs for MMR-coded regions.

So the question stopped being "how do we contain a C decoder" and became two separate
questions: whether to take these crates or write the decoders here, and whether the sandbox
still has a reason to exist.

## Decision 1 — take the crates

This is the same decision already taken four times in this tree, and it should be taken the
same way: `zune-jpeg` owns `DCTDecode`, `skrifa` owns font parsing, `flate2` owns
`FlateDecode`, `tiny-skia` owns rasterisation. None of those is a shortcut; each is a piece
of well-trodden format work with no project-specific content, and this project's value is not
in re-deriving them.

Writing the two decoders here would be roughly 19 000 lines of MQ arithmetic coding, EBCOT
tier-1, wavelets, symbol dictionaries, Huffman tables and halftone regions — the most
error-prone code in the format, and validated by a corpus of 104 documents against their
20 000. The comparison is not close.

**Principle 5 is untouched.** The specification remains the only source of truth, and these
crates are not consulted about *PDF*. They implement ITU-T T.88 and T.800; ISO 32000-2 §7.4.7
and §7.4.9 say what a PDF filter built on them delivers, and that difference is written here,
in `pdf-sandbox`'s `decode.rs`: the embedded segment organisation, `/JBIG2Globals`, the sense
of a bilevel sample, whether a codestream's palette should be applied, the eight-bit sample
format. Where one of them disagrees with its standard, that is an issue to report upstream —
the same relationship this project already has with `tiny-skia` and `skrifa`.

### What it costs, stated plainly

Two decoders this project does not own. A spec question we hit becomes an issue report
rather than a fix, and the answer arrives on someone else's schedule. The JBIG2 crate is
young — three months old at adoption — where the JPEG 2000 one is established. Against that,
the alternative was not "own it" but "not have it", which is what the last eight months
actually were.

### The evidence taken before adopting them

Not the download count. The pdf.js corpus contains 96 documents named `bitmap-*.pdf` which
are **the same drawing encoded ninety-six ways** — every generic-region template, TPGDON and
TPGRON, MMR, refinement, symbol dictionaries arithmetic and Huffman, text regions transposed
and refined, halftone regions with skip and grids, striped pages, and all five composition
operators. All ninety-six decode to byte-identical pixels here. That is now
`crates/pdf-model/tests/jbig2.rs`, and it is a stronger statement about a JBIG2 decoder than
any comparison against another renderer could be, because it needs no reference at all.

## Decision 2 — build the sandbox anyway, and route both codecs through it

The specific justification in `PLAN.md` is dead: there is no C to contain. Three arguments
survive, and none of them is about JBIG2.

- **Panic containment.** Release binaries are built with `panic = "abort"`. One bad index in
  any decoder, ours or theirs, takes the viewer down with the user's document open. In a
  worker it costs one image and the page reports it.
- **Resource exhaustion.** `CLAUDE.md` principle 3 already says memory safety is not enough
  and that Rust does not prevent exhaustion. `RLIMIT_AS` on a separate process enforces a
  ceiling without every allocation site in a dependency cooperating. This is not theoretical:
  `issue19517.pdf` carries a 12608×16806 JPEG 2000 scan whose full-resolution decode wants
  several gigabytes for a page that will be drawn at about four megapixels.
- **It is the architecture principle 3 already states** — the renderer runs unprivileged with
  no filesystem and no network — and Spike D was an open phase item irrespective of these two
  formats.

Routing the codecs through it rather than merely building it is what keeps it honest. A
sandbox nothing uses is a sandbox nobody notices breaking.

### The shape

A separate program, `pdf-sandbox-worker`, started lazily on the first image that needs one
and reused for the rest of the process. It applies `lockdown::apply` before reading a byte:
resource limits, then a Landlock domain that permits nothing, then a seccomp-BPF allow-list
of 23 system calls with `KillProcess` for everything else. `openat` and `socket` are simply
absent from it.

Requests and responses cross two pipes, not shared memory. Shared memory would be faster and
needs `unsafe` to map; a pipe copy of a decoded image is under a millisecond and the crate
that exists to contain dangerous code is the last place to spend an `unsafe` block for a
speed nobody has measured a need for. `pdf-sandbox` is `#![forbid(unsafe_code)]` — `landlock`,
`seccompiler` and `rustix` all expose safe wrappers, and `libc` is used only for system-call
*numbers*, which are constants.

The parent validates the worker's answers as carefully as it validates a document's: the
worker is the untrusted side of this boundary, and a length it reports is a claim.

### Landlock is best-effort; seccomp is not

If the seccomp filter cannot be installed, lockdown fails and the worker refuses to start.
Landlock's achieved level is *reported* instead, because the property that matters — no
filesystem, no network — is enforced by seccomp on any kernel, and refusing to decode images
on a kernel booted without Landlock would trade a real capability for a redundant layer. The
achieved level travels in the handshake and a test asserts full enforcement on a kernel that
supports it, so a silent weakening fails the build.

## Decision 3 — the isolation is a flag, defaulting to on

`--no-sandbox` on the viewer, `pdf_sandbox::set_isolation` in the library. Someone whose
documents are their own scanner's output is paying a process spawn and a pipe round trip for
a threat they do not have.

This can be a flag *because* of decision 1. Both decoders are memory-safe in process or out
of it, so the choice trades away panic containment and a memory ceiling — real things, and
bounded ones — rather than trading away memory safety, which would not be offerable.

The default is confinement, an unrecognised policy value reads as confinement, and turning it
off prints a line saying what was given up.

## Consequences

The corpus gate's incomplete count fell from 368 documents to 250, the largest single fall so
far and the first that came from a dependency rather than from code written here. The `Image`
row of that report went from 161 to 42.

Two things were found on the way that had nothing to do with either codec.

**A filter chain ending in an image codec was being mishandled.** `[/FlateDecode /DCTDecode]`
is legal and occurs, and the JPEG decoder was being handed the compressed bytes because the
old code read `stream.data` directly. `Document::image_stream` now runs every filter before
the codec and hands over the rest. `firefox_logo.pdf` draws because of it.

**Two of the three reference renderers are one implementation on JBIG2 pages.** `mupdf` and
`ghostscript` both link `jbig2dec`, so the oracle's triangulation rule — two independent
renderers agreeing is evidence — does not hold there, and seven pages were reported as
contradicted where `jbig2dec` renders a blank page or one strewn with noise. `poppler`, which
has its own decoder, agrees with us on six of them, and the ninety-six-way self-consistency
check settles the seventh. The rule was never wrong; its premise simply is not satisfied by
those two renderers on those pages, and nothing in the harness could have known. See
`CONTRADICTED_SHARED_JBIG2_DECODER` in `oracle.rs`.

## What was rejected

**Writing both decoders here.** See decision 1. It stays open: if a spec disagreement or a
performance problem ever justifies owning one of them, this ADR is the record of why it was
not owned first.

**Decoding in-process by default.** The panic and exhaustion arguments apply to every
document, not to suspicious ones, and a default that is safe only for the careful is not a
default.

**Falling back to in-process decoding when the worker cannot start.** There is deliberately
no such path. A fallback that silently removes the confinement is worse than a reported
failure, because the failure is visible and the fallback is not.

**Shared memory for the response.** Deferred until something measures the copy. Written down
so the reason is available when someone does.
