# 3391 — A soft-mask dictionary that breaks Table 142 is reported

Session 2172. Status: accepted and built (`pdf-model`: `soft_mask.rs`'s `table_142_faults`).
Answers finding 2171-B03.

## The clauses

Table 142's `/G`: "If the subtype S is Luminosity , the group attributes dictionary shall contain a
CS entry defining the colour space in which the compositing computation is to be performed." Its
`/BC`: "The array shall consist of n numbers, where n is the number of components in the colour
space specified by the CS entry in the group attributes dictionary".

## Decision

The drawing is unchanged — a reader must draw something, and the tree's choices stand: a group
without `/CS` composites on device RGB, and a `/BC` that is not `n` numbers gives way to the space's
initial colour, the absent entry's default. What changes is that each is said: the `/Luminosity`
derivation adds a departure for each, and departures reach `Unsupported::TransparencyGroup` where
the mask's group is run, beside the existing departures of the same derivation. A `/BC` beside a
group with no usable `/CS` is not judged a second time; the missing `/CS` is the report.
