# 3475 — issue13316_reduced stops at the embedding that R11 restates over

Session 2193. Status: **accepted**; refused by name, the build named. Context: ADR 2773 (the
machine's face embedded), ADR 2880 (a code the file takes to no glyph), ADR 3125 (a code on
`.notdef` given an empty glyph in a font made symbolic), ADR 3326 (R11's restatement over the face
the window drew), ADR 3463 item 4.

## What refuses first

`issue13316_reduced` shows one TrueType font, `/YouYuan`, whose `/FontFile2` decodes whole and fails
its Adler-32, so the window draws it from a substitute face (ADR 2870); its `/Differences` name five
of the shown codes `/g5167` and the like, which neither the Adobe Glyph List nor the face's `post`
reaches, and the window draws nothing for them (ADR 2880). `as-if-printed` converts it by the
photograph (ADR 3193). Under `as-shown`, R11's row
(`fonts/non-symbolic-truetype-differences-are-listed-names`) restates the route over **the face the
substitution embeds** (ADR 3326), and the substitution refuses at `fonts/font-programs-embedded`
with ADR 2773's sentence: the machine's face has no glyph, by the embedded font's route, for those
five codes, and embedding it would reference `.notdef`. So R11's refusal is the embedding's,
carried; it is not a route R11 lacks.

## Decision

**Refused by name, as it stands.** The construction that would answer it is ADR 3125's, extended:
a code the file's own route takes to no glyph (ADR 2880's population, which is a statement about
the file and not about the machine) is given an empty glyph, with the font made symbolic, the same
way a code the encoding sends to `.notdef` is. ADR 3125 builds that only for a font the file does
not embed (`face_widths`' `empty_allowed`); this font embeds an unreadable program, which ADR 3326
routes through the substitute's face, and the extension must hold both ADR 3326's proof (every
other code on the glyph the window drew) and A48's condition (the empty glyph states no character).
It is the first item of R09's remainder in `doc/pdfa-blocking-issues.md` section 1.
