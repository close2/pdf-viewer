Status: complete
Given: 2026-09-14, in conversation — transcribed by the round
Owes: redaction application — the annotation read, the region's content removed, the result written by the serializer; the row's reason rewritten; the removal semantics documented as a choice (acting round)

> Owed.

Reading: applying a redaction is in scope as a debt this project will build — the exclusion's
stated reason ("an incremental update cannot express a removal") died when RFC 0002's serializer
was written, and an exclusion resting on a false reason is the state `CLAUDE.md`'s own rule about
its restrictions exists to end. Three things come with the answer:

- **The overlay text and the fill are excluded by name**, as the marks this program does not
  compose — the fence is `A65`'s provenance line, and those marks are partly this program's
  invention. `/OverlayText` is optional, and a redaction applied without them is still a
  redaction; a document whose redaction specifies them gets the removal and a reported
  departure from §12.5.6.23's full application semantics.
- **Application is a write-new-file operation, never an incremental update** — §7.5.6 appends,
  and a removal is not an append. In the viewer this is the one edit that produces a new file
  rather than an in-place update, which the surface says plainly; batch application through the
  transform suite has no such wrinkle.
- **Where the standard under-specifies which content is "within the region", the choice is
  documented as a choice**, per principle 5's honest-limit rule — not curve-fitted to what
  another reader removes.