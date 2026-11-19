# 3158 — A signer attribute's value is compared by every X.520 equality rule but six

Session 2114, a clause round in the `lane3` worktree. Status: accepted and built. Row: §12.8.3.4.4
(stays `partial`). Context: ADRs 1085, 2782, 2895; ETSI TR 102 272 clause 6.9; ITU-T X.501 (10/2019)
clauses 8.9.2.1, 9.4, 13.4.2 and 13.4.7; ITU-T X.520 (10/2019) clauses 6, 7, 8.1, 8.2, 8.9 and
8.10. The ITU-T texts are cited and paraphrased (ADR 1085).

## Context

ADR 2895 held `caseIgnoreMatch` for 26 `id-at` types on ASCII only, and left every other rule, and
every character outside ASCII, undecided. §12.8.3.4.4's note named that as work.

## Decision

1. **Clause 7's preparation is written from the clause** (`values/prepare.rs`), over reviewed data:
   RFC 3454 table B.2 from `stringprep`'s tables, Form KC from `unicode-normalization`, and the
   general categories clauses 7.2 and 7.4 name from `unicode-properties` (Unicode 17). Both
   packages were already locked under `stringprep`. `stringprep::x520prep` is not called: with
   folding it skips clause 7.3's normalisation, and it tests the input's first code point rather
   than the prepared string's. The clause prints the variation selectors as "180B-180D,FF00-FE0F";
   the second range is empty as printed and is read as the block FE00–FE0F. Non-characters are
   unassigned by general category, so the one test prohibits both.
2. **A census of X.520's `id-at` types** (86 definitions, collective subtypes included, each with its
   rule resolved through `SUBTYPE OF` by X.501 clause 13.4.2) is `values::RULES`. Held: clause
   8.1.1's case-ignore and case-exact, 8.1.4's numeric-string, 8.1.7's case-ignore-list, 8.2.4's
   bit-string, 8.2.5's octet-string, 8.2.8's telephone-number, 8.2.11's unique-member, 8.2.12's
   protocol-information, 8.2.13's facsimile-number and 8.2.15's UUID-pair rules, and X.501's
   object-identifier and distinguished-name rules. A name's values are each compared by their own
   type's rule, to a bounded depth.
3. **The policy's value is the presented one.** X.501 clause 8.9.2.1 (a)(3) asks that it conform to
   the rule's assertion syntax, so where the assertion syntax differs from the attribute's (a
   facsimile number's is `TelephoneNumber`, a protocol information's an octet string) the listed
   value is read as the assertion and the carried one as the stored value; `uniqueMemberMatch`'s
   asymmetry (an absent `uid` in the attribute value is enough) follows from it.
4. **Undefined, and named**: types with no equality rule (X.501 clause 13.4.7: a limit);
   `countryCode3n`'s `NumericString` under `caseIgnoreMatch` and `dc`'s `IA5String` (outside the
   syntax: a limit); a `TeletexString` (clause 7.1's permission) and a constructed bit or octet
   string (not read).

## What is left, in order

`uriMatch` (RFC 3986 section 6.2.2's four normalisations and scheme defaults), `dnsNameMatch` (U-label
to A-label by RFC 5890 and RFC 3492, wildcard rules), `intEmailMatch` over it,
`presentationAddressMatch`, `userPwdMatch` (clear text by `caseExactMatch`; an encrypted side needs
its `pwdEncAlg`), `jidMatch` (RFC 7622's PRECIS profiles), and `id-at-role`'s definition in ITU-T
X.509, which is free from the ITU and not yet held. None needs the owner's word, so no Q936.
