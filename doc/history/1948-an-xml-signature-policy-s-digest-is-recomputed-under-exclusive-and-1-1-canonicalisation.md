# 1948 — An XML signature policy's digest is re-computed under exclusive and 1.1 canonicalisation

Session 1948, a single ledger round (§12.8.3.4.4), the ledger lane. ADRs 2710 and 2711; no question.

**The premise held for the methods.** `canonical_xml::Comments::of` placed only the two 1.0 URIs, so a
TS 119 172-2 document whose `Digest` named `xml-exc-c14n#` or `xml-c14n11` was
`NotEnforced::CanonicalDigestNotComputed`, said as a method this program does not compute. No
policy naming either method is on this disk. **It did not hold for the status** (ADR 2711): the note's not-evaluated
list keeps owed work, the trust conditions against a policy's own trust points and the full-path rules.

**Built (ADR 2710).** `canonical_xml::Method` (algorithm, comments, `PrefixList`) replaces `Comments::of`
and places six URIs. Exclusive section 3: visibly-utilized namespaces through section 3.1's
`ns_rendered` stacks, `xmlns=""` only under a rendered default, no `xml` attribute copied, the
`PrefixList` (section 4, read by `ts_119_172_2` from `ec:InclusiveNamespaces`) under 1.0's rules and
charged to the entry budget. 1.1 section 2.4: only `xml:lang`/`xml:space` copied, `xml:base` fixed up
by the new `canonical_xml::base::join` (RFC 3986 section 5.2 with the section's five changes).

**Tests from the specifications**: exclusive sections 2.1 and 2.2 (both envelopes, inclusive and
exclusive forms), section 4's `PrefixList`, 1.1's appendix table (64 rows), section 2.4's four join
examples, section 3.8 as `e1` and as `e3` apex (DTD default written in), a constructed end-to-end
policy under three methods with each other's digest refused. **Plant**: three defects, 2, 1 and 2 of
22 fail.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` (`pdf-signature`): exit 0. nextest:
`pdf-signature` 309 of 309, `viewer-core` 443 of 443, `viewer-host --test signature_policy` 11 of 11.
`cargo test -p conformance` after `git add -N`: exit 0. `state.sh --round 1948 docs`: exit 0.
Tier 2 not run: rule 2 owes all of it, but only `viewer-core` and `viewer-host` call policy
enforcement and no tier-2 line does, so nothing a walk prints can move.

**Left of the row**: human-readable policies (a limit), a C caller's fetched copy, and the
path-dependent constraints (ADR 2711). The W3C texts were prepared outside the tree, for the merge to
place in `doc/md/`.
