# 1292 — An FDF submission is sent as `application/vnd.fdf`, because §12.7.8.1 says so

Session 1227. Status: **accepted**.
Context: `crates/pdf-model/src/submission.rs` (`Format::content_type`),
`crates/pdf-model/tests/submission.rs`, `crates/viewer-host/src/submit.rs` (`FDF_TYPES`).
Clauses: ISO 32000-2 §12.7.8.1, §12.7.6.2.

## The finding

`Format::content_type` answered `application/fdf` for an FDF body, and its comment said "ISO
32000-2 states none for FDF, so the third comes from the registry". That is false. §12.7.8.1:

> FDF shall use the MIME media type application/vnd.fdf.

The tree took IANA's 2022 registration of `application/fdf` by ISO TC 171/SC 2 as the source and
called the standard's own name "a vendor-tree name". `CLAUDE.md` principle 5 decides it the other
way: the specification is the source of truth, and a registry is evidence beside it. It went unseen
because nothing sent the body anywhere — a media type nobody transmits is a string nobody reads —
and it surfaced the round the client was built, which is when a wrong one would first have reached
a server.

## The decision

Sent: `application/vnd.fdf`, from `Format::content_type`, and the test that pinned the registry's
name now pins the clause's under the clause's sentence. Received: both, `FDF_TYPES`, because a
server naming FDF by the registry's name has named FDF, and refusing its answer would be refusing
the right bytes for their label — a reading choice, written down, not a reading of the clause.

XFDF is unchanged: ISO 32000-2 states no media type for it, so `application/xfdf` stays the
registry's, as its comment already says.

## Not to re-open

The comment claiming the standard is silent was a claim about the specification, and `CLAUDE.md`
says those decay: before recording a silence, read the titles around the subject. §12.7.8.1's
sentence is four lines under its heading.
