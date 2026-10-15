# Q448 — Should an annotation stating no flags be given Print without a profile?

Source: session 1553, `tmp/2026-10-14/Pol.pdf`'s 35 Link annotations; ADR 1942 section 2.2.
Status: **open.**

## Why it needs the owner

ISO 19005-2 section 6.3.2 (and ISO 19005-4 section 6.3.2) requires every annotation but a Popup
to state an `/F` with the Print bit set. An annotation stating none has Table 166's default of 0,
which Table 167 reads as "never print". The standard leaves two futures — set Print, or take the
annotation off the page — and no third: kept as it is, the file is not PDF/A.

Setting Print removes nothing and changes nothing on screen; what changes is paper, where an
annotation with an appearance now prints. Whether that is a loss is a policy, and the owner said
on 2026-10-15 that policies are the profiles' ("Other companies have different policies").

## What the tree does meanwhile

Without a configuration the site asks for `--authorise annotation-printing`. With a profile it is
answered: `as-if-printed`, `as-shown` and `keep-everything` all keep the annotation
(`remedy = "preserve"`, ADR 1942), and `refuse-any-loss` stops.

## Recommendation

**Keep it as it is.** A conversion with no profile states no policy, and a company archiving
under `refuse-any-loss` would otherwise find its archived copies printing marks the originals did
not. If the owner would rather every run without a profile behave as `as-if-printed` does, the
change is one table row (`Answer::Stated` in place of `Answer::Loses`), and this question is that
row's permission.
