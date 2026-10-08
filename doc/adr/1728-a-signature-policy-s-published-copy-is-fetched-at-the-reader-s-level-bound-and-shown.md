# 1728 — A signature policy's published copy is fetched at the reader's level, bound by the signed digest, and shown

Session 1446. Status: **accepted**. Builds what ADR 1709 §3 left to the next round; amends nothing.
Context: ISO 32000-2 §12.8.3.4.4; ETSI EN 319 122-1 clauses 5.2.9.1, 5.2.9.2 and 5.2.10, cited by
clause and never quoted (ADR 1085); the A98 client (ADR 1291) and its level (ADR 1527).

## 1. Decision

**The copy of a policy document at the URL clause 5.2.9.2's qualifier names is fetched only under
the reader's `Submissions` level, compared with the digest the signer signed, and opened beside the
document only when it is bound.**

- `pdf_signature::policy::SignaturePolicy::published` hands out each URL with clause 5.2.9.1's
  digest (`Commitment`, the borrowed `PolicyHash` owned) and any named specification, as a
  `PublishedPolicy` that can cross to a host. `PublishedPolicy::binding` is `SignaturePolicy::binding`'s
  comparison over fetched octets: a server is no more signed than clause 5.2.10's store, and that
  clause's note is the argument that the signed digest is the one check against substitution. The
  outcomes stay asymmetrical for ADR 1219's reason — a match is decisive, a mismatch is reported with
  the specification the signature named or the statement that it named none.
- `viewer_host::policy::may_fetch_signature_policy` asks the level. A GET to the policy's server
  tells it this document is being validated here, which is the act class of Annex O's fetched import,
  so the level is `Submissions` and no fifth one is invented; a scheme outside `SUBMIT_SCHEMES` and a
  URL `submit::check_url` refuses are refused at every level first. The fetch is `submit::fetch`, with
  its bounds and its rule that a redirect is not followed.
- `signature_policy_fetched` is the sentence beside the verdict, and it is `submit::Reply`: a bound
  copy carrying §7.5.2's header is `Reply::Document`, which every window already opens beside the
  document; everything else is `Reply::Say` — a mismatch, an all-zero digest, a digest this crate does
  not compute, a non-2xx answer, a network failure — and `signature_policy_declined` says "not fetched
  at this level". **Every one ends by saying §12.8.3.4.4's constraints were not enforced**, naming the
  specification where the signature named one and saying it named none otherwise.

## 2. What the corpus's thirteen do through it (2026-10-08)

The thirteen `2.16.724.1.3.1.1.2.1.9` signatures (`cc-main-2021-31`, SHA-1 `1bbae8b9…02cd`) were
read through `published` and the fetch, by a scratch test removed after the run. **Three carry the
URL qualifier and ten carry none**, so for ten there is nothing to fetch and the core's "this file
carries no copy" sentence stands. The three name `https://sede.060.gob.es/politica_de_firma_anexo_1.pdf`,
and the fetch says it could not be fetched (no address for the host). The archive's 2016 capture of
that URL is a 301 to `sede.administracion.gob.es`; through the same path it is said as a redirect not
followed. That address answers today with 208 306 bytes, a 26-page PDF, and the fetch over it — and
over the archive's 2022 capture of it, byte-identical — **binds**: SHA-1 is the signed digest, and the
answer is `Reply::Document`.

## 3. Consequences

- §12.8.3.4.4 stays `partial` on enforcement, as ADR 1709 has it; its note says the human-readable
  half is built. Tests: four in `policy.rs` (the published shape, the bind, its one-octet control
  and the all-zero digest; planted, an always-true comparison fails both controls) and six in
  `viewer-host/tests/signature_policy.rs` against a loopback listener.
- **Not yet in the program.** No window calls it: the policy is read in `viewer_core::notes`, and the
  core hands no `PublishedPolicy` to a host. The missing hunk is an event of `viewer-core`'s carrying
  each signature's `published()` list, which the windows answer as they answer `NeedsFile` for an
  import — asking, then `fetch_signature_policy` on a thread, then the `Reply`.
- The level's name, `restriction::SUBMITTING` ("sending a form or fetching its data"), now governs a
  policy fetch its words do not mention; widening it is that file's owner's edit.
- A redirect is not followed here either. The digest would bind a copy from anywhere, but following
  one tells a server the person was not asked about, which is ADR 1291's reason, unchanged.
