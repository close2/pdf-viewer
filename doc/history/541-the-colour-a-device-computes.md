# 541 — The colour a device computes, and the two witnesses it will not

2026-08-15. One round, adopting the `Paint::Function` quorra built against
`doc/QUORRA_FUNCTION_PAINT.md`. ADR 0376 is the decision and the argument; this is the record.

## What was taken

`cargo update -p quorra-gpu -p quorra-scene`, `a64a9084` → **`05fadc52`**, sixteen commits.
`doc/QUORRA_UPGRADE.md`'s new section has the range. **It cost nothing to compile** — two hashes
in `Cargo.lock` and a clean `check --workspace --all-targets` — which is four releases in a row,
and it held even though `Paint` gained a fourth variant and `ResourceId` a fifth: no `match` in
this tree scrutinises either type, or `DeviceError`, `RenderError` or `ReportKind`.

What it cost was the boundary. `render-quorra` sees `pdf-render` and the compiled §7.10.5 program
lives in `pdf-model`, so the display list gained a device-facing statement of one —
`pdf_render::program`, and `ShadingKind::Sampled { program: Option<ShadingProgram> }` beside the
producer that was already there. ADR 0376 §1 argues why that form is not `pdf_model::function`'s
and why both new enums are deliberately **not** `#[non_exhaustive]`.

## The finding, which is the round's headline

**The device refuses both of the documents the ask was written about.** `pi_seven_segment.pdf`
and `type4_pi.pdf` divide and then truncate to extract a digit of π, which is an inexact operator
into an amplifier, so quorra's `Agreement::Unbounded` declines them by name at the upload:

```
`div` at 234 reaches `truncate` at 354, so no bound on the disagreement with an
independent evaluation can be stated
```

That is the classification working, and it is also exactly the pair `…_ANSWER.md` §4 had measured
at zero differing pixels before their own §0 withdrew the exactness claim. So the measured win on
the owner's `pi.pdf` is **nothing**, and session 529's grid carries those documents as §6 of the
ask said it would if the answer had been no.

## The population, measured

`crates/render-quorra/examples/function_paint_census.rs`, new, driving the shipping path over
1 479 files: **three** pages carry a §8.7.4.5.2 program at all, and **one** carries one the device
takes — `function_based_shading.pdf`, eight of its nine programs evaluated on the device and the
ninth refused for `` `mod` was given a real ``. Two of quorra's fifteen grounds are reached by a
real file.

## The tolerance: none, and the number that decides it

Their §0 recommends 1e-3 relative-or-absolute for function-shading pages. **The gate keeps its
current strictness and no tolerance is added anywhere.** The argument is that ADR 0339 bought a
difference of *branch* rather than of bit — the standard states no precision for a function's
arithmetic (§7.3.3 and §7.10.5.2 both defer) — and that a difference of colour is §10.7.3's own
currency. The measurement is the one page that takes the path, both arms in one working copy on
RADV:

| `function_based_shading.pdf` | grid | device | bound |
|---|---:|---:|---:|
| mean, scale 1 | 0.0178 | 0.0392 | 1.5 |
| worst tile, scale 1 | 1.171 | 1.201 | 7.0 |
| mean, scale 4 | 0.0047 | 0.0191 | 1.5 |
| worst tile, scale 4 | 1.555 | 1.582 | 7.0 |

0.03 of 255 used against 5.4 of headroom, one more channel of 1 453 824 differing at all. The
paint is therefore on by default and behind no flag: there is no population for a flag to
protect. **The project owner may want to review this**, and ADR 0376 §4 says why it is a
judgement rather than a measurement.

## What moved: no verdict, on any gate

- quorra corpus, all four lanes: scale 1 `cpu` **931/23/2/18**, scale 1 `gpu` **929/25/2/18**,
  scale 4 `cpu` **936/10/5/23**, scale 4 `gpu` **937/9/5/23** — each on its previous number.
- oracle 1 794 pages, 906 agrees / 67 contradicted / 786 ambiguous; corpus 974 documents /
  64 incomplete; text extraction 99.2%, 99.8%, 98.26%; dates 97.99%; xmp 318 read / 1 refused;
  jpeg2000 14 byte-identical. `nextest` 1 985/1 985 with 15 skipped, doctests, `fmt`, `clippy`
  and `conformance` all clean.
- The two witnesses rendered at 2× and **looked at**: both still read 3.141. The device and the
  processor draw them identically — `type4_pi.pdf` at mean 0.000000 and ssim 1.000000 at 1× and
  4×, and `pi_seven_segment.pdf` at 0.0000 on every rung of `examples/zoom_ladder` from 100% to
  6400% bar one of 0.0019 at 800%. That is the fallback being whole rather than a coincidence:
  both pages take the grid on both backends.
- The frame line and launch table on `tmp/pi.pdf`, `pi_seven_segment.pdf` and `type4_pi.pdf` are
  unchanged within the machine's load. The reading is structural: the rows this change cannot
  touch moved by more than the row it could — `event loop` from +40 ms to +76 and `graphics
  device` from +17 to +29 between the two sittings, against `first scene built` at +104 → +118 —
  so the sitting was ~1.7× busier and the scene column moved 1.13×.

`doc/todo/00`'s step 7 ink sweep is owed only for a round that changes what gets drawn; one page
of 974 changed by 0.03 of 255, and the ink instrument's resolution is nowhere near that.

## Written back

`doc/QUORRA_FEEDBACK.md` §27 — the two refused witnesses, the census, why no tolerance was taken,
that a program must be cached rather than transient because their shader cache dies with the last
id, and one question: whether they would convert a real under `idiv`/`mod`/`bitshift` the way
this tree does, which is their own §3(b) question with the types the other way round.
`doc/QUORRA_FUNCTION_PAINT.md` §9 records the ask as answered and adopted.

## Files

`crates/pdf-render/src/program.rs` (new), `crates/pdf-render/src/{lib,shading}.rs`,
`crates/pdf-model/src/{function,shading}.rs`, `crates/render-quorra/src/{lib,scene,cache,present}.rs`,
`crates/render-quorra/examples/function_paint_census.rs` (new),
`crates/render-quorra/tests/headless_quorra.rs`, `crates/render-cpu/tests/sampled_shading.rs`,
`crates/test-scenes/src/lib.rs`, `crates/viewer-ui/src/bin/pdf-viewer/surface.rs`, `Cargo.lock`,
`doc/conformance/ledger.toml` (§8.7.4.5.2), `doc/adr/0376-…`, `doc/QUORRA_UPGRADE.md`,
`doc/QUORRA_FUNCTION_PAINT.md`, `doc/QUORRA_FUNCTION_PAINT_BUILT.md` (arrived), 
`doc/QUORRA_FEEDBACK.md`, this file.
