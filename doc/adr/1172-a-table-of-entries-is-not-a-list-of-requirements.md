# 1172 — A table of entries is not a list of requirements

Status: accepted. Session 1167.
Context: `doc/conformance/ledger.toml` (§7.7.3.3, §7.7), `crates/pdf-model/src/page.rs`
(`PAGE_ONLY_ENTRIES`, `declares_a_node`, `Pages::detached`),
`crates/pdf-model/src/content/colour.rs` (`output_intent_space`),
`crates/pdf-model/src/attachment.rs` (`associated`), `crates/pdf-model/src/document_part.rs`.
Builds: ADRs 0786, 0305, 1035 section 4, 0101.
Clauses: ISO 32000-2 §7.7.3.3 (Table 31), §12.7.7, §14.3.2, §14.5, §14.10.6, §14.11.2.2,
§14.11.4, §14.11.5, §14.12.3, §14.13.4, §6.2.

## 1. What the row was counting

§7.7.3.3's row had been `partial` for years on a list of Table 31 entries "whose values nothing
reads": seven of them, plus `/PZ` as declined. The list was maintained honestly — it shrank twice
when a sweep found an entry that had quietly become read — and it was measuring the wrong thing
the whole time. The ledger's word `partial` means *some of the clause's normative requirements are
executed and some are not*. An unread entry is not a requirement. Table 31 is a vocabulary: most
of its rows define a value and then hand the meaning of that value to another clause, and it is
that clause which says what a processor shall do about it.

So the entries were read one at a time against the clause each points at. All ten of them — the
seven, `/PZ`, and two the list had never contained — land on a row that has already disposed of
the question:

| entry | clause Table 31 sends it to | that row |
|---|---|---|
| `/LastModified`, `/PieceInfo` | §14.5, private processor data a NOTE says general-purpose processors can ignore | `inapplicable` |
| `/BoxColorInfo` | §14.11.2.2, a guideline display interactive processors *may* offer | `inapplicable` |
| `/ID`, `/PZ` | §14.10.6, Web Capture object attributes | `inapplicable` |
| `/SeparationInfo` | §14.11.4, whose own first line deprecates it with PDF 2.0 | `inapplicable` |
| `/TemplateInstantiated` | §12.7.7, named pages | `implemented` |
| `/Metadata` | §14.3.2, metadata streams | `implemented` |
| `/AF` | §14.13.4, associated files linked to a page | `implemented` |
| `/DPart` | §14.12.3, connecting the DPart tree to pages | `writer-side` |

Each of those `shall`s has a subject, and in every case the subject is a processor doing something
this one does not do — reading a producer's private data, drawing a boundary guideline, generating
separations, navigating a Web Capture content set — or a producer writing the file. None of them
is a requirement on a program that draws a page, which is why not one of them belongs to
§7.7.3.3's status.

The general form, and it is ADR 0101's shape with the sign reversed: a list of entries understates
what is read and *overstates* what is owed, and both errors come from counting the wrong
population. ADR 1035 section 4 already says an aggregate row flips with its last child; this says
the same thing one level down, about the rows inside a table.

## 2. A list written from a list cannot gain an entry the standard gained

Three of the ten had never appeared in the note: `/AF`, `/OutputIntents` and `/DPart` — which are
exactly Table 31's three PDF 2.0 additions. The list had been maintained by editing the previous
list, so an entry the table gained could not enter it. `/OutputIntents` turned out to be read
already (`content::colour::output_intent_space` asks the page's array before the catalog's, which
is §14.11.5's own order), and the other two are disposed of above. The rule this leaves: **a list
of a table's entries is rewritten from the table, never from the list**, and the cheap instrument
is the table's own key column.

## 3. `/PZ` is a permission declined, and that is not a departure

The premise had no ADR. It lived in the ledger note and in a session record, which meant that a
round asked to re-derive it had to re-read §14.10.6 to find out whether it still held —
`tools/state.sh departures` cannot show it, because a `departed` row is a *requirement* decided
against and this is not one. Re-derived, it holds, and it is now here in one hop.

§14.10.6 says Web Capture "frequently scales the contents down to fit on fixed-sized pages" and
that the entry "specifies a magnification factor by which the page may be scaled" back to its
original size. *May*: a permission, granted for one producer's scaling habit, in a clause whose
own row is `inapplicable`. Declining a permission owes nothing at all, so the status does not turn
on it and the row is not `departed`. What makes declining the better answer rather than merely a
permitted one is that a magnification is already decided twice — by `Zoom::FitPage` and by
§12.3.2.1's `/OpenAction` view — and the standard states no precedence among the three, so
honouring `/PZ` would mean inventing a ranking. 18 of the 974 corpus documents state one, on 33
pages.

## 4. The one requirement in §7.7.3.3 that binds nobody here

"A page tree shall not contain multiple indirect references to the same page object" is the
clause's last sentence, and nothing in this tree had ever cited it. It is a rule about a *tree*,
so the only party that can meet it is whoever builds one — the same shape as §6.2's requirement on
a conforming file, and the same conclusion. The three places that build a `/Kids` array here
cannot break it: `pdf_transform::merge` writes the array from the number each page was *placed*
at, `pdf_transform::update`'s insertion copies an incoming page into this document's numbering as
a new object, and its deletion only removes an entry.

A file that breaks it anyway is drawn as its tree states it, page by page, and that is a decision
rather than an oversight. The clause states no reader behaviour, dropping the second occurrence
would lose a page some producer meant to repeat, and noticing the duplicate at all means walking
the page tree to its leaves — which is the one thing `CLAUDE.md` section 2 keeps off the open
path. The bound that already exists (`MAX_NODES_VISITED`) is what stops a tree that names a node
in a cycle, which is the case that would otherwise not terminate.

## 5. What this leaves for somebody else

§14.13.4 is `implemented` and its subject is a page's associated files.
`pdf_model::attachment::associated` reads any dictionary's `/AF` and is general enough to answer
for a page, and its production callers pass the catalog, a `/DPart` node and an annotation —
never a page dictionary. That is that row's question, not §7.7.3.3's, and it is recorded here
because Table 31 is where a reader of this clause would go looking for it.
