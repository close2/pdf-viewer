# 3161 — X.520's identifier rules are written from the RFCs they name, and `role` has no rule

Session 2115, a clause round in the `lane3` worktree. Status: accepted and built. Row: §12.8.3.4.4.
Context: ADRs 1085, 2895, 3158; ETSI TR 102 272 clause 6.9; ITU-T X.520 (10/2019) clauses 6.2.12 to
6.2.17, 6.9.1, 6.14.2, 6.15.15, 8.2.10, 8.9.1 to 8.9.4 and 8.10.1; ITU-T X.509 (10/2019) clause 16.5.1
and Annex B; RFC 3492, RFC 3986, RFC 5890 to RFC 5893, RFC 5895, RFC 7622, RFC 8264, RFC 8265. The
ITU-T texts are cited and paraphrased (ADR 1085). X.520 and X.509 were fetched from the ITU's
free catalogue (`https://www.itu.int/rec/T-REC-X.520-201910-I`, PDF SHA-256 `78cac8b6…80bd3fd1`;
`https://www.itu.int/rec/T-REC-X.509-201910-I`, `1f4ea293…8822e8c`) and the RFCs from the RFC
Editor; none is tracked. The session worked in `~/scratch/agent/r2115/`.

## Context

ADR 3158 held every X.520 equality rule an `id-at` type states but six, and left `id-at-role`
undecided because ITU-T X.509 was not held. Each of the six is defined by X.520 in a paragraph that
hands the substance to an RFC.

## Decision

1. **`id-at-role` has no equality rule.** X.509 clause 16.5.1 defines `role` over `RoleSyntax` with
   no `EQUALITY MATCHING RULE`, and leaves its values' specification outside the Recommendation. By
   X.501 clause 13.4.7 its values are never compared: a limit, beside `searchGuide`'s. X.509 is free
   to obtain from the ITU and, like T.801, not free to reproduce; it is cited by clause rather than
   placed in `doc/`, as the brief allowed.
2. **`uriMatch`** (`values/uri.rs`): RFC 3986 Appendix B's components, then section 6.2.2's four
   normalisations as clause 8.9.1 lists them. Its item a) names only the hexadecimal digits; the
   clause also says it conforms to section 6.2.2, whose 6.2.2.1 lowercases scheme and host, so both
   are taken. Item d)'s "empty or default" components are removed — empty port, user information,
   query and fragment, and the default port of `http`, `https`, `ws`, `wss`, `ftp` and `ldap` from
   their own specifications — and an empty `http`/`https`/`ws`/`wss` path is `/` (RFC 9110 section
   4.2.3, RFC 6455 section 3). A non-ASCII value is an IRI, not a URI, and is undefined.
3. **`dnsNameMatch` and `intEmailMatch`** (`values/domain.rs`, `values/punycode.rs`): clause 6.2.15's
   three label forms, a U-label valid by RFC 5891 section 4.2 (NFC, hyphens, no leading mark, RFC
   5892's derived property with Appendix A's contextual rules, RFC 5893's Bidi Rule over a name with
   a right-to-left label), an A-label valid where it decodes to such a U-label and encodes back, and
   RFC 3492's Punycode for the conversion. Wildcards follow clause 8.9.2 d) and e) and its note.
4. **`jidMatch`** (`values/jid.rs`, `values/precis.rs`): RFC 7622 section 3's parse-then-enforce,
   RFC 8265's two profiles (it obsoletes the RFC 7613 RFC 7622 names, with the same profiles), RFC
   5895's three mappings for the domainpart, and octet equality of the three parts.
5. **The derived properties are computed, not tabled** (`values/code_points.rs`): both RFCs define
   them as algorithms over Unicode properties, and the properties come from `icu_properties`'
   compiled data (`doc/stack.md`). RFC 5892's `Unstable` is read as `Changes_When_NFKC_Casefolded`,
   which differs only on default-ignorable code points, which the next step disallows anyway. The
   width mapping's 226 code points are a table read off `UnicodeData.txt` and checked against NFKC.
6. **`presentationAddressMatch`** (`structured.rs`): explicit tags (the module states no default);
   selectors equal, a selector one side states and the other omits being unequal; the presented
   network addresses a subset of the stored.
7. **`userPwdMatch`**: clear against clear by `caseExactMatch`; encrypted against encrypted by the
   algorithm identifiers' DER encodings, then `octetStringMatch`. **Clear against encrypted is
   undefined, a limit**: the clear text would be encrypted by the other side's algorithm, and no
   Recommendation says how — clause 6.14.2 points to X.509 Annex B, which lists no password
   encryption, and clause 6.15.15 to an Annex L that X.520 (10/2019) does not contain.

The policy's value is the presented one throughout (ADR 3158 decision 3). Every value outside its
type's syntax is undefined, never met and never broken.
