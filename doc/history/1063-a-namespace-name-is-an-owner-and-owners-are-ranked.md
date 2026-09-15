# 1063 — A namespace name is an owner, and owners are ranked
§14.8.5.3's priority was a filter and an array order; it is a rank now, and §14.7.4.2's closing
paragraph places a namespace owner in it. Both rows close, and §14.8.5 with them.

## The rank
Bands 1 and 2 are the element's `/A` and band 3 the class map, so a band is an object's **route**
and **owner** together — `AttributeObject` carries `from_class` for it. Band 1 is "owned by an owner
… excluding Layout, PrintField, Table , List and Artifact , if present, and if processing based on
the format indicated by the owner value", band 2 those five, band 3 the class map.
`Tree::attribute` searches rather than filters: the best band wins between bands, §14.7.6's
later-wins rule inside one. `AttributeObject::effective_owner` puts an `NSO` object's namespace name
through `Owner::read` — "the namespace name shall be considered as identifying the owner" — the whole
of the equivalence: session 1024 was right that the clause prints no pair, Table 376's rows being
names and not URIs. A name that **is** one of the five is that owner (band 2); an export
format's is refused with the formats; a namespace outside §14.8.6.1's two is refused **by name** and
listed by `Tree::unranked_owners`. The two standard structure namespaces meet band 1's condition,
because §14.8.6.1 makes them the schema for §14.8.4 and §14.8.5 — ADR 1077, with why band 2 fails.

## The census, and the calibration under it (trap 13)
`examples/attribute_owner_census` over `doc/*.pdf`, pdf.js, `doc/corpora*`: 991 opened, 107 tagged,
242 422 elements, 99 091 carrying 99 201 objects; bands **0** / 13 831 / 84 375, 995 refused (each
owner named — `Link` 978 of them). **36** elements carry attributes under more than one owner; **5**
state one attribute twice, all five band 2 against band 3, where array order already gave the
clause's answer — so the rank moves no value in reach. All **8** namespace-owned objects are
`MathML`'s, stating `lspace`, `rspace`, `mathvariant`, `display`: a vocabulary nothing here would ask
for, admitted until today. No note fires on a refusal — declining an export format is obedience to
§14.8.5.2, and reporting it is trap 11.
`a_namespace_owner_ranks_where_the_clause_puts_the_owner_it_names` carries the evidence:
`/TextAlign` twice in one `/A` with the higher-ranked object **first**, three times over — the PDF
2.0 namespace (band 1 wins), the name `Layout` (band 2, later wins: same shape, opposite answer, the
equivalence the only difference), MathML (refused, named). Array order planted back fails it; a
class object planted into band 1 fails `a_class_map_answers_only_what_the_elements_own_attributes_did_not`.

## Rows, and §14.8's other three
`14.7.4.2`, `14.8.5.3`, `14.8.5` `partial` → `implemented`; §14.8.5.2's stale "thirteen format
owners" dropped (Table 376 lists eleven). §14.8.2.2.1, §14.8.2.2.2, §14.8.2.3 stay `partial` with
half their debt **refuted**: each said no consumer exists, and `tools/pdf-retrieve`'s
`without_artifacts`, `viewer_accessibility::role`'s `speaks: false` for `Artifact` and
`viewer_core::search`'s literal readback match are three. What is owed is the computation.

## Gates
`nextest --workspace` 4804/4804, doctests 0, `-p conformance` 0, both `fuzz/` lines 0;
`accessibility_census` passed with **every ratchet at slack 0**, the gate agreeing that the rank
changes no answer in reach. `fmt --all --check` and the workspace clippy line are red only in files
a sibling is mid-edit; this round's two pass `rustfmt --check` and lint silent.
