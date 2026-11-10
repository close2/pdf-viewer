# 1985 — A certification path a policy mandates is the one validated

Session 1985, a single ledger round (§12.8.3.4.4), the second ledger lane. ADRs 2781 and 2782; no question.

**The slice, and why.** Of ADR 2775's residue the brief offered attribute certificates and a policy-supplied path.
The path was one construction over `certification_path`'s search; attribute certificates are several (an RFC 5755
reader, the authority's signature and path, the holder's binding, validity, revocation, type and value constraints,
in three syntaxes), and with the machine's load near 440 the first `cargo check` waited 37 minutes behind the
warm-up build. So the path was built, and the other three pieces were read rather than built (ADR 2782).

**The premise held for the path**: both parsers pushed `UseCertPath` / `userCertPath` as a name and the condition
was never met. **It did not hold for retention**: RFC 6960 section 4.4.4's archive cutoff lets an OCSP response
state its responder's retention interval, so TS 119 172-2 clause 4.23.3.1 item 2 is decidable where an answering
response carries it; the note says so, as work. Trusted-list anchors: the port is ADR 1728's policy fetch, not
ADR 1076's anchors. None is owner-blocked, so no Q821.

**Built.** `TrustPoint::mandated_path`, read from TS 119 172-2's `X509Certificate` list and TS 119 172-3's `path [1]`
(`asInSignature` false, naming no path, stays named). `search_from` builds over the list in place of the signature's
certificates (`pool`), counts listed certificates left off (`left_off`, the signer's and trust point's own counted
on), and applies clause 4.22.5.1 item 5's precedence: `pathLenConstraint` and the policy and name inputs yield, each
named in `Found::Reached`'s `mandated`. A validated path that leaves one off is `MandatedPathNotFollowed`, not
evaluated. `attempt` and `reached` keep `search_from` under clippy's line bound.

**Tests.** `certification_path/tests.rs`: the policy's intermediate completes a path the signature cannot; the
signature's certificates do not stand in; a listed certificate left off or unreadable is not evaluated; a path past
`pathLenConstraint`, and one refused under an explicit-policy input, met with the yielding member named.
`ts_119_172_2/tests.rs` and `ts_119_172_3/tests.rs`: each syntax's list is the path used. **Plants**: the pool
ignoring the list fails five; dropping only the inputs' yield fails one; reversed, all pass.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` of `pdf-signature`: exit 0. nextest of
`pdf-signature`: 461 of 461, 5 skipped. No crate outside `pdf-signature` names `TrustPoint` or `Found`.
`cargo test -p conformance` after `git add -N`: exit 0. `state.sh --round 1985 docs`: exit 0.
Tier 2: `pdf-model corpus` exit 101 as at HEAD (974 documents, 62 incomplete against a ceiling of 60, the same two joined names); no other line was run, the change drawing nothing and no corpus signature stating a machine-readable policy (ADR 1709).

**Left of the row**: attribute certificates and their trust condition, trusted-list anchors, the retention of
expired revocation notices (each work, ADR 2782), and the limits ADR 2725 lists.
