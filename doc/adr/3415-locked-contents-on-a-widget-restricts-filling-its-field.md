# 3415 — LockedContents on a widget restricts filling its field

Session 2178. Status: accepted. Context: audit 2175 (B04); CLAUDE.md, "a document's restrictions
are the reader's to set".

## Decision

Table 167 bit 10 forbids modifying "the contents of the annotation", and bit 8's row names "the
value of a form field" as such contents. `restriction::asserted` for `Operation::FillInForm`
therefore asserts `Restriction::AnnotationLocked` where any widget of the named field (or the
widget named) sets bit 10. It is asserted, never refused at the keyboard: the policy's four levels
attach to it as to every restriction, and `off` lets the reader fill the field.
