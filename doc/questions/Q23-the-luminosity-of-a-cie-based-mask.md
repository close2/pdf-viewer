# Q23 — Do we keep the clause's luminance, which no other renderer computes?

Source: ADR 0797 (session 879) and ADR 0857 (session 907), arrived at independently one component count apart.
Status: **open** — answered when `A23-the-luminosity-of-a-cie-based-mask.md` exists beside this file.

## Why it needs the owner

§11.5.3 says that for CIE-based spaces the luminosity is the Y of the colour converted to XYZ, and its own example generalises that to other calibrated spaces. Every other renderer uses the plain device grey with no calibration. Following the clause moves 38 of 41 affected crawl pages away from the reference consensus, and the population is roughly thirty-three thousand mask groups in the crawl.

## What the tree does meanwhile

The clause is followed, per principle 5, and both ADRs record it as a reading awaiting the owner's ranking rather than a settled decision. A pinned witness shows both directions on one page.

## Recommendation

Keep the clause. If the owner wants the convention available, the honest shape is a host-supplied policy offering it beside the clause, never a silent switch, which is what ADR 0797 proposed when it first hit this.
