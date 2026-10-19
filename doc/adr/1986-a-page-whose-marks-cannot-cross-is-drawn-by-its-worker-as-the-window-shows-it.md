# 1986 — A page whose marks cannot cross is drawn by its worker as the window shows it

Session 1575, slot 5 of batch ninety-one, ADR 1944 §4's residue under the project owner's "switch to
code improvements … until I say otherwise" of 2026-10-14. Status: **accepted** and **built** in
`viewer-confined` and `quorra-confined`. Amends ADR 1944 §2.2, where such a page was refused.
Context: ADRs 0607, 0640, 0725, 1871, 1892, 1930, 1944. Code: `protocol::windowed`, `Marks::
{hold_window, windows_owed, drew, refused_window}`, the wire's `PAYLOAD_WINDOW` (greeting
`PDFVCF15`), `decode_window_payload`, the worker's `draw_windows`, `viewer_confined::Payload::
Window`, the confined screen's `Content::Sent`. Tests: the protocol's
`a_window_drawn_here_crosses_where_the_page_could_not` and `a_window_no_worker_could_have_drawn_is_
refused`, `windowed`'s three, `tests/confined.rs`'s `a_page_past_the_bound_whose_marks_cannot_
cross_crosses_as_its_window`. Drive: step `82-zoom-past-the-bound-uncrossable`. Cites no clause
and moves no ledger row.

## 1. What the brief supposed, and what held

The brief named the residue as "a page past the bound whose display list is over 1 GiB" and the
choice as the list in parts or the worker's window, with the hypothesis that the parts are cheaper
"for a window of a page whose list is mostly outside it". **The population was the wrong one, and
the hypothesis did not hold.**

- **No measured page has a list over a gibibyte.** The witness's list is 431.7 MB (4 408 405
  marks, `examples/list_against_raster`); a 150-document sample of the disk's corpus tops out at
  80 MB. What `Marks::decide` sends to the refusal is wider than size: every page the list format
  does not carry (`Uncodable` — a Type 1 shading or a mask resolved at the device's scale, the
  transfer channel, a nest too deep) took the raster arm below the bound and was refused above it.
  The toolkit windows draw all of them. The list in parts would have reached none of these.
- **On the witness the list is the dearer crossing.** `examples/list_over_the_wire` prices its
  crossing at 618 ms to choose and encode, then 742 ms to encode exactly and 700 ms to decode, for
  each notch. The window this process would draw instead is 1 201 × 1 501 pixels, which ADR 1944
  measured the processor drawing in 139–309 ms, and its 7.2 MB cross at the pipe's rate. A list
  mostly outside the window still crosses whole; only the window's pixels are the window's size.

## 2. Decision

1. **A page past the bound whose marks cannot cross is drawn here as the window shows it**, from
   the list this process holds, and the viewer is told `Rendered::Listed`. The window is the part
   the viewport shows and a quarter of the viewport round it, inside the page — `viewer_host::
   oversize`'s rule, written again in `protocol::windowed` because this process links no host crate
   (ADR 1871). The frame states the window's corner, and the host places it there, so the two
   statements of the rule need not agree for the page to land where it belongs. That is the cost:
   twenty lines said twice.
2. **It is drawn after the command that moved the view**, beside `Marks::place`: a zoom's first
   window, and a scroll's next one where the viewport leaves what was drawn. The window then
   crosses with the next frame like a page's pixels. A window the rasteriser refuses lets its page
   go and is said as `Event::Reported`.
3. **The wire carries it as a third payload** (`PAYLOAD_WINDOW`), and the greeting moves. The
   decoder admits a window only for a target past the bound and only for a host that said
   `Oversize::Listed`, and only where the window lies inside its page and holds a pixel. Its
   samples are checked against the bytes sent.
4. **A page whose marks cross is not touched.** That includes the witness. The device draws its
   marks at the window's size as ADR 1944 built, and a scroll costs it no crossing and no
   processor draw. Whether a page past the bound with a 432 MB list should also cross as windows,
   giving up §1's 0.6 s a notch for a processor draw at every margin a scroll leaves, would change
   what the device draws. It is a later round's measurement.

## 3. What this does not do

- It builds no list in parts. The brief's own page, a list over a gibibyte that the format
  carries, reaches the same arm and is drawn as windows too, though no page measured has one.
- The worker draws a window inside the command, so a heavy one is a command still out past `WARN`
  (ADR 1827). That is the same as any page that crosses as pixels.
