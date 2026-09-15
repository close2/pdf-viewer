# ADR 1122 — A rich-text value drawn plain is a departure, said out loud on the entry that carries the formatting

## Status

Accepted, 2026-09-15. Session 1111. Narrows what §12.7.5.3's ledger row calls settled about
Table 231 bit 26, and corrects a claim §12.7.5.3 and §12.7.4.3 both carried.

`§N` is ISO 32000-2 and nothing else.

## Context

§12.7.5.3's Table 231 bit 26 makes a text field's value "a rich text string" and, "[i]f the field
has a value", makes Table 228's `/RV` "specify the rich text string". The row read those two
`shall`s as "addressed to the file", and so recorded bit 26 as met by carrying the flag to a host —
a `RichText` field's plain `/V` laid out, the bit crossed so a host could decline XFA markup.

That misses the clause that lays a field out. §12.7.4.3 writes a third `shall`, at the processor:

> For these fields, the following conventions are not used, and the entire annotation appearance
> shall be regenerated each time the value is changed.

The conventions that sentence sets aside are this module's whole construction; what replaces them is
XFA 3.3's, which `CLAUDE.md`'s closed exclusion list names. So laying out the plain `/V` and saying
nothing is trap 5's silence inside a feature otherwise built — a field the standard describes drawn
wrong, without a word.

## Decision

**The plain characters of `/V` are drawn and the missing formatting is reported.** `field_text`'s
layout carries `Owed::RichTextFormatting`, which `viewer-core` surfaces like every other shortfall.
The value is not refused — a `RichText` field with an `/AP` still shows it, and the plain text is
what a producer put in `/V` — but a reader is told the XFA formatting was not applied.

**The report fires on Table 228's `/RV`, not on the flag alone.** Bit 26's second sentence is what
puts formatting in the file: a field setting the flag and stating no `/RV` has its whole value in
`/V`, so drawing that owes nothing (trap 11 — a report is only as good as the condition it fires
on). `examples/field_flag_census` counts the two an order of magnitude apart, and the difference is
the whole reason the condition is the entry and not the bit.

**The `/RV` is read up the field's own `/Parent` chain and no further.** Table 228 marks `/DA` and
`/Q` inheritable and `/RV` not, so this is a merged-widget walk for an entry that sits on the field
dictionary above the annotation, not §12.7.4.1's inheritance.

## Alternatives

**Carry the bit to a host and call it met, as the row did.** Rejected: the host is not the only
processor, and §12.7.4.3's regeneration `shall` is unmet whether or not a host exists. The tier-2
viewer draws its own page and reaches no host, and it was drawing the field wrong in silence.

**Read the XFA and lay the formatting out.** Rejected: XFA is `CLAUDE.md`'s closed exclusion, and
composing it from another reader's output is what principle 5 forbids. A wrong rich-text layout
would fail worse than a refusal — a plausible appearance in the wrong place.

**Fire on the flag alone.** Rejected above: it would report on the field an order of magnitude of
widgets that state no formatting to lose, which is a report the clause does not state.

## Consequences

`variable_text::Owed` gains `RichTextFormatting`; `appearance` gains `rich_text_unformatted`. No
public type changes and nothing crosses the ABI. §12.7.5.3's row stays `partial` and its bit-26
disposal is a report rather than a hand-off. Four fixtures in `tests/variable_text.rs`, each planted
back (trap 13): the flag with `/RV`, the flag without one, an `/RV` without the flag, and the pair
found up a `/Parent` chain. No corpus page moves — every witness has an `/AP` and none sets
`/NeedAppearances`, so the construction is reached by a regeneration, not a render.
