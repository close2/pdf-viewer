# 1575 — A page whose marks cannot cross crosses as its window, and a click reaches the page

Slot 5 of batch ninety-one, 2026-10-19, a host round. ADRs 1986 and 1987; no question; no ledger
row moves.

**Premise.** (1) ADR 1944 §4's refusal was real, but its population was not the one named. No page
measured has a list over a gibibyte. The witness's is 431.7 MB, and a 150-document sample of the
corpus tops out at 80 MB. What the arm refused past the bound was every page the list format does
not carry (`Uncodable`: a Type 1 shading, a mask at the device's scale, the transfer channel).
(2) Held, one level nearer the window. `Request::Rendition` never crosses, and `Command::Pointer`
and `Event::Media` already did. The window sent no pointer at all (ADR 1987 §1). **Hypothesis**:
did not hold. The witness's list costs 618 ms a notch to encode for the crossing, plus 700 ms to
decode. Its window costs the 139–309 ms ADR 1944 measured to draw, and 7.2 MB on the pipe.

**Built.** ADR 1986: such a page is drawn by the worker as the window shows it (the viewport and a
quarter round it). It is drawn after the command that moved the view and crosses as the wire's
third payload (`PAYLOAD_WINDOW`, greeting `PDFVCF15`). The decoder admits it only past the bound,
only for a host that said `Listed`, and only inside its page. The screen places it at the corner
it states (`Content::Sent`). Pages whose marks cross are untouched. ADR 1987: a left press and its
release go to the worker as `Command::Pointer`, and the keyboard then follows the focus. The link
pins stay at `refuse`.

**Proof nothing else moved.** Only the refusal arm is replaced; no `pdf-render`, `render-cpu` or
`raster` file changed. Planted back, the refusal fails the new integration test. **Files.**
`viewer-confined`'s `src/{lib,protocol,worker}.rs`, new `protocol/windowed.rs`, `tests/confined.rs`
and `examples/confined_page.rs`; `fuzz/fuzz_targets/confined_wire.rs`; `quorra-confined{.rs,
/screen.rs}`; `tools/drive-windows.sh`; `doc/todo/30`; `doc/state-of-play.md`; two ADRs.

**Measured.** Clock holds, Xvfb, HEAD's export against this round's paths alone, twenty `=` notches.
The witness settled in 48.9–53.0 s at HEAD and 48.5–53.2 s here (four runs each, both orders), with
no refusal and identical photographs. The shaded page: HEAD 6.34–6.37 s with seven refusals and the
surround; here 6.37 s with none and the page drawn (looked at). An earlier hold of the shared tree's
build is set aside: neighbours' hunks were in it, and its photograph came before the last window.

**Gates.** `rustfmt --check` on my files: 0. Clippy `--all-targets` on `viewer-confined` and
`viewer-ui`: 0 findings in my files (a neighbour's `pdf-archive` fails `-D warnings`). Nextest:
`viewer-confined` 111 passed, `viewer-ui` 196 passed. Fuzz `cargo check`: 0. `cargo test -p
conformance`: 0. `tools/state.sh docs`: 0. `confined`: 0, with 0 killed. Drive, `quorra-confined`:
26 works, 1 not offered, 0 wrong, the two new steps included. About 8 900 s.
