Status: complete
Given: 2026-10-14, in conversation — transcribed by the round
Owes: the datum transformation built — the Helmert transformation from the held IOGP Guidance Note 7-2 (its formulas cited to the Note's own sections, its worked examples the test inputs, the pattern ADR 1587 set), applied where the file states its parameters, and refused by both datums' names where it does not: a WKT1 `TOWGS84[…]` clause is read and obeyed, a `/DCS` whose WKT states no parameters keeps `Refusal::DatumTransformation`, and no registry or grid is carried (acting round)

> Q436: I don't care how many documents use it.  Why wouldn't we implement it.  It doesn't sound difficult.  If there is a good reason not to implement it, fine.  But just because there is no document in our corpus is not a reason.

Reading: **option 2 — build it.** The recommendation's ground was the empty
census, and the owner's word retires that ground as a reason: it is the same
sentence `CLAUDE.md`'s two denominators state — "a corpus cannot rank a
requirement no document exercises" — and a question asked of the wrong
denominator, which is worth recording because the next round will face the
same shape again.

Of the reasons a round could still have given, none survives as a good one:
the parameters' provenance is the only real cost, and option 2's shape bounds
it — transform where the *file* states the parameters, refuse by name where it
does not, which is the shape every other requirement here takes. The
transformation itself is a seven-parameter Helmert whose formulas and worked
examples are in the Guidance Note this tree already holds and already builds
from; no registry, no grid, no epoch question is opened, because a string that
states no parameters is refused rather than guessed for. A271's reading
already declined the registry, and this does not reopen it.

The row: §12.10.2 stops being `partial` on this requirement once the
transformation runs and the refusal is a stated one.