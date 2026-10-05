# 1548 — An out-of-scope row names its exclusion and quotes its clause; §13.4's poster is owed

Session 1356. Status: **accepted**. Applies ADR 1535's discipline to `out-of-scope`; moves one
row. Context: `CLAUDE.md` principle 5's closed list of exclusions; `doc/PLAN.md` §5a's
`out-of-scope`; ADRs 0906, 1035, 1367, 1535; `doc/questions/Q33` and the owner's `A33`;
`tools/conformance/tests/ledger_notes.rs`; ISO 32000-2 §12.5.6.17, §12.5.6.25, §12.6.4.9,
§12.6.4.10, §12.6.4.14, §12.6.4.16 to §12.6.4.18, §12.11.5, §13, §14.9.2.4, Annex K.

## 1. The question asked of each row

`out-of-scope` was held by one thing: an `exclusion` field naming one of four values. Nothing asked
whether the clause's own text puts it there, and 81 of the 94 rows — every clause-13 row — carried
no note at all. ADR 1535's three questions, read for this status: does the clause state anything
(a heading with no text takes its subclauses' status); does the note name the exclusion in words;
and does a sentence of the clause, quoted where `conformance::ledger::grounding` verifies it,
show the clause's subject is what the exclusion covers.

## 2. The thirteen outside clause 13

All thirteen rest on one of the four, on the clause's own text; the notes now say which and quote it.

| row | exclusion | the sentence |
|---|---|---|
| §12.5.6.17 | clause 13 | "The features described in this subclause are deprecated in PDF 2.0. They are superseded by the general multimedia framework described in 13.2"; its one `shall` on a processor is "When the annotation is activated, the movie shall be played" |
| §12.5.6.25 | clause 13 | "3D and RichMedia annotations are defined in 13.6.2" — a pointer with no requirement of its own |
| §12.6.4.9, §12.6.4.10 | clause 13 | the hand-over sentence both open with (ADR 1367) |
| §12.6.4.14 | clause 13 (and script) | "A rendition action ( PDF 1.5 ) controls the playing of multimedia content" |
| §12.6.4.16 | clause 13 | "identifies a 3D annotation and specifies a view for the annotation to use" |
| §12.6.4.17 | script | "Upon invocation of an ECMAScript action, a PDF processor shall execute a script that is written in the ECMAScript programming language" |
| §12.6.4.18 | clause 13 (and script) | "specifies a command to be sent to that annotation's handler" |
| §12.11.5 | script | the note's own argument holds: every `shall` binds a processor that invokes a handler, and a handler is a script — the requirements are vacated, not executed (ADR 0896) |
| §14.9.2.4 | clause 13 | every entry the standard lets hold the array is a clause 13 object: Table 285's and Table 288's `/Alt` and Table 295's `/TT`, the last of which the note did not name |
| §K, §K.1, §K.2 | XFA | §K.1's permission, "a PDF processor may choose to not implement this feature" |

**§12.5.6.17 stays `out-of-scope`, and its display half is not a residue of it.** A movie
annotation's `/AP` is drawn, but that is §12.5.5's requirement; the clause's own requirements are
all about playing. Neither `implemented` nor `departed` describes a clause whose every `shall` is
the excluded half.

## 3. The row that moves: §13.4, `out-of-scope` → `reported`

Q33 asked whether Table 306's `/Poster`, in its stream form — "it shall contain an image XObject
(see 8.9, "Images") to be displayed as the poster" — comes off the clause 13 exclusion, and
recommended taking it while leaving the boolean form, "[i]f it is the boolean value true , the
poster image shall be retrieved from the movie file", excluded. The owner approved that
recommendation in A33. Nothing was built: `appearance::construct`'s `Movie` arm still refuses with
"which principle 5 excludes with the rest of clause 13", and its comment still says Q33 asks the
owner. So §13.4 holds a requirement in scope, unbuilt and reported by that refusal: `reported`,
with the playing and the boolean form named as excluded in its note, and placed in
`doc/todo/65`'s bucket 6. The build is one arm reading `/Movie` → `/Poster` and drawing the image
`XObject` in `/Rect` through `Stream::form`, as §12.5.6.19's icon is drawn; where the image goes
inside `/Rect` the clause does not say, and the round that builds it writes that down as a choice.

## 4. The eighty clause-13 rows

Each now carries a note naming the exclusion and quoting, verbatim, a sentence of its own clause
that shows its subject is the media or 3D engine; the seventeen headings with no text of their own
say so. §13.3, §13.6.6, §13.6.7.3.6 and §13.6.7.4 also name the half that is *not* excluded — the
sound annotation's icon, a markup or projection annotation's own appearance — which their clause 12
rows draw.

## 5. The gate

`every_out_of_scope_row_names_its_exclusion_and_quotes_its_clause` in `ledger_notes.rs`: the note
contains `principle 5` and the exclusion's words; it quotes a sentence `grounding` finds (`Quoted`),
or it says it is a heading with no text of its own and `doc/md/` prints nothing between that
heading and its first subclause. Planted on §13.2.5 — a clause with text, claimed to be a heading —
it fails naming the row, and passes with the row restored.
