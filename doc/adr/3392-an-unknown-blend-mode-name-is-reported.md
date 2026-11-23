# 3392 — A blend mode name outside Tables 134 and 135 is reported

Session 2172. Status: accepted and built (`pdf-model`: `Unsupported::UnknownBlendMode`,
`content/ext_gstate.rs`, `content/annotations.rs`; `viewer-core`'s sentence for it). Answers
finding 2171-B04.

## The clause

§11.6.3: the `/BM` value "shall be either a name object, designating one of the standard blend modes
listed in "Table 134 -Standard separable blend modes" and "Table 135 -Standard non-separable blend
modes" in 11.3.5, "Blend mode" or its value shall be an array of such names", and of the array a
processor "shall use the first blend mode in the array that it recognizes (or Normal if it
recognizes none of them)".

## Decision

The fallback is the array's, so the array stays silent about the names it passes over. A single
name the tables do not list is input the clause does not admit: it is still drawn as `Normal`, and
now reported as `Unsupported::UnknownBlendMode` naming the graphics state and the name. An
annotation's `/BM` (Table 166) takes the same names and is reported the same way.

A new variant rather than `Operator` or `TransparencyGroup`: neither names what is wrong, and a
host's sentence for it ("a blend mode §11.6.3 does not list, drawn as Normal") is its own. It is a
public type's change, so `fuzz/` is checked.
