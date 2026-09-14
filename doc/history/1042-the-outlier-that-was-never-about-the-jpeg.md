# 1042 — The outlier that was never about the JPEG (corpus slot, `batch-1038-1043`)
## The head, verified first
`tools/state.sh oracle` before anything: the *ambiguous, undiagnosed* ranking printed nothing and
the seven counts were the thousand-and-thirty-sixth's, so the three documents that round left —
`issue2391-1.pdf`, `issue4575.pdf`, `issue6413.pdf` — were the work, each rendered by this tree and
`mupdf`, `poppler`, `ghostscript`, `hayro` at 72 dpi, inks read, pixels looked at.
## Three documents, three of principle 5's cases
**`issue4575.pdf`**: the file's. `/Width /Height`, no `/Height`; every renderer refuses the image
and draws the text. The refusal said *missing or invalid*, true twice without saying which; it
names the entry's fault now, four sentences, witness read out of the file (`image_dimensions.rs`).
**`issue2391-1.pdf`**: the file's, twice. `undefined 10 Tf` — the keyword §7.8.2 does not
recognise was reported; the `Tf` it left one operand of Table 103's two was refused in silence, as
every operator short of operands was. `Unsupported::OperandShortfall` now, a §7.8.2 corpus row of
five documents, all already incomplete. Stripping the tokens one at a time showed `poppler` blanks
the page on the short `Tf`, not on the keyword.
**`issue6413.pdf`**: round 505 read `mupdf`'s ink (3.55 against 6.3) as the JPEG whose frame
contradicts its dictionary. The pixels: nobody draws that JPEG — its form states `/BBox [0 0 0 0]`
— and what `mupdf` loses is the *black bar* after the form, whose stream ends `Q W`. A `W` the
stream ends on is now reported beside `BT` without `ET` (§8.5.4, §8.10.1); `mupdf` carries it into
the page and clips everything to the red bar. A `W` with no path in front of it is deliberately
not reported, on ADR 0563's argument: `issue14438.pdf` states one and loses nothing. **And the zero
BBox is a pixel this tree owes**: §8.5.3.3.1's degenerate subpath "shall be considered to enclose
the single device pixel lying under that point", §10.7.4 makes a clip the fill's pixels, and both
rasterisers give it no coverage — invisible here, that pixel's centre being outside the image at
72 dpi; on §8.5.3.3.1's ledger row, not taken.
## Calibration and cost
Both raise sites disabled: seven tests red. Zero new incomplete documents; `MAX_INCOMPLETE` stays 61.
The report found a defective fixture: `colour_paths.rs`'s flatness stroke wrote a `c` with four
operands, skipped in silence, so the curve it compared was never drawn — six now. Three
`raster_golden` movers are *list only* on pages no change here reaches (`bug1703683_page2_reduced`,
`bug1721218_reduced`, `personwithdog`), a sibling's in-flight change; their rows stay at `HEAD`.
## Gates
`rustfmt --check` on the twelve files touched 0 (`cargo fmt --all --check` 1, siblings' three files);
`clippy -D warnings` 0 on every target of mine (workspace 101 on siblings' `nested_probe.rs`,
`transparency_groups.rs`, `appearance.rs`); `nextest --no-fail-fast` 4624/4631 — the fixture above,
fixed; the FFI trio, broken by a sibling's momentary edit, 45/45 re-run; three siblings'; `--doc` 0;
`fuzz` fmt 0, clippy 0; `conformance` 0 on my rows, 101 on two siblings' rows naming unwritten tests;
`corpus` ok, 61 incomplete, 0 slow, §7.8.2 row 5; `raster_golden` 10 named — seven *reports only*,
regenerated, three *list only* left (re-run held 971, moved 3); `oracle` ok, counts identical, head empty.
