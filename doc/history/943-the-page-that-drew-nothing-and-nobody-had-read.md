# 943 — The page that drew nothing, and nobody had read it

Date: 2026-09-10.
ADR: 0935 (the clarification that adds failures and the one that withdraws them); 0940 (a font
program stream of no bytes is no program).
Files: `crates/pdf-font/src/program.rs`, `crates/pdf-archive/src/survey.rs`,
`src/table/graphics.rs`, `src/clarification.rs`, `crates/pdf-model/tests/silent_fonts.rs`,
`crates/pdf-model/examples/empty_font_program_census.rs` (new), `doc/todo/00`,
`doc/conformance/ledger.toml`.

## The sweep, and why its stability is the point

`doc/todo/02` §7 makes a round that changes what gets drawn re-run `doc/todo/00`'s step 7: our ink
minus the lightest reference's, over every ambiguous page. Session 942 implemented `/FixedPrint`,
so it was owed.

The sweep's head came back as **session 806's six entries, to the thousandth, 137 rounds later**.
That is the finding before any defect: the instrument is stable, so a name in it that nobody has
read is worth reading, and a name that moves is a real movement rather than noise. The change that
occasioned the sweep could not have moved it, and that was measured rather than assumed — no
watermark annotation exists in any of the 964 gate documents that open.

Sixteen incomplete names had never been read to the end; ADR 0433 named eleven of them and
stopped. **Two were named nowhere in this tree at all.**

`bug866395.pdf` page 1 drew **0.000** against ghostscript 8.549, hayro 11.502, poppler 11.780 and
mupdf 11.801. Its `/FontFile3` is a ten-byte `FlateDecode` stream that reaches RFC 1951's final
block and produces zero bytes. The empty string went to the sfnt reader *as a program*, came back
malformed, and the font was refused — where a descriptor stating no program at all would have been
substituted for and the page would have drawn.

§9.8.1's Table 120 says the entry **is** a stream containing a font program, and §9.9's Table 124
requires the program to conform to a named format. No zero-length string does, so the descriptor
stated none. The ordering carries the argument: ADRs 0343 and 0836 refuse a *truncated* program
because a prefix draws glyphs in place of the producer's, and zero bytes can be read as nothing at
all. One such stream in 1 237 documents across five corpora, and none between one and eleven
bytes.

`issue5954.pdf` is the other unread name and is **not** a defect: its page states an empty
`/XObject` dictionary while the `/Pages` node above states one defining `/F1`, and §7.7.3.4
inherits as-is without merging and stops at the first dictionary found. `poppler` reaches the same
reading. It is diagnosed into `doc/todo/00` rather than left as a number — which is the difference
between a bucket that shrinks and a bucket that is understood.

## Measuring a rule's blast radius before changing it

TechNote 0010's A028 was flagged by session 942 as the clarification that *adds* failures,
including to files that break nothing today. Rather than take it and watch `over`, the round
measured first, over 1 552 documents by 6 targets: 41 documents have a device colour whose licence
turns on a default in force, 7 run a content stream against a resources dictionary it fell back
on, and **the two sets are disjoint**. No witness could move. None did.

The shape it forced is worth more than the row. `DefaultSpace` now carries two readings —
`in_force`, which ISO 19005-4 states in its own words and which the renderer obeys, and
`explicit`, which is A028's for part 2. **A validator reads §8.6.5.6 differently from a renderer**,
and the ledger row says so, so that `pdf-model`'s inheritance stops looking like an oversight.

A010 is deferred with an argument. What is missing is section 6.2.2's published exemption, which
no row states at all and which A010 only narrows; and proving a named resource unreferenced is a
whole-file reachability problem where an error in the permissive direction **withdraws real
failures in silence**. That is worse than the over-report it would fix.

## Two comments that had been welded together

`program.rs`'s `extracted_cff` carried `is_bare_cff`'s opening paragraph and `simple_units_per_em`
carried `parsed_type1`'s, leaving two functions undocumented and invisible to every gate that
counts documentation. It is the same failure `doc/todo/00` records for its own group constants,
found in a different file by a round that was looking at something else.
