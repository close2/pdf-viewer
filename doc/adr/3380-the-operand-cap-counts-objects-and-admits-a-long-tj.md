# 3380 — The operand cap counts objects, and admits a long `TJ`

Session 2169. Status: accepted. Context: round 2141's record, ADRs 3267, 3354; principle 3.

## The question

`MAX_OPERANDS` was 8 192 objects, an array's elements each counted, because the content lexer keeps
an array flattened. A valid `TJ` array past it was cut and the bound reported. Should the cap count
an array as one operand?

## The clauses

§7.3.6 sets an array no length. Annex C, informative, has "[a]lthough a PDF array object ... can
theoretically contain any number of elements, PDF processors are likely to have some limitations",
and names no figure. Nothing in the standard bounds a `TJ` array, so the bound is this program's and
answers to principle 3 alone.

## Decision

**The cap stays a count of objects, raised to 65 536.** What a hostile stream makes one operator
allocate is objects, and an array's elements are objects: counting an array as one operand would
need a second bound on its elements, which would be this one under another name. 65 536 objects of
about forty bytes are a few megabytes, and a paragraph set as one array — ten thousand elements, the
test — draws whole. The figure matches `pdf-transform`'s archive language survey's own.

The bound is also how far an array a damaged stream never closed is believed (ADR 3267); it now
believes one eight times as far, and the report — `LimitReached { limit: "MAX_OPERANDS" }`, the name
ADR 3267's check reads — is unchanged.
