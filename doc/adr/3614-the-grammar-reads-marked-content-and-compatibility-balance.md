# 3614 — The validator's grammar reads §14.6.1's and §7.8.2's balance, so the fence knows every page before the photographs

Session 2228. Status: **accepted and built**; extends ADR 3542 and ADR 3557. Context: round
2224's diagnosis of `050734.pdf` (govdocs1 error set 2): pages 19 and 21 (objects 53 and 59) hold
`syntax/string-unterminated`, `page-as-image` is granted under `as-if-printed`, the photograph
survey is asked, and section 5.1 still refuses. Clauses: ISO 32000-2 §7.8.2, §14.6.1 Table 352;
ISO 19005-2 and ISO 19005-4 section 5.1.

## Context

The photographs were taken: the fence (`fenced::pages`) named pages 19 and 21, and both were
photographed. The refusal came after them. Section 5.1's answer (`base_standard::answer`) reads
the repair log once the photographs are drawn, and by then it held
`model/marked-content-unbalanced` at object 19 — page 8's content, which leaves a `BDC` open —
that the fence had not seen. The interpreter logs that class; `pdf-archive`'s survey, which
`pdf_archive::check` runs over the whole file before the fence asks, does not interpret
(ADR 3542), and its grammar read five of the interpreter's content classes, not this one. The
photograph step interprets every page to find the fonts it shows, so the class appeared between
the fence and the answer, on a page nobody photographed.

ADR 3542's rule is that the grammar states, off the tokens, each content class a validator needs
— and Table 352's balance ("terminated by a balancing EMC operator") and §7.8.2's ("These
operators shall occur in pairs") are functions of the tokens alone.

## Decision

1. **`pdf_model::content::grammar` reads both balances**: `BMC` and `BDC` open a section whatever
   their operands (the interpreter runs them with what it was given), `EMC` and `EX` with nothing
   open answer `model/marked-content-unbalanced` and `model/compatibility-sections-unbalanced`,
   and `Grammar::end` answers a section left open at the stream's end — where the interpreter
   makes the same two repairs, per run, a page's `/Contents` one run and a form one of its own.
2. **The survey asks `end`** at each stream's end and notes what it answers at the stream's
   content site, so the checker's log holds the class before the fence reads it.
3. The fence's assumption, that the log is the whole file's after `check` (ADR 3506), is now true
   of these two classes as well; an interpreter-only class the grammar does not read
   (`model/operands-left-over`, `model/property-list-unresolved`, the inline-object and bound
   classes) can still appear late, and is the next to move into the grammar where it is a
   function of the tokens.

## Evidence

`050734.pdf` converts under `as-if-printed` at 2b and 4, with pages 8, 19 and 21 photographed;
veraPDF 1.30.3 passes the outputs. Under `as-shown`, `issue15629.pdf` — whose object 15 closes
one `BDC` with two `EMC`s, and whose output used to carry the stray `EMC` past both checks — is
now refused by the class's name, and `504076.pdf` and `050734.pdf` are refused there under it
first. Nothing else in the census moved, with or without the CJK pack.
