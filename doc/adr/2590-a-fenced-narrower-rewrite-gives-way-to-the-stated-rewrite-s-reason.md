# 2590 — A fenced narrower rewrite gives way to the stated rewrite's reason

Status: implemented — session 1878, the eighth slot of batch 128 (an archive round). Context: ADR 2543 (the
`.notdef` row's two answers, `Answer::OrStated`), ADR 2557 (a shown CID 0 keeps the refusal), ADR 0816 (the fence),
`doc/rfc/0014` section 3 (the report names each object's step).
Code: `crates/pdf-transform/src/archive/decision.rs` (`narrower_or_stated`).

## Context

`Answer::OrStated` tries a narrower rewrite and then a stated one. Where both refused, the narrower one's refusal stood.
At `fonts/no-notdef-glyph-shown` the narrower rewrite is the Type 3 rename, and `sites::type3_notdef_renames` refuses
with `Because::TheFence(THE_GLYPH_IS_NOT_IN_THE_PROGRAM)` wherever a font the row names is not a Type 3 font. So every
font with a program that the glyph-given remedy also refused was reported with the fence sentence. That sentence says
the rename does not reach the font. It never says why the remedy that does reach it refused: `CID_NOT_GIVEN` for a
shown CID 0, the copy's own condition (`NOTDEF_NOT_GIVEN`), or `CMAP_NOT_PREPARED` where another row refuses the file
anyway.

## Decision

Where both answers refuse, the narrower rewrite's refusal still stands, unless it is `Because::TheFence`. A fenced
narrower rewrite has said only that it does not apply to these fonts. The stated rewrite is the one that applies, so
its refusal is the reason shown. A narrower refusal of any other kind is a reason about the fonts themselves, and it
stays.

## Consequences

- **The premise "four rows' reasons change" did not hold.** Four rows are `OrStated`
  (`fonts/symbolic-truetype-states-no-encoding`, `fonts/non-symbolic-truetype-uses-a-standard-encoding`,
  `fonts/no-notdef-glyph-shown` and `fonts/truetype-codes-reach-glyphs-by-the-standard-route`). In the census against
  HEAD's, only `fonts/no-notdef-glyph-shown` changes, because only its narrower rewrite refuses with the fence. Without
  the pack: in the pdf.js sections the six documents under the fence become five under the copy's condition and
  `issue11768_reduced.pdf` under `CID_NOT_GIVEN`. In the corpora's `as-if-printed` sections the ten become seven under
  the copy's condition, two under `CID_NOT_GIVEN` (`emi15514-sup-0002-SupinfoS2.pdf`, `4387ba48…`) and `507676.pdf`
  under the TrueType route's refusal. In their `as-shown` sections the eleven become seven, two under `CID_NOT_GIVEN`
  and two under `CMAP_NOT_PREPARED`. Every summary line of the census is unchanged; only these sentences differ.
- `a_width_no_advance_can_hold_is_refused_and_no_file_is_written` now expects the copy's condition, not the fence.
  `a_font_both_remedies_refuse_states_the_cid_remedy_s_reason` is this decision's test, and it fails with the rule
  taken out.
