# 3606 — Two owners in one attribute band: the later object, as a stated choice

Session 2226. Status: **accepted**. Context: pdf-association/pdf-issues #226 and #289
(published on pdf-issues.pdfa.org), ISO 32000-2 §14.7.6, §14.7.6.2, §14.8.5.3, ADR 3581.

## Context

#226 confines §14.7.6's array-order precedence in an element's `/A` to an attribute *for a
specific owner (as defined by the O and NS entries)*. §14.8.5.3 ranks bands, not owners inside a
band. So the corrected standard names no winner where two objects of different owners in one
band state the same attribute: a PDF 2.0 and a PDF 1.7 standard-namespace object in band 1, or
two PDF-native owners under `Tree::attribute`'s admit-everything filter.

## Decision

`structure::ranked` keeps giving the attribute to the later object between different owners in
one band. This is a **choice**, and it is stated as one. Its ground: #289's inserted sentences
give the later of a class's attribute objects precedence *across* owners, which is the only
cross-owner precedence the standard states, and one rule for both routes keeps the producer's
last statement the one read. The earlier object and no value at all were weighed. Neither
follows a clause, and the second drops a value the producer stated.

The narrowed accessors (`layout_attribute`, `list_attribute`, `table_attribute`,
`print_field_attribute`, `artifact_attribute`) already admit one PDF-native owner each, so in
production the choice reaches only band-1 namespace ties.

## Consequences

`two_owners_in_one_band_give_the_attribute_to_the_later_object` pins it in both orders.
`Tree::attributes`' doc restates §14.7.6 per ADR 3603.
