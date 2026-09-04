# Q18 — Should a converter ship a colour profile, or require the caller to supply one?

Source: RFC 0006 §10 question 4.
Status: **open** — answered when `A18-shipping-an-icc-profile.md` exists beside this file.

## Why it needs the owner

Every part requires an output intent with a profile, and a converter cannot invent one. Shipping one is a data dependency; requiring one makes the verb unusable without an argument most callers cannot supply.

## What the tree does meanwhile

Nothing built.

## Recommendation

Ship the standard sRGB profile, whose grant is permissive and whose version satisfies part 1's bound, with a flag to override. Note separately that adding an output intent reinterprets every existing mark in this renderer, which is the sharpest finding in RFC 0006.
