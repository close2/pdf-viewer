# 1942 — A profile is a lower bound, and three sites a viewable document met are answered

Session 1553, on the project owner's words of 2026-10-15 ("I thought that the profiles were lower
bounds, they would specify what information we would accept to lose without declining"; "maybe we
should have a profile: 'like shown'"). Status: **accepted** and **built**. Context: RFC 0007
§5a, ADRs 0816, 1099, 1123, 1200, 1285, 1943; `doc/questions/Q448`.

## 1. What the owner met

`quorra-transform archive tmp/2026-10-14/Pol.pdf --to 2b` exited 4 and wrote nothing, over two
sites. The `as-if-printed` profile existed (`doc/profiles/`) but answers by `--config <path>`
only, the window offers no archive path at all, and the profile answered neither site: it had no
row for `annotations/flags-entry-present`, and `fonts/truetype-codes-reach-glyphs-by-the-standard-route`
was ADR 0816's fence. Answering both exposed a third, `fonts/no-notdef-glyph-shown`, which this
validator did not report for a Type 3 font and veraPDF did.

## 2. Decisions

1. **A profile is a lower bound on what may be lost, never an instruction to lose it.** Where the
   standard leaves a lossless answer and a lossy one, a shipped profile picks the lossless one
   even inside its bound. Both `as-if-printed` and the new `as-shown` answer the absent-flags
   site with `preserve`.
2. **`preserve` at `annotations/flags-entry-present`** (ISO 19005-2 section 6.3.2, first
   sentence) keeps the annotation and writes `/F 4`: the in-place `preserve` of ADR 1285, which
   needs no `--authorise`. Without a configuration the site still asks for `--authorise
   annotation-printing`, because what goes is Table 167's statement that the annotation is not
   printed, and whether that is a loss is a company's policy — the owner's word that profiles are
   that policy. Q448 asks whether the default should change.
3. **The TrueType route is answered in the font dictionary, not the program.** §9.6.5.4's route
   for a non-symbolic font runs code, name, Adobe Glyph List, (3, 1) subtable. Where the file's
   own `/ToUnicode` states the code's one character and the (3, 1) subtable maps it to the glyph
   this reader drew (ADR 1943), `/Differences` names that character's glyph: `[160 /nbspace]` in
   Pol.pdf, ISO 32000-2 Annex D.2's own note on that code. The preparation reloads the font and
   requires every shown code to reach the same glyph by the route; anything else keeps a refusal.
   `Stated`, because a reader that chose another mapping drew another glyph. The program is not
   touched, so ADR 0816's fence stands.
4. **A Type 3 `.notdef` is renamed** (ISO 19005-2 section 6.2.11.8). The validator now reports a
   shown code a Type 3 encoding names `.notdef`; the remedy gives the procedure another name in
   `/Differences` and `/CharProcs` alike. §9.6.4 keys a Type 3 glyph by name, so the same
   procedure draws the same marks. A program's `.notdef` is glyph 0 by position and keeps ADR
   1200's fence.
5. **`as-shown` ships beside `as-if-printed`**, and they differ only where the standard lets the
   screen and paper differ: Table 167's Print and NoView bits and the forbidden subtypes' drawn
   appearances (preserved by ADR 1123). Its `printable-and-visible` row is `stop` until a
   per-annotation split exists, because one answer cannot keep a visible non-printing annotation
   and remove a hidden one.
6. **`--profile <name>`** names a shipped profile, compiled in; `--config` and `--profile` are one
   choice. The run names the profile once on stderr; each site it answered says so in the report.

## 3. Consequences

Pol.pdf converts to PDF/A-2b under `--profile as-if-printed` and veraPDF 1.30.3 passes it; its pages
render as the source does except DeviceCMYK text, which the existing `DefaultCMYK` row (§10.4.2.5)
moves by up to 34 of 255 — that row's sentence says this renderer already showed those colours so,
and for CMYK it does not (`doc/todo/72`). 2u and 4 still refuse (`doc/todo/72`).
