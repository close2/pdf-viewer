Status: complete
Given: 2026-10-21, in conversation — transcribed by the round
Owes: the notices — every Foxit face the converter writes carries `LICENSE_FOXIT` verbatim in its program stream's `/Metadata` XMP packet under `xmpRights:UsageTerms` (§14.3.2's stream, one round's work in `pdf-transform`'s archive `program_stream`), and a restated Liberation face is renamed in its `name` table, the OFL's own condition for a Modified Version, so no restated program keeps the Reserved Font Name (acting round)

> I agree with the proposed answer (and recommendation) for Q550.  Please answer

Reading: the recommendation adopted is the question's own, quoted and attributed: *"Carry the
notice in the output, which costs nothing a reader sees"* — and, for the restated Liberation
face, the rename that is the OFL's own condition.

What the answer settles:

- **The Foxit faces carry their BSD-3-Clause notice in the output.** ISO 32000-2 §14.3.2 lets any
  stream carry a `/Metadata` stream, and the program stream's XMP packet holds `LICENSE_FOXIT`
  verbatim under `xmpRights:UsageTerms` for every Foxit face the converter writes — the safe
  reading of "redistribution in binary form", whatever "redistribution" turns out to mean, so
  the embedded face ships with the notice the licence asks the distribution to carry.
- **A restated Liberation face stops using the Reserved Font Name.** The older embedding route
  restates `hmtx` where the file's stated widths do not match, which is a Modified Version under
  the SIL OFL; the rename in the `name` table is the licence's own remedy for exactly that, and
  it is taken rather than the refusal, so the route keeps embedding with its condition met —
  the descriptor-less route's refusal stays as it is, because it restates nothing.
- **The unmodified copies owe nothing**, as before: the OFL's preamble exempts documents created
  with the fonts, and the descriptor-less route embeds only the unmodified program.