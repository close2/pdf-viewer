# 700 — The decoder the gate was not given

An instruments round on `doc/traps/instruments-and-reports.md` trap 16 — the defect five rounds have
now looked at, three of which named it wrongly and one of which recorded it as not happening. It is
**trap 10**, which has been in that file since the seventh session.

## What it is

`pdf-sandbox-worker` is a separate program; Cargo does not build another package's binaries when it
tests this one; and a build without it decodes no `CCITTFaxDecode`, `JBIG2Decode` or `JPXDecode`
image at all. Six of the eight corpus gates never checked.

Conditions, because that is this trap's own rule: worktree `r700` at `2ac19e0f`, a `target-dir` that
did not exist before this round created it, built by the census's own `--no-run` line and nothing
else, and **one test binary of one digest run twice each way** — the second pair after
`cargo build --profile gates -p pdf-sandbox --bins` and no recompilation of the binary itself.

| the one binary, in the one directory | placed by their own marks | with no place | ratchet |
|---|---|---|---|
| no worker beside it | 93 258 | 1345 | **fails** |
| the worker beside it | 93 267 | 1336 | passes |

Instrumented per document, the two runs differ on **one file**: `issue5481.pdf`, which carries a
`JPXDecode` image. §14.8.3.3 derives a structure element's rectangle from what its marked content
*drew*, a refused image drew nothing, and nine elements lose the only place they had.

## The three questions the handover asked

- **Which feature?** None. The two numbers were never a feature difference — but the features were
  enumerated anyway rather than left as "we did not find one". Diffing the resolved unit graphs
  three ways, the census's subset resolves ten crates differently from the workspace (`num-traits`,
  `once_cell`, `rustix`, `linux-raw-sys`, `bytemuck`, `log`, `either`, `enumflags2`, `syn`,
  `proc-macro2`) and **every one was traced to its consumer and changes no value the program
  computes**. `hayro-ccitt`, which session 660 blamed, has no `[features]` section at all.
- **Which number is right?** 1336, and **no floor moves**. The two are not two readings of the
  standard: they are one reading by two programs, one of which is missing a component it ships with.
- **What does the shipped binary carry?** The whole-workspace feature set, differing only in
  `either`'s `std` and `serde`'s `alloc`, both additive. **The odd scope is the gate's, not the
  user's.**

## What changed

`pdf_model::image::sandboxed_decoder()`; `require_the_sandbox()` in the six gates that lacked it and
a reasoned `// no sandbox worker:` line in the two that need none; and
`tools/conformance/tests/sandbox_gates.rs`, which reads `doc/todo/02` §2's own command block and
fails a gate line that does neither. Calibrated against `HEAD`'s versions of three of those files —
it names all three and passes when they are restored.

What each gate does without the worker was measured rather than assumed: the accessibility census
**passes with nine elements in the wrong column**, `jpeg2000` and `fixed_documents` fail in words
that name no cause, and `selection_census` and `text_extraction` move nothing. The last two keep the
requirement all the same, and the ADR says why.

ADR 0557. Trap 16 rewritten with the mechanism and both earlier accounts kept below it; trap 10
gains the six gates and the fact that §7.4.6 travels the same pipe; `doc/verify.md` gains the
unit-graph method, because the feature answer decays.

## Gates

A fifth round, so the whole of `doc/todo/02` §2 ran and §5 rebuilt the binaries. Every line green,
every figure identical to the merge round's. `cargo deny` all four ok. The `pointers` and
`quotations` sweeps report only their standing hits.

## Owed

- **`render-quorra`'s corpus gate without the worker** is the one of the six not measured either
  way — it costs a device run, and the requirement is on it regardless.
- **The census counts no reports.** A refused image is loud in the interpreter's `Unsupported::Image`
  and silent in the census's own output. Making it count reports would change what the census
  measures, which is a round of its own.
- **Nothing guards a measurement that is not a line in §2's sequence** — an example, a benchmark, a
  figure taken by hand.
