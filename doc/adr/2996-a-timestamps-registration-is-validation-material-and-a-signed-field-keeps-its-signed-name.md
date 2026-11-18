# 2996 — A timestamp's registration is validation material, and a signed field keeps its signed name

Session 2078. Status: **implemented.** Amends ADR 1049 (Table 257's carve-out) and ADR 1096 (what a transform
scopes). Context: review 2038 (findings B18, B42). Clauses: ISO 32000-2 §12.8.1, §12.8.2.2.2 Table 257, §12.8.2.4
Table 259, §12.8.4.3, §12.8.5.

**The carve-out.** Table 257: "Changes to a PDF that are incremental updates which include only the data necessary
to add DSS's … and/or document timestamps … shall not be considered as changes to the document". The comparison
recognised the timestamp dictionary and the store, but not what adding a timestamp requires around them: §12.8.1
makes the timestamp dictionary "the value of a signature field", a field belongs to the form only through
`/Fields` (Table 224), and its widget is on its page only through `/Annots` (Table 31). A genuine timestamp-only
update was therefore unclassified under `/P 1`. Now the signature field whose `/V` is a `DocTimeStamp` (and its
widget), the `/Fields` and `/Annots` arrays where they kept every member and gained only such fields, `/SigFlags`
beside `/Fields`, and the catalog gaining `/DSS`, are `Kind::ValidationMaterial`. Anything else in the same update
still makes it `Mixed`, so a timestamp does not carry a page change through (the control test).

**The signed name.** Table 259's `/Fields` was signed against the fully qualified names the signer saw. The
comparison named a field as it is named now, so a field renamed after signing and then changed fell outside an
`Include` list that named it. The clause does not say which name identifies a field across a `/T` change; this
program takes the reading that cannot be used to escape a lock: an object is inside a selection if its signed name
or its current name is. The rename itself is a change §12.8.2.2.2 already ranks (`/T` is a field-defining entry).

Tests: `pdf-signature` `docmdp_dts_carveout` (both), `revision::tests::field_mdp_signed_name_spec_audit`.
