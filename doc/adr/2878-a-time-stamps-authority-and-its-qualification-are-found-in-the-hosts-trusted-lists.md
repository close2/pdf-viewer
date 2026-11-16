# 2878 — A time stamp's authority, and its qualification, are found in the host's trusted lists

Session 2031, a signatures round (`doc/todo/77` item 7). Status: implemented. Rows: §12.8.3.4.5,
§12.8.3.4.8, §12.8.5.2. Context: ADRs 1039, 1071, 1085, 2834, 2842, 2864, 2866, 2872; ETSI TS 119
615 V1.3.1 clauses 4.3, 4.6 and 4.7, TS 119 172-4 V1.1.1 REQ-4.2-03 and REQ-4.5-01, EN 319 102-1
V1.4.1 clause 5.6.2.1, TS 119 612 V2.3.1 clause 5.5.1.1. Cited by clause and paraphrased (ADR 1085).

## Context

A token's authority — a document timestamp's (§12.8.5) or a signature timestamp's (§12.8.3.4.8) —
was established only against the reader's own anchors, so a host that supplied the EU's lists
and no anchor saw a qualified signature's timestamp as a claim. ADR 2872's report said d) i) (the
absence of the authority from a Member State's list as a `TSA/QTST` or `TSA` service) and d) iii)
3) (whether the token is an EU qualified time stamp) as *not determined*: clause 4.6, which both
NOTEs point at, was not run.

## Decision

1. **Clauses 4.6 and 4.7 are run as written** (`trusted_list::time_stamp`). Clause 4.6 names the
   list by the *subject's* `countryName` of the token's signing certificate (PRO-4.6.4-02, the
   conversion PRO-4.4.4-01 makes, now shared), runs clause 4.3, and reads -04 to -09 in order;
   clause 4.7 runs it for `TSA/QTST` at the instant asked and at `genTime` and compares (-05, -06).
   Choices where the text leaves one: a `CC` whose list the host did not supply is not run
   (`Answer::NoListForTerritory`, as clause 4.4's); -07's "public key values" are the keys of each
   found service's clause 5.5.3 certificates; -08's "matching" is `names::texts_match`, every
   `organizationName` must match and none stated matches nothing; -06's "appropriate values" are
   one, `ResultsDifferAtGeneration`, carrying the second answer.
2. **Clause 4.3's path test states the time-stamping purpose for a time-stamping service type.**
   RFC 3161 section 2.3's authority carries a critical `extKeyUsage`, which RFC 5280 section 4.2
   refuses where the use is unknown, so a search for `TSA/QTST` or `TSA` could otherwise never
   find one; the service type is what says the use.
3. **A host's lists anchor a token's authority where the reader's anchors do not.** EN 319 102-1
   clause 5.6.2.1 validates a time-stamp against the validation policy's trust anchors for
   time-stamps, and TS 119 172-4 REQ-4.2-03's NOTE 2 says such an anchor need not be a Member
   State's time-stamping service — a list's service is one source and the reader's anchors
   another. The anchors are the identities of the services of types `TSA/QTST` and `TSA` that
   clause 4.3 returns for the token's authority **at the token's `genTime`**, REQ-4.2-03 a)'s shape
   for a signer and clause 4.7's second instant; no status filters them, as none filters
   REQ-4.2-03 a)'s, because the status is clause 4.7's question, not the path's. The reader's
   anchors are asked first, so supplying lists changes no instant they already established, and
   `timestamp::Time::Established::through` says whose anchor the path ended at; the notes name the
   list and the service.
4. **d) iii) 3) is clause 4.7 at the instant the token's authority was asked about** — the next
   token's established instant, or the host's clock — and at `genTime`. Clause 4.7.1 determines a
   status only for a token that is valid, so a token not established is said as not determined for
   that reason (`EuQualified::TokenNotEstablished`), and with no lists nothing is asked.
   **d) i) is clause 4.3 for both types at `genTime`** over every list given; an empty answer with
   lists given is the absence the item asks the report to express.

## Consequences

`timestamp::chain`, `timestamp::established` and `timestamp::signature_timestamp_established`
take the host's lists beside the anchors; `applicability::Asked` carries the lists and the
document's signatures, from which a document timestamp's token is read. A qualified signature's
document timestamp from a granted `TSA/QTST` service now fixes the best signature time for a host
that supplied only the EU's lists. Annex A's `id-etsi-dst-euqtst` type for a document timestamp
itself is not reported: TS 119 172-4 clause 4.4.2 assigns a level to a signature, and nothing in
it runs that join over a time-stamp token; that is left as it was.
