# 1932 — An XFA form is said at the report and a print through an output intent before its grant

Session 1548, slot 3 of batch eighty-seven. Status: **accepted, built**. Context: RFC 0009 §2.6, §4.4,
§11 questions 5 and 9 and §12 item 1; `doc/questions/A254`, `A33`; ISO 32000-2 §K.1, §K.2, Table 29,
Table 224, §14.11.5 and Table 401; ADRs 0272, 0821, 1044, 1561, 1622. It moves no status. It re-reads
§13.4, §14.11.5, §K.1 and §K.2.

## 1. The poster was already drawn

RFC 0009 §12 item 1 names §13.4's poster as owed since A33, and so did this round's brief. The RFC
was right when it was written: its §3.3 read the row as `reported` on 2026-10-05. ADR 1561 drew Table
306's stream-form `/Poster` the next day and moved the row to `departed`. A254, given on 2026-10-08,
carried the RFC's list over unchanged.
The poster belongs to a `Movie` annotation's movie dictionary. Neither a rendition nor a screen
annotation has one; Table 306 is the only place the word appears. What the RFC's item 1 still owes
is the two sentences below. The §13.4 row now says what A254 changed: the departure stands until the
clause 13 entry is replaced, and that replacement is RFC 0009 §12 item 3.

## 2. The XFA sentence: at the report, from the annex's two entries

`pdf_model::xfa::stated` reads Table 224's `/XFA`, which counts only in one of §K.2's two forms,
a stream or an array. It also reads Table 29's `/NeedsRendering` when it is stated `true`. It decodes
nothing. `notes::about` turns those into one sentence of three kinds: a hybrid form, whose
`AcroForm` is shown; a form that asks to be regenerated, whose pages are the ones the file holds;
and the flag with no resource. Every sentence says this program does not implement XFA, which
§K.1 permits.

It goes in the report and not in the open, because `Command::Report` is where every window already
asks what a document says about itself, once, after its first frame (ADR 1044). So the launch path
pays nothing for it, and all four windows say it without a host edit.

## 3. The print-time sentence: the interpreter's selection, on every intent in force

**One selection.** The output intent a print names is the one the page was drawn through. So
§14.11.5's choice has moved out of `content/colour.rs` into `pdf_model::output_intent::selected`.
The page's array comes first, then the first intent with a readable profile. `output_intent_space`
now calls it. That is a move: the same tests pass and no pixel moves.

**Every intent in force, including PDF/A's sRGB.** Table 401 says "[t]he output transformation uses
the profile's "from CIE" information (BToA in ICC terminology)". This program never performs that,
whichever subtype the intent has. One could argue the sentence is noise for a `GTS_PDFA1` sRGB
intent, but that argument rests on NOTE 2's "assumed to imply semantics", which is a NOTE and
ISO 19005's rule, not this clause's. The condition stays the clause's (trap 11). The sentence is a
print-time line, not a page report, so it costs no gated page. It fires on 16 of the 970 corpus
documents that open.

**Where it is said.** In `begin_printing`, as an `Event::Reported` with no page, before
`Event::Printing`. So it comes once per job, ahead of any dialogue, and no host needs an edit.

## 4. Not built

Two open-time reports on the same row. A `/DestOutputProfileRef` with no readable profile is still
silent. The row states its condition, and no corpus document reaches it. A `/OutputCondition`
panel would need a host surface, which belongs to the windows.
