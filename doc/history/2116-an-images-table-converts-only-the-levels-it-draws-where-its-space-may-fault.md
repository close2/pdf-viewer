# Round 2116 — an image's table converts only the levels it draws, where its space may fault

A measure round in the `audit` worktree on ADR 2848's two items left beside §7.10.5.3.

## The GPU's type 4 evaluator — already closed (ADR 3166)

- Owed (by the brief): ADR 2849's two numbers and the remedy they favour.
- Found: ADR 2940 (session 2063) took both — no corpus type 1 shading `may_fault` sends to the
  processor; the fault word cost 0.96 ms a frame — and built remedy 2. Neither number was retaken.
- Built: the device side's test, `headless_quorra`'s
  `quorra_draws_a_program_some_input_makes_fault_from_the_grid_and_the_fault_is_said`, on the RX 580.

## An image's table of levels (ADR 3165)

- Built: `ColourSpace::may_fault` and `BlackGeneration::may_fault`; where either holds, `levels_drawn`
  scans the samples and `palette` converts only those levels (padding, uncarried samples and
  colour-key-masked levels left out).
- Cost (callgrind, 2000 × 2000 eight-bit `Separation`): a cleared program 35 704 136 → 35 699 862; a
  program that may fault 35 767 804 → 45 200 696, 2.36 instructions a sample. `issue9940.pdf`
  reads 80 M or 102 M at random in both arms and was not used.
- Population (`type4_operator_census`, extended): pdf.js suite and `corpora-own` 1 127 tabulated
  images, `doc/corpora` 404, none whose space may fault.
  The corpus cache run was killed by the 6 GiB tree ceiling before printing; not taken.
- Tests: three in `image::unpack::tests`, three in `function_faults` (one replacing the test that
  held the over-report). Planted back, the three fail.

## Ledger

§7.10.5.3 `implemented` → `implemented` (code, tests and note moved).

## Trap

A fixture that errs where the interpreter's own opacity sample reaches is said by the
interpretation, and the question after drawing then rightly answers nothing; put the error where
only the grid goes.

**Gates.** Over `-p pdf-colour -p pdf-model -p render-raster`: `cargo fmt --check` exit 0; clippy
`-D warnings --all-targets` exit 0; `cargo build -p pdf-sandbox --bins` exit 0; nextest `--lib --bins
--tests` under `xvfb-run -a` 3243 passed, 0 failed (exit 0), 0 against the manifest's 0; rustdoc
`-D warnings --document-private-items` exit 0; `cargo test -p conformance --no-fail-fast` exit 0.
