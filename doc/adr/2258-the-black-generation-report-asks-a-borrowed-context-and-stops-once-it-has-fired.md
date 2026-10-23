# 2258 — The black-generation report asks a borrowed context, and stops asking once it has fired

Session 1712, the measure round of batch one hundred and ten. Status: **accepted**. Context: ADR
2187 (the report and its condition, ISO 32000-2 §11.7.5.3's first bullet); review 1704 finding 2;
principle 2's rule that an optimization carries a benchmark and a comment. Moves no ledger row.

## 1. What was paid

ADR 2187 asks, at every solid fill, stroke, glyph and stencil, whether the paint is `DeviceRGB`
painted straight into a profiled `DeviceCMYK` target under a stated pair. It asked by building an
owned `Conversion` — up to five `Arc` clones and their drops — and asked again after the page's flag
was already set. Text pays it once a glyph.

## 2. The price, counted (habit 164: the ceiling first)

`valgrind --tool=callgrind` over `cargo run --release -p pdf-model --example callgrind_interpret`,
three release builds of one HEAD export: HEAD, HEAD with the report's three call lines removed (the
ceiling), and this change. Instructions inside `pdf_model::content::interpret`:

| workload | marks | HEAD | ceiling | change | HEAD − ceiling | change − ceiling |
|---|---|---|---|---|---|---|
| ISO 32000-2 p. 101, ×50 (the example's default) | 150 350 | 1 207.33 M | 1 186.87 M | 1 187.63 M | 136.0 a mark, 1.69 % | 5.0 a mark, 0.06 % |
| `tracemonkey.pdf` p. 2, ×1 | 5 315 | 58.81 M | 58.09 M | 58.12 M | 136.1 a mark, 1.23 % | 5.1 a mark, 0.05 % |

Marks are callgrind's call count of the report on HEAD; 5 276 of the second page's are `fill_paint`,
nearly all glyphs. The display list's `--digest` is the same on all three builds of both pages.
**The brief's hypothesis of 3–8 % did not hold**: 1.2–1.7 %. Its "under 1 % after" held.

## 3. The decision

- **One definition, asked of the borrowed form.** `ConversionContext::bypasses_black_generation`
  holds the test; `Conversion::bypasses_black_generation` asks it of `self.context()`. The
  interpreter's `paint_context` borrows the same seven fields `solid_paint` hands its memo, and the
  pattern tint's binding asks through it with its own intent and pair.
- **The report fires once.** `Interpreter::report_solid_black_generation` asks nothing while
  `black_generation_painted` is set. The flag only rises within its scope and a checkpoint restores
  the value the guard reads, so the answer is the `|=`'s.
- The borrowed context is the 5 instructions a mark above, and no epoch or cache is added to save them.

## 4. Proof that nothing moved

The four routes' tests each fail on a planted defect (trap 13): the short-circuit inverted and the
flag never set each fail `a_stated_black_generation_is_reported_and_the_page_keeps_its_space` and
`black_generation_report_requires_a_device_rgb_paint_under_the_current_pair`; a tint that never
bypasses and a stencil that never reports each fail
`a_profiled_pattern_reports_the_bound_rgb_tint_only_when_painted` (`transparency_groups.rs`).

## 5. The accepted costs at the code

ADRs 2198, 2212, 2223 and 2235 priced costs accepted for §11.7.5.3 in their own text only. Each is
now a comment beside the construction it prices, with its figures and the `black_generation_order`
mode that reproduces them: `SolidColours` (2198), `TintBinding` (2212), `IccSource` (2223, 2235).
ADR 2249's was already beside `SpaceSource`.
