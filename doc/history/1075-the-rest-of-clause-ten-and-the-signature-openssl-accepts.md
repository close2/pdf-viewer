# 1075 — The rest of clause ten, and the signature openssl accepts

Round 1048 named its residue: "X.690 clause 10's other DER restrictions". **ITU-T X.690 is free.** Fetched the in-force edition
(**2021-02**, 38 pages) from `itu.int/rec/dologin_pub.asp` with a browser `User-Agent` and prepared by `tools/spec-md.py`. **Its
notice is ETSI's shape** — "© ITU 2021. All rights reserved. No part of this publication may be reproduced, by any means
whatsoever, without the prior written permission of ITU" — so every rule below is paraphrase under a clause number (ADR 1085's
rule, second publisher). ADR 1089.

**Clause 10 is not where most of the rules are**: its opening sentence makes DER clause 8's encoding plus clause 10's
restrictions *and clause 11's*, so reading only under that number would have implemented three of nine. `der::is_canonical`
states 10.1 (definite, **and minimal length octets**), 10.2 (no constructed string), 11.6 with 10.3 (set members ascending),
11.1 (BOOLEAN), 11.2.1 (no unused bit set), 11.7/11.8 (time terminated, seconds, no trailing zeros, midnight) and — BER's,
binding DER through that first sentence — **8.3.2** (INTEGER minimal) and **8.19.2** (OID subidentifiers minimal). Absent as a
statement: 11.5's `DEFAULT` and 11.2.2's named bit list need the module, 11.3 and 11.4 name no type here.

**Census, 90 763 files, 2027 `SignedData`.** 1826 signed-attribute regions DER, 198 state no attributes, **3** depart — all in
`DSS-1260-1.pdf`, a length of 5199 in five octets where 10.1 needs three, confirmed by `openssl asn1parse` at offset 5063 (`hl=6
l=5199`) inside `adbe-revocationInfoArchival`. **`openssl cms -verify` calls that file's signatures good**, so the refusal is
argued rather than inherited: RFC 5652 section 5.4 digests "the complete DER encoding of the SignedAttrs value", no DER encoder
writes those octets, and that encoding is not in the file. `Authenticity::SignedAttributesNotDer { rule }` carries which of the
nine. The three secondary sites — RFC 3161's `TSTInfo`, RFC 6960's responses, Table 261's streams — refuse **nothing** new (545
of 551 still read). §12.8.3.4.2's own first sentence became a *report*, `PadesDeparture::ValueNotDerEncoded`: **162**
`ETSI.CAdES.detached` signatures depart, every one for an indefinite length, a population nothing could count before.
Calibration (trap 13): nine planted/twin pairs in `der`, seven more through `authenticity`; `BOOLEAN` and `BIT STRING` get no
CMS pair, no defined attribute being either type. `fuzz_targets/revocation.rs` asks that `is_canonical` is total and never calls
canonical what the length check calls indefinite.

**Curves re-measured 2026-09-15, unchanged, with the reasons now written down**: `bp512` still does not exist (RustCrypto's is
in draft); `bp512-nestler` 0.2.1 is **PolyForm-Noncommercial-1.0.0** over a release-candidate fork, which an Apache project
cannot take; `ed448` 0.5.0 is stable but is RustCrypto's *type* crate, no verification; `ed448-goldilocks`'s scheme line is
still 0.14.0-pre.15.

**The download made the PDF a corpus document** — `doc/*.pdf` is what the oracle walks for page one (the `ICC.1-2022-05.pdf`
precedent; no rule excludes a specification fetched for reading) — so it now lives in the main checkout's `doc/` and page 1 is
**held by name**. Only the differing fraction is out (8.90% of 5.30%). Over the page's 122×843 striped image, magnified 1.006 so
each pixel centre falls in one sample, the renderers split two and two: ours and `ghostscript` byte-identical on 99.0% of that
quarter, `poppler` and `mupdf` 63.7% with each other and 10.0 of 255 from us. That is §10.7.4's "There shall not be averaging
over the pixel area", so the page joins `CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE`, which held that reading and was empty;
its smaller half (42% of differing pixels) is non-embedded Arial and Times, named. `text_extraction`'s 8563 → 8266 is round
1077's, over `doc/pdf.js`. Rows: §12.8.3.4.2 `partial` → **`implemented`**; §12.8.3 and §12.8.3.1's curve sentences re-dated.
