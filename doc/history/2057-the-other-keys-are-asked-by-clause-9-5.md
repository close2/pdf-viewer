# 2057 — The other keys are asked by clause 9.5, and compared only within a family

Session 2057, a signatures round in the gwg worktree. Row §12.8.3.4.5; `doc/todo/77` item 3; ADR 2928.

**The slice.** What round 2056 left of `doc/todo/77` section 2 item 3: ETSI TS 119 312 V2.1.1 clause 9.5's
*should* (the other keys resist over their certificates' validity) and *shall* (at least as strong as the keys
they certify), under TS 119 172-4 REQ-4.5-01 h).

**The premise.** Held: nothing after ADR 2927 and `doc/history/2056-*` names clause 9.5, and `Trust::Anchored`
and `verdict::Valid` still kept the path's length only. The carry is a field, not a redesign: `trust` already
held the validated path (`Validation::path`), so `Trust::Anchored` gains `keys`, owned.

**Built.** `trust::CertifiedKey` (a key's Table 2, 3, 6 and 7 parameters and its certificate's validity) is
carried by `Trust::Anchored`, `verdict::Anchored` and `Valid`, into `applicability::Asked::path`. `Report::of`
adds a delegated OCSP responder (RFC 6960 criterion 3, by `ResponderID`) and each token's time-stamping unit.
`ts_119_312::Assessment::other_keys` asks each key's rows as clause 9.3 asks the signer's, over its period (an
authority's runs to the latest `notAfter` below it on the path); `certifications` compares each certifying key
with the key it certifies within a family and says `NotCompared` across families, clause 3.1's levels bounding
classes, not keys. `viewer_core::notes` words the five new issues under "clause 9.5".

**Tests.** `every_other_key_is_asked_to_resist_over_its_period`,
`a_certifying_key_is_compared_with_the_key_it_certifies_only_within_its_family`,
`the_authorities_keys_are_asked_by_clause_9_5_in_the_report`, `a_tokens_unit_is_the_certificate_its_signer_info_names`
(the fixture unit's 2048-bit key to 2036 outlives Tables 2 and 6), `a_delegated_responder_is_asked_and_compared_with_its_issuer`
(a 2048-bit CA certifies a 3072-bit responder), and the carried keys pinned in `a_chain_to_a_supplied_anchor_validates`.
**Plant**: dropping `other_keys` from `Report::of` fails the report test; carrying no keys fails the trust test and
`a_signature_whose_path_reaches_a_supplied_anchor_is_valid`.

**Gates.** Over `pdf-signature`, `viewer-core`: fmt exit 0; clippy `-D warnings --all-targets` exit 0; nextest
1194 of 1194 passed; rustdoc `-D warnings --document-private-items` exit 0; `cd fuzz && cargo check --bins`
exit 0. Conformance after `git add -N`: exit 101 on this record's gates paragraph, then exit 0.

**Left.** National rules; a token's own path (its unit's issuer) and responses about the path's other
certificates are not carried; clause 9.5's last sentence (clause 9.4's rule, to whenever the signature needs
verifying) is not asked.
