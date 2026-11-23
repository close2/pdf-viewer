# 3416 — An absent /AS, and a /Rect or /Matrix of the wrong shape, are said

Session 2178. Status: accepted. Context: audit 2175 (B05, B06).

## Decision

1. **An `/AP` subdictionary with no `/AS`** (or a non-name one) is `Normal::NoState`: nothing is
   drawn, as for a state with no appearance, and the absence of Table 166's required entry is
   reported. §12.5.5's "reasonable behaviour (such as displaying nothing)" covers an `/AS` naming
   an undefined state, not a missing one.
2. **A rectangle is exactly four numbers and a matrix exactly six** (§7.9.5; Table 93's "array of
   six numbers"). `annotation::rectangle` and `matrix` no longer drop a non-number and shift the
   rest, nor take the first four or six of a longer array. A malformed `/Rect` is no rectangle:
   with a stored appearance it is placed by its `/BBox` and `/Matrix`, as for an absent `/Rect`,
   and said; with none, the existing "no usable /Rect" is said. A malformed `/Matrix` is read as
   the identity Table 93 states for an absent one, and said. The hit tests and `link`'s reader
   take the same reading.
