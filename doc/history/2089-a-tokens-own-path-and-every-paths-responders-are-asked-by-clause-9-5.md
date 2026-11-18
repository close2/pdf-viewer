# 2089 — A token's own path and every path's responders are asked by clause 9.5

Session 2089, a signatures round in the lane4 worktree. Row §12.8.3.4.5; `doc/todo/77` section 2 item 3;
ADRs 3056, 3057 and 3058. What record 2057 and the review 2038 queue left of TS 119 312 clause 9.5.

**The premise.** Held in part. Nothing after ADR 2928 and record 2067 carried a token's path or a responder
about a certificate other than the signer's: `timestamp::Time::Established` dropped `Trust::Anchored`'s keys,
and `applicability::ocsp_responders` asked about the signer's serial alone. The queue's fixture line had moved:
record 2067's `two_timestamps_by_a_unit` carries a real unit, but one issued by the root itself, which the
token's path cannot show and which never established a time in the report.

**Built.** `Time::Established` carries `keys` and `responders`, and `Report::of` asks each established
token's path — `KeyRole::TimeStampingAuthority { position }` for the authorities above the unit, each paired
with the key it certifies — keeping the unit's padding from the token's signature; an unestablished token asks
its unit alone. `revocation::delegated_responders` names, per certificate, the delegate RFC 6960 section
4.2.2.2's authorisation accepted (`authorised` now says which criterion held); the path walk collects them as
`trust::Responder { about, key }` on `Trust::Anchored`, `verdict::Anchored`, `Valid` and
`applicability::Asked::responders`. A responder is paired with the path's key at `about + 1`, and with none
under the anchor. `KeyRole::OcspResponder { about }` and `TimeStampingResponder { about }`; `notes` words them.

**Not built, stated.** Clause 9.4's period is TS 119 172-1 BSP (j)'s prose, which no input carries; national
rules are h)'s "either" and unheld (ADR 3058). No question for the owner.

**Fixture.** `crates/pdf-signature/tests/fixtures/token_path/generate.sh` (openssl 3.5): a test root, a 2048-bit
intermediate, a 3072-bit unit, and two delegated responders' "good" answers, about the unit and the intermediate.

**Tests.** `a_tokens_own_path_and_its_responders_are_asked_by_clause_9_5_in_the_report` (five clause 9.5 issues
with the root supplied, none without); `a_delegated_responder_is_asked_and_compared_with_its_issuer` now reads
the responder from the path validation, with another leaf of the same CA naming none.
**Plants**: the established path ignored in `token_keys` fails the report test; responders not kept by the
path walk fail both tests; responders taken without the `CertID` match fail the responder test.

**Gates.** Over `pdf-signature`, `viewer-core`: fmt exit 0; clippy `-D warnings --all-targets` exit 0 (after
one `too_many_lines` in the new test, split); nextest `--lib --bins --tests` 1293 of 1293 passed, the audit
manifest's difference empty; rustdoc `-D warnings --document-private-items` exit 101 on an ambiguous `unit`
link, fixed, then exit 0; `cd fuzz && cargo check --bins` exit 0. Conformance after `git add -N`: exit 101 on
ADR 3058's status word and this record's missing gates paragraph, both fixed, then exit 0.

**Left.** Clause 9.4's period, until a host or a policy states one; national rules, until a host chooses them.
