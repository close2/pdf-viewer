# 2826 — An RSAES-OAEP key-transport recipient opens under the parameters it states

Session 2004, a ledger round (§7.6.5.3). Status: implemented. Rows: §7.6.5, §7.6.5.3.
Takes the first of the four effort-only refusals ADR 2821 left on §7.6.5.3; the owner's A800 ordered
the handler built.

## Context

§7.6.5.3's key "shall be encrypted for each recipient, using that recipient's public key", and RFC
5652 section 6.2.1's `KeyTransRecipientInfo` names how in its `keyEncryptionAlgorithm`. ADR 2802
opened RFC 8017's `rsaEncryption` (section 7.2, `RSAES-PKCS1-v1_5`) and refused every other
identifier by name. RFC 3560 assigns the other RSA scheme to CMS: `id-RSAES-OAEP`, RFC 8017 section
7.1, with `RSAES-OAEP-params` choosing the hash, the mask generation function and the label. It is
the scheme a current writer is told to prefer, `openssl cms -encrypt -keyopt rsa_padding_mode:oaep`
writes it, and no document this tree holds uses it — so the refusal cost a reader of such a file its
document, and effort was the only obstacle.

## Decision

1. **`pdf_pki::rsa::oaep` is RFC 8017 section 7.1.2 and appendix B.2.1, written out.** RSADP is
   the module's existing primitive; EME-OAEP's decoding and MGF1 are a page of code over the
   `sha1`/`sha2` packages the crate already pins, so no package is added. The decoding folds `Y`,
   the label hash comparison, the zero run of `PS` and the search for its `0x01` into one flag over
   every octet before answering, and every failure after the length checks is the one
   `RsaError::Decryption` — section 7.1.2's note against Manger's attack, as section 7.2.2's
   decoding beside it already does.
2. **The parameters are RFC 3560 section 3's, with its defaults.** They "MUST be present"; each of
   the three members, absent, is its default (SHA-1, MGF1 with SHA-1, the empty label), because a
   reader "MUST recognize both" the stated default and its absence. A hash is any of RFC 8017
   appendix A.2.1's `OAEP-PSSDigestAlgorithms` — RFC 4055 section 2.1's five, which RFC 3560 permits
   ("MAY support other one-way hash functions"), and SHA-512/224 and SHA-512/256, which the scheme's
   own module lists — with NULL or absent parameters, which RFC 4055 makes "legal and equivalent".
   MGF1 is the only mask generation function either RFC defines, and `id-pSpecified` the only label
   source. Anything else is `EnvelopeError::KeyTransport` naming the member and its identifier;
   absent parameters, members out of DER's order, or a hash with non-NULL parameters are
   `EnvelopeError::Malformed`.
3. **`RecipientInfo::KeyTransport` carries its algorithm's parameters**, as `contentEncryptionAlgorithm`
   and the key-agreement wrap already do, so the scheme reads what the recipient states rather than
   assuming the default.
4. **It is pinned twice.** Section 7.1.1's encoding written by the tests from a chosen seed decrypts
   back under every hash pair of the seven with and without a label, and each condition of step
   3.g, planted, is the one error; and five envelopes `openssl cms -encrypt` wrote to `pdf-pki`'s
   own test certificate under different `rsa_oaep_md`, `rsa_mgf1_md` and `rsa_oaep_label` settings
   open to the plaintext the script put in, after the test has asserted that this reader read back
   the hash, the MGF1 hash and the label OpenSSL stated. Those five are what pins MGF1 against a
   second implementation: the tests' own encoder shares this module's.

## Consequences

`pdf-pki`'s public surface gains `rsa::oaep` (`OaepParameters`, `OaepHash`, `OaepParameterError`,
`ID_RSAES_OAEP`, `PrivateKey::decrypt_oaep`) and a `parameters` member on
`RecipientInfo::KeyTransport`. Nothing that opened before moves. SHA-512 as the scheme's hash needs
`k >= 130`, past a 1024-bit key, which step 1.c refuses as it states.
