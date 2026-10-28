# Q706 — May a profile answer a site with an ordered list, and state a loss it takes on a page appended to the archive?

Asked by round 1840, the design slot of batch 125 (RFC 0014, ADR 2514), under your words of
2026-10-27: more profiles, `no-data-loss` among them; every remedy choosable; several remedies per
problem; external programs where no remedy of this tree reaches; and "a profile should be able to
for instance say that it accepts the loss, but wants a page appended stating what had to be
removed." The table those words asked for is `doc/pdfa-remedies.md`; RFC 0014 is the format's next
step, and its section 9 is these five items. Nothing below is built.

**1. The ordered list (your answer A57, reopened on its own condition).** A57 gave `on-failure` one
alternative and said a chain comes only when a real configuration wants two steps. Three do now:
`keep-everything.toml`'s `prefer` at every attachment site (marked `NOT-YET-IN-FORMAT`), the 3D
site's three steps, and your sentence above, which is two steps — take the loss, then append the
statement. RFC 0014 section 3 proposes `try = ["preserve:attach", "derive:office-to-pdf:attach",
"append-statement"]`: evaluated per object, the first step the target admits and that succeeds is
taken, a list that runs out stops, and every file written against A57 reads unchanged.

*Recommendation*: **yes**, as section 3 writes it.

**2. The statement page.** `append-statement` takes the site's loss and appends one page listing,
per object, what was removed, in the file's own facts — the page it sat on, an attachment's name,
its length, its media type where the file states one, its SHA-256 — with the site's identifier, the
ISO 19005 section and the loss word. `CLAUDE.md`'s first amendment admits a page "composed solely
of content the document already holds", and ADR 1014 adds "no sentence added that the source did
not contain". The statement page's facts are computed from the source, as ADR 1014's signature page
is, but its labels and loss descriptions are a closed vocabulary fixed in this program. Does that
fit inside the first amendment, or does it need its own?

*Recommendation*: **inside it**, with one sentence added to the first amendment, for you to place in
`CLAUDE.md` if you agree: *"A page may also state what an archival conversion removed, in facts
computed from the file and a fixed vocabulary naming the requirement and the loss; it carries no
other text."*

**3. Information and presentation.** `no-data-loss` is defined by sorting every loss word into
**information** (something a reader could want back — metadata, an attachment, an annotation's
content, a signature's assertion, readable text turned to pixels) and **presentation or behaviour**
(a hint, a press statement, a behaviour, a choice this reader made). The sort is the loss index of
`doc/pdfa-remedies.md` section 5.

*Recommendation*: **adopt it as written**, each word's class open to your correction by name.

**4. The two profiles.** `no-data-loss`: an information loss is kept, moved, derived or written out
on an appended page, and otherwise the conversion stops; a presentation loss is discarded and
stated on the appended page as well as in the report — your "document metadata losses" taken to
every loss. `state-every-loss`: convert whatever the window shows, keeping what can be kept and
stating every loss on the appended page; it stops only at the blocking list's exceptions (a dynamic
XFA form, a document with no page drawn, the unreadable). Ship both, under these names?

*Recommendation*: **both**, with `state-every-loss` the profile a company is pointed at first.

**5. The converted attachment and your own profile.** RFC 0014 section 5: a declared program makes
a PDF of what this tree cannot read, this converter makes that PDF a part the target admits, and
the result is attached or appended — and ISO 19005-2 section 6.8 admits an attachment conforming to
part 1 or 2, which corrects RFC 0007 section 4.6's reading that a derived file is no remedy at
PDF/A-2. Section 6: a `--config <file>` checked at all six targets before a document is read, and
`base = "<shipped profile>"` so a file states only what it changes.

*Recommendation*: **yes to both**, built after items 1 and 2.

**Meanwhile**: every site keeps its one answer and its one alternative (A57); no page states a loss;
the seven shipped profiles stay as they are; batch 125's archive slots build ADR 2490's rows as
their briefs say; and batch 126 builds the two reasons you named first — an attachment no reader
here draws and multimedia — as `doc/pdfa-remedies.md` section 4 lists their steps, within whatever
your answers here allow.
