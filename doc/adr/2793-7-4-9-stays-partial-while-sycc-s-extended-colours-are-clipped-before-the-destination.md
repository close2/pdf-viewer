# 2793 — §7.4.9 stays `partial` while sYCC's extended colours are clipped before the destination

Session 1991, a ledger round (§7.4.9, §7.6.6). Status: accepted. Row: §7.4.9; its aggregate §7.4
follows (ADR 1599). Builds on ADRs 1383, 1510, 2292 and 2293; A348 is the owner's word it reads.

## Context

The row's note said it was `partial` "because CIEJab (19) alone still takes the device fallback".
A348 answers 19 outright: CIE Publication 131 will not be bought, and only the owner reopens that
half. Read alone, that is the shape `departed` has: one requirement decided against, its cost
recorded, nothing owed. So this round asked whether everything else in the clause runs.

It does not, quite. ISO 32000-2 §7.4.9 asks that "PDF processors shall support the JPX baseline set
of enumerated colour spaces", and T.801 Table M.25 lists sYCC (18) among them. ADR 2794 holds the
codec's sYCC to PIMA 7667's normative Annex C, the held text that converts the same encoding, and
the codec agrees with it inside sRGB's gamut. Outside it, the two part. Annex C.2 converts to
e-sRGB, whose range runs past 0 and 1. The codec clips each nonlinear component to 0 to 1 before
this crate sees the raster. A screen of sRGB's primaries clips at the same bounds, so it shows the
same pixels. An output intent's press or a blending colour space does not, and it receives a colour
that was clipped first.

## Decision

1. **§7.4.9 stays `partial`, on two reasons.** CIEJab is the owner's decision under A348. sYCC's
   extended range is effort only. The note names both, and says only the second is owed.
2. **The row becomes `departed` on 19 when sYCC is drawn by Annex C in this crate**, as e-sRGB
   (20) and e-sYCC (24) already are (ADR 2292): `ExtendedEncoding` gains Annex C's C7 and C8 in
   front of B9. A later round need not argue 19 again. A348 is the argument, and this ADR is the
   departure's record once nothing else is left.
3. **That build was not taken in this round, because of its cost to writers.** Today a writer
   asking for a sYCC image's samples (redaction re-encoding, the archive converter's transcoding)
   receives the codec's RGB. Under Annex C, ADR 2293's refusal would reach it: no §8.6 space
   restates e-sRGB's extended range. The corpus census of ADR 1510 found two sYCC boxes that
   decide a colour, so the refusal would turn two writable images into refusals by name. That is
   a change to the archive lane's behaviour, and it is the converter's to price first.

## Consequences

- §7.4 stays `partial`: it is an aggregate (ADR 1599), and §7.4.9 owes.
- ROMM-RGB (21) remains drawn by the codec and pinned only for neutrals. `doc/questions/Q825`
  asks whether PIMA 7666 is to be bought, so that it can be held to its text the way 18 now is.
- Reopened when: sYCC is drawn by Annex C (see 2); or the owner reopens A348.
