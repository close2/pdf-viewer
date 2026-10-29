# 2592 — A test that decodes states its isolation, because the isolation is the process's

Session 1879, slot 9 of batch 128, a "gates" round (robustness).
Status: accepted and built in `crates/pdf-transform/tests/archive.rs` (`decode_in_this_process`).
Context: ADR 1333 (`Request::JpxWhole`), ADR 1412 (a transcoded opacity channel), ADR 2140 (the
transform suite parses in a confined worker), ADR 2242 (a red gate is judged; this one is the
batch's, its cause fixed), ADR 2484 (`deep_jpx`'s tests). It cites no new clause and moves no ledger
row. ADR 2593 and Q755 are unused.

## 1. What failed, and why it was not a budget

Two tests, `a_jpx_image_whose_data_carries_opacity_is_transcoded_with_a_soft_mask_image` and
`a_premultiplied_jpx_image_is_transcoded_with_a_matte_of_zero`, failed inside `cargo test -p
pdf-transform --test archive` and passed under the filter `jpx`. The refusal they printed is
`transcode.rs`'s `NOT_DECODED_WHOLE`, whose words name "the ordinary decoder's budget", and that is
why the failure was read as a budget reached under load. It was not one. `not_decoded` gives that
sentence to every `ImageError::Sandboxed`, and a worker that is absent, or of another build
identity, is one: with `PDF_SANDBOX_WORKER` naming no file the suite fails exactly these two at the
same two lines, and the filter `jpx` passes all fourteen, as the report of the failure had it.

Two facts make the route order-dependent and build-dependent:

1. **The isolation is the process's.** `pdf_sandbox::set_isolation` stores one atomic, and
   `deep_jpx`'s seven tests set it to in-process; nothing sets it back. A test of the binary that
   leaves the default decodes through the confined image worker if it runs before the first of
   those, in this process if after — the threads' order decides. Under the filter `jpx` a
   `deep_jpx` test is among the first fourteen, so the two decoded in process and passed.
2. **The image worker is a program `cargo test` does not rebuild** (trap 10). Its greeting carries
   a build identity hashed from `pdf-sandbox`'s sources, `pdf-ccitt`'s and `Cargo.lock`
   (`build.rs`), and a test binary of another identity is refused (`WorkerMismatch`). Batch 127
   changed `Cargo.lock`, so the shared target's worker went stale for every test binary built after
   it, and a test that reached the worker failed.

No wall-clock bound was reached: the thirty seconds of `REQUEST_TIMEOUT` is four orders above an
8×8 decode, and the fixtures' sample budget is `ORDINARY_JPX_SAMPLES`, a count.

## 2. The decision

**A test of this binary that decodes, and is not a test of the confinement, sets the in-process
route itself.** The two tests call `decode_in_this_process` first. That is the route the archive
conversion takes by default in production: `quorra-transform` runs a conversion in
`quorra-transform-worker` (ADR 2140), whose `confine` sets the in-process route before any document
is read. Every writer of the atomic in this binary then writes the same value, so the route is the
same in every order, and no program outside the test binary is consulted.

Rejected:

- **The two in a test binary of their own.** The order would be fixed, and the verdict would still
  depend on a worker binary cargo does not build for the test: a stale worker fails them, which is
  the same red with a different cause.
- **A wider budget, or a deadline set inside the test.** Neither was reached; widening one hides
  nothing and fixes nothing.
- **`deep_jpx` restoring the default.** A test that resets a process-wide value races every test
  running beside it; the fix is that no test relies on the value another test leaves.

## 3. What is left

The other tests of the binary that decode a JPEG 2000 image without stating the route are still
order-dependent in the same way; they are named for a later batch, not changed here. The sentence
`NOT_DECODED_WHOLE` names a budget for a worker that could not be reached, which is a report whose
condition is wider than its words (trap 11); it is `src/archive/`'s, named for a later batch.
