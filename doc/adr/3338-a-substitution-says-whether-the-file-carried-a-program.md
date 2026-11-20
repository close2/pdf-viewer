# 3338 — A substitution says whether the file carried a program

Session 2159. Status: accepted. Context: ADR 2636 (a cut program dropped and substituted), ADR 3326
(`507676`'s fonts whose programs their checksums disown), ADR 0836 (`Damage::CheckValue`).

## The defect

The substitution's `xmpMM:History` line said of every font it embedded a face for that it "was not
embedded" — "XIMJKA+TTE201B838t00 (resource R19) was not embedded and … was embedded in its place".
`507676`'s font *was* embedded: its `/FontFile2` decodes to its end and the Adler-32 over it
disagrees. And the cut program's own clause then said its data "does not decode whole …, so the
file held only the front of its program", which is true of `Truncated` and `Corrupt` and false of
`CheckValue`, `ForeignCharacter` and `Unframed`, each a decode that reached its end.

## The decision

- `SubstitutedFont::what_the_file_held` gives the verb phrase: "was not embedded" where the font
  carried no program, "carried a program this conversion removed" where `cut_program` is set; the
  history line uses it. The run report's "drawn from" line states no premise and is unchanged.
- `CutProgram::describe` says, per damage, what the file held: the front of a program
  (`Truncated`, `Corrupt`), a program whose own check value disagrees with it (`CheckValue`), or a
  program decoded past what its filter's format admits (the rest), each "not counted as embedded".
- The operator's supplied-fact subject ("which this file did not carry") becomes "whose own program
  this conversion removed" for such a font.

The test plants both a refused header and a disowned checksum and reads the history line of each.
