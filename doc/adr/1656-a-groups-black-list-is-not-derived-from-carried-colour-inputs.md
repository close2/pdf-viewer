# 1656 — A group's black list is not derived from carried colour inputs

Session 1410. Status: **accepted**. Answers ADR 1645 section 3, the exact form it named and asked a
builder to price on ordinary pages first. Amends nothing; the black run stays as ADR 0272 built it.
Context: ISO 32000-2 §11.3.4, §11.4.6, §11.4.7, §11.7.4.4; `CLAUDE.md` principle 2's rule that an
optimisation is justified by a benchmark; ADRs 0272, 1632, 1645, 1657; habits 43 and 45; trap 13.
Code: none kept. The probe was built in an export of HEAD and measured there.

## 1. The rule the brief set

Carry each mark's colour inputs while a four-component group's chromatic run runs and resolve them a
second time under the black plane, so the black list is derived rather than interpreted again —
built only if an ordinary page pays under 0.1% for the carrying. ADR 1645 put the most it could save
at about 356 M instructions (18 ms) of `bug1721218_reduced.pdf`'s turn.

## 2. The tax, measured

A probe of the carrying's shape on every page that runs no such group: an `Option<Box<_>>` on the
interpreter tested in `Interpreter::colour`, in `conversion_under` and the image conversion, at both
arms of `overprint_blend` and in `draw`, and two `u32` indices on `GraphicsState` set where a colour
is set. Never taken on these pages. Callgrind, `--profile gates`, one thread, `examples/turn_interpret`;
ISO 32000-2 page 101 as a turn (a run of eleven less a run of one, over ten), the launch rows'
page one as one interpretation; HEAD and the probe as md5-distinct exports in their own target
directories.

**The instrument's floor first.** One binary measured twice moved by +0.037% on page 101, +0.057% on
`bug1815476.pdf` and −0.069% on `bug1721218_reduced.pdf`'s turn — every instruction of it in libc's
allocator and `memmove`, `pdf_model`'s own functions at +0 each time (`HashMap`'s random seeds move
the allocator's path). So the tax is read off `pdf_model`'s functions, and the totals are not.

| page | `pdf_model` with the probe | without its `draw` test |
|---|---|---|
| ISO 32000-2 p101, turn, 174.3 M | +26.1 k, **+0.015%** | +9.2 k, +0.005% |
| `opt_demo.pdf` p1, 4.04 M, 463 commands | +4.8 k, **+0.118%** | +2.1 k, +0.051% |
| `xfa_filled_imm1344e.pdf` p1, 5.05 M, 574 commands | +6.2 k, **+0.122%** | +2.7 k, +0.054% |
| `bug1815476.pdf` p1, 120.8 M | +12.3 k, +0.010% | +3.7 k, +0.003% |
| `PDF20_AN001-BPC.pdf` p1, 11.3 M | −1.1 k, −0.010% | — |
| `Well-Tagged-PDF-WTPDF-1.0.pdf` p1, 91.6 M | −0.6 k, −0.001% | — |
| ISO 32000-2 p1, 122.7 M | +3.3 k, +0.003% | — |

The tax is about ten instructions an emitted command — `show_text` pays most of it, where the
overprint arm and the `draw` test sit in its per-glyph path — so it is a share of the page that grows
as a page's commands get cheaper. Page 101 pays under the brief's line; the two smallest launch rows,
whose commands cost about nine thousand instructions each, pay over it.

## 3. What exactness needs that the site list did not name

ADR 1645 counted the places a run asks which plane it is on. A run also decides on what a colour
*resolved to*, and that decision is per plane: `blend_at_the_do` moves a knockout group's blend mode
to its `Do` where the parts are one solid colour, compared bit for bit on the run's own three
channels. `0 0 0 0.3 k` and `0 0 0 0.7 K` are white on the chromatic plane and two greys on the black
one, so a `B` under `/BM /Darken` inside a `/DeviceCMYK` group took two constructions and HEAD gave
the pair up with a report — on four hand-built fragments: a `B`'s implicit group, a form's knockout
group, a soft mask's group, and the form's again with both parts carrying `0.2` cyan. Each kind has a
twin whose parts are one colour, which keeps its pair, and the form's a twin under `/BM /Multiply`,
which keeps it too (trap 13). ADR 1657 makes the two runs agree.

A list derived by substituting paints would inherit the chromatic plane's construction and pair by
construction, so the parting the second run reports would be drawn silently instead. Exactness
needs every such construction decided again over the derived black parts — and the chromatic list
holds the rewritten group, not the parts it was decided over. That is the reproduction of every
colour decision ADR 1632 section 5 named as the danger, with a site the count had not found.

## 4. Decided

Not built, on both findings: the carrying costs the smallest launch rows over 0.1%, and a derived
list is not exact without re-deciding each construction from parts the list no longer holds. What
would change it: a carrying shape that costs nothing where no group runs (the `draw` test folded
into `marking`'s, the overprint record moved to the cold arm), and a carry that keeps each knockout
construction's parts beside its result.
