# 3496 — 020747 is within the dash bound on its page

Session 2198. Status: **accepted**. Context: ADR 3474 (the refusal it revisits), ADR 3495 (the
count over the region), ADR 2759, `doc/pdfa-blocking-issues.md` R03 and R08.

## The measurement

ADR 3474 refused `020747` because four dashed strokes of page 7 state about 10⁸ dashes each over
paths its damaged stream made up to 10⁹ units long; `dash_count` over the whole paths gives 0.94,
1.12, 1.52 and 1.76 × 10⁸. Counted over the page as ADR 3495 counts —
the 592 by 807 crop box under the stated width, and the target at 72 and 150 dpi — they are
7 056, 4 116, 1 393 and 3 289 dashes (7 064 at most on a target), and a fifth stroke of 7.2 × 10⁶
dashes lies wholly off the page. The figure ADR 3474 read as the page's is the off-page length's:
the page shows one dash in 25 000 to 67 000 of what each path states.

## Decision

**The bound no longer stops the page, and nothing replaces it.** The window now draws page 7
whole, its on-page dashes as stated, from the bytes the stream decoded; the Adler-32 failure is the
same damage every other damaged page converts under, by the losses a profile grants. Under
`as-if-printed` the document converts at part 4 (veraPDF 1.30.3 passes it) and stops at 2b on
ISO 19005-2 section 6.1.13's values; under `as-shown` it stops at ADR 0816's fence as before.
