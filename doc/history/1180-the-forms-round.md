# 1180 — A rich text string is not the XFA the exclusion names

Four §12.7 rows, read against the clause rather than against their notes. One moved; the other
three kept their status and two of them lost the reason they were giving for it.

## §12.7.4.3 — the claim that decayed

`/DS` and `/RV` were recorded as "XFA rich text, excluded by principle 5", in two ledger rows,
`doc/todo/22`, ADR 1122 and ADR 1186. Both halves fail on reading. `CLAUDE.md` excludes XFA on
§K.1's permission, which is about schema-driven page generation and not about an AcroForm entry
whose format the standard describes by reference — and this tree has read the identical construct
since ADR 0224, for Table 172's and Table 177's `/RC`, whose cells carry the same words. ADR 1197.

So a rich text value's characters are drawn: bit 26 says the value "shall be a rich text string",
§12.7.5.3 says its contents construct the appearance, and a rich text string's contents are its
character data. Before this a conforming PDF 2.0 field drew its own brackets. The walk is
`popup::rich_text_characters`, shared with `/RC`; a value the flag calls rich text and the parser
cannot is drawn as it stands (ADR 0111). The formatting stays ADR 1122's reported departure, with the condition the count corrected: over
the 90 763 documents reachable from `corpus-cache`, `doc/corpora` and `doc/pdf.js/test/pdfs`,
`field_flag_census` counts 451 widgets setting bit 26, **60** stating `/RV` and **411** stating
`/DS` — so the report fired on an eighth of the fields that lose formatting. **0 of the 451 state
a `/V` that parses as markup**: no page on this disk moves and the fixtures are the whole defence.
Picture looked at, `scratchpad/r1180/rich.pdf` at scale 3: the characters, not the brackets.

## §12.7.4.1 — `partial` → `departed` (ADR 1198)

The clause forbids bounding inheritance; principle 3 requires a bound; that is a decided departure
and the row now says so. Its recorded reason — "a depth no legitimate form approaches" — was
measured and was wrong: the deepest `/Parent` chain on this disk is **32 links** and **25 widgets
over two documents reach it**, so the bound was refusing real fields at exactly its own value. It
is 256 now, and the census walks past it so the instrument cannot move with what it measures.
Table 227 bit 2 was the other residue — "carried only, because §12.7.6.2's submission is
excluded", a reason the tree removed when it built the export — and a `Required` field with no
value is now named in `Submission::owed` at the moment the sentence is about.

## §12.7.5.4 and §12.7.8.3.3 — unmoved, with better reasons

§12.7.4.3's NOTE-driven list-box layout is built; the unmarked selection is `doc/questions/Q72`'s
open class, unanswered, and nothing here pre-empts it. §12.7.8.3.3's `/Rename` refusal stands, and
Table 253's `/F` becomes the host question ADR 1186 made of `/APRef`.
