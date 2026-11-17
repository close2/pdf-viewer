# 2926 — The counted open keeps no viewport, because the program's open has none

Session 2055. Status: **accepted.** It moves no band in `doc/checks/launch-path.toml` and no ledger
row. Context: ADR 2916 (the counted open stopped raising Table 197's `/PV`), ADR 2666 (the counted
figure is this program's own object), ADR 1531 (page one interpreted on the document's thread).

## 1. The question

ADR 2916 §2 read the fall of two `open_kinstructions` floors as work "a window's open still does",
and left the move of the counted child to a sized viewport to a later round. The premise was that a
window always has a size, so the viewport-less child counts less than a reader's open.

## 2. What the program's open is

It has no viewport either. `quorra.rs`'s `open_document`, which the harness's `open_document` copies
step for step, builds `Viewer::new(0, 0, 1.0)` ("the window does not exist and may not for another
50 ms"); `viewer-host`'s `Launch::start` does the same for the toolkit windows. `Viewer::settle`
returns at once with no extent, so the open neither arranges nor interprets. The size arrives with
the window's first `Command::Resize`, after the join and after `Viewer::anticipate` has interpreted
page one on the document's thread. That `Resize` settles: it arranges, `visibility_of` raises `/PV`
(§12.6.3, "when the page containing the annotation becomes visible"), and the page's render is
asked for. So `/PV` left the open in the program as well as in the gate, at `79d0c6c4`, and is now
part of the first frame, on the main thread after the join. The gate's `first_page_ms` contains it,
since the first-page child gives its viewer the gate's `VIEWPORT` by the same `Resize`.

## 3. What a sized counted child would count, measured

The counted child built with `Viewer::new(800, 1000, 1.0)`, `quorra`'s window size, under
`callgrind`, against the same release build of this tree with no viewport (own object, thousands):

| document | no viewport | 800 × 1000 | `raise` inclusive, then | `interpret_arranged` |
|---|---|---|---|---|
| `bug1815476.pdf` | 1744.3 | 66839.3 | 188.4 → 286.3 | 81922.9 |
| `opt_demo.pdf` | 396.5 | 3247.6 | 343.8 → 386.5 | 3365.5 |

`raise` returns to ADR 2916's figures for `77514286` (285.0 and 386.7): `/PV` is back. But the
`settle` that ends `Command::Open` with a viewport also interprets page one, which the program does
in `anticipate`, outside the open; on the font-walk row that is 38 times the open. The figure would
stop being an open's count and become a count of a sequence no host runs.

## 4. The decision

1. **The counted child keeps `Viewer::new(0, 0, 1.0)`.** It is the program's open, and the bands
   ADR 2916 restated stand.
2. **`/PV` is measured where the program does it**: in the first frame, by the first-page clocks.
   No counted figure holds the first `Resize`'s `settle` today. A counted first frame (open,
   `anticipate`, then the window's first `Resize`) would be a new figure with its own bands, not a
   redefinition of this one.

## 5. The window's size is a choice nobody wrote down

`quorra` and `quorra-confined` open an 800 × 1000 logical window
(`with_inner_size(LogicalSize::new(800.0, 1000.0))`), `viewer-gtk` asks 1000 × 1100
(`set_default_size`), and `viewer-confined`'s worker starts at 794 × 1123 ("a page of A4 at 96
dpi"). No ADR or comment argues the first two. The launch gate's first-page and page-turn figures
are taken at its own `VIEWPORT`, 1600 × 1000, which its comment states as "a window a person would
have", not as any host's default.
