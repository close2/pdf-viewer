# 2810 — A crypt filter with recipients of its own decrypts under a key of its own

Session 1999, a ledger round (§7.6.6, Table 27). Status: implemented. Rows: §7.6.5, §7.6.5.3, §7.6.6.
Carries out item 3 of ADR 2801's "What is left, in order".

## Context

Table 27 gives `/Recipients` two shapes. For the filter `/StmF` or `/StrF` names it is an array of
CMS objects whose enveloped data is a seed and permissions. For a filter "referenced from a Crypt
filter decode parameter dictionary" it "shall be a string that shall be a binary-encoded CMS
object", whose enveloped data "shall be a 20-byte seed value that shall be used to create the file
encryption key", and §7.6.5.3 adds that "the 4 bytes of permissions shall be absent". So one
document can hold several file encryption keys. `Encryption` held one, and ADR 2802 refused the
second shape by name rather than decrypt such a stream under the document's key.

## Decision

1. **`Encryption` keeps a key per such filter**, by its `/CF` name (`own_keys`), or `None` where the
   reader's certificate is on no recipient of that filter's list. A stream reaches it through
   `StreamFilter::Named`, which `Document::stream_filter` returns for a `/Crypt` decode parameters
   `/Name`; every other stream keeps its default method under the document's key. The writer's
   `encrypt_stream` uses the same key, so §7.5.6's incremental update keeps such a stream readable.
2. **The key is §7.6.5.3's digest over the seed and the one CMS object.** Step c) — the four 0xFF
   bytes — is for "document-level encryption", which a stream's filter is not. The width follows
   the filter's own `/CFM` as the document filter's does: SHA-1 and 16 bytes (or `/Length`'s RC4
   width) at `/V` 4, SHA-256 and 32 bytes for `AESV3`. Table 27 names Algorithm 1 for this case,
   and at `/V` 5 Table 20 makes every filter Algorithm 1.A's `AESV3`, which `crypt_filters` already
   enforces; the reading taken is the one that lets both sentences hold, and the stream is then
   decrypted by the algorithm its method names. No document anywhere here exercises `/V` 5 with
   such a filter, so that branch is reasoned, not witnessed.
3. **Table 25's `/AuthEvent` is asked of each key separately.** The document's key is wanted at the
   open where `/StmF` or `/StrF` encrypts or a filter without recipients of its own says `DocOpen`;
   a filter with its own list wants its own key at the open where its event is `DocOpen`, the
   default, and then a reader its list does not name is `SyntaxError::RecipientRequired` — "[i]f
   authorization fails, the event shall fail". At `EFOpen` the stream alone refuses, which is
   §7.6.6's "treat any attempt to access a stream for which authorization has failed as an error".
4. **An array-valued `/Recipients` on a filter neither `/StmF` nor `/StrF` names is refused by
   name**: Table 27 gives the array form only to those, and a reading of it for another filter
   would be a guess at which key its streams want.
5. **The key is derived at the open, one RSA decryption per such filter.** A deferred derivation
   would have to carry the reader's private key inside `Encryption` for the document's life; the
   eager one costs a filter's RSA operation only for documents that declare such filters, which no
   document in the corpora here does.

## Consequences

The fixtures are written by `gen_stream_recipients.py` (kept with the round): the document list and
the stream's own list enveloped by `openssl cms -encrypt` for the tree's `testnutzer` and `test`
keystore certificates, every key and stream computed from §7.6.5.3 and §7.6.3.2 by the script.
