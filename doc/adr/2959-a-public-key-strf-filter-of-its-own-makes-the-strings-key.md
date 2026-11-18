# 2959 — A public-key `/StrF` filter of its own makes the strings' key

Session 2072. Status: **implemented.** Context: review 2038 finding B57, ISO 32000-2 §7.6.5.2, §7.6.5.3, Table 27,
ADR 2810.

## 1. The finding

Under `adbe.pkcs7.s5`, `/StmF` and `/StrF` may name two different crypt filters. Table 27 says a crypt filter
"referenced from StmF or StrF in the encryption dictionary" carries `/Recipients` as "an array of byte strings", and
§7.6.5.3 makes a file encryption key from one filter's seed and that filter's lists. The reader keyed the whole
document from `/StmF`'s filter alone and read every other filter as Table 27's second case (a stream-only filter
with one CMS string), so a valid document with distinct default filters was refused at open
(`separate_stmf_and_strf_defaults_use_their_own_recipient_keys`).

## 2. The decision

`/StrF` naming a filter other than `/StmF`'s is the second default filter. Its `/Recipients` array is scanned for
the reader's certificate as `/StmF`'s is, and its key is §7.6.5.3's digest over its own seed and its own lists, without
step (c)'s four `0xFF` bytes, because Table 27 says `/EncryptMetadata` is "used only by crypt filters that are
referenced from StmF". The key is held as `Encryption::string_key`, and every string is decrypted and encrypted
through `decrypt_string`, `encrypt_string` and `encrypt_string_with_iv`, so the reader and both writers cannot
disagree on which key a string is under. The permissions are still the `/StmF` filter's seed's (§7.6.5.3 states
no rule for two permission words; the document-level filter is the one the stream data is under).

A reader the `/StrF` list does not name fails the open, as for `/StmF`: Table 25 wants both default filters' keys
when the document opens.
