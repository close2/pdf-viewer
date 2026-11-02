# 2644 — An XML signature policy is bound through its own digest by inclusive Canonical XML 1.0 over its components

Status: accepted. Session 1905, slot 5 of batch 131, a clause round on the owner's answer A713; ADR 2645 is unused.
Context: ISO 32000-2 §12.8.3.4.4 ("During validation conforming signature handlers should not enforce seed values
constraints, if present, but shall enforce signature policy constraints."); ADRs 1035, 1085, 2604, 2624; ETSI TS 119
172-2 clauses 4.2.2, 4.3.1, 4.3.2 and 4.4 (pages 11 and 12); W3C Canonical XML 1.0 (2001-03-15) sections 2.1 to 2.4 and
3.2, 3.6; XML Signature 1.1 section 6.5.1 for the two algorithm identifiers; XML 1.0 sections 2.11, 3.3.3 and 4.6; XML
Namespaces 1.0 sections 3 and 6.3. ETSI's text is cited by clause and never quoted (ADR 1085).

## Decision

**What the digest is over.** TS 119 172-2 clause 4.3.2 computes a policy document's `Digest` by applying the
canonicalisation algorithm its `ds:CanonicalizationMethod` names to the `PolicyComponents` element of clause 4.4, which
clause 4.2.2 places as the root's second child after `Digest`. The clause names no method, no transform and no
node-set expression, so the input is read as the element with its descendants, the document subset an XPath selection
of that element makes, and the method is whatever the document states. Where the signer signed the value that
`Digest` states, `policy_rules::enforce` now re-computes it: `policy_rules::canonical_xml` canonicalises the element
and streams the form into the digest function the signer named, and a match binds the document exactly as a match on
its octets does, so its constraints are enforced and a broken one withholds the word valid.

**The signer's function, not the document's.** The signed value and its function are the commitment; the document is
unsigned until it is bound, so its own `DigestMethod` is not allowed to choose what it is checked under.

**Inclusive 1.0, both forms, and nothing else.** `Comments::of` places the identifier without comments and the one
ending `#WithComments`. Every other method, Exclusive Canonical XML and Canonical XML 1.1 among them, stays
`NotEnforced::CanonicalDigestNotComputed`, naming the method; components that canonicalise to another digest are the
new `NotEnforced::CanonicalDigestDiffers`, which `viewer-core`'s note says as the document not being the one the signer
committed to.

**A second pass over the tokens rather than the reader's tree** — the premise that did not hold. The brief's
hypothesis was one pass over the module's tree. That tree keeps only unprefixed attributes and concatenated text, and
no comment, instruction or per-element declaration, which is everything sections 2.2 and 2.3 render. So the
canonicaliser reads the same `xmlparser` tokens again, under the same three budgets charged in the same places (so it
accepts what the reader accepts), and writes as it reads: a prefix lookup is a table, the apex's inherited namespaces
and `xml` attributes are gathered once, and the form is never held. The work is linear in the octets and the memory is
the open-element stack and the bindings.

**What is refused** is what would otherwise be canonicalised into octets nobody else computes: a relative namespace
URI (section 2.1's note requires failure), an undeclared entity, a character reference outside XML's `Char`, an
undeclared prefix, an expanded name twice, a processing instruction whose target is `xml` or runs into its data (XML
1.0 section 2.6; the fuzz target found the first, its form not read back as itself), an encoding declaration other
than UTF-8, and a document type declaration,
whose absence is also what makes the rest of section 2.1's list (default attributes, attribute types) vacuous here.

## Evidence

lxml 6.1.3's element serialisation (`method="c14n"`) agrees on the namespace axis of the apex, the attribute order and
the escapes, and disagrees twice. It renders `xmlns=""` on a grandchild that inherits the default namespace, where
section 2.3 renders it only on an element with no default namespace whose parent has one, and it leaves out an
ancestor's `xml:lang`, which section 2.4 copies onto the apex of a subset. Both times the Recommendation decides, and
the tests state its form.

## Measured

`getrusage` on the ignored probe `canonical_xml::tests::hostile_documents_are_canonicalised_within_their_budgets`
(five documents the reader's budgets admit, each canonicalised whole a hundred times: `xml` attributes filling an
ancestor, the same split between the ancestor and the apex, 4 093 declarations, sixteen levels each re-declaring 254
prefixes, one start tag of 17 000 attributes): maxrss 15 608 KiB in 5.35 s, against 12 168 KiB for TS 119 172-3's
hostile probe and 12 364 KiB for a unit test in the same binary. The slowest document takes 23 ms a run. The probe's
first reading took 118 s for the first document alone: the apex's inherited `xml` attributes were de-duplicated by a
scan of those already found, and merged by a scan of the apex's own, which is quadratic in a document that fills one
start tag with them. Both are set look-ups now, and the same document takes 7 ms a run.

## Status of the row

§12.8.3.4.4 stays `partial` under ADR 1035's rule. Its residue is human-readable policies, which are a limit, and an
XML policy whose `Digest` names Exclusive Canonical XML or Canonical XML 1.1, which is buildable and therefore keeps
the row open. A C caller's fetched copy is the host lane's.
