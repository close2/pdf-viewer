# 3283 — A `/Crypt` filter naming a crypt filter in an unencrypted document is undecryptable

Status: accepted. Session 2145. Context: ISO 32000-2 §7.4.10 (Table 14), §7.6.6, Table 26; audit 2142 findings B07
and D02; ADR 3233; trap 5.

## The defect

In a document with no encryption dictionary, no decryption runs, and `decode_reported`'s `Crypt` arm passes its bytes
through, so `/Filter /Crypt /DecodeParms << /Name /StdCF >>` handed over the stream's bytes — ciphertext, if the name
means what it says — as decoded data. The same name in an encrypted document whose `/CF` lacks it is refused
(`Encryption::named_method` answers `None`, ADR 3233).

## Decision

1. ADR 3233's rule, that a crypt filter name means the same wherever it is read, is applied to the unencrypted
   document. Table 14's `/Name` "shall correspond to an entry in the CF entry of the encryption dictionary ... or one
   of the standard crypt filters"; with no encryption dictionary there is no `/CF`, so every name but Table 26's
   `Identity` names nothing. When such a stream is read (`Document::decrypt_object`'s unencrypted branch) it is marked
   `decryption_failed` with no data, exactly as an encrypted document marks it, so every route — the buffered decode,
   the window, the image reader, the writers — refuses it by the one flag they already ask.
2. A missing `/Name` is §7.6.6's Identity, as before. A `/Name` that is not a name is read as missing, as
   `stream_filter` reads it.
3. Every stream of an unencrypted document passes the check, so a `/Filter` that states no `/Crypt` where it stands
   is answered from the dictionary without resolving the chain.
4. D02: ledger 7.4.10's note is restated. The decryption runs on the raw bytes before the first stage wherever
   `/Crypt` stands, so a misplaced `/Crypt` does not commute with the stages before it; this reader decrypts first
   regardless, and that is a choice on a file that breaks "[t]he Crypt filter shall be the first filter".

## Cost

A stream in an unencrypted file that names a crypt filter is no longer drawn. No corpus document is known to carry
one; the refusal is `StreamRefusal::DecryptionFailed`, said where every other undecryptable stream is said.
