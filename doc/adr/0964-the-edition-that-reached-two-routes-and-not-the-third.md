# 0964 — The edition that reached two routes and not the third, and the base standard part 2 actually names

Session 961. Status: **accepted**. Three findings from one sweep of `pdf-archive`'s
`Check::Unchecked` and `Check::Processor` reasons and of the crate's claims about what this
project holds. Two were reasons that had stopped being true; the third is a *ground* that was
never true for half the targets it was applied to.

## Why the sweep was run again so soon

Session 958 (ADR 0956) read every `Unchecked` reason on every target and found three stale out of
twenty. It did not read the twenty-three `Check::Processor` reasons, and it did not read the
crate's ordinary doc comments for the same class of claim — a sentence about what this tree or this
project *has*, written truthfully once and never re-read. `doc/habits.md`'s six shapes are about
exactly that, and two of the three below are shape three: a capability arrived and announced
nothing.

All twenty-three `Processor` reasons were read against ISO 19005-2 clause 6 and ISO 19005-4
clause 6 this session. Every one of them is accurate: each names a sentence whose subject is a
processor, and in each case the file's half of the rule is a neighbouring row that is
`Implemented`. Nothing moved there, and that is the answer rather than the absence of one.

## 1. Four ICC editions arrived, and one of the three routes to a profile never heard

`missing_required_tags` and `IccEdition` judge a profile against the edition its own header names,
for the four editions this project holds. Three kinds of profile can reach them:

| route | clause | judged before this session |
|---|---|---|
| an `ICCBased` colour space's stream | ISO 19005-2 section 6.2.4.2, ISO 19005-4 section 6.2.4.2 | yes |
| the output intent's `DestOutputProfile` | both parts' section 6.2.3 | yes |
| a JPEG 2000 `colr` box of `METH` 2 | ISO 19005-2 section 6.2.8.3, ISO 19005-4 section 6.2.7.3 | **no** |

The third row's doc comment said the sentence asking a profile to conform to the specification its
own header names "needs the ICC texts, which this project does not hold". That was true when it
was written. ICC.1:2022 and ICC.2:2023 were already here; ICC.1:1998-09 and ICC.1:2001-12 arrived
in session 950, and session 958 corrected three doc comments in the same file that said the same
thing — and missed this one, because it is not a `Check::Unchecked` reason and no sweep looks at
it.

**The clause hands the whole of §8.6.5.5 to the `colr` profile.** ISO 19005-4 section 6.2.7.3:
where the selected colour space specification uses an ICC profile, that profile shall conform to
the requirements of ISO 32000-2, 8.6.5.5. The tags of its class and the `Profile ID` are two of
those requirements, and both were already computed from the same bytes for the other two routes.
`jpeg2000_profile_conforms_to_its_own_edition` is the join; it adds no table and no reading, only a
third call site.

**It stays inside `graphics/jpeg2000-one-best-colour-space-specification` rather than becoming a
new row.** That row's own `asks` already says "any ICC profile it names shall conform to the base
standard", so this is the row's stated subject and not a second requirement — and no requirement
identifier is added, removed or re-keyed, which the converter's census ratchet
(`crates/pdf-transform/tests/archive_unconsidered.txt`) is held to in both directions.

No corpus document exercises it. `CLAUDE.md` is explicit that this is not an argument against
doing it: the corpus can rank nothing it does not exercise.

## 2. The sentence that decides the edition is not in the base standard part 2 names

This is the finding the first one produced, and it is the larger of the two.

ISO 32000-2 §8.6.5.5 closes with

> Profiles shall conform to the specification version indicated by the Profile version number in
> its header.

`IccEdition`'s doc comment cited that sentence as "what makes this the right key", full stop, and
the `Check::Unchecked` reason on `graphics/destination-profile-conforms-to-an-icc-edition` said the
base standard "sends it to the ICC specification — and to a particular edition of it, since that
clause requires a profile to conform to the version its own header states". Both rows bind **both
parts**, and a PDF/A-2 file's base standard is ISO 32000-1:2008, whose own 8.6.5.5 states no such
sentence: its Table 67 maps the *PDF* version to an ICC specification version and tells a reader to
process an embedded profile according to the PDF version being processed, which is a rule about the
reader. So for three sessions two part 2 rows have been failing documents on a reading of the wrong
edition — `Part::Two`'s standing caveat, met in the concrete.

The corpus's `6-2-3-t01-fail-d` is the witness, and under PDF/A-2b those two rows are the *only*
ones that fail it.

**The verdict survives and the ground changes**, which is why nothing was narrowed. Each held ICC
edition says what its own version field means: ICC.2:2023 section 7.2.6 and ICC.1:2022 section
7.2.4 each say the field encodes the version the profile conforms to and give the number consistent
with that document (5.0.0.0 and 4.4.0.0), and ICC.1:1998-09 section 6.1.3 and ICC.1:2001-12 section
6.1.3 say the revision numbers match the editions of the specification and give 2.2.0 and 4.0.0. A
profile stating 5.0.0.0 therefore asserts conformance to iccMAX by iccMAX's own account of the
field, whichever part is asking — and a profile that asserts an edition and fails its clause 8 is
not a *valid* ICC profile stream, which is the undefined word ISO 19005-2 section 6.2.3 actually
uses. Two grounds for one conclusion, and only one of them is part 4's.

**So the check reached the right answer for a reason that did not apply.** That is the shape
`CLAUDE.md` principle 5 is about — a test's expected value must be derivable from the
specification, and its comment must say *from where* — and it is invisible to every gate this
project has, because the verdict was right.

The new JPEG 2000 call site is part 4 only for the same reason, and there the difference is not
recoverable: ISO 19005-2 section 6.2.8.3 delegates to ISO 32000-1:2008, 8.6.5.5 *by name*, so a
part 2 `colr` profile is held to that clause's Table 68 fields and nothing more. The asymmetry is
the clause's, not a limitation.

## 3. ISO 32000-1 has been readable since 2026-09-07, and one comment still said otherwise

`ADDED_BY_ISO_32000_2` explained how ISO 19005-2 section 6.3.1's set of annotation subtypes is
derived "without reading ISO 32000-1, which this project does not hold" — and cited `Part::Two`'s
note for it. That note has said the opposite since the owner obtained ISO 32000-1:2008 (Adobe's
free copy; `doc/questions/A49`, `doc/PDF32000_2008.pdf`, `doc/md/ISO_32000-1_2008.md`). A comment
that names its own source and disagrees with it is the cheapest kind of stale claim to find and the
easiest to leave standing.

The derivation itself is correct, and now it is *checked* rather than argued:
`the_part_two_subtypes_are_the_ones_iso_32000_1_defines` transcribes ISO 32000-1:2008, 12.5.6.1,
Table 169 less the three both parts strike by name, and asserts that subtracting ISO 32000-2 Table
171's two PDF 2.0 rows from `PERMITTED_IN_PART_FOUR` gives the same twenty-three subtypes. A
subtraction that has been held to the document it stands in for is a different claim from one that
has not.

## 4. One reason sharpened rather than closed

`metadata/file-identifier-changes-with-history` said the clause's condition — whether an
`xmpMM:History` entry was *added* — needs "an object resolved as of an earlier section, which
`pdf_syntax` does not offer", and stopped there. There is a route, and it is one this tree already
uses elsewhere: §7.5.6's prefix property means the bytes from the start of the file to the end of
an earlier revision *are* a document, so `Document::open` over that prefix resolves its catalog and
its `/Metadata` with no revision-aware resolver at all. What actually blocks it is one number —
`pdf_syntax::xref::SectionRecord` states where a section begins and not where its revision ends,
and the end is where the prefix has to be cut. The reason now says that, because a blocker priced
at one field is a different decision from a blocker priced at a resolver.

## What did not change

- No requirement identifier was added, removed or re-keyed.
- The corpus harness reports `over` = 0 on all six targets before and after, with the one standing
  miss (`6-6-2-3-3-t03-fail-b`, errata A029) unchanged.
- The coverage census is unchanged on every target: the work here is a third call site inside an
  existing predicate and three corrected claims, none of which moves a row between states.
