# 1574 — The fifteen open rows each state one reason, and two reasons had expired

Session 1369. Status: **accepted**. A reading of every `partial` and `reported` row against the tree
and the owner's answers; amends no ADR.
Context: `doc/PLAN.md` §5a; ADRs 0161, 1035, 1191, 1291, 1459, 1547, 1573; `doc/questions/A66`,
`A168`, `A169`, `A171`, `Q227`; ISO/IEC 15444-1:2000 (`doc/md/ISO_IEC_15444.md`); ISO 32000-2
§7.4.7, §7.4.9, §7.6, §12.8.3.4.4, §12.10.

## The rule applied

A `partial` or `reported` note states one current reason and names what would move the row: a
crate release, an owner's answer by number, or a trigger. Each was checked against
`tools/state.sh main-checkout`, the code its note names, and the owner's answer files.

## The rows

| row | status | the one reason | what moves it |
|---|---|---|---|
| §7.4.7 | partial | the pinned `hayro-jbig2` ignores EXTTEMPLATE, refused by segment | the fork takes `hayro-jbig2-extended-template.patch` |
| §7.4.9 | partial | e-sRGB, e-sYCC, CIEJab and non-D50 CIELab take the device fallback; their texts are not held | the owner buys a text, or `jpx_colour_census` finds a document stating one |
| §7.6 | partial | aggregate: the public-key handler | §7.6.5's family moving |
| §7.6.5–§7.6.5.3 | reported | the handler is refused by name | A66's trigger, unlit (A168) |
| §7.6.6 | partial | Table 27, the handler's | the same trigger |
| §12.1 | partial → implemented | none: states no requirement (ADR 1573) | — |
| §12.8, §12.8.3, §12.8.3.4 | partial | aggregates of §12.8.3.4.4 | that row moving |
| §12.8.3.4.4 | partial | "shall enforce signature policy constraints"; no signature in reach states a policy | a signature stating one, its policy document reached, its named specification held |
| §12.10, §12.10.2 | partial | the projection is not built | the build A171 owes, census first |

## Two reasons that had expired

**§7.4.9's thirteen codestreams.** The note gave two reasons, the second being thirteen corpus
codestreams that decode one level off `opj_decompress` on the irreversible path, on the premise
that ISO/IEC 15444-1 defines the decoding exactly, so a codec has one right answer per codestream. The held
text says otherwise for that path, and it is cited here by section, never quoted. Section E.1.1.2
leaves the reconstruction parameter of a dequantised coefficient to the decoder's choice. Section
F.3.8.2 gives the 9-7 filter no rounding operation, with its lifting parameters stated as exact
expressions. Section G.3 says the irreversible component transform implies no required precision.
So two decoders a level apart on 0.02% to 0.1% of a plate are both inside what the standard leaves
open. The reversible path is defined exactly, and there the two agree byte for byte. The thirteen
stay held by name as evidence, so a change in either codec is seen. They are no longer debt.

**§12.10's dependency.** The notes said the projection needs the EPSG registry or an ISO 19162
string, neither of which this tree holds, and the frontier map put the wait on the open
`doc/questions/Q171`. The owner answered it on 2026-10-05 (`A171`, uncommitted in the main
checkout): build it of the tree's own. A census over the crawl comes first. Then the `/WKT` fork,
the inverse methods cited to IOGP Guidance Note 7-2, an accuracy budget documented as a choice,
and `/EPSG` answered from tables of the codes documents use. The rows stay `partial` until that is
built. They leave `doc/todo/65`'s bucket 2 for bucket 6.

## Aggregates

ADR 1035's rule settles the question the contract asked. A row is an aggregate when a row it is a
strict ancestor of still owes. Its counterpart, `Problem::AggregateWithoutDebt`, fails a head that
owes after every child has settled. So an aggregate is `partial` while any child owes and moves when
the last one does. That is not the worst of its children. A head over one `reported` child and many
`implemented` ones is `partial`, not `reported`, because some of what it holds is executed. §7.6,
§12.8, §12.8.3, §12.8.3.4 and §12.10 are aggregates by that test and stay `partial`. §12.1 is not one
(ADR 1573).

## PAdES validation and the network

§12.8.3.4.5's four steps and §12.8.3.4.6 to §12.8.3.4.8's revocation model are `implemented` from
the material the file carries: §12.8.4's store and §12.8.3.3.2's attribute (ADR 1067). The
premise that validation needs certificates and a network throughout is gone. A98's `ureq` client (ADR 1291) could add
RFC 5280 section 6.3.3 (a)(1)'s fetch of a newer list. §12.8.3.4.6's either-or does not require
that fetch, so it is not owed, and a round could build it under a reader's level as an addition.
For §12.8.3.4.4 the client means only that fetching a policy document is not what is missing. What
is missing is a signature that states a policy.
