Status: complete
Given: 2026-09-22, in conversation — transcribed by the round
Owes: Annex F's linearised output behind `optimize --linearize`, `doc/qpdf/` read as input where the specification is unclear, and `CLAUDE.md`'s "Annex F stays excluded until linearisation is separately ratified" sentence ending with it (acting round)

> I think we should now also implement linearize (I was told to use qpdf source code as
> input, when the spec isn't clear -- I have cloned it into doc).

> no, don't implement linearize.  Just write the answer

> Do not implement anything!  We just answer the questions!

Reading: the question was "declined permanently, or deferred", and the answer is neither —
linearisation is ratified, and this word replaces the one this file carried since 2026-09-06
("declined for now", by the owner's hand). The three sentences quoted are one decision in
sequence: the first ratifies Annex F for the serializer, and the second and third scope the
round that received them — it writes the answer down and implements nothing, and the build
is a later round's, which is what the `Owes` line names. The qpdf checkout at `doc/qpdf/`
is input where the specification is unclear, which is principle 5's own position on another
implementation: agreement with it is evidence that we read Annex F correctly, never the
definition of correct, so a test's expected value stays derivable from the standard and a
disagreement goes back to the clause rather than toward qpdf.