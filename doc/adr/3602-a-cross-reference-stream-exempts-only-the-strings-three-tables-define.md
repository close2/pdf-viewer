# 3602 — A cross-reference stream's dictionary exempts only the strings three tables define

Session 2225. Status: **accepted**. Context: ADR 3581, pdf-issues #784, ISO 32000-2 §7.5.8.2 and
§7.6.2.

## Context

As printed, §7.5.8.2 exempts every string "appearing in the cross-reference stream dictionary"
from encryption. `Document::decrypt_value` and `encrypt_value` therefore left a cross-reference
stream's whole dictionary as written. The C&P TWG found that wording too wide (#784): a
cross-reference stream dictionary is also the trailer. The TWG decided a replacement on
2026-10-15, and pdf-issues.pdfa.org publishes it. The exemption is now *all strings appearing in
the cross-reference stream dictionary defined in* Table 5, Table 15 and Table 17. The same
change adds that such a stream shall not have an `/F` entry.

## Decision

The corrected text governs (ADR 3581). `encryption::is_defined_cross_reference_key` lists the
keys of those three tables as `doc/md/` prints them. On both the decryption and the re-encryption
path, an entry under one of those keys is kept as written, and an entry under any other key is
decrypted or encrypted like every other string in the body. The test
`a_cross_reference_stream_dictionarys_private_string_is_decrypted` fails without the change: a
private `/PrivateNote` came back as ciphertext. The `/ID` test beside it still passes.

The new `/F` prohibition binds a writer. This tree writes no `/F` on a cross-reference stream, so
nothing is added to read it.

## Consequences

A file whose producer followed the printed text and left a private string in the clear now
decrypts that string into noise. The corrected text names that file non-conforming. The
population is not counted: no corpus census here asks what keys a cross-reference stream
carries.
