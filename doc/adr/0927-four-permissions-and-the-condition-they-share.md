# 0927 — Four things a converter may now write, and the condition all four carry

Session 940. Status: **accepted**. Recorded under the questions-directory rule: when an `A` file
appears, the round that acts on it records the decision in an ADR.

## The answers

Four arrived together, and they are the first permissions this project has granted to write
something a producer did not.

| | asked | answered |
|---|---|---|
| `A18` | May the converter ship an ICC sRGB profile and add it as an output intent? | Ship it, with a flag to override it. **And report that adding an output intent reinterprets the marks already in the file** — the difference must be visible in the report, not only in the bytes |
| `A21` | May a converter write the annotation appearances this tree constructs? | Allow it, and report every appearance written |
| `A48` | The DeviceN `/DefaultCMYK` construction, and an empty glyph in place of `.notdef` | Allow both, report both per document, and record both in `xmpMM:History` naming the clause |
| `A50` | May the deprecated `F` operator be replaced with `f`? | Allow it narrowly — a closed list of operator spellings the standard itself documents as equivalent, applied only where a target's deprecation rule requires it, and reported. No broader licence to rewrite content streams is granted |

## The condition

Every one of the four is conditional on the same thing, and the owner wrote it four different ways
so it is worth stating once: **what was written is reported.** Not logged, not inferable from a
diff — named in the report the conversion produces, per document, and for `A48` also in the file's
own `xmpMM:History` with the clause that licensed it.

That is what makes these permissions rather than a licence. A converter that silently produced a
conforming file would be indistinguishable from one that produced a *wrong* file, and the
distinction this project cares about is not whether the output conforms but whether the reader can
see what was done to it.

## The line, and where it will be tested

`A48` states the boundary the four share, and it is sharper than the "does the operation invent
marks?" fence in `CLAUDE.md`:

> state an interpretation the standard defines; never fill in an absence

The two constructions the answer names sit on opposite-looking sides of it, which is why it is
worth writing the reasoning down rather than only the rule.

- **`A18`'s sRGB output intent states an interpretation.** A `DeviceRGB` colour in a file already
  means something — §10.4.2's rules already decide what this renderer shows for it — so declaring
  sRGB records the interpretation the renderer was applying anyway. **But it is not free**, and the
  answer's second sentence is the reason: an output intent is what a conforming reader colour-
  manages *through*, so adding one can change what a different reader shows. The report must say
  so; that is not a courtesy, it is the whole of what makes the default safe.
- **`A21`'s constructed appearance states an interpretation** too. §12.5.5 and the subtype clauses
  describe what an annotation looks like; a constructed `/AP` writes down the appearance the
  standard already specifies, which is why this tree could construct it in the first place.
- **`A50`'s operator substitution is the purest case.** The standard documents `F` and `f` as the
  same operator; the substitution changes the spelling and not the marks, and the closed list is
  what keeps it that way.
- **`A48`'s empty glyph is the hard one, and the next round to touch it should say so.** A code
  that reaches `.notdef` has no glyph the standard defines; substituting an empty one is closer to
  filling an absence than the other three are. The owner allowed it with reporting attached, which
  is the right shape — but the honest description is that this is the case where the line is
  thinnest, not one where it is comfortably satisfied.

## What the tree does now

Nothing in `crates/` changes: the converter does not exist yet, and `A46` puts the validator first.
What changes is `doc/pdf-a-conversion-limits.md`, which is the document a converter will be built
from. Its §4.1, §4.4, §2.2, §5.2 and §10.1 carried these as *provisional defaults with a question
attached*; they now carry them as decisions, each with the reporting condition stated as part of
the decision rather than as a nicety, and its §8 no longer lists them as open.

The report format those conditions imply is not designed here. It is the first thing the converter
round has to design, and it now has four requirements to design against rather than none.
