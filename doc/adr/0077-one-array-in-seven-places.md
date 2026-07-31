# ADR 0077 — One array in seven places

Status: accepted, 2026-07-31.

## Context

§14.13's ten rows were the last block of `silent` rows in clause 14 outside §14.8's vocabulary,
and the session before had just built the piece they all stand on: §7.11.4's embedded file
streams, read through §7.7.4's name tree.

An associated file is the same file specification reached from a different direction. §14.13.1
lists the objects that may carry an `/AF` array — the catalog, a page, a graphics object through a
marked-content property list, a structure element, an `XObject`, a `DPart`, an annotation, a
metadata stream — and then says the same sentence about each of them: "[t]he relationship that the
associated files have to the … is supplied by the `AFRelationship` key in each file specification
dictionary."

Measured before writing: **7 corpus documents state an `/AF`**, 6 on catalogs and 30 on structure
elements, with `Supplement` 33, `Source` 4 and `Unspecified` 2. All thirty of the structure-element
ones are MathML fragments from one LaTeX producer — which is the clause's own example, written by
somebody.

## Decision

**One function against any dictionary, because the clause is one sentence repeated seven times.**

`attachment::associated(document, dict)` reads an `/AF` array wherever it is, and the six rows
that differ only in *which dictionary* become six rows citing it. Table 43's eight relationship
values are read, with Annex E's second-class names kept rather than flattened — the table's NOTE 2
says `Unspecified` "is to be used only when no other value correctly reflects the relationship",
so a producer that registered a name has said something a reader should not throw away.

§14.13.5 is the one that is not a dictionary entry, and it goes through the interpreter:
`/AF … BDC` associates its files with the graphics objects the section encloses, recorded over the
same readback range §14.8.2.2's artifacts use. Two details are the clause's rather than a design
choice — it is `BDC` only, because NOTE 2 says "[t]he BMC operator does not take properties and
therefore cannot be used with the `AF` key", and the property list must be a **named resource**,
because §14.6.2 forbids an inline list from holding indirect references and an `/AF` array is
nothing else. The first draft of the test used an inline list, and the reference vanished into
three tokens exactly as the clause says it would.

## What is deliberately not done

**Anything with the relationship.** The clause says a processor need not: "[t]he value of
`AFRelationship` does not explicitly provide any processing instructions for a PDF processor. It
is provided for information and semantic purposes for those processors that are able to use such
additional information." So this reads it and hands it on.

**External associated files.** §14.13.2 permits them — "[b]oth types are allowed for associated
files but the embedded form is recommended" — and an external one is §7.11.1's refusal, which no
filesystem here can lift.

**Validating a producer's obligations**: the `/ModDate`, the valid MIME type,
`application/octet-stream` where it is unknown. Those are requirements on whoever wrote the file,
and this reader does not validate files.

## Consequences

- `silent` falls 129 → 119, and clause 14's remaining silences are §14.8's vocabulary (32) and
  §14.7.7's worked example.
- `Interpretation` gains `associated_files` beside `artifacts`: two spans over the same readback,
  neither consumed by anything in this program yet, both of them what a repurposing consumer will
  ask for first.
- No gate moves. §14.1 again: the clause's features "do not affect the final appearance of a
  document".
