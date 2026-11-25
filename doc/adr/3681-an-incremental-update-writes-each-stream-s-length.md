# 3681 — An incremental update writes each stream's `/Length`, and does not count the rewrite

Session 2245. Status: accepted. Context: review 2243 (B01), ISO 32000-2 §7.3.8.1, §7.3.8.2 and
Table 5, §7.5.6; `serialize`'s `Written::relengthed`.

## Decision

`write::incremental_update_extending` (and so `incremental_update` and `incremental_update_freeing`)
writes every stream's `/Length` as the number of bytes it writes between `stream` and `endstream`,
after §7.6.2 has encrypted the object for an encrypted document. A `/Length` that disagreed, or that
was an indirect reference, is replaced by that integer.

§7.3.8.2's entry "indicates how many bytes of the PDF file are used for the stream's data": it is a
statement about the bytes the writer puts down, which only the writer sees. So the key is the
writer's, as `/Size`, `/Prev` and a cross-reference stream's `/Length` already were in the same
function, and it is overwritten rather than obeyed.

## Why it is not counted

`serialize` counts its rewrites (`Written::relengthed`) because it rewrites a whole document and its
caller reports on the source. Here the disagreements arrive by three routes, and none needs a
count from the writer:

- a stream the reader found by `endstream` past a short `/Length` — already logged when read, as
  `STREAM_EXTENT_FROM_ENDSTREAM`;
- a stream carried out of an encrypted document (`pdf-transform`'s insert-pages), whose `/Length`
  counted ciphertext the reader has removed — correct input, not a defect;
- an indirect `/Length` whose object the update does not rewrite.

Counting would mean a new return type for a function seven callers use, one of them in another
lane's file, to report something that is either already reported or not wrong. If a caller ever
needs the number, a `_tallied` variant returning it beside the bytes is the change, not a log.
