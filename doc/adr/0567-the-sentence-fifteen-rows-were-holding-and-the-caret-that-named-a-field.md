# ADR 0567 — The sentence fifteen rows were holding, and the caret that made a reading a requirement

Status: accepted, 2026-08-24. Session the seven-hundred-and-fifth, a clause round under
`doc/todo/01`, reading one family's `partial` rows against each other as well as against the code —
ADR 0538's method, now in its fourth round (0551, 0560, this). Amends §12.8.2.4, §12.8.3, §12.8.3.1,
§12.8.3.2, §12.8.3.3, §12.8.3.3.1, §12.8.3.3.2, §12.8.3.4, §12.8.3.4.1, §12.8.3.4.2, §12.8.3.4.3,
§12.8.3.4.4, §12.8.3.4.5, §12.8.3.4.6, §12.8.3.4.7 and §12.8.3.4.8 in the ledger; corrects two doc
comments in `crates/pdf-model/src/signature.rs`; adds one test to that module; adds one section to
`doc/errata-read.md`. **No status moves, no pixel moves, and no report is added or removed.**
Extends ADRs 0215, 0229, 0314, 0322, 0403, 0532 and 0538.

## 1. The family, and why it was the one to read

The blame ordering over `doc/conformance/ledger.toml`'s `note =` lines was re-derived on this base
rather than taken from any document that predicts it (616's rule): 951 commits, 875 rows, 222 of
them `partial` and 240 `partial`-or-`reported` with a blamed note. §7.6.4.4 is rank 1, §11.3.4 is 2,
and §11.3.7, §12.5, §8.6.6, §8.9.6, §8.9.6.2, §9.8.3 and §9.8.3.1 share 3–9, with §7.7 at 10 — the
same head 0551 and 0560 report, one round further on.

**The family was chosen by a measurement rather than off that list**, which is this round's one
addition to the method. ADR 0560's criterion is that *a claim held in duplicate has somewhere to
disagree with itself*, and the criterion can be made into a search: for every parent whose subtree
holds two or more `partial` rows, count the rare five-word sequences the notes share pairwise, where
*rare* is a sequence at most four rows in the whole ledger carry. §12.8's family scores highest and
§12.8.3's subtree is the reason — five rows all `partial`, sharing among other things a list of
digest algorithms and a sentence about Table 260's three families.

The duplication turned out to be larger than the search could show. **A paragraph of 92 words stood
byte-for-byte identical in fifteen rows** — every row of §12.8.3 and its whole subtree, `partial` and
`reported` alike — and it ends in two counts.

§7.6.4.4, rank 1, was read first and left alone: its arithmetic is right in every part (twelve rows
below, Algorithms 3 to 13 and §7.6.4.4.1's framing; six of the eleven compute what a writer stores;
three of those six `writer-side` because a reader re-runs the others), which is what ADR 0538's own
round left behind. 697's rule that a corrected row is not a safe row cuts both ways: a row *left
uncorrected* beside four that were rewritten is worth one reading, and this one held.

## 2. What was wrong

| row | shape | was | is |
|---|---|---|---|
| **fifteen rows at once** | a counted claim, copied | "The disjointness this row used to record — eight signed documents, twenty-six encrypted, no overlap — was therefore false; that document is both." | **nine** and **twenty-five** over `doc/pdf.js`, both from commands. §7.6's row has said twenty-five since ADR 0538's round, four rounds ago, while this family went on saying twenty-six in fifteen places |
| **§12.8.3.4** | a capability that arrived and announced nothing here | the signature-value question is answered "where the signer's key is RSA (ADR 0229) or, since the four-hundred-and-seventy-ninth, DSA (ADR 0314)" | every family: `Signature::authenticity` matches the `signatureAlgorithm` against the certificate's key and never against `/SubFilter`, so ECDSA and EdDSA (ADR 0532) reach a `PAdES` signature exactly as they reach any other detached CMS one — and ISO/TS 32002 puts them here by name |
| **§12.8.3.4.5** | the same, in the step that states the requirement | step (a)'s second half is checked "for an RSA key and — since the four-hundred-and-seventy-ninth session (ADR 0314) — a DSA one" | the same four families, under this `/SubFilter`, now asserted by a test |
| **`Signature::authenticity`** | the same, in the code | step 4 verifies "with the construction the `signatureAlgorithm` states — RFC 8017 section 8.2.2's encode-and-compare, its section 9.1.2's `EMSA-PSS-VERIFY`, or FIPS 186-4 section 4.7" | five constructions, two of them missing from the list while the module comment twelve lines above said "for all four" and named both modules |
| **§12.8.3.1** | a count its own next sentence contradicts | "**Six** of those eight are computed or would be" | **four** computed and four refused, which is ADR 0532's own arithmetic, `signature.rs`'s module comment's, and what §12.8.3, §12.8.3.3 and §12.8.3.3.1 all say. The six counted the two Brainpool curves whose package exists as a release candidate, as though a pre-release were an implementation |

## 3. The first one is ADR 0101's shape at fifteen times the scale

A round corrects a claim in the row that states it and leaves the copy standing. ADR 0101 recorded
that with two rows; 0551 recorded it again with two. Here the copy is fourteen-fold, and the reason
is legible: the paragraph was written once, by ADR 0215's round, as a per-row reminder that a
`partial` or `reported` status under §12.8.3 costs no pixel — a signature's appearance is an ordinary
widget annotation — and the two counts were appended to it when ADR 0215's own defect (`issue17069.pdf`,
signed *and* encrypted, its signature value destroyed by the cipher) retired the disjointness they
were part of.

**The repair is not to correct the number fifteen times.** The aggregate row, §12.8.3, keeps the
account and states both populations as the commands that produce them; the fourteen rows below it
keep the part that is about *them* — the appearance sentence, and the fact that §7.6.2's exception
is implemented one clause over on a document that is in both populations — and defer the counting.
One place to be right is the whole of the fix, and the sweep that would have found this earlier
does not exist: `--bin counts` reads a cardinal only where it governs one of the ledger's own words
for a row, and "eight signed documents" governs *documents*.

## 4. The second and third are the sixth failure shape, in a row and in the code together

`doc/habits.md`'s sixth shape is a row *corrected* by naming a capability that arrived, while the
entry it turns on stays unread. This is its neighbour and it is simpler: two rows and one doc
comment describe an answer as two families' four rounds after the other two arrived, while four
sibling rows and the module comment above the same function record all four.

**What makes it a finding rather than bookkeeping is that the standard puts the curves here by
name.** ISO/TS 32002 section 5.1.3 says

> Table 3 defines ECDSA elliptic curves and associated message digest algorithms supported for the
> adbe.pkcs7.detached, ETSI.CAdES.detached or ETSI.RFC3161 SubFilter values in ISO 32000-2:2020,
> Table 260.

and its section 5.1.2 says the same of Table 4. So a reader who took §12.8.3.4's row at its word
would conclude that an elliptic-curve `PAdES` signature is not answered here, when the requirement is
addressed to this subclause explicitly.

`signature.rs::a_pades_signature_verifies_under_an_elliptic_curve_key` is the assertion the tree did
not have. `a_pss_signature_verifies_through_the_whole_path_a_document_takes` was the only test
carrying `ETSI.CAdES.detached` through `authenticity`, and it exercises RSA's second padding; the
three curve tests all use `adbe.pkcs7.detached`. Calibrated per trap 13 by making `authenticity`
refuse an `ETSI.CAdES.detached` `/SubFilter` outright: the new test fails there with its own
message, `an_ecdsa_signature_verifies_through_the_whole_path_a_document_takes` passes, and the
plant was restored.

## 5. The erratum, which again states the authority for something this tree already did

`emit` files five annotations inside §12.8.3, three of which ask nothing of a reader. **The other two
are filed there and belong one subclause back**, which is 0551's finding about the instrument met
again: `emit` attributes an annotation by the page the outline puts in a clause, and §12.8.3's
heading sits at the foot of page 592 while the top of that page is Table 259 — §12.8.2.4's.

**Issue #33, `Review/Completed`, inserts "fully qualified" and a reference to §12.7.4.2 into Table
259's `/Fields` row.** Two carets with nothing struck out, so `check` is blind to both by
construction, which is the shape that has paid in every clause round since ADR 0538's. The arithmetic
that places them: the rects are `[249.018 594.369 256.952 600.833]` and
`[298.23 594.369 306.163 600.833]`, which is 241.1 to 247.6 from the top of an 841.92-tall page,
and `pdftotext -bbox` puts `containing field names.` at 235.4–247.1 with `field` starting at
x 253.0 and `names.` ending at x 304.1. One caret before `field`, one after `names`.

**This tree has read that entry as fully qualified since `FieldSelection` was written, on an argument
and a producer's file.** §12.8.2.4's row says so in as many words. It has the clause's authority now
— and only for one of `FieldSelection`'s two callers: §12.7.5.5's Table 236 is worded identically
and gains no such insertion, over eighteen annotations `emit` files under that subclause. So the
same comparison is *required* for the `FieldMDP` transform and remains the argued reading for the
signature field lock, and `covers`'s comment now says which is which. A function whose two callers
stand on different footings should not let the weaker one be forgotten, which is the general form of
this and is why the comment keeps both.

## 6. What was checked and left alone

Three claims in this family looked wrong and are not, and each is worth recording so the next round
does not spend the hour again:

- **§12.8.3.4's "all ten signature dictionaries in the 974" against `pades_departures`'s "all six the
  974 carry in a signature field".** Two populations, both exact: `signature::signatures` walks the
  AcroForm, and `signature_algorithm_census` adds §12.8.6's permissions route, where three of
  `doc/pdf.js`'s signed documents keep their only signature dictionary. Measured with a throwaway
  program (ADR 0481's method), which was not committed; §12.8.3.4's row now states the difference,
  because two figures a screen apart with no explanation between them are a defect waiting to be
  "found".
- **§12.8.3.2's "the clause names SHA-1 while Table 260 permits five — so each of the six is
  tried".** Five and six are both right: Table 260's `adbe.x509.rsa_sha1` column lists SHA1, SHA256,
  SHA384, SHA512 and RIPEMD160, and MD5 is the sixth, from Table 256's `/DigestMethod`. §12.8.3's own
  "the ten digests it and Table 256 name between them" is the same arithmetic with ISO/TS 32001's
  four added.
- **§12.8.3.4.3's "three of the eleven are checked".** (a) to (k) is eleven; (a), (d) and (e) are
  checked whole and half of (i) — counter-signature, whose identifier RFC 5652 states — is checked
  too, which the row says in its next sentence and `pades_departures` implements in seven rules.

## 7. Consequences

- Fifteen rows no longer state a stale count; one row states it as two commands.
- Two rows and one doc comment describe the signature-value answer as it is rather than as it was
  four rounds ago, and a test holds the description in place under the `/SubFilter` the subclause is
  scoped to.
- §12.8.2.4's fully-qualified reading rests on the standard for the transform it is required for,
  and is marked as an argument for the one it is not.
- **The search that picked the family is reusable and is not a program.** Pairwise rare-n-gram
  overlap among a subtree's `partial` notes ranks families by how much they restate each other, which
  is exactly ADR 0560's criterion mechanised. It was run as a throwaway and is described in
  `doc/todo/01` so the next round can run it in a minute rather than choosing by eye. It is not
  committed as a sweep: its output is a *ranking* rather than a hit list, nothing in it is a defect,
  and a sweep whose every line needs a person is a reading list with a build step.
