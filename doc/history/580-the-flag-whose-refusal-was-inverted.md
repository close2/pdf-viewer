# 580 — `/AIS`, honoured, and the refusal that stood where the clause guarantees the answer

Date: 2026-08-18. ADR: [0415](../adr/0415-the-flag-whose-refusal-was-inverted.md).

Touched `crates/pdf-model/src/content.rs`, `content/ext_gstate.rs`, `content/path.rs`,
`content/text.rs`, `content/transparency.rs`, `crates/pdf-model/tests/transparency.rs` and
`transparency_groups.rs`, `doc/conformance/ledger.toml`, `doc/todo/23-transparency-departures.md`,
`doc/todo/28-a-catalogue-that-draws-nothing.md` and `doc/todo/README.md`.

## The ranking was stale, and saying so was half the round

`doc/todo/23` was taken as the demand half. The instruction named its two open constructions — a
second colour space for a group, and the knockout group whose elements blend against a backdrop
that is not transparent — and **both had been built in the four-hundred-and-ninety-second** (ADR
0327); the file says so and `doc/todo/README.md`'s one-line index did not. Ranked again from the
witnesses the file itself names, every row of its table has **0** corpus documents and 5 to 9 of
65 944 web ones, and the largest corpus population anywhere in the file is the nine documents that
state §11.6.4.3's `/AIS true` — the refusal ADR 0234 left behind.

## The demand half — §11.6.4.3's `/AIS`

Read against §11.6.4.2, §11.6.4.4, §11.3.7.1 and §11.3.7.2, the flag turns out to *guarantee* the
thing this tree refused for it. Under `/AIS true` the mask is shape and both alpha constants are
shape, §11.6.4.2 has already made an elementary object's intrinsic opacity 1.0 everywhere, and
§11.3.7.2 multiplies the three together — so the source opacity is 1.0 and §11.3.7.1's alpha *is*
the source shape. **The number a rasteriser already draws the element with is the shape.**

So the shape half of `Command::Shaped` is the element itself with its blend mode dropped, §11.4.6's
two stages are the `DestinationOut`/`Plus` pair all three backends have drawn since ADR 0291, and
nothing was added to the display list, to a backend or to a raster. The refusal was not narrowed but
**inverted**: `/AIS true` is the one reading under which a rasteriser's single number per pixel
cannot disagree with the shape, and the condition admitting a *bare* knockout draw had to be
tightened there rather than loosened, because a bare draw reads the paint's own alpha as opacity.

What is refused in its place is a scope rather than a construction. `Interpreter::alpha_sources` is
a three-valued record of which readings a group's content painted under, seeded at the `Do`, folded
outward when the group closes, and narrowed by `alpha_sources_mark` so that a reading nothing was
painted under is replaced rather than mixed in — the shape a real file has, a form whose content
opens with the `gs` that states the entry. A group that painted under both readings is refused with
a report that says so. §11.6.2's `B` pair and §9.3.8's text object take the same reading through
`transparency::knockout_group_elements`, where they had asked a page-wide flag.

## The populations, and they are the reason the round is worth reading twice

Measured with an `eprintln!` at each of the four sites over both corpora, before writing anything
(trap 11).

- **974-document corpus: zero.** Not one knockout group with `/AIS` in force, on any page of the
  nine documents that state it. §11.6.4.3's row had said "none of their knockout groups is drawn
  today" and that had expired when ADR 0327 scoped the flag.
- **65 944 crawled web documents: one.** `6573550.pdf`. Its knockout group draws now; 13 736 of its
  1 128 099 pixels move, by at most 3 of 255, all inside the artwork the group paints, and the page
  keeps its unrelated §11.4.4 report.

The hand-built witness is what discriminates: one fixture, two readings, **127 of 255 apart**.

## The spec half — four rows of the clause 11 family

§11.6.4.3 and §11.6.4.4 carried the debt and now carry the derivation; §11.4.6's residue list is
re-stated (it gained a non-isolated group used as an element under the shape reading, and lost
`/AIS`); §11.3.7.2's two named debts are down to one, the shape channel every command carries. The
`retired` sweep over `AIS` found four more sentences in the ledger and one in `doc/todo/28` still
saying the entry refuses a knockout group, and each is corrected in place.

## What `doc/todo/23` still owes

Nothing with a corpus witness. Three rows with 5, 8 and 9 web witnesses out of 65 944, and the
largest of the three wants a conversion between **two presses** per pixel at a group boundary — a
function rather than a quantity, which is the one thing the file has always said a display list
cannot carry.
