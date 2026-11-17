# 2054 — Both font findings were built before the brief, and SAML joins the XML row

Session 2054, a fonts round in the gwg worktree. Rows 9.6.2.1, 9.8.1, 9.6.5.4 and 9.10.2 read, none moved.
ADRs 2924 and 2925 and Q883 were assigned and are not used: no decision was made.

**The slice.** The brief's two items from round 2027's "Left" — a substituted simple font with no `/Widths`
advancing by the face's widths, and a substitute that reaches no glyph for a code not being reported per code —
then `doc/stack.md`'s XML row.

**The premise.** Failed for both items: each was built by an earlier round under its own ADR.
- Item 1 is A859's order, built by round 2041 (ADR 2898) and carried to the embedded-program and metrics-only
  routes by round 2045 (ADR 2906). `metrics::simple_widths` gives a substitute outside §9.6.2.2's fourteen the
  resembling standard metrics and then Table 120's `/MissingWidth`, never the face's advance; a font missing the
  array Table 109 requires is reported. Inside the fourteen, Table 109 lets the array be omitted and the face still
  answers a code the published metrics do not name (ADR 2687); that is not this finding.
- Item 2 is ADR 2880, built by round 2032. `LoadedFont::unreached_in_substitute` reports a code whose route
  (§9.6.5.1's encoding, §9.6.5.4's Glyph List and then the face's `post` table) ends at what the file states.
  The brief's "its `/ToUnicode` cannot be decoded" did not hold there either: it decodes (65 is U+5F00), and §9.10.2
  selects no glyph. `issue13316_reduced` now says "5 of the code(s)", the first `/g5167`, and stays blank.
- Neither needed the owner's word, so no Q883.

**Built.** Only the chore: `doc/stack.md`'s XML row names a signer's SAML 2.0 assertions among `pdf-signature`'s
readers over `xml.rs`, and their signature among the canonicalisation's users (ADR 2922).

**Tests and plant.** No test added. On HEAD, nine pinning tests pass (`substitute_unreached_codes` four,
`unstated_widths` three, `a_substitute_with_no_widths_keeps_the_resembling_standard_metrics`,
`a_name_the_glyph_list_cannot_map_is_looked_up_in_the_faces_post_table`). **Plant**: `unreached_in_substitute`
answering `None` and `simple_widths` asking the substitute face fail four, including the witness, then reversed.

**Gates.** No crate touched, so fmt, clippy, nextest, rustdoc and `cd fuzz && cargo check --bins` are not owed and
`raster_golden` cannot move. Pinning nextest over `pdf-sandbox`, `pdf-font`, `pdf-model`: 9 of 9 passed on HEAD,
4 failed planted. Conformance after `git add -N`: exit 101 on this paragraph's wording, then exit 0.

**Left.** Nothing of the brief. Round 2045's two leftovers stand (`state-every-loss` has no line for
`type3-width-as-drawn`; our validator misses a Type 3 code with no glyph name).
