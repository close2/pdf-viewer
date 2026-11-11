# 2825 — A descendant stating no descriptor is given one read off the machine's face

Session 2003, a single archive round.
Status: accepted and built. In `pdf-transform`: `archive/fonts/machine_cid.rs` (`Home`, `home_of`,
`built_descriptor`; `proved` takes the descriptor as it will stand), `archive/fonts/pack.rs`
(`Bounds` shared).
Tests: `crates/pdf-transform/tests/archive/machine_cid.rs`
`a_descendant_stating_no_descriptor_is_given_one_measured_on_the_program`.
Context: ADR 2807 (the route), ADR 2461 (the pack route's rule for where a subset's descriptor
stands, which this keeps), `doc/pdfa-blocking-issues.md` R09 (a).
Clauses: ISO 32000-2 Table 117, Table 120, Table 121, §9.8.1, §9.9.1, §9.9.2, Table 124; ISO
19005-2 section 6.2.11.4.1; ISO 19005-4 section 6.2.10.4.1.

## Context

ADR 2807's route wrote the program into the descriptor object the descendant named and stopped
everywhere else: `90ms_rksj_h_sample`'s `CIDFontType2` states no `/FontDescriptor` at all, which
Table 117 requires, and a descriptor written inside the descendant or already holding another
route's program had no home either. The pack route answers the same three cases (ADR 2461).

## Decision

1. **The same rule as ADR 2461.** The program goes into the descriptor the descendant names where
   no other route wrote one into it; otherwise the descendant is given a descriptor object of its
   own (Table 117 makes the entry an indirect reference), carrying every entry the producer stated
   but the program keys and `/CIDSet`, which describe a program this file did not carry.
2. **A descendant stating none is given every entry Table 120 requires, measured, not chosen**:
   `/FontBBox` encloses every glyph of the program written, `.notdef` included; `/Ascent` and
   `/Descent` are the highest and lowest points of the glyphs but `.notdef`; `/CapHeight` is the
   face's own `OS/2` `sCapHeight`; `/ItalicAngle` the face's `post`; each scaled from the face's em
   to glyph space's thousandth (§9.2.4). `/Flags` is Table 121's `Symbolic` alone, the collection's
   characters lying outside the Standard Latin set; `/StemV` is 0, Table 120's "unknown", since a
   `glyf` program states no stem. `/FontName` is the subset's name, as `/BaseFont` is (§9.9.2).
3. **A face that cannot state one is not this route's**: no point below the baseline (Table 120's
   `/Descent` is negative) or no `sCapHeight`; the route falls back with `machine-face`, named.
4. **The proof holds the font as it will stand**, its descriptor included, so a built descriptor is
   read by the window's reader before anything is written.

## Consequences

`90ms_rksj_h_sample` converts under `as-shown` at both parts. Nothing about the page changes: a
descriptor's metrics are not what the window draws a glyph by, and the output is held to the
window code by code.
