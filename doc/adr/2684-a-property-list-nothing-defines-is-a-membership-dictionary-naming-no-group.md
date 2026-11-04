# 2684 — A property list nothing defines is a membership dictionary naming no group

Session 1931, a single archive round.
Status: accepted and built. In `pdf-archive`: `survey/marked_content.rs` (`property_list` reads a
named list through `look_up`). In `pdf-transform`: `archive/reader_resources.rs`
(`empty_membership`, `Empties::membership`). Tests:
`a_property_list_name_nothing_defines_is_reported_undefined` (`crates/pdf-archive/src/survey.rs`)
and `a_property_list_nothing_defines_is_a_membership_dictionary_naming_no_group`
(`crates/pdf-transform/tests/archive/reader_resources.rs`).
Context: ADR 2500 (a name nothing defines is defined empty), ADR 0816, `TechNote 0010` A002.
Clauses: ISO 19005-2 and ISO 19005-4 section 6.2.2; ISO 32000-2 §7.8.3, §8.11.2.2, §8.11.3.2,
§8.11.4.2, §14.6.2.

## What veraPDF measures, and why the checker passed it

`issue14438`, `state-every-loss` at 4: annotation 19 (a `Text` annotation on page 1). Its normal
appearance, object 49, opens `/OC /MC0 BDC`. The appearance's `/Resources` hold only `/ExtGState`
and `/ProcSet`, so `MC0` is defined nowhere. The source has the same shape on two appearances.
Part 4's section 6.2.2 asks the resource dictionary to define every named resource the stream
references. §7.8.3's Table 34 lists `/Properties` among the categories, and §14.6.2 places a named
property list in that subdictionary. So the output fails, and veraPDF is right. The 2b output
passes veraPDF only because veraPDF's part 2 profile does not test the sentence. `TechNote 0010` A002
reads it into part 2, and our row binds part 2 as well.

Our row, `graphics/named-resources-are-defined`, missed it because the survey looked a `BDC` or
`DP` list name up with `peek` rather than `look_up`. Its comment had deferred the change to "its own
evidence", and this document is that evidence. Both 6.2.2 rows now see the name: the
`Properties` miss, and the stream as one that references a resource.

## The construction

ADR 2500 defines a missing name as the empty value of its category, one that draws what this reader
drew for it. The reader found no list, so the section was not optional content (§8.11.3.2's last
condition) and it was drawn. A list that §8.11.3.2 accepts under the `OC` tag must be a group or a
membership dictionary. A group is listed in `/OCProperties` and becomes a layer the producer never
made. A membership dictionary needs only `/Type /OCMD` (Table 97), and with no `/OCGs`, "the P entry
shall have no effect on the visibility of any content". The reader's `membership` evaluates it as
visible, and with no `/OCProperties` at all §8.11.4.2 ignores it. So the name is defined as one
shared indirect object, `<< /Type /OCMD >>`, and no content-stream byte changes. Under any other tag
the list is no optional content. It carries no `/MCID`, `/ActualText` or `/Lang`, which matches what
the reader read from a missing list.

## What it leaves, measured

A miss in a stream ADR 2500 cannot follow (a Type 3 glyph procedure, an inherited direct dictionary)
is still refused by name, as every other category is. In round 1931's census no output was lost to
that: the same 4500 outputs were written, and six gained `/Properties`. Those six are `issue14438`
(`state-every-loss`, 2b and 4) and `issue17679` (`/oc1` and `/oc7`, both stating profiles, 2b and
4). Before the change, veraPDF 1.30.3 failed `issue17679` at 4 on 6.2.2-3 under both profiles. All
six now pass.
