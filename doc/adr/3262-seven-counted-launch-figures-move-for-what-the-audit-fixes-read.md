# 3262 — Seven counted launch figures move for what the audit fixes read

Session 2140, a measure round in the `lane3` worktree. Status: **accepted**. Context: ADR 0917 and ADR 2944 (the
counted open and first frame), ADR 2950 (the bands taken under Rust 1.100), ADR 3261, round 2135's failing run;
`doc/checks/launch-path.toml`. Clauses: ISO 32000-2 §14.9.4, §9.10.3, §7.6, Annex D.

**Where the bands were taken.** Round 2068 measured them on `bf59984d` with its own work rebased on top; main then
took seven audit-fix commits (`7471eeff` to `24f948d5`) before `373e7eb6` landed, and none of those, nor the 64
after it, ran the gate. Rebuilt in release under 1.100.0 with the gate's own instrument (callgrind, this program's
object only), `bf59984d` reads every counted figure inside its band — EC3's first frame 159 061.3 against the
band's stated 159 060.5 — and `373e7eb6` reads seven outside. So the drift is the audit fixes', not the compiler's.

**What each figure pays, by function, `bf59984d` against this tree with ADR 3261** (thousands, own object):

- **EC3 first frame, 159 061.3 → 169 113.2.** §14.9.4 makes the outermost `/ActualText` govern what its element
  encloses (ADR 2976, `2ef6887a`), so the walk up `/P` now reaches the structure root where the `/Lang` walk it
  replaced stopped at the nearest `/Lang`. Each ancestor is read once a page (ADR 3261), but reading one is a
  parse of its whole dictionary, and the specification's top-level elements carry `/K` arrays of thousands:
  `Parser::parse_object_from` +6.6 M, `inflate_fast_help_avx2` +1.9 M (object streams decoded for them). Nothing
  smaller reads one key of an indirect dictionary, so this is the clause's price on this document.
- **AN001 first frame, 10 853.3 → 11 085.5.** `ToUnicode::own` +180 (`eb2a7f72`: a code matches a mapping only at
  the length its codespace gives it, §9.10.3), `Document::expand_object_stream` +55 and the stream's map +22
  against `Document::get` −68 (`5ab240a3`, §7.6's which-stream rules: an object stream's contents
  are now expanded and kept as one map, read through `parse_plain_at`), `LoadedFont::drawing`
  +37 (`fa29fe4c`: a borrowed face in the outline cache's key), `folded_base_font`'s name matching +72 in
  `memcmp` and `str::contains` (`3a742aaf`: a CJK name read through its collection), the marked-content walk +39.
- **`opt_demo` first frame, 3 079.6 → 3 150.5**: `read_fonts::ps::agl::lookup_char` +235 (`eb2a7f72`: a glyph name
  by Annex D's whole AGL algorithm, components and `uniXXXX` forms included), `drawing` +35, `notdef_advanced`
  +5 (`fb3699c3`).
- **`xfa_filled_imm1344e` first frame, 3 266.7 → 3 340.3**: `drawing` +35, `ToUnicode::own` +27, `LoadedFont::text`
  +12, `lookup_char` +12, `notdef_advanced` +5, and its open below.
- **The opens — `xfa_filled_imm1344e` 272.1 → 282.5, `opt_demo` 396.8 → 404.9, `bug1815476` 1 752.2 → 1 761.7.**
  A near-constant 8 to 10 thousand per document, spread thin: `parse_plain_at` (`5ab240a3`), more dictionary
  copies and drops through `Document::get` and `catalog` for the open's audit checks, the page tree's set-backed
  cycle test (ADR 2981, `2ef6887a`). No one function holds more than 3 of it.

**The decision.** Each of these is a clause the commit now honours, read at the open or on page one because that
is where the clause's answer is needed. The one cost that was the implementation's rather than the clause's —
`structure::enclosing` copying every ancestor on every sequence — is removed by ADR 3261 (AN001 −1 261, WTPDF
−1 117, EC3 −789). The seven bands move to this file's rule (the measured figure widened by one per cent,
rounded outwards) around this tree's figures, each with a comment naming this ADR. WTPDF's first frame returns
inside its band with ADR 3261 and does not move.

**Not decided here.** EC3's `peak_anon_mib` (21.07 and 21.92 in round 2135's runs, band 15.5 .. 20.5) is a memory
figure (21.457 in this round's gate run) the instruction profile cannot apportion; the ancestors §14.9.4's walk now parses stay in the document's
object cache and are the likely cause, unmeasured. It is reported, not moved.
