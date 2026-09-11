# Q55 — Is `derive` acceptable at all in an archival converter?

Source: `doc/rfc/0007` §7 question 2, raised by session 954.
Status: **open** — answered when `A55-is-deriving-content-acceptable-in-an-archival-converter.md`
exists beside this file.

## Why it needs the owner

The owner's own example is a movie that becomes screenshots, or an audio track that becomes a
transcript. Both are *useful*: without them the alternative is losing the annotation entirely.

But a `derive` remedy makes something the document did not contain, and this project's line —
`A48`, and `doc/adr/0927` — is **state an interpretation the standard defines; never fill in an
absence**. A transcript is not the audio. A PDF/A file whose movie has become three screenshots is
a *different document* from the one that went in.

The question is not whether it is useful. It is whether **this program offers that mode at all**,
given that its output is supposed to be an archive of what somebody had.

## What the tree does meanwhile

No remedy of any kind exists; multimedia and 3D annotations are `doc/pdf-a-conversion-limits.md`
§3.2's authorised loss or a refusal.

## Recommendation

Offer it, and make it impossible to get by accident: never a default, never reachable without the
configuration naming the site *and* the tool, reported per document in the words "this is derived,
not original", and recorded in the file's own `xmpMM:History`. The alternative — refusing to offer
it — does not keep archives faithful; it just means the annotation is dropped instead, which loses
more and says less.
