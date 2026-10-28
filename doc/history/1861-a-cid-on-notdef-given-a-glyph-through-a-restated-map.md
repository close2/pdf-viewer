# 1861 — a CID on .notdef given a glyph of its own through a restated CIDToGIDMap

Session 1861, Slot 1 of batch 127, an archive round in two sessions; ADRs 2556 and 2557 used, Q727 unused. I worked
in the shared worktree and checked, tested and counted in an export of HEAD plus my own hunks.

**Built.** R03a for `CIDFontType2` (ADR 2556): `fonts/cid_notdef_glyph.rs` takes the survey's shown strings of a
`Type0` font over a `CIDFontType2` with a `glyf` program, groups the shown CIDs on glyph 0 by their `/W` width in
program units, gives one copy of glyph 0 per width by `notdef_glyph::with_notdef_copied` unchanged, and restates the
`/CIDToGIDMap` as a stream (Table 115; an `/Identity` or absent map written out for every glyph the program held), a
stated `/CIDSet` given the new CIDs' bits (Table 124). A reload from in-memory objects proves every shown code on its
old glyph or its copy at its width. The descendant names the new descriptor and map; the content stream and `/W` are
the producer's. `LoadedFont::cid` (new, `pdf-font`) gives the CMap's CID before §9.7.6.3's fallbacks, and a code
reaching glyph 0 by a fallback is refused. The fence and rewrite sentences name the composite case.

**Second session.** veraPDF failed the copy's output for `4387ba48` on 6.2.11.8, CID 0: §9.7.4.2 makes CID 0's
glyph the CIDFont's `.notdef` analogue, so a shown CID 0 now keeps the refusal (ADR 2557) and the identity test is
inverted. The `.notdef` program row stated the placeholder for an unasked construction where a font it names is not
reached; it now states the remedy's own condition (`prepare.rs`, `obstacle`), with a test.

**Premise.** The copy is reused unchanged: held. The composite caller beside `truetype_cmap`'s: held, as a branch of
`restated`. "Four documents": six carry a `CIDFontType2` with `glyf`; one converts (`issue10519_reduced.pdf`), two show
CID 0, and three are refused on other grounds (ADR 2556 section 4). Not held: that the copy reaches any count of shown
codes, since the fonts are named from the checker's place list, cut short past 32.

**Bounds.** Map at most 65 536 entries; at most 256 widths and 1 MiB of outline (ADR 2543's); 32 MiB of copied program.
`getrusage` beside HEAD (debug): 60 000 CIDs at 256 widths 25 492 KiB against 25 132 KiB, at 60 000 widths 27 252 KiB
against 25 872 KiB; 32 CIDs at 32 widths 18 816 KiB against 14 872 KiB.

**Gates.** In the export: `cargo build -p pdf-transform --bins` and `-p pdf-sandbox --bins` 0; census no-pack and
with the pack under `bounded.sh` 0 (405 s, 2.40 GiB peak; the pack's 0), each pdf.js line +1 converted against HEAD's,
corpora unchanged; `rank.py` 0 on both, R03a 24 to 23 documents (alone 15 to 14); veraPDF on `issue10519_reduced`
compliant at 2b and 4, the planted `4387ba48` input non-compliant (8 rules); `cargo test -p pdf-transform --test
archive --test profiles` 0 (484 and 12 passed); clippy `-p pdf-transform -p pdf-font --all-targets --no-deps -D
warnings` 0; rustfmt on my Rust files 0. Plants: the CID 0 check removed fails
`cid_zero_under_the_identity_is_refused_and_no_file_is_written`, the reason fix removed fails
`a_descendant_the_copy_does_not_reach_states_the_copy_s_own_reason`, both restored. In the worktree: `--bin cited` 0, `--bin quotations` 0;
`cargo test -p conformance --no-fail-fast` 101, 68 binaries ok and one red, `every_record_since_the_rule_states_its_gates`
on round 1864's record in progress.
