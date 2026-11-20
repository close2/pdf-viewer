# 3233 — A crypt filter name means the same wherever it is read, and a name `/CF` does not define names no filter

Session 2133. Status: accepted and built. Context: review 2131 findings B02, B03, B04, D01, D04; ISO
32000-2 §7.6.6, Table 20 (`/CF`, `/StmF`, `/Length`), Table 25 (`/Length`), Table 26; trap 5; ADR 1040.

## Decision

1. **One predicate, `crypt::is_standard_filter_name`, answers Table 20's "Any keys in the CF
   dictionary that are listed in "Table 26 - Standard crypt filter names" shall be ignored by a PDF
   processor"**, and every reader of a crypt filter name asks it: `crypt_filters` (the `/CF` entries
   and `/StmF`, `/StrF`, `/EFF`), `key_length`, `Encryption::named_method`, and the public-key
   handler's `filter_lists`, which no longer reads an `/Identity` entry's `/Recipients` as a filter
   of its own (B04).
2. **`key_length` reads the filter the document's defaults name** — `/StmF`'s, then `/StrF`'s,
   then the first entry not under a Table 26 name — so an `/Identity << /Length 40 >>` beside a
   16-byte `StdCF` no longer derives a 5-byte key (B02).
3. **A bit count that is not a multiple of 8 is refused by name** rather than rounded down: Table
   20's `/Length` "shall be a multiple of 8, in the range 40 to 128", Table 25's "a multiple of 8 in
   the range of 40 to 256". 44 and 129 to 135 were read as 5 and 16 bytes; they are now refused,
   which is what ledger 7.6.3.2's note already said (D01). A negative count is still unreadable
   and takes the default, as `a_key_length_outside_table_20s_range_is_refused` records.
4. **`named_method` returns `Option<Method>`.** Identity is §7.6.6's default for a *missing*
   `/Name` — "(if missing, Identity is used)" — and a missing name never reaches the function. A
   name in neither `/CF` nor Table 26 is `None`, and `decrypt_stream` / `encrypt_stream` refuse the
   stream, which marks it `decryption_failed` as a failed authorization is (B03, D04). The writer
   (`serialize::Protected::method`) meets the same name in a source stream it carries and stops
   with `SerializeError::UndefinedCryptFilter`, because writing it would write a stream no reader,
   this one included, can decrypt.

## Why

`crypt_filters` already refused an undefined `/StmF` by name, as trap 5 asks: reading a name that
selects nothing as the default draws a page from its own ciphertext with nothing said. The same
name read in three other places meant three other things. Table 20's sentence about Table 26 names
is not about one entry — it is about the `/CF` dictionary, so it binds every reader of it.

`/V` 5's inference in `select` (an undefined `/StmF` at `/V` 5 is `AESV3`, because the row states
the document's cipher) is not extended to a stream's `/Crypt` entry: that row says how the
document's default filters encrypt, and a `/Crypt` entry is the stream that departs from them.

## Cost

A writer input whose stream carries a source's `/Crypt` name is refused where it was written in the
clear and counted in `Written::cleartext`. No corpus document is known to carry one; the refusal
names the filter. `SerializeError` is `#[non_exhaustive]`, so the new variant breaks no caller.
