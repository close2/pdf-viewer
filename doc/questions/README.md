# Questions for the project owner, and their answers

**The convention, stated by the owner on 2026-09-04:** every question a round is waiting on the
owner for gets a file here whose name begins with **`Q`**. When the owner answers, they add a file
with **the same name, `Q` replaced by `A`** — so `Q07-the-scheme-name.md` is answered by
`A07-the-scheme-name.md`. A question with no matching `A` file is open; that is the whole index,
and no round needs to read anything else to know what is waiting.

**Rules for a round.**

- **Ask here, not in a report.** A question raised only in a history file or an ADR is a question
  nobody can find. If a round needs the owner's word, it writes a `Q` file in the same commit as
  the work that raised it.
- **One question per file**, numbered in the order they were asked. A number is never reused, and
  an answered question keeps its `Q` file beside the owner's `A` file, because the argument that
  raised it is worth as much as the answer.
- **Every `Q` file says four things**: the question, why it cannot be settled without the owner,
  **what the tree does meanwhile** (there is always something: a default, a refusal, a reading
  held), and a recommendation. A question with no recommendation is a round asking the owner to do
  its thinking.
- **A question is not a blocker unless it is.** Say plainly whether work is stopped or merely
  provisional. Most of these are provisional: the code ships with a stated default, and the answer
  would change it.
- **When an `A` file appears**, the round that acts on it records the decision in an ADR and amends
  whatever the provisional answer was. The `A` file is the owner's word; the ADR is what the tree
  did about it.

**Where these came from.** `Q01` to `Q06` are RFC 0002 §13's, less its first, which the owner
ratified on 2026-09-03. `Q07` to `Q14` are RFC 0003 §9's plus the layout departure session 899
made. `Q15` to `Q22` are RFC 0006 §10's. `Q23` is a rendering reading two sessions arrived at
independently.
