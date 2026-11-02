# 2624 — A signature policy copy a window fetched goes back to the document's validation and is enforced as a stored one is

Session 1895, slot 5 of batch 130, a "clause" round (ISO 32000-2 §12.8.3.4.4).
Status: accepted and built in `crates/pdf-signature/src/policy.rs` (`SignaturePolicy::fetched_binding`),
`crates/pdf-signature/src/policy_rules.rs` (`enforce_fetched`, `NotEnforced::FetchedCopyNotBound`),
`crates/viewer-core/src/command.rs` (`Command::PolicyCopy`), `crates/viewer-core/src/notes.rs`,
`crates/viewer-confined/src/protocol/command.rs` (`POLICY_COPY`), `crates/viewer-host/src/submit.rs`
(`Returned::policy_copy`) and `crates/viewer-host/src/policy.rs`.
Context: ADRs 1728, 1738 and 1753 (the copy fetched, sent to every window, bound for a C caller), 2560,
2580 and 2604 (the three syntaxes enforced over a stored copy), A713 (the ETSI texts placed), A98 (the
network level). ADR 2625 is unused.

## 1. The shape: the copy travels back as a command

§12.8.3.4.4's validator "shall enforce signature policy constraints", and before this round only a copy
the file stores (EN 319 122-1 clause 5.2.10) was enforced; a copy fetched at clause 5.2.9.2's URL was
bound and shown. The hypothesis was one call where the fetched copy is bound. It does not hold: the
host binds the copy in `viewer_host::policy`, but enforcing it needs the `SignedData` and the signer's
facts, which only the core holds — the host is handed `PublishedPolicy` values and never the CMS. So
the copy goes back: a window that fetched a 2xx answer dispatches `Command::PolicyCopy { document,
policy, copy }` after the reply's sentence, in all three loops, and across the confined protocol as
command 47. The core binds it from the signature's own policy identifier and digest
(`fetched_binding`), never from `policy`, which only says which published entry the copy answers; so a
host cannot make a copy bind by what it sends. `enforce_fetched` and `enforce` share one reader over
the three syntaxes, and a copy whose octets do not digest to the signed value is
`NotEnforced::FetchedCopyNotBound` and nothing in it is enforced. A stored copy bound to the signature
wins and a fetched one is said not enforced beside it. The answer is a `Reported` carrying the
sentences new to the report, never a second `SignaturePoliciesPublished`, which would fetch again.

## 2. The level

The brief said the fetch is off by default. It is not: it is governed by A98's `Submissions` level,
whose default is `ask`, and `refuse` is that level's off (`may_fetch_signature_policy`, ADR 1728). This
round changes neither; the fetch asks what it asked, and at `refuse` nothing is requested and the note
ends "none was enforced from that URL, because no copy came from it".

## 3. The C interface

`quorra_event_policy_bind` binds a copy a C caller fetched and says the sentence, but no call of the
interface hands a copy to a session, so `signature_policy_handed` cannot say the copy goes on to
validation. A session call was priced: the entry is small, but its C test needs a signed fixture built
in C, which is past this round. So the closing sentence is three-valued (`Onward`), and a C caller's
copy ends "none was enforced from this copy: this program's C interface has no call that hands a copy
to the document's validation". The call is batch 131's.

## 4. The row

The residue is two things: human-readable policies, whose constraints are prose no validator can
enforce, and a TS 119 172-2 document bound only by its own `Digest` over canonicalised XML. The first
is a limit; the second is work this program has not done. A status is a claim about a residue (ADR
1035), and a residue holding buildable work is not a documented limit, so the row stays `partial`.

## 5. The finish line and the plant

`viewer-host/tests/signature_policy.rs` serves an XML policy from the loopback to a viewer opened on a
signed fixture: at `send` the copy is enforced (three constraints, the `signingCertOnly` rule broken);
one octet changed inside the XML comment is said not to hash and is not enforced. The fixture's
signing period is said and not evaluated, because only a covering signature timestamp establishes the
time and none is built; and no viewer fixture verifies to an anchor, so the word valid withheld under
a broken constraint is proven over the verdict fixture in `pdf_signature`'s test.
