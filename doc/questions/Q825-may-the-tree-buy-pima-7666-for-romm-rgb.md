# Q825 — May the tree buy PIMA 7666, the text that defines ROMM-RGB?

Source: round 1991 (ADRs 2793, 2794), the ledger lane, reading §7.4.9's remainder.

## What stands

- ISO 32000-2 §7.4.9: "PDF processors shall support the JPX baseline set of enumerated colour
  spaces". T.801 Table M.25 hands ROMM-RGB (enumerated colour space 21) to "PIMA 7666".
- **21 is drawn by the codec**, with a ROMM-RGB profile of the codec's own. Nothing held defines
  it. PIMA 7667's Annex E.1, which you bought (A348), converts between e-sRGB and ROMM RGB, but it
  is informative and it sends the reader to "4.4.4 of PIMA 7666" for the nonlinear encoding. So
  the one test pins only that three equal components draw a neutral grey. Whether a colour is
  right is not tested, and under principle 5 the codec is evidence, not the definition.
- **sYCC (18) is now held to a held text.** PIMA 7667's normative Annex C converts it, and the
  codec agrees (ADR 2794). IEC 61966-2-1 Amd. 1 is therefore not needed for the test, and this
  question does not ask for it.
- **No document in reach states 21** (ADR 1510's census: 105 108 JPX images, none). Buying the
  text would make the support testable. It would change no page anyone has.
- CIEJab (19) stays as A348 left it, and this question does not reopen it.

## The question

May the tree buy PIMA 7666:2001 (*Reference Output Medium Metric RGB color encoding: ROMM RGB*),
which IS&T offers as a legacy PIMA standard beside PIMA 7667? A scripted read of IS&T's standards
page returned an error page, so its price is to be read in a browser. PIMA 7667 cost USD 25.

## Recommendation

**Buy it if IS&T sells it at PIMA 7667's legacy price.** It closes the last baseline space that
is drawn but not testable from a held text, at the cost of one fixture. If it is not sold that way,
decline: ISO 22028-2 restates ROMM RGB, but at ISO's price for one unexercised space.

## What the tree does meanwhile

21 is drawn by the codec and pinned for neutrals. §7.4.9 stays `partial` on ADR 2793's reasons,
and this question is not one of them.
