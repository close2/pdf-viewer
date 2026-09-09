# Q53 — The basic types a clarification lists, and the two it does not

Asked by session 942, which took `TechNote 0010` A020. **Provisional, not a blocker**: the row is
implemented, `over` is zero on all six targets, and what is open is how far one list reaches.

## The background, in one paragraph

ISO 19005-2 section 6.6.2.3.1 requires every property an XMP packet states to use a predefined
schema or a conforming extension schema, and `metadata/properties-use-known-schemas` reads *using*
a schema as carrying the value type the schema gives the property. A020 is the ISO working group
resolving how far a validator may take that: parts 1 to 3 are read as if an XMP value were
validated **on its type alone**, with anything else inferable from the property's name or
description disregarded. The item then lists the rule for each basic type — booleans, integers and
reals as the XMP Specification defines them; `Text`, `ProperName`, `URI`, `URL`, `AgentName`,
`Rational`, `RenditionClass` and `XPath` as any string; `MimeType` by RFC 2046; `Date` by
ISO 8601.

Session 942 carried the one consequence the list states outright: a `Rational` is admitted as any
string, so `exif:XResolution` and its kind are no longer held to a quotient. Two points of the same
list are not decided by it, and this is them.

## The first: a basic type the list does not mention

`GPSCoordinate` is a basic type of the XMP Specification — `exif:GPSLatitude` and its three
siblings carry one — and it has a written form: degrees, minutes and either seconds or fractional
minutes, closed by a direction letter. This crate checks that form (`Lexical::Coordinate`).

A020's list does not mention the type at all, and the two readings of that silence go opposite
ways:

- **The check stays.** A resolution that says nothing about a type withdraws nothing about it. The
  requirement being clarified is still ISO 19005-2's, the type still has a form the specification
  spells out, and A020's own principle — validate on the type — is what checking that form *is*.
- **The check goes.** The list is introduced as the exact validation rules for the basic types, and
  a type absent from an exact list has no rule. That reading is supported by the company
  `Rational` keeps in the list: it has a definite written form too, and the working group put it
  among the types admitting any string rather than among the ones with a rule.

**What the tree does meanwhile: the check stays.** ADR 0931's third condition is that this project
acts on a working group's resolution rather than on the case around it, and dropping a check the
resolution does not name would be acting on the item's spirit. It is the reading that risks a false
failure rather than a missed one, which is the wrong direction for this crate — so it is asked
rather than left.

## The second: a rule stated in a standard this project does not hold

A020 gives `Date` as ISO 8601. This crate implements the six date profiles the XMP Specification
lists, which are profiles *of* ISO 8601 — so everything it accepts, A020's rule accepts too. The
gap is the other way: a date written in some other ISO 8601 form, the basic format `20260910` for
instance, is reported here and would not be under A020's rule read literally.

Closing that gap means implementing ISO 8601, which this project does not hold, and `CLAUDE.md`
principle 5 forbids implementing a standard from somebody else's reading of it. So the question is
which of three:

1. **Leave it.** The divergence is narrow, no corpus witness exercises it, and the XMP
   Specification's profiles are what an XMP packet is written in.
2. **Widen the check by hand** to the ISO 8601 forms a reader can enumerate without the text. This
   is the option principle 5 argues against and it is named so that it is refused deliberately.
3. **Obtain ISO 8601.** A purchase, and a small one for what it settles — this row and nothing
   else this project has met.

**What the tree does meanwhile: option 1**, with the divergence written down at `Lexical`.

A third point of the list is not a question but a debt, recorded here so it is not mistaken for
one: A020's `MimeType` rule is RFC 2046, and no property is judged against it — `dc:format` and its
kind carry `Lexical::Any`. That under-reports rather than misreports, and RFC 2046 is freely
readable, so it is work rather than a decision.

## What an answer changes

Only the first point changes code either way, and by four properties. The value of asking is that
the alternative to asking is a round quietly reading a silence in a clarification as a licence,
which is the failure mode ADR 0931 exists to prevent.
