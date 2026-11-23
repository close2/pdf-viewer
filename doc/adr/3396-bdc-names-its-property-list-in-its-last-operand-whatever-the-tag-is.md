# 3396 — `BDC` names its property list in its last operand, whatever the tag is

Session 2173. Status: **accepted** and built. Context: ADR 2684; ISO 32000-2 §14.6.2, §7.8.3
Table 34; ISO 19005-4 section 6.2.2 (and ISO 19005-2 section 6.2.2 under TechNote 0010 A002).

## Context

`issue17679_2`'s form XObject writes `123 /oc1 BDC` and has no `/Properties`. Its part 4 outputs
failed veraPDF at 6.2.2 (a named resource the stream references and its dictionary does not
define), and this tree's `graphics/named-resources-are-defined` passed them: the survey took a
named property list only where two *names* stood before the operator, so a tag written as a
number left `/oc1` read as nothing.

## Decision

§14.6.2's second form is `tag properties BDC`, and the operand the operator reads as its property
list is the last. The survey now asks whether the last operand was a name with another operand
before it, whatever that one was; a lone name (`BMC`'s form) and a name followed by a number name
no property list, as before. The row then reports `/oc1`, and `Rewrite::EmptyResourceDefined`
(ADR 2684) defines it as an empty property list, which draws nothing.

## Measured

`issue17679_2` converts under both profiles at 2b and 4, and veraPDF 1.30.3 passes all four.
