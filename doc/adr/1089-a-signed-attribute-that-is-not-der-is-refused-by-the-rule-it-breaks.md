# 1089 — A signed attribute that is not DER is refused by the rule it breaks

Session 1075. Status: **accepted**.

ADR 1048 read RFC 5652's one DER-only region and built the length-form half of the check. X.690
clause 10 states the rest, and the residue was named in four rows and in `der.rs`'s own comment:
"it checks only the *length form*". ITU-T X.690 is free — fetched from itu.int, the in-force
edition (2021-02) — so the residue was a `curl` (ADR 1085's rule, second publisher).

## 1. What "the whole of clause 10" turned out to be

Clause 10's opening sentence makes DER the basic encoding of clause 8 together with clause 10's
restrictions **and those listed in clause 11**. So `der::is_canonical` states nine rules from three
places, and the ones a round would guess wrong are the ones that are not in clause 10 at all: a
`SET OF` out of order is clause 11.6, a `BOOLEAN` that is not 0x00 or 0xFF is clause 11.1, and a
non-minimal `INTEGER` is **clause 8.3.2** — a *BER* rule that binds DER only through that opening
sentence. A check that had looked only under the clause number the residue named would have
implemented three of the nine.

**Three of clause 11's rules are not decidable from octets**, and their absence is a statement:
clause 11.5's `DEFAULT` omission and clause 11.2.2's named bit list each need the ASN.1 module, and
clause 11.3's `REAL` and clause 11.4's `GeneralString` describe types nothing under RFC 5652 that
this crate reads carries. Saying which rules a checker cannot reach is the difference between a
clean answer about the file and a clean answer about the checker (trap 13).

ITU's notice permits no reproduction without written permission. Every rule is therefore a
paraphrase under a clause number, in source, in the ledger and here; ISO 32000-2 and the RFCs are
quoted as always. That is ADR 1085's rule, and it now has two publishers under it.

## 2. The refusal stands, and openssl verifying the file is why it needed writing down

**The census found the rule firing on real files**: over 90 763 documents, 2027 `SignedData`, of
which 1826 signed-attribute regions are DER, 198 state no attributes, and **three** — all in
`corpus-cache/tika-issue-tracker/batch5/DSS/DSS-1260-1.pdf` — write a length of 5199 in five octets
where clause 10.1 needs three. `openssl asn1parse` confirms it at offset 5063, `hl=6 l=5199`,
inside the `adbe-revocationInfoArchival` attribute's value.

**And `openssl cms -verify` calls both of that file's signatures good.** So this program now refuses
a signature a widely used verifier accepts, and the decision must be argued rather than inherited.
It is argued from RFC 5652 section 5.4: the digest is over "the complete DER encoding of the
SignedAttrs value". A conforming DER encoder never emits five length octets for 5199, so the octets
in this file are not that encoding, and the encoding the section names is not in the file to digest.
Digesting the file's own octets instead would be this program choosing a message the RFC does not
name, and would answer *this signature does not verify* or *verified* about a comparison the RFC
does not define. `Authenticity::SignedAttributesNotDer` carries the rule so that a reader is told
which of nine restrictions the producer broke — a padded length octet and a `SET OF` out of order
are not the same kind of departure and a reader deciding what to do about the file needs to know
which one it has.

**This is the same clause as ADR 1048's, not an extension of it.** Clause 10.1 is one sentence with
two halves; refusing the indefinite form and accepting a non-minimal length would have made the
row incoherent. Agreement with openssl would have raised confidence and disagreement is a question
for the clause — which answered (`CLAUDE.md` principle 5).

## 3. Where the check is asked, and where it is a report instead

Four sites refuse by name: the signed attributes (RFC 5652 section 5.3), RFC 3161 section 2.4.2's
`TSTInfo`, RFC 6960 section 4.2.1's responses, Table 261's streams. Over the same 90 763 documents
the three secondary sites refuse **nothing** they did not refuse before — 545 of 551 CRLs and
responses still read, the six that do not exceed `MAX_VALUE` — so widening them cost no capability.

§12.8.3.4.2's own first sentence is the fifth site and it is a **report**:
`PadesDeparture::ValueNotDerEncoded`. RFC 5652 section 2 permits BER throughout a CMS object, real
producers write it, and `der` reads it (ADR 0215); what the subclause asks for one `/SubFilter` is
stricter than the RFC, so a `shall` a file breaks is said out loud rather than acted on. **162 of
the corpus's `ETSI.CAdES.detached` signatures depart from it**, every one of them for an indefinite
length — a population this tree had never reported because nothing could answer the question.
