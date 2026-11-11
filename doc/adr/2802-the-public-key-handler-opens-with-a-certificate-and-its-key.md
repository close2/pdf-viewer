# 2802 — The public-key handler opens with a certificate and its key

Session 1995, a ledger round (§7.6.5). Status: implemented. Rows: §7.6.5 to §7.6.5.3, §7.6.6.
Builds on ADRs 1134, 1040 and 2801.

## Context

§7.6.5.1 puts one `shall` on a reader: it "shall scan the recipient list for which the content is
encrypted and shall attempt to find a match with a certificate that belongs to the user". §7.6.5.3
says what the matched recipient yields — "\[a] 20-byte seed" and "\[a] 4-byte value defining the
permissions, most significant byte first" — and how the file encryption key is digested from the
seed. The data is then §7.6.3's, which this tree runs. The handler lives in
`crates/pdf-syntax/src/crypt/public_key.rs`.

## Decision

1. **The reader's credential is a pair: certificate and private key** (`pdf_syntax::RecipientKey`).
   RFC 5652's `rid` names a recipient by issuer and serial number or by subject key identifier, and
   neither can be read off a private key. So "a certificate that belongs to the user" is an input,
   and both halves are read when they are handed over, so that a key that cannot be used is refused
   then and not at open. `Document::open_with_recipient` takes it, and nothing reads a key from a
   path or from the document (ADR 1134).
2. **A handler is a public-key handler by its `/SubFilter`.** Table 20 makes `/SubFilter` the entry
   that completely specifies the format, and §7.6.5.2's NOTE names three handler `/Filter` values as
   examples only. The three values are `adbe.pkcs7.s3`, `s4` and `s5`. s5 needs `/V` 4 or 5, and s3
   and s4 need `/V` below 4 ("shall be used when not using crypt filters"). Any other pairing is
   refused by name.
3. **Where the lists are.** For s3 and s4 they are Table 23's `/Recipients`. For s5 they are Table
   27's, in the filter `/StmF` names, or `/StrF`'s where `/StmF` is `Identity`.
   `/EncryptMetadata` is read from that filter only when `/StmF` names it ("used only by crypt
   filters that are referenced from StmF"). The first list that names the certificate wins ("the
   first matching list"). A list that does not parse cannot match, and when every list fails to
   parse, the refusal says so.
4. **No match is `SyntaxError::RecipientRequired`**: this handler's `PasswordRequired`, which a host
   answers by offering another certificate. Table 25's `/AuthEvent` applies to it as to a password
   (ADR 1040). A matched list that the key does not open is `SyntaxError::RecipientKey`: the
   document is fine and the key is wrong.
5. **The key.** It is SHA-256 when the key is 256 bits (`/V` 5, Algorithm 1.A), and SHA-1
   otherwise. The digest runs over the seed, then every list's bytes in array order, then four
   `0xFF` when the metadata is plaintext, and the key is the first n/8 bytes. n is 256 at `/V` 5,
   and 128 for `AESV2` (Errata Collection 3 Issue #184). For an `RC4` filter, n is Table 25's
   `/Length`. For s3 and s4, n is Table 20's `/Length`.
6. **Permissions** are Table 24's word. They are read with the standard handler's revision 2 for
   s3, and revision 3 for s4 and s5, as §7.6.5.3 assigns. Bit 2 "enables all other permissions",
   which is what `Permissions::owner` already means.

## Evidence

`crates/pdf-syntax/tests/public_key_witnesses.rs` opens A168's four `PDFBox` witnesses with their
keystores' keys. These are AES-128 and AES-256, two of them with `/EncryptMetadata false`, which is
step c)'s case. The page text that comes back is what `PDFBox`'s test states. The keys are the
published keystores converted with `openssl` (`tests/public_key_witnesses/`). The expected values
are the pages' text and the clause's algorithm; no other reader's output is used.
The four DER files are this tree's one copy out of a corpus. `doc/third-party-data.md` records them as
the exception to "nothing is copied", and `/NOTICE` section 3 carries PDFBox's NOTICE, as Apache-2.0
§4(d) asks.
