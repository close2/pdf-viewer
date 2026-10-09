# 1478 — A glyph description already running is refused where it is re-entered

Slot 5 of batch seventy-five, 2026-10-09, a measure round. ADR 1792; no ADR 1793, no question. No
row's status moved: §9.6.4 stays `implemented`, its code, tests and note corrected to the build.

**Premise.** Held in its figures, not its place: `scratchpad/r1471/` is gone, and the unit is the
disk seed `fuzz/corpus/page/021ace25…` (164 694 bytes). Release: 2.24 s and 40 695 905 071
instructions for page one, 99.8 % under nested `show_text`; the glyph path is `draw_type3_glyph`.

**Hypothesis.** Half held. The cost is a glyph description showing text in the font it inherits —
two crawled documents spliced, so `/F5`'s `/CharProcs` resolve to page streams whose codes name
`/g0` — but the depth *was* bounded, at `MAX_FORM_DEPTH` 64, and the time was not the sandbox's:
`REQUEST_TIMEOUT` bounds decodes, and `MAX_OPERATIONS` ended the page at 2.2 s, with every mark
after the text. No stage held the time (decode 21.7 %, lexing 15.8 %, allocation 15 %), so no
per-run lever; the volume was exponential in the bound.

**Built.** `draw_type3_glyph` refuses a description already on `Interpreter::descriptions_running`
(by `/CharProcs` stream object, before decoding) as `NestingCycle`, on §9.6.4's erratum, which makes
"all such cases" implementation-dependent; forms and the rest keep ADR 0793's depth bound. The unit:
71 ms, 385 420 755 instructions (106 times fewer). Comments in `content.rs`, `run.rs`, `text.rs`
state the exception. Tests: three new in `hostile_budgets.rs`, the bytes bound's test moved to a
form cycle (a Type 3 cycle no longer reaches it), `type3.rs`'s cycle test renamed; removing the
refusal fails both new re-entry tests (trap 13), the control stays green.

**Pages.** One moves, `ContentStreamCycleType3insideType3.pdf`, and it is the one the brief's "every
corpus Type 3 page byte-identical" did not foresee: 1 762 627 commands cut by `MAX_LIST_BYTES` inside
its first glyph, so the page's own triangle was never drawn; now 44 commands, triangle drawn, looked
at. `Type3Test.pdf` and the no-cycle twin do not move. `nesting_census`, page one of 90 763
documents, 803 s: 10 report a re-entry, ADR 1411's nine and `poppler-102718-0.pdf`, a glyph cycle
at HEAD too (773 ms, its second `b` lost; now 0.7 ms, both drawn) — none a finite chain cut short.

**Gates.** `rustfmt --check --edition 2024` on the five Rust files exit 0; `RUSTFLAGS="-D warnings"
cargo clippy -p pdf-model --all-targets` exit 0; `cargo nextest run -p pdf-model` 2016 passed, exit
0; `cargo test -p conformance` exit 0. Tier 2, behind the lock: `display_list_digest` over 1 477 first
pages, before and after, one line moves; `raster_golden` exit 101 (973 held, the one moved), then
`PDFVIEWER_RASTER_GOLDEN=update` exit 0, its diff that one line; the six arms against
`/home/AI/arms-1474/`, each exit 0, each moving that page alone, agree counts unchanged (gpu-1x's
one differing page, `bug1743245.pdf`, is HEAD's too); `nesting_census` exit 0 as a large walk, after
`--tree 6` killed it at 6.77 GiB and `--long --tree 12` was refused (exit 64). Waits 0 s, the
census's 844.8 s. Duration 5 940 s.
