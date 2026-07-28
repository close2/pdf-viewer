# hayro vs this project

I should be upfront about the limits of this comparison: I've read `hayro`'s crate metadata, its README summary, and the source of three of its nine crates (`hayro-jbig2`, `hayro-jpeg2000`, `hayro-ccitt`) during the codec search. I have *not* read `hayro-syntax`, `hayro-interpret`, `hayro`, `hayro-svg`, `hayro-postscript` or `hayro-cmap`. So the structural comparison below is grounded, but claims about the quality of its renderer internals are inference from its test results, not from reading it.

## What the two projects actually are

They are not the same kind of thing, and that explains most of the differences.

**hayro is a library.** Nine crates, `no_std`-compatible, published to crates.io, ~500 k downloads on the JPEG 2000 crate alone. Its output targets are PNG and SVG. Its success criterion is that a Rust program can render a PDF correctly.

**This is an application** with a stated goal of being "the noticeably fastest PDF viewer available" and "clean enough to be taught from" — a winit window, AccessKit, portal dialogs, a GPU backend, perf gates in CI. The library layers exist to serve that.

That difference is why `hayro` ships nine independently useful crates and we ship a workspace where `viewer-core` and (until now) `pdf-sandbox` were empty shells with documented responsibilities.

## Where hayro is probably ahead

**Feature completeness, and it isn't close.** Its README claims 1400+ PDFs from the PDFBOX and pdf.js regression suites. Our own oracle numbers say we contradict the reference consensus on 120 of 1340 pages, with 81 unexplained. The gaps we have written down — transparency groups and soft masks (45 corpus documents), encryption (20), CID encodings and embedded CMaps (76), Type1 and Type3 fonts, optional content — are gaps `hayro` appears to have closed. It has a `hayro-cmap` crate; we have "3 documents need predefined CMaps and that's a licensing decision". It has `hayro-postscript`; we have a PostScript calculator function evaluator but nothing broader.

**Codec breadth.** Three format decoders written from the ITU specs, memory-safe, and validated far beyond what our corpus could do — 20 000+ images scraped from PDFs plus much of the OpenJPEG suite for JPEG 2000. Our JBIG2/JPX gap was 152 documents and we just closed it *by adopting their work*, which is the most direct evidence available about who was ahead there. `hayro-ccitt` also means CCITTFaxDecode is solved for them and still open for us.

**SIMD as a deliberate, contained choice.** `hayro-jpeg2000` puts its vectorisation behind a `simd` feature using `fearless_simd`, defaulting it on and documenting that turning it off "eliminates any usage of unsafe in this crate as well as its dependencies". That is a cleaner answer to the safety/speed tension than we currently have anywhere — the *user* picks the point on the curve, and the crate documents both ends. Our equivalent lever is the `--no-sandbox` flag, which is the same idea applied to a different axis.

**Shipping discipline.** Versioned releases, `no_std` support, an `image`-crate integration hook, 100% public API documentation coverage. We have a repository with no published crates and a `repository` field pointing at `github.com/example/pdf-viewer`.

## Where we are ahead, or deliberately different

**Principle 5 — the specification as the only source of truth.** I have seen no equivalent commitment in hayro's materials. Its stated validation method is agreement with PDFBOX and pdf.js regression suites; ours is a triangulation rule where agreement is *evidence about our reading* and disagreement is a question for the clause. These produce different code when they diverge. Our `CalGray`/`CalRGB` work is the case in point: the shortcut was nearly right for `/Gamma 2.2`, which is what most producers write, so a corpus-agreement standard would never have flagged it, and §8.6.5.2's derivation did.

**The oracle.** Comparing every page against poppler, mupdf and ghostscript, with our tolerance taken from the references' own mutual disagreement on that page, ratcheted in both directions, and every contradicted page named in the source. I would be surprised if hayro has an equivalent — it is expensive (1033 s of external-renderer CPU per run) and it only pays off if you are trying to be *right* rather than *compatible*.

**Two backends that check each other.** `tiny-skia` and Vello consuming a byte-identical display list is a same-scene oracle far tighter than any cross-viewer comparison. hayro has `hayro` (bitmap) and `hayro-svg`, which is a similar shape, but SVG-vs-raster can't be diffed pixel-for-pixel the way two rasterisers can.

**Process isolation.** We now have seccomp-BPF + Landlock confinement with tests that were confirmed to fail when lockdown is removed. A library cannot reasonably do this — sandboxing is a process-level decision that belongs to the application — so this is a genuine architectural advantage of being the thing that owns `main`.

**Reporting what we cannot draw.** `Interpretation::is_complete()`, `ImageError`, `FontError`, `CpuRasterError::UnsupportedCommand`, and the title bar naming undrawn content. The corpus number that matters to us — 587 of 974 first pages report nothing — went *down* from 68% when annotations landed, because a class of silent absence became visible. That is a design commitment (principle: unsupported input must stay loud) rather than a feature, and it is what makes every other number here trustworthy.

**Documented reasoning.** Fourteen ADRs, a handover that records what each mistake taught, comments that say *why*. This is principle 4 and it is not free — it is a large fraction of the effort — but it is the stated goal.

## What we can learn

Five things, in rough order of how much I'd act on them:

1. **Adopt more of their codec work.** `hayro-ccitt` is already in our tree as a transitive dependency of `hayro-jbig2`. `CCITTFaxDecode` is one of our named gaps and closing it is now nearly free. The same argument that decided JBIG2 and JPEG 2000 applies unchanged.

2. **Their crate boundaries are worth studying for ours.** `hayro-cmap` as a separate crate is a better shape than our "embedded CMaps are 14 documents in the text gap". A CMap parser is a self-contained, independently testable, independently fuzzable thing — exactly the argument that made `pdf-syntax` separate from `pdf-model`.

3. **The `simd` feature pattern.** Default the fast path on, document the safe path, let the consumer choose, and state the cost at both ends. If we ever vectorise `to_rgb_at`, that is the shape to copy — and it fits our rule that an optimisation carries the benchmark that justifies it.

4. **A published-crate discipline would improve the libraries.** `missing_docs` is already enforced; actually publishing `pdf-syntax` would force the API surface to be defensible to someone who is not us. Their 100% doc coverage on `hayro-jpeg2000` is the evidence that this works.

5. **Their test corpus scale.** 20 000 images scraped from real PDFs for one codec is an order of magnitude past our 974 documents. Trap 8 in our handover says a corpus finds what documents contain, not what the spec says — but the converse is also true, and 974 documents is a small sample of what producers actually emit.

## The thing worth being careful about

There's a temptation, having just adopted three of their crates, to treat hayro as the reference for what "done" looks like. That is exactly the inference direction CLAUDE.md forbids. `hayro-jbig2` and `hayro-jpeg2000` are dependencies implementing ITU-T T.88 and T.800 — the same status as `zune-jpeg` for DCTDecode or `skrifa` for fonts. If one of them ever disagrees with the specification, the answer is an upstream issue, not a local workaround, and certainly not a revised expectation. We recorded that in the dependency comment for a reason.

The more interesting question their existence raises is a strategic one: if `hayro` is the most feature-complete Rust PDF renderer and it is a library, then our differentiators have to be the things a library cannot be — startup latency, the sandbox, the GPU path, the viewer itself, and a correctness standard anchored to the spec rather than to consensus. That is roughly what CLAUDE.md already says, which is mildly reassuring.
