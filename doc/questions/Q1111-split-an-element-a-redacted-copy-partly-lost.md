# Q1111 — Should a redaction split a structure element a page's copy of a shared form partly lost?

Asked by round 2290 (ADR 3862). Non-blocking: the build restates the element whole meanwhile.

**Situation.** A tagged form is drawn on two pages; a redaction on page 1 removes part of a
sequence. Page 1 gets its own copy of the form; page 2 still draws the producer's form, whole. The
structure element of that sequence names both (ADR 3811). Today it loses its `/Alt`,
`/ActualText` and `/E` (§14.9.3, §14.9.4), so page 2's unredacted placement loses its description
(no mark is lost).

**The alternative.** Split the element: the producer's element keeps naming the producer's form
with its descriptions; a new sibling element, without them, names page 1's copy, and the copy's
parent-tree entry names the new element. Precise for both pages, but it rewrites the logical
structure (one unit becomes two), a step the redactor has not taken before.

**Question.** Keep restating the whole element (current), or build the split?
