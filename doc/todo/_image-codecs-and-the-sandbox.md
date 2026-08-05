# The three sandboxed image codecs — what they are, what the sandbox is for, and what would replace them

Not a todo. Shared background for [34](34-sandbox-the-interpreter.md),
[35](35-confinement-off-linux.md) and [24](24-image-sampling-intent.md), written in the
**three-hundred-and-eleventh session** when the project owner asked whether the code that needs
`pdf-sandbox` could be replaced by safe alternatives, and how hard the algorithms would be to
implement here.

**The question contains a premise, and the premise does not hold.** That is the first section,
because everything after it follows from it.

---

## 1. There is no unsafe code to replace. There never was.

`pdf-sandbox` confines three decoders. All three are already pure safe Rust with **no
dependencies at all**:

| crate | filter | `unsafe` | dependencies | lines |
|---|---|---|---|---|
| `hayro-ccitt` 0.3.0 | §7.4.6 `CCITTFaxDecode` (ITU-T T.4/T.6) | `#![forbid(unsafe_code)]`, **0 blocks** | none | 1 089 |
| `hayro-jbig2` 0.3.0 | §7.4.7 `JBIG2Decode` (ITU-T T.88) | `#![forbid(unsafe_code)]`, **0 blocks** | `hayro-ccitt` | 8 720 |
| `hayro-jpeg2000` 0.4.0 | §7.4.9 `JPXDecode` (ISO/IEC 15444-1) | `#![forbid(unsafe_code)]`, **0 blocks** | none | 9 626 |

`cargo tree -p pdf-sandbox -e normal` shows no C, no `-sys` crate, no build script linking
anything. The only reason `pdf-sandbox` fails to compile off Linux is `landlock`, `seccompiler`
and `rustix` — **the confinement, not the codecs**.

`pdf-sandbox`'s own module documentation says this, in a section headed *Why this is not
redundant with Rust*:

> It would be, if memory corruption were the only thing that goes wrong. The decoders this
> isolates are written here, in Rust, with `#![forbid(unsafe_code)]`, so the class of bug that
> made JBIG2 famous — FORCEDENTRY was an integer overflow into a heap write — cannot occur in
> them.

So "replace them with safe alternatives" has no work in it: they *are* the safe alternatives, and
the search for others found nothing safer. Every other JBIG2 crate on crates.io (`jbig2dec`,
`jbig2dec-rs`) is a binding to the C library; the JPEG 2000 field is `jpeg2k` and `jp2k` wrapping
OpenJPEG, against `oxigdal-jpeg2000` and `dicom-toolkit-jpeg2000` in pure Rust — and the second
of those is **a fork of `hayro-jpeg2000`**, which is to say of what is already here (§6).

## 2. What the sandbox is actually for

Three things, and only the third is about the code being dangerous:

1. **Resource exhaustion.** Rust bounds neither memory nor time. A JBIG2 symbol dictionary can
   name a hundred thousand symbols in a few hundred bytes; a JPEG 2000 codestream can declare a
   tile grid whose product overflows any sane working set. `decode.rs` has hand-placed bounds
   (`MAX_PIXELS`, `MAX_SAMPLES`) *and* the worker has an address-space limit, because "refusing
   those by hand at every allocation site is a discipline; an address-space limit on a separate
   process is a fact."
2. **Panics.** Release builds are `panic = "abort"`. A slice index wrong on one malformed file
   takes the viewer down **with the document open**. In a worker it costs one image, and the
   page draws around it.
3. **What comes later** — insurance against a codec that has to be linked from C one day.

**Rewriting the codecs in this tree changes none of these.** Our own JBIG2 would allocate from
document-derived arithmetic and could panic exactly as hayro's can. This is the load-bearing
conclusion of the whole evaluation: **the codec question and the sandbox question are
independent, and no amount of work on the first removes the second.**

What *would* remove reason 2 is `panic = "unwind"` plus `catch_unwind` around each decode — and
that trades a stated architecture for a caught panic in a process that has already corrupted
whatever invariant it panicked about. Reason 1 has no in-process answer at all short of a custom
allocator.

## 3. How much of the corpus asks for each, measured

`cargo run --release -p pdf-model --example filter_census`, over the 974 pdf.js documents:

| filter | documents | images | largest codestream | all codestreams |
|---|---|---|---|---|
| `CCITTFaxDecode` | 10 | 172 | 409 KB | 1.0 MB |
| `JBIG2Decode` | **104** | 112 | 40 KB | 362 KB |
| `JPXDecode` | 12 | 30 | 2.9 MB | 7.9 MB |

**The JBIG2 number is not what it looks like.** 97 of the 104 are `bitmap-*.pdf` — pdf.js's
synthetic JBIG2 conformance suite, one tiny image each, and their *names* are the feature list:
`-halftone`, `-refine`, `-symbol`, `-symhuff`, `-texthuff`, `-template1/2/3`, `-customat`,
`-tpgdon`, `-tpgron`, `-mmr`, `-stripe`, `-transpose`, `-randomaccess`. Seven are real documents:
`bomb_giant.pdf`, `issue12963.pdf`, `issue17871_bottom_right.pdf`, `issue17871_top_right.pdf`,
`issue20439.pdf`, `jbig2_file_header.pdf`, `jbig2_symbol_offset.pdf`.

So the corpus's JBIG2 demand is **seven real documents and a deliberate full-coverage suite**,
and those two populations answer the "maybe only a subset" question in opposite directions.

## 4. Could a codec simply be dropped?

No, and this is settled rather than arguable. `CLAUDE.md`'s *what done means* puts **clause 7
complete, "including encryption and every filter"**, in scope with no exclusions, and lists the
closed exclusions — clause 13, XFA, script behaviour, authoring. An image filter is not among
them, and "a clause may not be declared out of scope after the fact because it turned out to be
hard" is the rule that forbids adding it.

A filter this tree refuses would also be visible: `filter.rs` already leaves four image codecs
deliberately `None` so that a *content* stream naming one is loudly unsupported.

## 5. Could a *subset* be implemented?

This is the interesting question and the answer differs by codec.

**`CCITTFaxDecode` — yes, and it is nearly free.** 1 089 lines, T.4 and T.6, a small set of
modified-Huffman and 2D vertical/pass/horizontal codes that are mostly tables. This is a
weekend's work to own outright, it is fully specified, and §7.4.6's `/DecodeParms` are already
implemented on this side. **If any codec should come in-tree, it is this one** — not because
hayro's is bad, but because it is small enough that owning it costs less than reasoning about
whether to.

**`JBIG2Decode` — a subset is real, but the corpus is the wrong evidence for its size.** Generic
region decoding (arithmetic, four templates, TPGDON) plus symbol dictionary plus text region is
what a scanner emits and would cover the seven real documents; halftone regions and the Huffman
variants of symbol and text are the long tail. But 97 of the corpus's JBIG2 files exist precisely
to exercise that tail, so a subset would go from 104 documents to about seven and **the corpus
gate would say so loudly**. Whether that matters depends on whether those 97 are treated as
documents or as somebody else's test suite — and this project has been careful, elsewhere, not to
let a corpus define correctness. It is a real decision and it has not been taken.

**`JPXDecode` — no useful subset exists.** The obvious cut is the irreversible 9/7 wavelet,
keeping only reversible 5/3 — and `tests/jpeg2000.rs` measured the split: **14 of 30 corpus
codestreams are reversible and 13 are irreversible**, so the "subset" would drop nearly half the
population. Worse, the 13 irreversible ones are exactly the ones `hayro-jpeg2000` already gets
*wrong* (`doc/JPEG2000_FEEDBACK.md`), so that half is both the harder half and the unfinished
half. JPEG 2000 has no small core: EBCOT with its three coding passes, the MQ arithmetic coder,
both wavelets, tiling, precincts, packet headers with tag trees, and multiple progression orders
are all reachable from ordinary files.

## 6. Implementing them here — the price, empirically

The honest estimate is not a guess, because someone has already done it in the same language
under the same constraint: **19 435 lines of safe Rust for the three**. That is the price of
matching what is here, and matching is the floor — `CLAUDE.md` principle 5 would require each
line derived from T.4/T.6, T.88 and 15444-1 rather than from another implementation, with the
clause citations the tree demands.

And the outcome would be *the same bug surface*. `hayro-jpeg2000`'s irreversible path is wrong on
13 corpus codestreams, and it took a comparison against ISO/IEC 15444-5's own reference software
to find it. Our own would need the identical instrument, which already exists (`tests/jpeg2000.rs`)
and would then be pointed at us.

**Recommendation.** Own `hayro-ccitt`'s job (1 089 lines, fully specified, small tail). Leave JBIG2
and JPEG 2000 where they are, and spend the effort on the JPEG 2000 *correctness* gap instead,
which is worth more per hour than a reimplementation: 13 codestreams that decode to the wrong
samples today are a defect a user can see.

## 7. The lead worth following, and its catch

**`dicom-toolkit-jpeg2000` is a maintained fork of `hayro-jpeg2000`**, MIT/Apache-2.0, whose
authors say it is "tested against 20 000+ images scraped from random PDFs on the internet" and
"passes a large part of the OpenJPEG test suite". If any of that testing reached the irreversible
path, it may already fix the 13 — and `tests/jpeg2000.rs` is precisely the instrument that would
say, in about a minute, by swapping the dependency and re-running.

Two things to check before believing it. It has **dependencies where `hayro-jpeg2000` has none**,
and its `simd` feature pulls `fearless_simd`, which contains `unsafe` — that feature would have to
stay off, and `cargo deny` would have to agree about the rest. And its extensions are
DICOM-shaped (native 8/12/16-bit decode, an encoder, HTJ2K through `openjph-core`), which is
surface this tree does not want; whether the parts we use are the parts they hardened is exactly
what the gate would answer.

`hayro-jpeg2000` 0.4.0 is the latest published version as of this session, so there is no upstream
fix waiting to be picked up.

## 8. What this means for the platform question

Nothing here unblocks macOS or Windows, and that is worth stating plainly because it was the
hope. The codecs compile everywhere already. `pdf-sandbox` does not, because of `landlock` and
`seccompiler`, and the fix for that is [todo 35](35-confinement-off-linux.md) — `cfg`-gating those
dependencies and then deciding what confinement means on a platform without seccomp. Reasons 1 and
2 of §2 are exactly as true on Windows as on Linux, so the answer there is a job object and a
process, not the absence of one.
