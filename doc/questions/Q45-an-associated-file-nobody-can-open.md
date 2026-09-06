# Q45 — Where does a file the document associates but does not carry belong?

Asked by session 939, which built the reader for it (ADR 0918). **Provisional, not a blocker**: the
form is read and reported today, and what is open is where the reader is shown it.

## The question

§14.13.2 gives an associated file two forms — "[t]he file specification for an associated file
represents either a file external to the PDF file or an embedded file stream … within the PDF
file" — and this program read only the second until this session. It now reads both: the external
one has no bytes here, and what it carries is the file's own name and the `/AFRelationship` the
producer asserted (`Source`, `Data`, `Supplement`, …).

**Where should that reach a person?** Two shapes, and this session took the smaller one:

1. **A note when the document opens** (what is built): one sentence beside §7.11.4's embedded-file
   sentences, saying the document associates a file that is not inside it, naming it and its
   relationship. It is a claim about the *file*, which is what that module is for.
2. **A row in the attachments panel**, greyed, with no size and no save action — so that the two
   forms sit in one list and a person sees at a glance that the chart's source data was never
   embedded.

The second is the more useful and the more dangerous: an entry a person can click and not get
anything is a worse experience than one they were told about once, and the panel's whole vocabulary
(size, media type, checksum, *save*) is about bytes this program has.

## Why it cannot be settled without the owner

- It is a **product** decision about a list a person uses, not a reading of the clause: the standard
  says the form exists and says nothing about how a reader presents it. Principle 5 gives no answer
  here, and `CLAUDE.md`'s "documented choice" rule is exactly the case where the owner's ranking is
  the input.
- The panel is under review as mockups rather than as code, by the owner's own word recorded in
  `doc/state-of-play.md` about the attach gesture, so adding a row to it now would be a round
  deciding a design that is deliberately being decided elsewhere.
- **A24 has just moved the boundary this touches.** The renderer may be *given* things by its
  broker. Nobody has proposed a port for a document's own external references and this session does
  not; but whether such a file should look, in the interface, like something that could one day be
  fetched is the owner's call rather than a consequence of the answer to A24.

## What the tree does meanwhile

Shape 1, in full: `attachment::external_associated` reads the form for any `/AF` carrier,
`viewer_core::notes::about` says it once when the document opens for the catalog's own array, and
the bytes stay refused. Neither corpus has a witness — 0 external specifications over the 974 and 0
over `CC-MAIN-2021-31`'s 65 944 — so nothing a person opens today reaches it, and the tests are
built.

## Recommendation

**Keep the note; add the panel row when the attachment flows are designed, not before.** If the
owner wants it in the list, the honest shape is a row that states plainly that the file is outside
the document, carries no save action at all, and shows the relationship in place of the size — and
that is a decision to take once for the whole panel rather than for this one case.
