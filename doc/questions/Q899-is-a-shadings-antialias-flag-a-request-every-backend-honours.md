# Q899 — Is a shading's `/AntiAlias true` a request every backend honours?

Source: review 2038's finding 2038-D15 (`doc/reviews/2038-specification-audit/fixes.md`, G033), taken by round
2077. Nothing waits on the answer: the ledger row says the reading is open, and no page changes until it is
answered.

## What was found

ISO 32000-2 Table 77 (in §8.7.4.3, the entries common to every shading dictionary) lists:

> AntiAlias boolean (Optional) A flag indicating whether to filter the shading function to prevent aliasing
> artifacts.

Its NOTE 3 says the shading operators sample a function at the device's resolution, that aliasing can occur where
the function is not smooth, and that anti-aliasing "is usually unnecessary". The default is false. The entry
states no `shall`, names no filter, and — unlike §10.7.2, which says a processor may ignore flatness — no sentence
permits a processor to ignore a true value. Errata Collection 3 says nothing about it.

No code reads `/AntiAlias` (`grep -rn AntiAlias crates raster` finds no read). ADR 0037 counted 19 corpus
documents that set it; ADR 1765 confirmed the claim "read nowhere". The ledger row said the flag "owes nothing".
The audit's point is that being optional for a producer, together with the lack of a stated kernel, does not
give a consumer permission to ignore `true`. Round 2077 rewrote the row's note to say the reading is open, and
left the row `implemented` on Table 77's three `shall`s.

## The question

Is a true `/AntiAlias` a requirement this program must honour?

- **(a) Recommended: yes, as a request honoured on every backend.** Where the flag is true, the shading function
  is filtered: for Types 1 to 3, each pixel's colour is the mean of the function over sub-pixel samples of the
  pixel's footprint in shading space (4 × 4 is enough to remove the step aliasing NOTE 3 describes); for the mesh
  types (4 to 7) the function is filtered the same way where one is present. The choice of filter is ours and is
  written down as a choice, because the standard names none. The cost is up to sixteen function evaluations per
  pixel, only on shadings that ask for it. A render round would build it on `render-cpu`, `render-raster` and
  `render-gpu` together, with a test that fails if the flag is ignored (a stepped sampled function whose
  edge pixel is a mixed colour only when filtered). The 8.7.4.3 row is then `partial` until it is built.
- **(b) No: treat the flag as a hint, like §10.7.2's flatness.** The row records this as a deliberate choice,
  argued from NOTE 3's "usually unnecessary" and from the missing `shall`, in an ADR. Nothing is built.

The audit marked this as a reading to settle rather than a defect it could show with a pixel.
