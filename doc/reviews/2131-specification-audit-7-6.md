# Specification audit of ISO 32000-2 §7.6 — round 2131

The project's second specification audit, made beside the other lanes in the shape of the owner's
review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audit 2128
([`2128-specification-audit-12-8.md`](2128-specification-audit-12-8.md)). Baseline: main at
`8cb345d8`. Evidence: [`2131-specification-audit-7-6/`](2131-specification-audit-7-6/) —
`findings.json` (2038's schema) and `expected-failures.json` (one failing test per behaviour
finding). The same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. Every `shall` and `should` of §7.6.1 to §7.6.7 (Tables 20 to 28, Algorithms 1 to 13 and 2.A,
   2.B) was read in `doc/md/ISO_32000-2_sponsored_EC3.md` (lines 2460 to 3036), with the two
   losses the conversion is known for: Table 25's `/AuthEvent` rows (cut, and read in the PDF by
   `crypt::AuthEvent`'s doc) and the errata `doc/md/` cannot show (Issues #24, #74, #184, #325,
   #542, #643, which the code already cites).
2. Writer-only sentences (how a password is chosen, which salt is random) were held against the
   one writer, `serialize::Protected` and `crypt::revision6`; everything a reader owes was traced
   to `pdf_syntax::crypt` (`crypt.rs`, `crypt/public_key.rs`), `document/encryption.rs`,
   `document/parsing.rs`, and `pdf_pki::enveloped`'s cipher and recipient handling.
3. 2128's habit was the lens: **what binds to what**. The file key to `/O`, `/U`, `/OE`, `/UE`,
   `/Perms` and `/P`; `/EncryptMetadata` to what is decrypted; each crypt filter name to the
   stream it governs; each exempt string to the object that makes it exempt; the public-key key
   to its seed and every recipient list. A finding is a sentence of the clause the code does not
   meet (B), a missing or weak test (T), or prose that says the code does what it does not (D).
4. Each B finding has a failing `*_spec_audit` test in
   `crates/pdf-syntax/tests/encryption_spec_audit.rs`, whose documents are encrypted by that
   file from Algorithms 1 to 5 (the builders are `encryption_fixtures.rs`'s, independent of the
   crate), each with a control in the same test that opens and decrypts. No real document's bytes
   and no real key are used; B04's recipient lists are a two-byte placeholder no reader opens.

What holds is not listed one by one. Among it: Algorithm 1's per-object key, including the
low-order bytes, the `sAlT` suffix and the 16-byte cap; Algorithm 2's `/P` and step (f) binding of
the key at revisions 2 to 4; Algorithms 6 and 7 in the order §7.6.4.1 sets; Algorithm 2.A's three
sections, both unwraps' 32-byte check and 2.B's self-bounding loop (ADR 2961); the `/Perms` block
read over a disagreeing `/P` as recorded policy (ADR 2962); AES padding removed strictly and a
structurally impossible body refused rather than passed through; the `/ID`, the encryption
dictionary and a signature's `/Contents` exempt (including a `/Type`-less signature); `/AuthEvent`
`EFOpen` against `DocOpen` (ADR 1040); the public-key key over the seed and every recipient list in
order with step (c)'s four bytes only for the document-level filter; the 20-byte seed with and
without permissions; the first matching recipient list; Table 24's bits; the writer's reserved
bits, random salts, filler and initialisation vectors.

## Counts

| kind | count |
|---|---|
| B (behaviour) | 6 |
| T (test) | 1 |
| D (documentation) | 4 |

## Findings

**2131-B01 — every `/Type /Metadata` stream is left in the clear, not the document-level one.**
Table 21's `/EncryptMetadata` "[i]ndicates whether the document-level metadata stream ... shall be
encrypted", and Algorithm 2's NOTE 1: "This provision pertains only to document-level XMP
metadata." `stream_filter` answers Identity for any stream whose `/Type` is `/Metadata`, so a page's
or an image's metadata stream, encrypted under `/StmF` as the clause requires, is returned as
ciphertext; the writer's mirror leaves every such stream in the clear. Test:
`only_the_document_level_metadata_stream_is_left_in_the_clear_spec_audit`.

**2131-B02 — the key length is read from whichever `/CF` entry sorts first, `/Identity` included.**
Table 20's `/CF`: keys Table 26 lists "shall be ignored by a PDF processor". `crypt_filters` skips
them (2038-B07's fix); `key_length` does not, and takes the first entry's `/Length` whatever its name.
A conforming `/V 4` file with `/Identity << /Length 40 >>` beside a 16-byte `StdCF` derives a 5-byte
key and refuses its own password. Table 20's `/V` 4 row states the length (128 bits) outright. Test:
`an_identity_cf_entry_does_not_set_the_key_length_spec_audit`.

**2131-B03 — a `/Crypt` filter name `/CF` does not define is read as Identity.** §7.6.6's default
is for a *missing* name: "(if missing, Identity is used)". `named_method` gives Identity to any name
it does not find, so the stream's ciphertext is its data, with nothing said — the outcome
`crypt_filters` refuses for `/StmF` by name as trap 5. Test:
`a_crypt_filter_name_cf_does_not_define_is_not_identity_spec_audit`.

**2131-B04 — the public-key handler reads an `/Identity` `/CF` entry.** The same Table 20 sentence;
`filter_lists` treats every entry other than the two defaults as a stream filter with its own
recipients, so `/Identity << /Recipients [...] >>` refuses the document where the same document
without it asks for a certificate. Test: `a_public_key_identity_cf_entry_is_ignored_spec_audit`.

**2131-B05 — a cross-reference stream's dictionary is decrypted.** §7.5.8.2: "strings appearing in
the cross-reference stream dictionary shall not be encrypted", and that dictionary is the trailer
whose `/ID` §7.6.2 exempts. `decrypt_value` decrypts every stream dictionary before
`stream_filter` exempts the data, so the stream read as an object has RC4 noise (or, under AES, an
empty string) for `/ID`. The trailer the cross-reference parser builds is raw and correct. Test:
`a_cross_reference_stream_dictionarys_strings_are_not_decrypted_spec_audit`.

**2131-B06 — an embedded file stream is known only by its optional `/Type`.** Table 20's `/EFF`
governs "embedded file streams", and Table 44 makes their `/Type` "(Optional)". An untyped
attachment is decrypted under `/StmF`: with `/EFF /Identity` beside AESV2 its plaintext fails the
padding and reads as nothing. §7.6.6's RF-equals-EF rule is held "by construction" on the same
assumption. Test: `an_embedded_file_stream_without_a_type_takes_the_eff_filter_spec_audit`.

**2131-T01 — an unverifiable `/Perms` is untested, and its behaviour is a question.** Algorithm 2.A
step (f) says to "[v]erify that bytes 9-11 of the result are the characters "a", "d", "b"". Where
the block is absent, short or lacks the marker, the document opens under the plaintext `/P` with
nothing said, and at revision 6 the key from `/UE` or `/OE` is then checked by nothing. No test
pins this; what it should do is [Q953](../questions/Q953-what-does-a-revision-6-file-whose-perms-does-not-verify-open-as.md).

**2131-D01** — ledger 7.6.3.2 says `key_length` refuses a `/Length` outside "a multiple of 8, in the
range 40 to 128"; it accepts 44 (as 5 bytes) and 129 to 135 (as 16). **2131-D02** — `crypt.rs`'s
module doc lists §7.6.5.3's deprecated ciphers and non-RSA key transport as refused; `pdf_pki`
opens all seven ciphers and key agreement (ADR 3157). **2131-D03** — `serialize.rs` quotes Table 25
as "in bits or bytes, depending on the application", a phrase neither edition prints. **2131-D04** —
`named_method`'s doc defends B03 with the argument `crypt_filters` makes against it.

## Ledger rows a behaviour finding contradicts

Of the 34 §7.6 rows, 33 are `implemented` and 7.6.7 is `writer-side`. The findings contradict
**7.6.2** (B02, B04, B05, B06), **7.6.4.2** (B01), **7.6.4.3.2** (B01, B02), **7.6.3.2** (B02, and
D01's note), **7.6.6** (B03, B04, B06) and **7.6.5.2** (B04), and through them the aggregates 7.6,
7.6.3, 7.6.4, 7.6.4.3 and 7.6.5. They are not moved here; the fix round decides.

## Suggested fix groups

| group | findings | files |
|---|---|---|
| A — what a crypt filter name means | B02, B03, B04, D01, D04 | `pdf-syntax/src/crypt.rs` (`key_length`, `named_method`, `decrypt_stream`, `encrypt_stream`), `crypt/public_key.rs` (`filter_lists`, `StringLists::read`), ledger 7.6.3.2 |
| B — which stream a rule is about | B01, B05, B06 | `pdf-syntax/src/document/encryption.rs` (`decrypt_value`, `encrypt_value`, `stream_filter`), `serialize.rs`'s cleartext rule for metadata |
| C — `/Perms` | T01, after Q953 | `pdf-syntax/src/crypt.rs` (`Encryption::new`, `perms_block`) |
| D — prose | D02, D03 | `crypt.rs` module doc, `serialize.rs:1076` |

A and B touch different files and can run as two rounds; B06 is the one that needs a design (an
embedded file stream is defined by the reference to it, which object-at-a-time decryption does not
see), so B may take B01 and B05 first. None of the six lets a forged document pass as genuine; the
most consequential are B02 and B06, which make a conforming file unreadable, and B03, which shows
ciphertext as content.
