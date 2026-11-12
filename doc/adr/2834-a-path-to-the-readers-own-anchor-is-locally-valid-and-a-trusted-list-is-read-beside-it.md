# 2834 — A path to the reader's own anchor is *locally valid*, and a trusted list is read beside it

Session 2008, a signatures round (`doc/questions/A800` item 3). Status: implemented. Rows: §12.8.1,
§12.8.3.4.4, §12.8.3.4.5. Context: ADRs 1039, 1076, 1085, 1728, 2725, 2782; ETSI TS 119 612 V2.3.1,
ETSI TS 119 615 V1.3.1, ETSI TS 119 172-4 V1.1.1. The ETSI texts are cited by clause and
paraphrased (ADR 1085); copies are under the round's scratch directory and are not tracked.

## Context

The owner, 2026-11-06: a reader should be able to tell European signatures' levels apart, and "we
should have a local valid certificate setting as well. A user should have the information that a
signature is locally valid." Before this round the tree had one anchor set, the host's
(`trust::Supply`, `--trust-anchors`), and a path to any of it was reported as *valid*, in the same
words whatever the anchor was. ADRs 2725 and 2782 named trusted-list anchors as work: "a TS 119 612
reader", reached through a host.

## Decision

1. **`verdict::Standing`**. A `Valid` carries whose anchor ended its path: `Local`, an anchor the
   reader's own setting names, or `Listed(Listing)`, a service digital identity a trusted list
   publishes. `Anchored::of` makes the first, as every route did before; `Anchored::listed` makes
   the second. `Verdict::reached` stays the only maker of `Valid`. `viewer_core::notes` says
   "locally valid" for the first, never "valid" alone, and names the list, the provider, the
   service and its status for the second.
2. **The reader's own setting is the existing anchors.** `--trust-anchors` was always the reader's
   own choice of authority (RFC 5280 section 6.1.1 input (d)), so it is the "local valid
   certificate setting" the owner asked for, and nothing about it changes but its name in reports.
   A company that adds its own root there gets *locally valid*, not *invalid* and not a sentence
   indistinguishable from a qualified one.
3. **`pdf_signature::trusted_list`** reads a TS 119 612 list (clause 5's scheme, provider, service
   and history fields, Annex C's element names) through the XML reader policies are read with, now
   `crate::xml` and shared, under octet, entry, depth and service bounds; a document type
   declaration is refused. `matching` is TS 119 615 clause 4.3: PRO-4.3.4-01's territory test,
   -03's type and identity tests (RFC 5280 section 6.1 with the identity as the only anchor, or the
   identity being the certificate), -03 b)'s `SI-at-Date-time`, -03A's history order, -05 to -10's
   duplication indications and -11's provider conflict. `listed_anchors` is TS 119 172-4
   REQ-4.2-03 a): services of type `CA/QC` at the signing certificate's `notBefore`.
4. **The host supplies lists as it supplies anchors.** `TrustPolicy::lists`, `--trusted-lists
   <dir>` in `viewer-host`, carried over the confined pipe. Empty by default; a document never
   names one (principle 3).
5. **A listed path is asked first**, then the local anchors; where neither reaches the word, the
   listed reservation is said if a list named a service for the signer.

## What is not done, and why it is not hidden

- **A list's own signature is not verified** (TS 119 615 clauses 4.1 and 4.2: the list of trusted
  lists authenticated against the Official Journal's certificates, then each list against the
  certificates the former names). `Listing::authentication` is `VouchedForByTheHost`, and the
  sentence says the list was believed for where it came from. This is a departure GPR-4.0-01 does
  not permit for a conformant implementation — the output differs where a list is forged — so it
  is reported, not claimed, and it is the first item of `doc/todo/77`.
- **The eIDAS level is not determined** (ADR 2835).
- **Fetching the list of trusted lists** is a host's transmission (ADR 1728, `doc/questions/A130`)
  and no host makes it yet; a person keeps the files.
- **A C host cannot supply lists**: the ABI entry is `doc/todo/77`'s.

## Consequences

Every report that used to say "that signature is valid" now says "locally valid" for the same
input. `tools/drive-windows.sh` greps the new sentence.
