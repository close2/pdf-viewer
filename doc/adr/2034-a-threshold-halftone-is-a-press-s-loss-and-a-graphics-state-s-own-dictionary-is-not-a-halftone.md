# 2034 — A threshold halftone is a press's loss, and a graphics state's own dictionary is not a halftone

Session 1599, the archive slot of batch ninety-five. Status: **accepted**, **built** in
`pdf-archive` and `pdf-transform`. Context: ADRs 0816, 1965, 2007; `doc/pdf-a-mitigations.md`
section 4.3; `CLAUDE.md`'s clause 10 entry. Code: `crates/pdf-transform/src/archive/halftones.rs`
(`defaults`, `origins`), `pdf_archive`'s `is_halftone_site`.

## 1. The site and its witness, read in its bytes

`graphics/halftone-type-is-one-or-five` (ISO 19005-2 and ISO 19005-4 section 6.2.5: every halftone
of type 1 or 5) was the census's first `not-built-yet` site over `doc/veraPDF-corpus` at 2b — two
documents, `6-2-5-t03-fail-a`'s part 2 and part 4 copies, counted as three halftones each (types
6, 10 and 16). The bytes say otherwise. The page's `ExtGState` table names three dictionaries:
`/GS0` states `/HT 20 0 R`, a type 6 halftone; `/GS1` and `/GS2` *are* halftone-shaped
dictionaries (`/Type /Halftone /HalftoneType 10`, and 16), named by the table itself. veraPDF
1.30.3 reports one failed check, the `HT` one.

## 2. A table entry is a graphics state

§8.4.5: `gs`'s operand "shall be the name of a graphics state parameter dictionary", looked up in
the `ExtGState` subdictionary; a graphics state reaches a halftone only through its `HT` entry
(Table 57, §10.6.5). So `/GS1` and `/GS2` are graphics states stating keys Table 57 does not
define, which set nothing; neither is any page's halftone. `pdf_archive` judged a halftone by its own entries alone; it now leaves
out a dictionary an `ExtGState` table names unless an `HT` entry or a type 5 halftone's colourant
names the same object too (`named_as_halftones`), for all three halftone rows. The premise's
"types 6, 10 and 16" was the instrument's; the witness has one threshold halftone.

## 3. Decision: the graphics state sets `Default`, and that is a loss

A threshold array is a whole screen, so no entry of it can go and leave the rest; what can change
is the state naming it. Its `HT` is written Table 57's `Default`, "denoting the halftone that was in
effect at the start of the page" — the device's own — and the halftone, which nothing names any
more, leaves the output with the objects only it reached (ADR 1099's walk). §10.6.1: "Halftoning
is not required for such devices", the continuous-tone ones this reader is, so nothing a screen
shows moves; §10.6.5.1 lets a processor ignore a file's halftones, but a press that would have
used the threshold screen now uses its own. As ADR 1965 ruled for the name, that is a loss, not
a statement: `Loss::Halftone` (`--authorise halftone`), `Rewrite::HalftoneWrittenDefault`;
`as-shown` answers it `discard`, `as-if-printed` leaves it at `stop`. Refused by name: a shortened
finding list, a halftone a type 5 names as a colourant's screen, one named from anywhere but the top
level of a graphics state's object. Part 4's `HTO` (Table 57, PDF 2.0) is the same loss — a press
places its screen at its own origin — removed as `Rewrite::HalftoneOriginRemoved`. Part 2's `HTP`
is not: ADR 2035. No preserve is built (keep-everything's `attach` stays unbuilt, as in ADR 1965).

## 4. What it changes

`6-2-5-t03-fail-a` converts to 2b and 4 under `as-shown`, veraPDF 1.30.3 compliant; `6-2-5-t01-
fail-b`'s part 4 copy (`HTO`) converts to 4. Each page renders byte-identically at 72 and 720 dpi
before and after (looked at: the green invader). Fixtures: a type 6 halftone unauthorised and
authorised (display list equal), a type 5 naming one refused, a table entry not a halftone.
