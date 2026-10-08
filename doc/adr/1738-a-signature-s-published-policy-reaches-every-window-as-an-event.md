# 1738 — A signature's published policy reaches every window as an event, answered under the submissions level

Session 1451. Status: **accepted**. Builds what ADR 1728 §3 left: the hunk that hands the fetch a
policy. Amends nothing. Context: ISO 32000-2 §12.8.3.4.4; ETSI EN 319 122-1 clauses 5.2.9.1 and
5.2.9.2, cited by clause and never quoted (ADR 1085); ADRs 1527 (the fetched import's level), 1291
(the submit client), 1728 (the fetch and the binding).

## 1. Decision

**`viewer_core::Event::SignaturePoliciesPublished { document, policies }` carries each signature's
`SignaturePolicy::published()` list to the host, after the `Reported` whose sentences name the
policy, and every window answers it as it answers a fetched import: the level, then the question,
then a thread, then the `Reply`.**

- **Where it is read.** `notes::about` already reads the policy for its sentences; it now also
  collects each `PublishedPolicy` (`notes::About { notes, published }`, kept in `Open::about`'s
  `OnceCell`), with a URL a second signature names under the same commitment listed once.
  `Command::Report` is the one place both are produced, so the event costs page one nothing (ADR
  1044) and arrives once per opened document, which is `viewer_host::report::Due`'s rule.
- **Why an event and not a sentence.** A URL is a server, and asking it is a decision about this
  machine at the reader's network level, which only a host holds — `Event::Submit`'s reason (ADR
  1062). `may_fetch_signature_policy` is that level, read once; no window decides it.
- **The windows.** `quorra-gtk`, `quorra-qt` and `quorra` ask `may_fetch_signature_policy` at the
  menu's submissions level: `send` and `warn` fetch, `ask` puts `asked_to_fetch_policy` under the
  title `Subject::Policy` ("Fetch this signature policy?"), `refuse` says
  `signature_policy_declined`. The GET is `Submitter::fetch_policy`, a `fetch-policy` thread of
  `Submitter::fetch`'s shape, so its answer is collected on the timer every other answer is, and
  `Returned::reply` for `Asked::Policy` is `signature_policy_fetched`: a bound PDF is
  `Reply::Document`, which every window already opens beside, and everything else is a sentence. The
  two windows with one dialogue (`quorra`, `quorra-qt`) decline a policy that would be asked about
  while another question waits, by that reason, rather than replace the question in silence (trap
  5). `quorra-confined` has no network and declines at every level, as it does a fetched import.
- **The wire and the C ABI.** The event crosses `viewer-confined`'s wire as kind 26, a digest as its
  object identifier; the greeting moves to `PDFVCF12`. The C ABI names it
  `QUORRA_EVENT_SIGNATURE_POLICIES_PUBLISHED` (26), `QUORRA_EVENT_KIND_COUNT` 27, and
  `quorra_events_describe` gives each URL. **The binding comparison is not offered to a C caller**:
  it is `pdf_signature`'s, a caller fetching the copy itself would need an entry point to hand the
  octets back, and none is built — written down here rather than left to be found.
- **The level's words.** `restriction::SUBMITTING` is "sending a form, or fetching its data or a
  signature's policy": three requests under one level, and the menu row says each.

## 2. Consequences

- §12.8.3.4.4 stays `partial` on enforcement (ADRs 1709, 1728); nothing here enforces a constraint,
  and every sentence still says so.
- Tests: two in `viewer_core::notes` (the list beside the sentence, with the same signature without
  its qualifier as the control; the event after `Reported` and absent without a URL), two in
  `viewer-host/tests/signature_policy.rs` (the window's thread to a loopback copy, bound, and a 404
  said), the wire's round trip over the three commitments and the three specification shapes. The
  drive's step 66 serves a policy from its loopback server and signs a CAdES signature under it
  (verifying under `openssl cms -verify`): bound and opened beside at `send`, told apart where the
  signature commits to other octets, not asked for at `refuse`, and declined in `quorra-confined`.
- A redirect is still not followed, for ADR 1291's reason, unchanged.
