# 1954 — The window converts to PDF/A under a profile a person names

Session 1559, on the project owner's words of 2026-10-15 ("[t]he pdf-a converter should be able to
convert *all* pdfs which are viewable") and `doc/todo/72` section 1. Status: **accepted** and
**built**. Context: RFC 0007 and `doc/questions/A53`–`A60` (apply returns a request; one shared
executor), RFC 0004 (the print command's shape), ADRs 1144, 1180, 1942.

## 1. What was missing

`quorra-transform archive --profile as-if-printed` converted Pol.pdf (ADR 1942); no window did.
`grep -n Archive crates/viewer-core/src/command.rs` found nothing, and the shipped profiles were a
constant in the command-line binary, so a second face could only have copied them.

## 2. Decisions

1. **`Command::Archive(Archiving { target, profile })` asks Table 22's bit 11**, "[a]ssemble the
   document (insert, rotate, or delete pages …)", at the reader's level — the operation
   `pdf_transform::Plan::operation` already gives the `archive` verb. All four levels behave as
   they do for print: `On` refuses by name, `Ask` holds the request until `Command::Answer`, `Warn`
   grants and then says what the document asserted (`crates/viewer-core/tests/archiving.rs`).
2. **The grant carries the file and the host runs the verb.** `pdf-transform` depends on
   `viewer-core`, so the core cannot convert; `Event::Archiving { document, bytes, asked }` carries
   the `FileBytes` the document was opened from (a handle, no copy). The bytes are the file as
   opened, not the window's unsaved edits: what a producer wrote is converted, and an edit a person
   wants archived is saved first. The host converts with the restriction level off, because the
   reader's level was asked once already.
3. **One runner for every host**, `viewer_host::archiving`: the profile is looked up in
   `pdf_transform::profiles::SHIPPED` (moved there from the binary, which now reads it too), folded
   into a plan by `profiles::Contribution` (also what the binary's `--config`/`--profile` reading
   uses), and a pass's tool requests run through `pdf_transform::executor::execute` — A54's one
   executor — before the next pass. A window supplies no font, no output intent profile and no
   external stream data, and never departs: each of those is an operator's statement at a call
   site, and a profile naming a departure is refused with the command that can make it.
4. **The conversion runs on an `archive` thread** that reports on a channel, collected on the timer
   the windows already run for a server's answer (`Converter`, `Submitter`'s shape): it validates
   the whole document twice, and `doc/todo/72` records a file on which validation has not ended.
   `quorra` is woken by its loop's proxy; GTK and Qt poll at `submit::LOOK`.
5. **The choice is `--archive <target>:<profile>`**, one of `ReaderWords`, and **Shift+A** asks for
   it in all three windows (`A` unshifted still selects everything; Shift is the one modifier the
   key table reads, as for print). There is no default target, for the command line's reason;
   pressed with nothing chosen, the key says how to choose and names the profiles.
6. **The file is written beside the document** as `<name>.pdfa-2b.pdf` (the target in the name),
   `.edited.pdf`'s rule. The report's lines are said in order — the profile once, each site, then
   what the run declined — and the conversion report's JSON gains a `profile` field. The plan does
   not carry the name, so the face that read the configuration sets `Conversion::configuration`;
   the 15 sites that build an `ArchivePlan` are untouched.
7. **The confined window converts nothing.** Its process parses no document and a conversion parses
   the whole of one; `Command::Archive` and its grant are `Uncarried` on the wire, and Shift+A
   there says so.
8. **The C library is the host for a C caller**: `quorra_archive` sends the command, any call that
   releases the grant runs the conversion on the calling thread, and `quorra_archived`,
   `_copy`, `_line` and `_json` read it back; `quorra_archive_profiles`/`_profile` list the
   profiles. Seven entry points, `QUORRA_EVENT_ARCHIVING` 29, `QUORRA_EVENT_KIND_COUNT` 30.

## 3. Consequences

A document opened in `quorra`, `quorra-gtk` or `quorra-qt` started with `--archive 2b:as-if-printed`
converts on Shift+A and the window says the report (`tools/drive-windows.sh` step 88). Owed: a menu
or card that chooses the target and the profile after launch; an encrypted document opened with a
password converts only where its user password is empty, because the core keeps no password.
