# ADR 1219 — A signature policy is named per signature, so what blocks enforcing it is too

## Status

Accepted, 2026-09-22. Session 1191. Amends nothing; it settles what §12.8.3.4.4's row had been
carrying as *the specification that defines the policy's syntax is not held*.

`§N` is ISO 32000-2 and nothing else. ETSI's texts are cited by clause and never quoted (ADR 1085).

## Context

§12.8.3.4.4 builds its explicit-policy profile out of one attribute and then hands the rules for it
to another document: "a signature-policy-identifier shall be present as a signed attribute. The
rules from clause 5.2.9 in CAdES (ETSI EN 319 122-1) shall apply." The one obligation the clause
states on a validator is in the same paragraph — during validation a conforming signature handler
"shall enforce signature policy constraints".

This tree holds ETSI EN 319 122-1 and -2 (ADR 1085), and the row had read enough of clause 5.2.9 to
count the attribute for ETSI EN 319 122-2 Table 1's cardinalities and for its requirement (c). What
it had not done was read the attribute's *contents*, and the reason recorded was that enforcement
needs the policy document and "the specification that defines the policy's syntax is not held" —
written as though one named document were missing.

**That was one step short of the fact.** Clause 5.2.9.2 defines a qualifier whose whole job is to
identify the technical specification the policy document's syntax is written in, and its note says
what naming it settles: whether the document is human-readable, XML or ASN.1. Clause 5.2.10's store
states the same thing again for the copy it carries. So the specification is not a single text this
project could acquire — **it is whichever one a given signature names**, and a reader that held one
of them would still be blocked on the next file.

The second thing the re-read found is that enforcement is not the only thing in clause 5.2.9. Three
of its rules bind a validating application and need no policy document at all: an all-zero digest of
any length, including a zero-length one, is the producer saying the policy's digest is not known and
a validating application is *required* to read it that way; a notice qualifier carries text meant to
be displayed whenever the signature is validated; and clause 5.2.10's note states that the digest in
the signed identifier is what catches an alteration of the unsigned store, because nothing else
covers it.

## Decision

**`pdf_signature::policy` reads clause 5.2.9's attribute whole, and clause 5.2.10's store with it.**
`SignaturePolicy::read` answers with the policy's object identifier, a `PolicyHash` that keeps the
all-zero value apart from a stated digest by name, the qualifiers in the file's own order — the URL
a copy can be fetched from, the user notice, the specification identifier, and any fourth kind named
by its own identifier rather than dropped — and the store where the file carries one.
`Signature::signature_policy` is the entry a reader reaches it through.

**`SignaturePolicy::binding` compares the stored document against the digest the signer signed, and
the two outcomes are not symmetrical.** A match is decisive: a digest does not agree by accident, so
it says the store holds the signer's policy whatever syntax that policy is in. A disagreement is
reported as `Binding::DoesNotMatchTheStoredOctets` carrying the specification the file named,
because clause 5.2.9.1 makes the digest's input depend on that specification and a canonicalisation
prescribed there would make the same document digest to something else. Calling that an alteration
would be this program asserting something it cannot support.

**Enforcing the policy's constraints stays unbuilt, and the row's note now says what blocks it in
the shape the block actually has**: per file, named at runtime by the signature itself. That is the
same shape §12.8.3.1's two uncomputed curves have — a certificate names brainpoolP512r1, this
program says so by identifier and computes nothing — and it is not the shape §8.6.5.9 has, where one
named text would unblock the work.

## Consequences

- §12.8.3.4.4 stays `partial`, for one sentence rather than for the attribute's contents, and its
  note names what the sentence needs.
- A host that wants to show the policy has something to show: which policy, whether the signer
  committed to a particular copy of its document, whether the copy in the file is that one, and the
  notice clause 5.2.9.2 says is meant to be displayed. **Nothing in `viewer-core` says it yet** —
  that file was another round's this batch — so the capability is in the crate and not in the
  program, which is a debt and is recorded as one.
- The population is zero on the curated corpus and the test holds it there, so a document arriving
  which does state a policy announces itself rather than passing silently;
  `examples/signature_algorithm_census` carries the column for everything else on the disk.
- The fixture is `cms::fixtures::pades_under_a_policy`, built whole from clauses 5.2.9 and 5.2.10,
  and its control is the same fixture with one octet of the stored document turned over — which is
  what makes the comparison a measurement rather than a second reading of the same bytes (trap 13).
