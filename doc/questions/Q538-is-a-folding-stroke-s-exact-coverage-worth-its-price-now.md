# Q538 — Is a folding stroke's exact coverage worth its price now?

Source: session 1643, ADR 2122, on the brief's ask for a cheaper exact measure of a folding stroke.

## Why it needs the owner

§10.7.4 owes a pixel the area of the filled set over it. A stroke that bends more tightly than its
half-width is drawn as a union of overlapping convex pieces (ADR 1348), and for cost the tree
measures those pieces with `tiny-skia`'s converter, which states a pixel in sixteenths: at the rim
of a fold a pixel is up to 50 levels from its area (`issue14415.pdf` at 4x), and the paperclip of
veraPDF's `6-3-1-t01-fail-f` reads 255 where it is 242. Round 1643 built the exact measure — the
union's boundary found once and measured once — and it agrees with the set walk to one level on
every mark it was compared on. Its price, `callgrind_rasterise` page 1, three draws:

| page | HEAD | exact | |
|---|---|---|---|
| `inks.pdf` | 281 870 234 | 694 269 784 | +146.3% |
| `rotated_ink.pdf` | 16 958 633 | 24 674 242 | +45.5% |
| `issue14415.pdf` | 984 881 667 | 1 150 169 962 | +16.8% |

The brief's bar was a fifth. Principle two makes the rasterisation time a feature; principle five
and "Acrobat-class fidelity" make the integral the target. Trading one against the other on a page
of ink annotations is not a round's choice.

## What the tree does meanwhile

The sixteenths, as before: no pixel moves. The construction is a patch kept outside the tree, `/home/AI/r1643-folding-union/folding-union-boundary.patch` (the owner reads it with `sudo -u AI cat`, as the T.88 bundle was read) in the batch's worktree, against HEAD `aa74b695`, with its test.

## Recommendation

**Wait for the cheaper measure.** Keep the sixteenths and their stated bound until a round brings
the construction under a fifth on the three pages, and then route it. ADR 2122 §5 names where the
price is: on `inks.pdf` two folds whose pieces cross 27 and 40 times an edge hold more than half
the crossings, and at least 43% of the page's crossings start or end no boundary stretch (124 000
a draw against the 35 000 stretches' 70 000 ends), so the lever is to stop finding those rather
than to find them faster. If the answer is instead "exact now", the patch applies as it stands and
moves the folding strokes' rim pixels on every page that has one: 25 of `raster_golden`'s 974 first
pages, raster only, the division census unchanged.
