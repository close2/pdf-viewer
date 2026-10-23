# 1712 — The per-mark conversion was 1.7 % of a text page, and the calibration edge is 0.74

Session 1712, 2026-10-23. Batch 110, slot 1 (measure). ADRs 2258, 2259; Q575 is unused.

**Premise held.** Review 1704 finding 2 was as stated: every solid mark built an owned `Conversion`
to ask about §11.7.5.3's bypass, even after the flag was set.

**Hypothesis.** Half held. Counted with callgrind (`examples/callgrind_interpret`; HEAD, HEAD with
the report removed, and the change), the report cost 136 instructions a mark, 1.69 % of page 101 of
ISO 32000-2 and 1.23 % of `tracemonkey.pdf` page 2. That is not the 3–8 % guessed. The change costs
5 a mark, 0.06 % and 0.05 %, under the 1 % guessed, and the display-list digests match on all three
builds. The calibration band declines at 0.74, not 0.72. The turn floor is not the machine's quiet
reading: every reading under it came from a child the gate declined for waiting.

**Change (ADR 2258).** `ConversionContext::bypasses_black_generation` is the one test, and
`Conversion` asks it of its borrowed form. `Interpreter::report_solid_black_generation` asks nothing
once the flag is set and asks the borrowed `paint_context`, and so does the tint binding. The
accepted costs of ADRs 2198, 2212, 2223 and 2235 are comments beside `SolidColours`, `TintBinding`
and `IccSource`, with their figures and the `black_generation_order` modes.

**Readings (ADR 2259).** `launch_path` now prints the child's readings beside every clock figure
judged in band. In two `--clock` holds of HEAD and the change, the change's 57 figures judged in
band had children at 0.667–0.715 ms, and the 15 failures 0.675–0.769. The edge is 0.74, the file's 4
% rule over today's good children. 0.72 would decline failures the probe does not see.
`turn-path.toml` keeps 0.78 and says why.

**Proof nothing moved.** Planted defects make the report's tests fail: the short-circuit inverted,
the flag never set, a tint that never bypasses, a stencil that never reports, each in
`transparency_groups.rs`. HEAD's six arms (`tools/batch.sh arms`) and this export's agree on every
page's digest.

**Gates.** callgrind digests equal on three builds of two pages. `rustfmt --check` 0; `cargo fmt
--check -p pdf-colour -p pdf-model -p viewer-ui` 0; clippy `-D warnings --all-targets` on all three
0. `cargo nextest run -p pdf-colour -p pdf-model` 2325 passed, 21 skipped; `-p viewer-ui` 137
passed, 1 skipped. `raster_golden` on the export 0 (974 entries). Six arms 0, 5 799 pages, 0 digests
differ. Hold: `t2-turn_path` 0 in all four runs (33 banded, 0 outside); `t2-launch_path` exit 101 in
HEAD's 4 of 9 runs and the change's 2 of 5, both in the warm-open mode, open instructions within 0.7
thousand by row. The counted launch run on the new band 0, 31 banded. `cargo test -p conformance`
410 passed, 1 failed on a sibling's viewer-ffi doc paths; `--test evidence --test adr_status` 0.
`tools/state.sh --round 1712 docs` 0.
