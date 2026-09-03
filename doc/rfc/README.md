# RFCs — proposals, not decisions

`doc/adr/` records decisions after they are taken; this directory holds **proposals before
anyone has decided anything**. An RFC is a document the project owner can mark up, accept,
amend or decline — it argues for a direction, it does not record one. Once a proposal is
accepted and work begins, the decisions that come out of it get ADRs like any other work;
the RFC stays here as the argument that started it.

## Conventions

- **The owner is the decider.** An RFC's status changes only by the owner's word, never by a
  round's judgement that the argument looked finished.
- **Status field**, at the top of every RFC: `draft` (being written), `proposed` (ready for
  the owner), `accepted`, `declined`. A declined RFC stays in the tree — the argument against
  is worth as much as the argument for.
- **Every RFC carries**: a motivation (what demand or gap it answers, with evidence), prior
  art (who else does this and how), a proposed design (or, for a survey, the material a design
  would start from), an easy/difficult assessment against this tree's architecture, and open
  questions the owner should rule on.
- **An RFC is not bound by the project's current rules.** Stated by the owner for this series:
  where a standing rule — an exclusion in `CLAUDE.md`, the immutability of a type, a scope
  boundary — is relevant to a proposal, the RFC *names it as a current restriction with its
  original rationale*, and then proposes what the unconstrained design would be. The owner
  amends rules by argument; an RFC that pre-trims itself to fit the rules has hidden exactly
  the argument the owner wanted to see. (The rules still bind *implementation* until amended —
  an accepted RFC that needs an amendment says so, and the amendment is its own decision.)
- **Registers stay separate.** Market research, tracker mining and other products' feature
  lists are evidence about *demand and convention*. They say nothing about rendering
  correctness, where principle 5 (the specification is the only source of truth) governs
  exactly as before. An RFC citing what Acrobat does is arguing about what users expect, never
  about what a page means.
- Numbering is sequential and four-digit, like the ADRs'. A number may be reserved for a
  round before its document exists.

## Index

| RFC | title | status | owner round |
|---|---|---|---|
| [0001](0001-the-survey.md) | The survey — what PDF tools provide, what users ask for, and where the gaps are | proposed | 784 |
| [0002](0002-the-transform-suite.md) | The transform suite | accepted (2026-09-01; §13 open, defaults in ADR 0800) | 785 |
| [0003](0003-file-system-faces.md) | File-system faces — a KIO worker and a FUSE filesystem over one core | draft | 786 |
| [0004](0004-print-and-print-preview.md) | Print support and print preview | draft | 786 |
| [0005](0005-text-editing-without-reflow.md) | Basic text editing, without reflow | draft | 786 |
| [0006](0006-pdf-a-validation-and-conversion.md) | PDF/A: validating a document, and converting one | draft | 895 |
