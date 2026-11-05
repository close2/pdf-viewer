# 2710 — An XML signature policy's digest is re-computed under Exclusive XML Canonicalization and Canonical XML 1.1

Status: accepted. Session 1948, the ledger lane of the owner's two parallel rounds (2026-11-05).
Context: ISO 32000-2 §12.8.3.4.4 ("During validation conforming signature handlers should not enforce seed values
constraints, if present, but shall enforce signature policy constraints."); ADRs 1085, 2604, 2624, 2644; ETSI TS 119
172-2 clauses 4.3.1, 4.3.2 and 4.4 (cited by clause, never quoted, ADR 1085); W3C Exclusive XML Canonicalization 1.0
(2002-07-18) sections 1.1, 2.1, 2.2, 3, 3.1 and 4; W3C Canonical XML 1.1 (2008-05-02) sections 2.1, 2.4, 3.8 and its
appendix; XML Signature 1.1 sections 6.5.2 and 6.5.3 for the four identifiers; RFC 3986 sections 5.2.1 to 5.2.4.

## Decision

**One writer, three Recommendations.** Both later Recommendations define themselves as Canonical XML 1.0 with stated
changes (Exclusive section 3's opening; 1.1's introduction and section 2.4), so `policy_rules::canonical_xml` keeps
ADR 2644's single token pass and differs only at those changes: `Method` carries the `Algorithm`, the comments and
the `PrefixList`, and `Method::of` places all six identifiers. A URI none of the three names stays
`NotEnforced::CanonicalDigestNotComputed`.

**Exclusive.** No `xml` attribute is copied onto the apex. A prefix is rendered on an element that visibly utilizes it
(its own name, or an attribute's; the default where the element has no prefix) unless the nearest output ancestor
rendered the same value; `xmlns=""` only below a rendered non-empty default. The bookkeeping is section 3.1's
`ns_rendered` as a table of stacks, which states section 3's normative conditions exactly because this subset holds
every element's whole namespace axis. The `PrefixList` prefixes (`#default` for the default) take 1.0's rules instead.
The parameter is read by `ts_119_172_2` from an `InclusiveNamespaces` child of `ds:CanonicalizationMethod` in the
method's own namespace (section 4), and every listed prefix is charged to the entry budget before the work.

**1.1.** Only `xml:lang` and `xml:space` are copied; `xml:id` is not; any other `xml` attribute is ordinary; and
`xml:base` is fixed up where an ancestor carries one — every ancestor's value and the apex's, folded innermost first
by `canonical_xml::base::join`, the value left out where it comes out empty. The join is RFC 3986's resolution with
section 2.4's five changes; its dot-segment removal is a segment stack, one branch per stated change.

## The subset, and the examples that do not fit it

TS 119 172-2 clause 4.3.2 names one element, so the input is that element and its descendants, and its ancestors are
all omitted and contiguous. Two of the Recommendations' examples (1.1 sections 3.7 and 3.8) select non-contiguous
subsets through a document type declaration this reader refuses. Section 3.8 is transcribed in the two shapes this
subset has: `e1` as apex reproduces its rendered start tag, and `e3` as apex folds to "something/bar/foo", the value
the section gives `e3`. Exclusive sections 2.1 and 2.2 and 1.1's appendix table (64 rows) are transcribed whole.

## Evidence

The two Recommendations and XML Signature 1.1 were fetched from w3.org and converted to Markdown outside the tree.
They belong in `doc/md/` beside the ETSI texts (ignored, untracked, ADR 1085), but this worktree's `doc/md` is a link
into the main checkout, which a round does not write; the merge places them. No other
canonicaliser was run: `lxml` is not installed, and the tests rest on the Recommendations' own outputs.
No TS 119 172-2 document naming either method is on this disk (the signature corpora state no machine-readable
policy, ADR 1709), so the end-to-end test builds one and says so.

## Measured

The ignored probe `canonical_xml::tests::hostile_documents_are_canonicalised_within_their_budgets` now runs every
document under all three methods and adds two: every level using each prefix it declares, and an `xml:base` of `../`
segments on each of fifteen ancestors, which the join keeps and so carries to the next. The slowest run of any is
42 ms (4 093 declarations under 1.1); the fifteen joins take 35 ms a run on 245 KB.

## Plant

Three defects, each planted and reverted: rendering every visibly-utilized prefix without the `ns_rendered` check
(fails the visibly-utilized test and the end-to-end test), folding only the nearest ancestor's `xml:base` (fails
section 3.8's `e3` test), and copying every `xml` attribute under 1.1 (fails that test and the simple-inheritable
test).
