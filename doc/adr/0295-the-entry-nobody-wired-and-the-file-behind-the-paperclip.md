# 0295 — The entry nobody wired, and the file behind the paperclip

**Status.** Accepted.
**Context.** The four-hundred-and-fifty-ninth session found a `shall` this program failed in
silence — §12.5.6.4's `/Open` — and named the shape that hid it: a ledger row that retires its
refusal by naming a capability that arrived. Such a row names no blocker, no missing vocabulary
and no absent architecture, so every one of `doc/todo/01`'s fourteen sweeps passes it. That round
left the shape in `doc/habits.md` **with no instrument**, which is the thing this project's ledger
discipline cannot afford: a sweep that is remembered rather than runnable is found again by
accident.

## The fifteenth sweep

It is described in full in [`doc/todo/01`](../todo/01-ledger-partial-rows.md); the design decision
worth an ADR is what it *refuses* to read.

**It reads no reason.** The fourteen before it all take a row's stated justification — a blocker,
a capability, a retired string, a citation, a table number, an arithmetic relation — and ask
whether it still holds. The sixth shape has no wrong justification to find: it names something
that genuinely exists. So this one takes the **entries the clause's own tables state** — every
`Table N` heading printed inside the clause's span of `doc/md/ISO_32000-2_sponsored_EC3.md`, whose
first column is `Key` — and asks who reads each.

**And it asks twice, which is the whole of it.** Question one — does any `.rs` file under
`crates/`, `tools/` or `fuzz/` name the entry — is what a person would write, and **it passes the
row this sweep exists for**: `"Open"` was named in `crates/`, by `popup::read`, under Table 186.
Question two is whether any file the row *itself* names in `code = [...]` names the entry. A
row's `code` array is its own claim about where the clause is implemented, so an entry stated by
the clause and absent from every file the row cites is either a defect or a refusal the row should
say out loud.

First run: **168 rows in the population, 30 stating an entry their own code does not name, 57
entries** — 38 named nowhere at all, 19 only elsewhere. Most are refusals `CLAUDE.md` already
closes (excluded actions, XFA's `/DS`, ECMAScript, a signer's seed values). One was work.

## §12.5.6.15's `/FS`, required and read by nothing

The row is `implemented`. Its note is an arrival — "**all four are drawn since the
two-hundred-and-sixty-sixth session**", about the icons — and it disposed of Table 187's
**required** `/FS` in eight words: *the embedded file, which is not a rendering question*. True
about rendering, and the reason nobody asked the other question.

> A file attachment annotation ( PDF 1.3 ) contains a reference to a file, which typically shall
> be embedded in the PDF file (see 7.11.4, "Embedded file streams").

and the clause says what activating one does:

> A table of data can use a file attachment annotation to link to a spreadsheet file based on that
> data; activating the annotation extracts the embedded file and gives the user an opportunity to
> view it or store it in the file system.

§12.5.1 says the same sentence at family scale — an activated annotation "exhibits its associated
object, such as by opening a popup window displaying a text note" — and this program has exhibited
§12.5.6.14's window since the three-hundred-and-twelfth session (ADR 0191) while the *file* stayed
unreachable.

**Unreachable is exact, and it is §7.11.4.1's two homes.** An embedded file may live in the
catalog's `/EmbeddedFiles` name tree or in any file specification's own `/EF`. This program walked
the first and never the second, so a document using only the second showed a paperclip with
nothing behind it: `Query::Attachments` listed no file, and `Command::Extract` had no name to be
given. `grep '"FS"'` over `crates/` found two readers and neither was this clause's —
`file_spec.rs` reads Table 43's *file system* entry of the same name, and `action.rs` follows
§12.6.4.4's target path into an attached document. **`/Open`'s shape one clause along**: an entry
read for somebody else's table.

### What the corpus says, counted before it was believed

`crates/pdf-model/examples/file_attachment_census.rs`, over the 964 openable documents of the 974:

```text
  34 835 annotations, 1 of subtype /FileAttachment
       1 states an /FS that resolves to a dictionary, 1 of which embeds a file
       0 of those embedded files are also named by /Names /EmbeddedFiles (23 files there in all)
       1 states /Contents, 0 of those beside a /Desc
```

and over `doc/ISO_32000-2_sponsored_EC3.pdf`, which is the finding that settles it: **six file
attachment annotations, six embedded files, and no name dictionary at all**. The standard's own
PDF carries six files this program could not hand a person.

## The clause's one `shall`, and why the corpus cannot rank it

> The Contents entry of the annotation dictionary may specify descriptive text relating to the
> attached file. Interactive PDF processors shall use this entry rather than the optional Desc
> entry ( PDF 1.6 ) in the file specification dictionary (see "Table 43 -Entries in a file
> specification dictionary") identified by the annotation's FS entry.

`attachment::of_annotation` puts Table 172's `/Contents` in `Attachment::description` where the
annotation states one. Where it states none there is no "this entry" to use instead, and
§7.11.4.1's `/Desc` keeps its own clause's meaning — "a textual description of the embedded file,
which can be displayed in the user interface". All seven of the corpus's and the standard's file
attachment annotations state a `/Contents` beside no `/Desc`, so **a reader preferring the wrong
entry is indistinguishable on every file this project has** (trap 8), and the fixture is a pair of
documents differing only in `/Contents`. Each half was watched fail with the rule removed.

## Where the file crosses, and the measurement that decided it

`viewer_core::exhibit` is §12.5.1's sentence in one function: a click that presses and releases on
one annotation toggles §12.5.6.14's window **and** hands over §12.5.6.15's file as
`Event::Extracted` — the same event `Command::Extract` produces, so every host already writes it
somewhere and **no message was added to the boundary**. `hand_over` is the shared half: the
decryption refusal, Table 45's `/CheckSum` and the bytes, said once for both callers.

**The obvious design was a document-wide list, and it is refused with a number.** Adding
§12.5.6.15's files to `Query::Attachments` costs a walk of every page's `/Annots` — measured at
**78 to 123 ms cold over three runs and 13 to 15 ms warm** on ISO 32000-2's 1023 pages and 11 462
annotations, by the census example, after the walk was already taken from 870 ms by asking
`Pages::indices` for one tree traversal instead of `Pages::get` per page. And `viewer-ui`,
`viewer-gtk` and `viewer-qt` all ask `Query::Attachments` on `Event::Opened`, before the first
frame — so the list would land squarely on the launch path, where `CLAUDE.md` says "no full
page-tree walk" without qualification. A click has already found its page and costs one
dictionary lookup.

**What that leaves owed** is a panel that lists such files, which needs the *hosts* to ask when
the attachments tab is first shown rather than when the document opens — a change in three hosts
and in what `viewer-ui`'s open-time summary line can count. It is written into
`Query::Attachments`'s own doc comment so that the next round finds the reason beside the code.

## Consequences

- Table 187's `/FS` has a reader for its own clause; `action.rs`'s duplicate read is now the same
  function, so the entry has one reader and two callers.
- A person clicking a paperclip gets the file. Verified in the real window under `Xvfb`, which is
  the only instrument that sees a host: `Test attachment`, 15 bytes, written beside the document,
  with the checksum note in front of it.
- **A real file's `/CheckSum` is not sixteen bytes.** `annotation-fileattachment.pdf` writes the
  MD5 digest as a UTF-16BE text string with a byte order mark, which `Attachment::checksum_matches`
  answers `Some(false)` for by construction — a checksum stated wrongly is not a checksum absent.
  Until now that arm had only fixtures; the corpus reaches it through this click.
- No pixel moves: nothing here draws. Both corpus gates, the oracle, the text gates and the quorra
  gate are unchanged, which is what a change that only adds a verb should do.
