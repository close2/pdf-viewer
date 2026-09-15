# 1096 — What identifies an instantiated template, and what a transform scopes

Session 1082. Status: **accepted**. Closes the two residues ADR 1049 §3 named by name: Table 257's
"instantiating page templates" had no classifier, and no transform parameter selected an object.
Adds `revision::Kind::TemplateInstantiated`, `revision::FieldRanking`, `revision::FieldJudgement`,
`revision::NotScoped` and `signature::FieldMdp`; `signature::field_mdp` now returns Table 256's
`/Data` beside Table 259's selection.

`§N` is ISO 32000-2 and nothing else.

## 1. An instantiated template is identified by content, because the clause fixes nothing else

§12.7.7 says where a template lives — "[i]f the page is not intended to be displayed by the PDF
processor, it shall be referenced from the name dictionary's Templates tree instead" — and what
instantiating one produces: "[a] script executed by an ECMAScript action can add the named page to
the current document as a regular page." It does **not** say whether the regular page shares the
template's content stream object or carries a copy of it. Both are conforming, and a reader that
insisted on the shared reference would refuse half the conforming instantiations there are.

So the test is the content itself: a page **added** after signing, stating Table 31's `/Type /Page`
and the `/Parent` that table requires — the two entries §12.7.7 says a template has *not* got — and
carrying the bytes of a page the **signed** revision held in its `/Templates`. Read out of the
signed revision on purpose: a template the update itself added is not one the signer put out of the
page tree's reach, and "instantiating" it would be composing a page after signing.

Two consequences follow and both are deliberate:

- **An empty content is never matched.** Two empty pages match each other, so keeping one would
  rank every contentless page added after signing as an instantiation of it — the lenient default
  the module exists to refuse.
- **Where the content was copied, the copy is ranked with the page it draws.** It is an object the
  signed revision never held and carries no `/Type` of its own, so it has no evidence of its own to
  be classified on; the page claims it, exactly as a filled field claims its appearance stream.

## 2. The page tree node that received the page is part of the same operation — under one condition

Adding a page to the tree rewrites the `/Pages` node that receives it, and nothing in Table 257
says that rewrite is part of "instantiating page templates". Ranking it as an unrelated change
would make every instantiation come out `NotClassified`, which would be a classifier that never
answers; ranking it freely would let a page tree be rebuilt under cover of one.

The condition drawn is the narrowest that makes the operation nameable: the node changed in
Table 30's `/Kids` and `/Count` and in nothing else, it lost no kid, and every kid it gained is a
page this same update instantiated. Anything else — a kid removed, another entry moved, a kid that
is not an instantiation — is `Kind::Unclassified` and refused. `a_page_tree_node_that_lost_a_page_
is_refused_even_beside_an_instantiation` is that line, planted.

## 3. `/Data` scopes, and its absence is a refusal rather than a default

Table 256 makes `/Data` "(Required when TransformMethod is FieldMDP, shall be an indirect
reference)" and says what it is for: "[a]n indirect reference to the object in the document upon
which the object modification analysis should be performed." Three objects reach every field there
is — Table 15's `/Root`, Table 29's `/AcroForm` and Table 224's `/Fields` — and a `/Data` naming a
*field* scopes the analysis to that field's subtree. A `/Data` naming anything else, or naming
nothing, is `NotScoped`: a ranking computed over a subject nobody chose is this module's standing
failure wearing a different hat.

**`UR` selects nothing, and the same table is what says so**: "[f]or transform methods other than
FieldMDP , this object is implicitly defined." So the entry belongs to one method, and reading it
for the other two would be inventing a scope the standard withholds.

## 4. The world, over 90 763 documents, and what it calibrates

`examples/signature_algorithm_census` now counts both halves. 170 documents state a `FieldMDP`
transform (against `doc/pdf.js`'s one), and of the 171 transforms between them **156 point `/Data`
at the catalog and 15 state none at all** — so the refusal above has real members and is not a
branch only a fixture reaches, while `DataIsNotTheForm` has none and is a fixture's alone, which is
worth saying rather than leaving to be discovered (trap 13).

The selection selects **both ways in the world**, which no fixture could establish: over every
signature of those documents, 15 answers are a covered field changed and 5 are that only fields
*outside* the selection changed. A reader that ignored `/Fields` would have given one answer to all
twenty.

Two more facts the walk states and nothing had: **327 documents name `UR3` where Table 256's value
list has `UR`** — a fourth method name, which `signature::usage_rights` has read since ADR 0159 and
which a census counting only the standard's spelling would have reported at six — and **one names
`FieldMDP2`**, which nothing reads.

## 5. What is still refused, by name

- **No host asks any of this.** ADR 1043's debt is unchanged and is now the whole of what keeps
  §12.8.2.2, §12.8.2.2.2 and §12.8.2.4 `partial`: the ranking and the selection are reached by
  `pdf-signature`'s tests and by no program.
- **`UR`'s own ranking is not made.** §12.8.2.3 states one — "a PDF processor shall examine the
  current version of the document to see whether there have been modifications to any objects that
  are not permitted by the transform parameters" — and for `UR` those parameters are Table 258's
  *rights* rather than a selection of objects. That is a second ranking, against a different table,
  and it is named here rather than guessed at.
- **Nothing says *valid*.** `FieldJudgement` has four answers and none of them is one, for
  `Judgement`'s reason and ADR 1076's: the word has one constructor and it takes an `Anchored`.
