# 2872 — A signature's applicability report is REQ-4.5-01's items, typed

Session 2028, a signatures round (`doc/todo/77` item 3). Status: implemented in part. Rows: §12.8.3.4.5.
Context: ADRs 1085, 2834, 2864, 2865, 2866; ETSI TS 119 172-4 V1.1.1 clauses 4.2 and 4.5. Cited by
clause and paraphrased (ADR 1085).

## Context

ADR 2866 made the level — QES, qualified seal, AdES/QC, AdES, indeterminate — clause 4.4.2's own
output. The owner asked to "tell the user the different levels" (`doc/questions/A800` item 3), and
REQ-4.5-01 says what a report that tells it holds besides: the scope of the check (a), the signer
as the certificate represents it (b) and any pseudonym (c), the times with their assurance (d), the
signed data (e), the attributes by set (f), the overall status (g), the cryptographic suites with
their security issues (h), optionally each step (i). REQ-4.2-03 d) and e) add the suites' warnings.

## Decision

`pdf_signature::verdict::applicability::Report` holds a), b), c), d), f) and h) as typed fields, made
from the signature, its second question's answer, its best signature time, the chain, the material
and the anchors; `Cryptography::of` is REQ-4.2-03 d) and e) over §12.8.1's second answer.
`viewer_core::notes` says one sentence per item beside a listed signature's level, and REQ-4.2-03
e)'s warning wherever lists were given. e), g) and i) are the report's other sentences already: the
byte range, the verdict and the level.

Choices where the clause leaves the wording to the implementer, each written down as one:

1. **c)'s "used at the best signature time"** is the signing certificate's subject stating X.520's
   `pseudonym` (`2.5.4.65`): that certificate is the one in use at that time, and ETSI EN 319 412-2
   clause 4.2.4 is where a pseudonym is stated.
2. **d) ii) 4) and 5)** are the material's statements about the signer's certificate — a list by
   the certificate's issuer name, a response by its serial — reported as statements by their kind,
   never as established instants; the verdict's revocation answer is what weighed them.
3. **d) iii) 3)** and **d) i)** need TS 119 615 clause 4.6 (and clause 4.3 over a time-stamping
   service type), which this tree does not run; each token says *not determined* (`doc/todo/77`
   item 7).
4. **h)'s security issues** are assessed "against either national rules or ETSI TS 119 312", with
   which said. This tree holds neither, so `SecurityRules::NoneHeld` says no assessment was made,
   and the sentence says against what not. Holding TS 119 312's tables is item 3's remainder.
5. **a)'s text is not reproduced.** The clause prints a scope text and requires it in the report;
   ETSI's notice forbids reproduction without written permission, and ADR 1085 keeps every ETSI
   sentence out of the tree. `Scope` is a type, the sentence is a paraphrase that says it is one,
   and `doc/questions/Q855` asks whether the owner will seek or accept the permission.

## Consequences

A reader of a listed signature now sees who signed as the certificate says, whether under a
pseudonym, which times bear on it and how far each is proven, what was signed beside the document,
and the suite. Left: a)'s verbatim text (Q855), h)'s assessment (TS 119 312 not held), and d)'s
qualified-time-stamp answer (item 7). ADR 2866's join is now pinned end to end: a listed signer
whose issuer's CRL was issued after the best signature time reaches `TOTAL-PASSED`, and the same
signer with a CRL issued before it does not.
