# ADR 0189 — A length every machine agrees on, and the wait that was never sixty seconds

Status: accepted, 2026-08-05 (session 311).

## Context

The GitHub pipeline had been red since 2026-08-02 and nobody had read it. Two jobs failed, for two
unrelated reasons, and both turned out to be defects in this tree rather than in CI.

## 1. `drive`'s wait was one second, and its constant said sixty

`render-gpu`'s `drive` runs the future Vello's asynchronous render returns, polling the device
between polls of the future. It bounds the wait, deliberately — "a wedged driver must surface as an
error rather than hang the viewer" — with `GPU_WAIT_TIMEOUT_SECS`, 60, whose comment reads
"generous enough for `lavapipe` rendering a large page in CI".

It never applied. Each `device.poll` was given a one-second timeout and **its expiry was treated as
a failure**:

```rust
device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(ONE_SECOND) })
    .map_err(|error| GpuRasterError::Readback(error.to_string()))?;
```

`wgpu::PollError::Timeout` means *the submission is not finished yet*, which is what the loop
exists to handle; returning it as an error made the effective bound one second. On this machine
`lavapipe` finishes a test page inside that and nothing showed. On the CI runner it does not:
`cpu_and_gpu_agree_on_a_deeply_reduced_image` — an 800×800 image reduced by ten — failed with the
timeout's own words, and then vello's `Drop` panicked while the first panic was unwinding, so the
process aborted and the failure that reached the log was "panic in a destructor during cleanup".

**Fixed by continuing the loop on `Timeout` and returning every other poll error.** The slice is
now a named constant whose documentation says it bounds nothing, and the deadline says what it is.

**The lesson is the constant, not the API.** A bound written down in one place and defeated in
another reads, to every subsequent reader, as a bound — and this one carried a comment about the
exact machine it was failing on.

## 2. `hypot` is not correctly rounded, and this crate decides pixels with it

The `nightly` job's Miri step failed four of `pdf-render`'s tests. Miri deliberately introduces
non-determinism into floating-point operations that are *not* fully specified — libm's, as opposed
to IEEE 754's — to model what a conforming platform is allowed to do. Run locally it failed a
different three, which is the tell: the failures are seeded, not fixed.

All of them traced to one function. `f32::hypot` is libm's, and Rust promises only that it
approximates the Euclidean distance; `pdf-render` was calling it in seven places, and in four of
them the result is immediately compared against a threshold:

- `Image::is_smoothed` — `across > self.width as f32`, which for an image drawn at exactly 1:1 is
  a comparison of a number against itself;
- `Image::reduction` — the *floor* of a ratio, which changes by one when the ratio crosses an
  integer;
- §8.4.3.5's miter limit — `if length > limit`, where a right-angled join at the limit is exactly
  on it;
- §8.5.3.2's zero-length dash — `if length <= ZERO_DASH * 2.0`.

**A last-place difference in `hypot` therefore changes what gets drawn**, and `render-cpu` is
`render-gpu`'s oracle: two backends compiled against different libms could disagree about every
image drawn at 1:1, on a page nobody had touched. The handover names these as "four device
decisions [that] live here so the two backends cannot differ" — and one libm apart, they could.

**Fixed with `geom::length`**, `(dx * dx + dy * dy).sqrt()`. Multiplication, addition and `sqrt`
are IEEE 754 operations, each correctly rounded, so the expression is the same number on every
conforming platform. All seven `hypot` calls in the crate now go through it. Under Miri,
`pdf-render`'s 82 tests pass with no flag.

The cost is `hypot`'s one real advantage — it scales to avoid intermediate overflow above about
1.8e19 and underflow below about 1e-19 — and page geometry is bounded by the format at 14 400
units, with every caller already guarding a zero or non-finite length.

**Nothing moved.** The corpus gate and the oracle are unchanged to the number: 856 agree, 68
contradicted, 749 ambiguous. That is the expected result and is not the point. The change is not
about this machine's answers; it is about there being one answer.

`-Zmiri-deterministic-floats` was tried first and made all 82 pass, which is how the cause was
identified. It was **not** taken as the fix: it suppresses the report rather than removing what the
report is about, and what the report was about was real.

## 3. What Miri found that this tree cannot fix

With `pdf-render` green, Miri reached `pdf-syntax` for the first time and stopped on
`zlib-rs` 0.6.6, which deallocates through a pointer other than the one its allocation was made
through. Both aliasing models reject it — Stacked Borrows with "deallocating while item is
strongly protected", Tree Borrows with "deallocation through the root of the allocation is
forbidden" — so it is not an artefact of the experimental one.

It is a dependency's `unsafe`, in a crate chosen for being pure Rust at C speed, and Miri here
exists to check this tree's code. Two tests are skipped by name in CI with the reason written
beside them, and `doc/todo/52` holds the upstream report.

## Consequences

- **CI can go green**, and the `snapshot` release job can run for the first time.
- **A rendering decision no longer depends on which libm the binary was linked against.** That
  property was assumed by the two-backend comparison and was not true.
- **Miri earned its keep**, which its own comment in `ci.yml` had said it had not yet. It found a
  cross-platform determinism defect that no gate in this tree could see: the corpus, the oracle and
  the cross-backend comparison all run on one machine, with one libm, and agree.
