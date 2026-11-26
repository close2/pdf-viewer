# 3745 — A linearization dictionary states a page count only while its length is the file's

Session 2261. Status: **accepted and built** in `pdf-syntax` (`linearize::stated_page_count`),
`pdf-archive` (`pages_held::linearized_page_count`, `NoPageHeld::given`) and `pdf-transform`
(`archive/recovered.rs`, `refuse_a_count_the_reader_fell_short_of`). Answers audit 2260's B02.
Clauses: ISO 32000-2 §F.1, §F.3.3, Table F.1.

## Decision

Table F.1's `/L`: "It shall be exactly equal to the actual length of the PDF file. A mismatch
indicates that the file is not linearized and shall be treated as ordinary PDF file, ignoring
linearization information." `/N` is linearization information. Both readers of it — the archive
converter's recovered-structure rebuild and the page-held check the checker and the converter share
— took it from any object stating `/Linearized`, so a linearised file later updated incrementally
(§F.1's own case; deleting pages is how its `/N` goes stale) was refused for pages it no longer
states, or given a statement page naming a count it does not make.

Both now read `/N` through one function beside `linearize::state`, which shares its location rule
(the object the cross-reference chain places lowest, beginning inside §F.3.3's first 1024 bytes) and
its `/L` comparison. Unlike `state` it does not require the rest of Table F.1: a count is read only
to be compared with the pages a reader reached, and a dictionary that states its length truly and
its pages states them whether or not its hint stream is described. `state` keeps its stricter
answer, because its callers go on to use `/H`, `/O`, `/E` and `/T`.

## Consequences

An object stating `/Linearized` that is not the file's first, or lies past its first kilobyte, no
longer states a count either: §F.3.3 makes the parameter dictionary that first object. The
existing test of the refusal (`tests/archive/recovered.rs`) gives its fixture a true `/L`, which was
its premise.
