# 3103 — A composite font whose script no face draws still draws what a face does

Session 2100. Status: **accepted.** Context: ADR 3067 (per-character borrowing, whose last consequence named this),
ADR 1154 (one matcher), ADR 0152 (`uncovered_character`), `crates/pdf-font/src/loading/opening.rs`
(`load_composite`, `machine_offers_a_substitute`), `crates/pdf-font/tests/borrowed_face.rs`. Clauses: ISO 32000-2
§9.7.4.2, §9.10.2.

## Context

A non-embedded composite font is chosen a face by its collection's script sample (한 for `Adobe-Korea1`), and each
character that face does not map is borrowed from a face that does (ADR 3067). Where no face on the machine maps
the sample, `load_composite` refused the font whole with `FontError::NoSubstitute`, so a Korean font's digits and
Latin letters were lost on a machine without Hangul, though faces drawing them were installed.

## Decision

§9.10.2 gives each code its character, and §9.7.4.2 leaves character the only way to a face; neither ties the
characters of one font to one face. So **where no face covers the sample, the family's own face
(`installed_styled`, the request and style the font derived) is the font's face**, every character it lacks is
borrowed as before, and each character no face maps is reported per code by `uncovered_character` — the glyph
coverage report, which trap 5 asks to stay loud. The refusal remains for a machine that offers no face at all.

`machine_offers_a_substitute` keeps asking whether a face draws the script: the tests that skip on it measure that
face, and a font drawn only in borrowed Latin would measure nothing they assert.

## Consequences

- No page moves on this machine, where Noto Sans CJK covers every registered collection's sample; the change
  reaches a machine without a CJK face, and a confined worker's port answers it by the same description.
- Test `a_font_whose_script_no_face_draws_still_draws_what_a_face_does`: a fixture machine of a kana face and a
  Latin face; an `Adobe-Korea1` font draws `A` and reports 한. Planted (the fallback removed), it is refused.
