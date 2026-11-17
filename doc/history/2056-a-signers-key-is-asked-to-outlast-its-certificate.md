# 2056 — A signer's key is asked to outlast its certificate

Session 2056, a signatures round in the gwg worktree. Row §12.8.3.4.5; `doc/todo/77` item 3; ADR 2927.

**The slice.** The first item of `doc/todo/77` section 2: TS 119 172-4 REQ-4.5-01 h)'s remainder, ETSI TS 119
312 V2.1.1 clauses 9.3 and 9.5's key lifetimes over a certificate's validity, and national rules.

**The premise.** Held: no ADR or record after ADR 2905 names clause 9.3 or a key's lifetime, and the code asked
neither clause. Clause 9 was read whole (9.1 to 9.5). Clause 9.3 is a *shall* on the signer's key over
`notBefore` to `notAfter`; clause 9.5 is a *should* and a *shall* on keys the report is never handed.

**Built.** `ts_119_312::Assessment::signer_key` asks the key's legacy rows — Table 2's algorithm, Table 3's
curve, Tables 6 and 7's sizes, never Table 1's digest or Table 4's combination — whether their phase-out passed
by the signer certificate's `notAfter`, and says `Issue::KeyOutlived` naming the table and the instant;
`Issue::ValidityNotRead` where the validity did not read; an unlisted row is not asked again. `Report::of` calls
it after clause 9.2's imprints; `viewer_core::notes` words both issues under "clause 9.3". Note 3's time-stamp
mitigation is not applied, a note being no requirement (ADR 2927).

**Tests.** `a_signer_s_key_is_asked_to_outlast_its_certificate`: a 2048-bit PSS key at the last second of 2026
and the next; a 3072-bit PSS key into 2035; a PKCS #1 v1.5 key at the last second of 2034 and into 2035 (Table 2
only, not Table 4); a 1024-bit key not asked twice; no validity. The report test's signer, valid to 2036,
now carries Table 2's and Table 6's clause 9.3 issues, and the viewer test words one. **Plant**: the call
dropped from `Report::of` fails `the_report_over_a_signature_holds_each_item_in_the_clauses_order`.

**Gates.** Over `pdf-signature`, `viewer-core`: fmt exit 0; clippy `-D warnings --all-targets` exit 0; nextest
1189 of 1189 passed; rustdoc `-D warnings --document-private-items` exit 0; `cd fuzz && cargo check --bins`
exit 0. Conformance after `git add -N`: exit 0.

**Left.** Clause 9.5 — the CAs', revocation issuers' and time-stamping units' keys — needs the validated path and
the material's and tokens' signers carried into the report, and a choice of what "at least as strong" compares
across families (ADR 2927 item 5). No national rules are held. `doc/todo/77` item 4 (a list a policy's
`TAsInTrustedList` names outside the LOTL's tree) was not taken: it fetches a location a policy names, and the
time went to item 3.
