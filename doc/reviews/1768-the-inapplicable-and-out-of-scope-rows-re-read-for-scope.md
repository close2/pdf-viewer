# 1768 — The `inapplicable` and `out-of-scope` rows, re-read for scope

Status: **review**, a record that is never rewritten. Session 1768, a clause round run beside batch
one hundred and sixteen's rounds on the branch `batch-1760-1767`. It answers the owner's request of
2026-10-26: "please use one round to evaluate if we should / could ledger rules from inapplicable and
out-of-scope into scope. Do the reasons still hold? (I intentionally started with a narrow scope
target and have continuously expanded it; it might be time to expand the scope once more). What is
blocking us, which ones sound like a good idea,..." ADR 2370 states the rule the same message set
for every batch. **It builds nothing and moves no status**: finding 1 says why the one move the
brief allowed fitted no row, and section 4 is the list the next batches' ledger slots take from.

**The population, by command.** `tools/state.sh ledger` against `ecb6bea0` with batch 116's
uncommitted edits: 883 rows, of which `inapplicable` 34 and `out-of-scope` 84; "34 quote their
condition from the clause" and "84 of 84 name their exclusion". The rows are
`grep -B3 -A4 -E 'status = "(inapplicable|out-of-scope)"' doc/conformance/ledger.toml`. Each was
read with its note and its neighbours' statuses. The clause's own text in
`doc/md/ISO_32000-2_sponsored_EC3.md` was opened where the reason was in question (§13.3, §13.4,
§13.5, §14.8.5.4.4, §14.11.2.2); for the rest, the opening sentence each note already quotes, which
the quotation gate holds verbatim, stood for it. The **classes** in section 2
are this review's judgement, not a count of the tree: **A** the standard's own condition, 33; **B**
`CLAUDE.md`'s closed exclusion, 56; **C** a reason about this program that has decayed, 28; **D** a
cost or scope judgement the owner can revisit, 1; **E** unstated, 0.

**What it builds on.** RFC 0009 (2026-10-05) asked the same question of the exclusions themselves,
and `doc/questions/A254` adopted its eleventh section whole on 2026-10-08: clause 13 amended to
exclude the player and admit the page, a `Media` level, sound playing ("Tier B") on demand, §10.6
`inapplicable` on a print path with its flipping route named in the row, XFA "rejected for now, but
we will implement it later". This review asks what has happened to those rows since, and reads the
34 `inapplicable` rows RFC 0009's section 7 read in passing.

## 1. Measured

### 1.1 The `inapplicable` sweep: `cargo run -q -p conformance --bin inapplicable`

34 rows stating 272 terms, 26 named by no source, 246 named. The rare terms (fewer than twenty
naming files) were each opened: `Halftoning` is `pdf-transform`'s `archive/halftones.rs`, which
removes what ISO 19005-2 section 6.2.5 forbids and cites §10.6.1 for why no screen moves (the
§10.6.5 rows, `implemented`); `StartIndent`, `EndIndent` and `SPS` are test fixtures; `PZ`,
`BoxColorInfo` and `SeparationInfo` are `page.rs`'s `PAGE_ONLY_ENTRIES`, a page-versus-node
discriminator and not the clauses' requirements; `DPM` is `document_part.rs` saying it is still
`inapplicable`; `ILSEs` is `viewer-core`'s `accessibility.rs`, which reads §14.8.3.3 and §14.8.5.4.5
— the cousin finding 7 is about. **The sweep reads `inapplicable` rows only.** No instrument reads
an `out-of-scope` row's vocabulary against the tree, and finding 2's rows are where that cost.

### 1.2 What `pdf_model::media` cites: `grep -oE "§13\.[0-9.]+[0-9]|Table [0-9]+" crates/pdf-model/src/media.rs | sort | uniq -c`

§13.2.2 11, §13.2.3.3 8, §13.2.7.2.2 7, §13.2.4.2 7, §13.2.4.3 3, §13.2.3.1 2, §13.2.3.2 2,
§13.2.3 2, §13.2.5, §13.2.6, §13.2.7.4 and §13.2.7.4.3 once each; Tables 277, 278, 279, 284 to
291 and 294. Its tests include `a_selector_plays_the_first_viable_rendition_and_says_why_the_others_were_passed`
(§13.2.3.3), `an_option_must_honour_makes_the_rendition_non_viable_and_best_effort_does_not`
(Tables 278 and 279) and `a_clip_with_no_content_type_names_no_player_this_program_could_start`
(§13.2.7.2.2's condition d). Of those clauses only §13.2.2 and §13.2.4.2 are not `out-of-scope`.

### 1.3 The dependencies clause 13's triggers name, on crates.io on 2026-10-26

`curl https://crates.io/api/v1/crates/<name>` for the metadata; `unsafe` counted with
`grep -rw unsafe --include=*.rs` over `https://static.crates.io/crates/<name>/<name>-<version>.crate`
unpacked, which is evidence about the crate and not a judgement of it.

| crate | version, first published | licence against `deny.toml` | lines, `unsafe` | what it is |
|---|---|---|---|---|
| `rusty_h264-decoder` | 0.16.1, 2026-06-27, 18 releases | BSD-2-Clause, admitted | 20 478, `forbid(unsafe_code)` | H.264 Baseline, B-slices and most of High; claims bit-exactness against openh264's conformance set |
| `rust_h264` | 0.4.0, 2026-04-05, 4 releases | MIT OR Apache-2.0, admitted | 20 915, 6 | H.264 decoder |
| `rusty_aac` | 2.0.1, 2026-07-29, 11 releases | Apache-2.0, admitted | 24 423, 41 | AAC LC, Main, HE-AAC v1 and v2 |
| `oxideav-aac` | 0.1.7, 2026-04-17, 17 releases | MIT, admitted | 100 404, 0 (and `oxideav-core`) | AAC-LC |
| `u3d` | 0.1.0, 2026-10-09, 1 release | MIT, admitted | 2 153, 0 | ECMA-363; its README leaves progressive-mesh continuation, fuzzing and `RHAdobeMeshResource` unticked |
| `prc` | 0.2.0, 2026-10-09, 1 release | MIT, admitted | 35 330, 50 | PRC |
| `pdf-xfa` | 1.2.0, 2026-05-02 | AGPL-3.0-only or commercial, **refused** | — | an XFA engine with layout |

RFC 0009 section 3.4 recorded that no pure-Rust H.264 or AAC decoder passed `deny.toml`; four of the
crates above were published before it was written, so **the trigger the owner ratified on 2026-10-08
was met by licence on the day it was ratified**. `u3d` and `prc` were published the day after.
Licence is one of `deny.toml`'s four checks; bans, advisories and sources were not run, because no
manifest here changed. The XFA 3.3 specification still answers 403 at `pdfa.org/norm-refs/XFA-3_3.pdf`;
ECMA-363's fourth edition answers at `ecma-international.org`.

### 1.4 The move the brief allowed, against the status vocabulary

The brief allowed a decayed row to go to `partial` or `unreviewed`. Every decayed row is cited in
code — `grep -rlE '§<clause>' crates` names `page.rs` for §14.11.2.2, `action.rs` and
`viewer-core/tests/headless.rs` for §12.6.4.9 and §12.6.4.10, `appearance.rs` for §12.5.6.17,
`media.rs` for the §13.2 family — so `unreviewed` fails `Problem::CitedButUnreviewed`. And `partial`
on a leaf makes its heading's derived status `partial` (`Ledger::derived_status` in
`tools/conformance/src/ledger.rs`, and `Problem::AggregateStatus`): §12.6.4.10 would move §12.6.4
and §12.6, both `implemented`; §14.11.2.2 would move §14.11.2 and §14.11; the §13.2 family would
move §13.2.3 and §13.2. Each would add aggregate entries to `doc/todo/65` in a ledger six siblings
are editing, to wear a word that is false for the row: nothing of §14.11.2.2 or of a movie's
hand-off is executed, so `partial` claims an execution, and the §13.2 family's in-scope half is
probably executed whole, so `partial` claims a debt.

## 2. The rows

Paraphrases, cited by clause; the ledger's notes hold the quotations. Classes as above.

### inapplicable

| row | recorded reason | class | what blocks | recommendation |
|---|---|---|---|---|
| §10.6.1 General | §10.6.1's condition: halftoning is for a device that needs it; every raster this program transmits, screen or print (RFC 0004 Route B), is continuous-tone | A | the condition flips only if this program itself writes a bilevel or few-level spool (RFC 0009 section 2.4); a threshold screen is a few hundred lines over the function evaluator, and nobody proposes the spool | keep; the note owes A254 question 8's flipping route |
| §10.6.2 Halftone screens | §10.6.1's condition: halftoning is for a device that needs it; every raster this program transmits, screen or print (RFC 0004 Route B), is continuous-tone | A | the condition flips only if this program itself writes a bilevel or few-level spool (RFC 0009 section 2.4); a threshold screen is a few hundred lines over the function evaluator, and nobody proposes the spool | keep; the note owes A254 question 8's flipping route |
| §10.6.3 Spot functions | §10.6.1's condition: halftoning is for a device that needs it; every raster this program transmits, screen or print (RFC 0004 Route B), is continuous-tone | A | the condition flips only if this program itself writes a bilevel or few-level spool (RFC 0009 section 2.4); a threshold screen is a few hundred lines over the function evaluator, and nobody proposes the spool | keep; the note owes A254 question 8's flipping route |
| §10.6.4 Threshold arrays | §10.6.1's condition: halftoning is for a device that needs it; every raster this program transmits, screen or print (RFC 0004 Route B), is continuous-tone | A | the condition flips only if this program itself writes a bilevel or few-level spool (RFC 0009 section 2.4); a threshold screen is a few hundred lines over the function evaluator, and nobody proposes the spool | keep; the note owes A254 question 8's flipping route |
| §14.2 Procedure sets | §14.2's `shall` binds printing to a PostScript device; no output here is one and no writer emits `/ProcSet` | A | a PostScript output, which RFC 0009 question 9 and Route A do not propose | keep |
| §14.8.3.2 Reference area | §14.8.5.4.1 defines layout attributes by the layout process; the clause's `shall` binds whoever lays out or defines the attribute object, and no writer here defines one | A | a reflowing view, which is a layout engine and sits on the authoring line; the row's half-sentence "exports to no format" is stale since `viewer-core`'s accessibility tree reads §14.8.5.4.2, .3 and .5 | keep; reword the stale half when the row is next touched |
| §14.8.5.4.1 General | §14.8.5.4.1 defines layout attributes by the layout process; the clause's `shall` binds whoever lays out or defines the attribute object, and no writer here defines one | A | a reflowing view, which is a layout engine and sits on the authoring line; the row's half-sentence "exports to no format" is stale since `viewer-core`'s accessibility tree reads §14.8.5.4.2, .3 and .5 | keep; reword the stale half when the row is next touched |
| §14.8.5.4.4 Layout Attributes for ILSEs | Table 380's inline attributes parameterise a layout or an export; this program draws the result and "exports to no format" | D | the accessibility tree is an export, and `accesskit` 0.24.1 carries text decoration and sub/superscript; an owner's word on what "as far as accessibility needs it" takes | ask the owner (Q634) |
| §14.8.5.4.6 Figure, Form and Formula attributes | §14.8.5.4.1 defines layout attributes by the layout process; the clause's `shall` binds whoever lays out or defines the attribute object, and no writer here defines one | A | a reflowing view, which is a layout engine and sits on the authoring line; the row's half-sentence "exports to no format" is stale since `viewer-core`'s accessibility tree reads §14.8.5.4.2, .3 and .5 | keep; reword the stale half when the row is next touched |
| §14.8.5.4.7 Column attributes | §14.8.5.4.1 defines layout attributes by the layout process; the clause's `shall` binds whoever lays out or defines the attribute object, and no writer here defines one | A | a reflowing view, which is a layout engine and sits on the authoring line; the row's half-sentence "exports to no format" is stale since `viewer-core`'s accessibility tree reads §14.8.5.4.2, .3 and .5 | keep; reword the stale half when the row is next touched |
| §14.10 Web capture | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.1 General | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.2 Web capture information dictionary | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.3 Content database | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.3.1 General | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.3.2 URL strings | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.3.3 Digital identifiers | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.3.4 Unique name generation | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.4 Content sets | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.4.1 General | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.4.2 Page sets | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.4.3 Image sets | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.5 Source information | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.5.1 General | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.5.2 URL alias dictionaries | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.5.3 Command dictionaries | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.5.4 Command settings | deprecated with PDF 2.0 on §14.10.1's first sentence, §3.15's definition; a capturing application's bookkeeping, no pixel; `split` and `merge` leave it behind, `optimize` carries it as written | A | nothing a reader could execute; §3.15 says ignore it | keep |
| §14.10.6 Object attributes related to web capture | deprecated with §14.10 (§3.15: ignored by a processor); `/PZ` is a permission declined (ADR 1172), `/PtData` is §12.10's | A | nothing; following §3.15's recommendation is what the row does | keep |
| §14.11.2.2 Display of page boundaries | §14.11.2.2's guidelines are a permission, declined until a viewer grows a prepress view | C | A254 (2026-10-08) adopted RFC 0009 section 12 item 2, which builds the guidelines in the print panel; nothing draws them yet, so no status but `inapplicable` or `implemented` is true | move by building: one round, straight to `implemented` |
| §14.11.4 Separation dictionaries | deprecated with PDF 2.0 (§3.15: ignored); a press's or a preseparating workflow's, and the archive path's `/OPI` removal is ISO 19005-2's | A | nothing; the deprecation is the standard's | keep |
| §14.11.6.1 General | deprecated with PDF 2.0 (§3.15: ignored); a press's or a preseparating workflow's, and the archive path's `/OPI` removal is ISO 19005-2's | A | nothing; the deprecation is the standard's | keep |
| §14.11.6.3 Trap network appearances | deprecated with PDF 2.0 (§3.15: ignored); a press's or a preseparating workflow's, and the archive path's `/OPI` removal is ISO 19005-2's | A | nothing; the deprecation is the standard's | keep |
| §14.11.7 Open prepress interface (OPI) | deprecated with PDF 2.0 (§3.15: ignored); a press's or a preseparating workflow's, and the archive path's `/OPI` removal is ISO 19005-2's | A | nothing; the deprecation is the standard's | keep |
| §14.12.4.2 Document part metadata | §14.12.4.2's audience is a production workflow; `split` and `merge` leave `/DPartRoot` behind and `optimize --linearize` carries it as written | A | a writer that constructs a DPart tree, which nothing proposes | keep |

### out-of-scope

| row | recorded reason | class | what blocks | recommendation |
|---|---|---|---|---|
| §12.5.6.17 Movie annotations | its one processor `shall` is the playing; the poster is §13.4's and drawn | C | the amended clause 13 entry admits the media file handed to this machine's player, and ADR 1961's hand-off exists; Table 306's `/F` is that file | move by building the hand-off (Q631 confirms the reading) |
| §12.6.4.9 Sound actions | deprecated, superseded by §13.2; refused by name; Table 212's entries are playing parameters | C | a §13.3 sound with `/F` is a self-describing file the hand-off admits; embedded samples need a RIFF wrapper or RFC 0009's Tier B | ask the owner (Q631) |
| §12.6.4.10 Movie actions | deprecated, superseded by §13.2; refused by name, the playing is §13.2's | C | as §12.5.6.17: `Event::Media` at `Operation::HandOverMedia` is what §12.6.4.14 already does for a rendition | move by building the hand-off (Q631) |
| §12.6.4.16 Go-To-3D-View actions | the view is §13.6's 3D artwork | B | a U3D reader: `u3d` 0.1.0 (MIT) appeared on 2026-10-09, a partial reader | ask the owner (Q632) |
| §12.6.4.18 Rich-Media-Execute actions | the handler is §13.7's, and Table 223's `/C` is a script call | B | Tier C or D and the script exclusion both; Flash, the usual handler, has no player anywhere | keep |
| §13.1 General | clause 13's general text, the media engine's | B | nothing of its own; it moves with its family | keep |
| §13.2 Multimedia | heading; every subclause rests on the entry except §13.2.2 and §13.2.4.2 | C | its note is stale with the rows below it marked C | re-read with its family |
| §13.2.1 General | clause 13's general text, the media engine's | B | nothing of its own; it moves with its family | keep |
| §13.2.3 Renditions | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` executes this clause's own viability or selection rule for §13.2.2 (ADR 1960), cited by number there; the in-scope half needs a sentence-by-sentence reading, and the honest end is `implemented` as §13.2.2 is | move by reading: one clause round for the family |
| §13.2.3.1 General | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` executes this clause's own viability or selection rule for §13.2.2 (ADR 1960), cited by number there; the in-scope half needs a sentence-by-sentence reading, and the honest end is `implemented` as §13.2.2 is | move by reading: one clause round for the family |
| §13.2.3.2 Media renditions | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` executes this clause's own viability or selection rule for §13.2.2 (ADR 1960), cited by number there; the in-scope half needs a sentence-by-sentence reading, and the honest end is `implemented` as §13.2.2 is | move by reading: one clause round for the family |
| §13.2.3.3 Selector renditions | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` executes this clause's own viability or selection rule for §13.2.2 (ADR 1960), cited by number there; the in-scope half needs a sentence-by-sentence reading, and the honest end is `implemented` as §13.2.2 is | move by reading: one clause round for the family |
| §13.2.4 Media clip objects | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` executes this clause's own viability or selection rule for §13.2.2 (ADR 1960), cited by number there; the in-scope half needs a sentence-by-sentence reading, and the honest end is `implemented` as §13.2.2 is | move by reading: one clause round for the family |
| §13.2.4.1 General | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` executes this clause's own viability or selection rule for §13.2.2 (ADR 1960), cited by number there; the in-scope half needs a sentence-by-sentence reading, and the honest end is `implemented` as §13.2.2 is | move by reading: one clause round for the family |
| §13.2.4.3 Media clip section | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` executes this clause's own viability or selection rule for §13.2.2 (ADR 1960), cited by number there; the in-scope half needs a sentence-by-sentence reading, and the honest end is `implemented` as §13.2.2 is | move by reading: one clause round for the family |
| §13.2.5 Media play parameters | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` reads these entries for the one question viability asks (an `MH` entry the hand-off cannot honour, a player named by identifier); whether that is the clause's whole in-scope half is the reading | move by reading, with the family |
| §13.2.6 Media screen parameters | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` reads these entries for the one question viability asks (an `MH` entry the hand-off cannot honour, a player named by identifier); whether that is the clause's whole in-scope half is the reading | move by reading, with the family |
| §13.2.6.1 General | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` reads these entries for the one question viability asks (an `MH` entry the hand-off cannot honour, a player named by identifier); whether that is the clause's whole in-scope half is the reading | move by reading, with the family |
| §13.2.6.2 Media offset dictionary | offsets, timespans and monitors are how and where a clip is played | B | Tier C (a player in a region of the page) | ask the owner (Q632) |
| §13.2.6.3 Timespan dictionary | offsets, timespans and monitors are how and where a clip is played | B | Tier C (a player in a region of the page) | ask the owner (Q632) |
| §13.2.7 Other multimedia objects | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` reads these entries for the one question viability asks (an `MH` entry the hand-off cannot honour, a player named by identifier); whether that is the clause's whole in-scope half is the reading | move by reading, with the family |
| §13.2.7.1 General | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` reads these entries for the one question viability asks (an `MH` entry the hand-off cannot honour, a player named by identifier); whether that is the clause's whole in-scope half is the reading | move by reading, with the family |
| §13.2.7.2 Media players dictionary | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` reads these entries for the one question viability asks (an `MH` entry the hand-off cannot honour, a player named by identifier); whether that is the clause's whole in-scope half is the reading | move by reading, with the family |
| §13.2.7.2.1 General | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` reads these entries for the one question viability asks (an `MH` entry the hand-off cannot honour, a player named by identifier); whether that is the clause's whole in-scope half is the reading | move by reading, with the family |
| §13.2.7.2.2 Algorithm: Media Player | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` executes this clause's own viability or selection rule for §13.2.2 (ADR 1960), cited by number there; the in-scope half needs a sentence-by-sentence reading, and the honest end is `implemented` as §13.2.2 is | move by reading: one clause round for the family |
| §13.2.7.3 Media player info dictionary | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` reads these entries for the one question viability asks (an `MH` entry the hand-off cannot honour, a player named by identifier); whether that is the clause's whole in-scope half is the reading | move by reading, with the family |
| §13.2.7.4 Software identifier dictionary | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` reads these entries for the one question viability asks (an `MH` entry the hand-off cannot honour, a player named by identifier); whether that is the clause's whole in-scope half is the reading | move by reading, with the family |
| §13.2.7.4.1 General | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` reads these entries for the one question viability asks (an `MH` entry the hand-off cannot honour, a player named by identifier); whether that is the clause's whole in-scope half is the reading | move by reading, with the family |
| §13.2.7.4.2 Algorithm: software identifier | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` reads these entries for the one question viability asks (an `MH` entry the hand-off cannot honour, a player named by identifier); whether that is the clause's whole in-scope half is the reading | move by reading, with the family |
| §13.2.7.4.3 Software URIs | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` reads these entries for the one question viability asks (an `MH` entry the hand-off cannot honour, a player named by identifier); whether that is the clause's whole in-scope half is the reading | move by reading, with the family |
| §13.2.7.4.4 Version arrays | "a media engine", one quoted sentence of the clause | C | `pdf_model::media` reads these entries for the one question viability asks (an `MH` entry the hand-off cannot honour, a player named by identifier); whether that is the clause's whole in-scope half is the reading | move by reading, with the family |
| §13.2.7.5 Monitor specifier | offsets, timespans and monitors are how and where a clip is played | B | Tier C (a player in a region of the page) | ask the owner (Q632) |
| §13.3 Sounds | a sound object's samples are to be played through the speakers, the excluded engine | C | as §12.6.4.9; §13.3's own formats need no decoder (RFC 0009 section 3.5), and Tier B is A254's "on demand" | ask the owner (Q631) |
| §13.5 Alternate presentations | alternate presentations, a slideshow, under the clause 13 entry | A | the clause itself says processors are not required to support them, is deprecated, and defines no slideshow format to execute; the entry is the wrong ground but the outcome stands | keep; the note's ground is the clause's own permission |
| §13.6 3D Artwork | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.1 General | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.3 3D streams | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.3.1 General | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.3.2 3D animation style dictionaries | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.3.3 3D reference dictionaries | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.4 3D views | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.4.1 General | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.4.2 Projection dictionaries | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.4.3 3D background dictionaries | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.4.4 3D render mode dictionaries | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.4.5 3D lighting scheme dictionaries | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.4.6 3D cross section dictionaries | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.4.7 3D node dictionaries | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.5 Coordinate systems for 3D | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.6 3D markup | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.7 Persistence of 3D measurements and markups | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.7.1 General | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.7.2 The 3D units dictionary | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.7.3 3D measurement/markup dictionary | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.7.3.1 General | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.7.3.2 3D linear dimension measurement | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.7.3.3 3D perpendicular dimension measurement | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.7.3.4 3D angular dimension measurement | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.7.3.5 3D radial dimension | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.7.3.6 3D comment note | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.6.7.4 3D measurements and projection annotations | rendering U3D or PRC artwork, the excluded half; the annotation's appearance is §13.6.2's and drawn | B | a U3D reader in Rust, the entry's trigger: `u3d` 0.1.0 reads part of ECMA-363 and is unfuzzed; a mesh renderer under `wgpu`; §13.6.4's view model | ask the owner (Q632) |
| §13.7 Rich media | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.1 General | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2 RichMedia annotations | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.2 RichMediaSettings dictionary | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.2.1 General | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.2.2 RichMediaActivation dictionary | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.2.3 RichMediaDeactivation dictionary | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.2.4 RichMediaAnimation dictionary | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.2.5 RichMediaPresentation dictionary | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.2.6 RichMediaWindow dictionary | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.3 RichMediaContent dictionary | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.3.1 General | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.3.2 The assets name tree | an asset is an embedded file the rich media engine plays | C | the amended entry admits the media file saved; the assets are §7.11.4 embedded files, and the attachments list does not reach them | ask the owner (Q631) |
| §13.7.2.3.3 RichMediaConfiguration dictionary | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.3.4 RichMediaInstance dictionary | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.3.5 View dictionary | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.3.6 View params dictionary | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.3.6.1 General | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.3.6.2 Saving state data | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §13.7.2.3.6.3 Loading state data | presenting rich media (3D, sound, video, Flash), the excluded half; the annotation's appearance is §13.7.2.1's and drawn | B | Tier C and Tier D; nothing plays Flash | ask the owner (Q632) |
| §K XFA forms | §K.1's permission to decline, the authoring exclusion (A254), and §K.2's consistency rules make the AcroForm the form | B | the XFA 3.3 specification (403 at the PDF Association); a layout engine, which is the authoring line's middle; the one Rust stack, `pdf-xfa`, is AGPL-3.0 and fails `deny.toml` | ask the owner (Q633) |
| §K.1 General | §K.1's permission to decline, the authoring exclusion (A254), and §K.2's consistency rules make the AcroForm the form | B | the XFA 3.3 specification (403 at the PDF Association); a layout engine, which is the authoring line's middle; the one Rust stack, `pdf-xfa`, is AGPL-3.0 and fails `deny.toml` | ask the owner (Q633) |
| §K.2 Access to the XFA template and other resources | §K.1's permission to decline, the authoring exclusion (A254), and §K.2's consistency rules make the AcroForm the form | B | the XFA 3.3 specification (403 at the PDF Association); a layout engine, which is the authoring line's middle; the one Rust stack, `pdf-xfa`, is AGPL-3.0 and fails `deny.toml` | ask the owner (Q633) |

## 3. Findings

1. **No status moved, and the premise that one would is the brief's that did not hold.** Section 1.4
   is the reason: the two words allowed are each false for every decayed row, and either drags two
   headings with it. The honest moves are three, none of them a review's: **straight to
   `implemented`** by the round that builds or reads the row (§14.11.2.2, the §13.2 family, the
   movie hand-off), which never passes through an aggregate; and **`reported`** for §12.6.4.9 and
   §12.6.4.10, whose actions are detected, refused by name and printed by every window today while
   the hand-off is owed — the ledger header's own definition of the word. The rule ADR 2370 leaves
   says a batch's clause slot takes a row from section 4 and moves it to the status its work makes
   true.

2. **Twenty-one rows of §13.2 are executed by `pdf_model::media` and still say "a media engine"**
   (section 1.2). Their notes were written to ADR 1548's template before ADR 1960 built the viability
   judgement, and when it was built only §13.2.2 and §13.2.4.2 were moved; the §13.2 heading's note
   records exactly that exception. Eight are plain — §13.2.3 and its three subclauses, §13.2.4 and
   §13.2.4.1, §13.2.4.3's sections, §13.2.7.2.2's player algorithm — each a clause whose viability or
   selection rule `media.rs` cites by number and executes for the hand-off. Thirteen more (§13.2.5,
   §13.2.6 and §13.2.6.1, §13.2.7 to §13.2.7.4.4) are read for the one question viability asks of
   them: an `MH` entry the hand-off cannot honour, a player named by a software identifier this
   program does not have. This is the habits file's "sibling rows became `implemented`" shape
   (ADR 0122's trigger), in the one status no sweep reads. **Recommendation**: one clause round reads
   the family sentence by sentence against `media.rs` and moves each to `implemented` for the in-scope
   half, the playing staying on the entry, as §13.2.2's note does; §13.2.6.2, §13.2.6.3 and §13.2.7.5
   stay on the entry. The proposed trap in the record's report is the sweep's other half.

3. **§14.11.2.2 is owed by the owner's word and still says it is declined.** Its note says the row
   becomes one to fill the day a viewer grows a prepress view; RFC 0009 section 2.10 said the print
   panel is that view, and A254 adopted section 12 item 2 — "§14.11.2.2's guidelines in the panel" —
   on 2026-10-08. The build is Table 396's four boxes in Table 397's colour, width and dash over the
   page the preview shows, with the program's own defaults where an entry is absent, which the
   clause asks for. **One round, no dependency, no question**: it is the best value in the list.

4. **A movie's file and a sound object's are the media file the amended entry admits, and nothing
   hands them over.** `CLAUDE.md`'s clause 13 entry puts "the media file saved or handed to this
   machine's own player" in scope, and RFC 0009 section 3.5's Tier A, which A254 adopted, names
   §13.4's movie dictionary and §13.3's sound object among what `pdf_model::media` reads. ADR 1961
   built the hand-off for a rendition's clip; `media.rs` reads no movie or sound, and `action.rs`
   still answers `Sound` and `Movie` with "clause 13's multimedia, excluded". Table 306's `/F` is a
   file specification like Table 285's `/D`, so §12.5.6.17's activation and §12.6.4.10's action can
   go through `Event::Media` at `Operation::HandOverMedia` exactly as §12.6.4.14 does. A sound object
   is in two halves: one with `/F` is a self-describing file and the same hand-off; one with embedded
   samples has no file until somebody writes a RIFF header over them (8-bit signed samples and
   big-endian 16-bit ones need re-encoding on the way, which is arguably decoding) or plays them, which
   is Tier B. §13.7.2.3.2's assets are §7.11.4 embedded files the "saved" half admits, and the
   attachments list does not reach them. `doc/questions/Q631` asks for the reading to be confirmed and
   whether Tier B's demand has come; none has by RFC 0009's measure.

5. **Both of clause 13's still-excluded halves have met their triggers by letter.** Section 1.3:
   pure-Rust H.264 and AAC decoders under licences `deny.toml` admits exist, one of them
   `forbid(unsafe_code)`, and a Rust U3D reader exists. **Neither trigger is a decision**; `CLAUDE.md`
   says an exclusion is revisited "by argument, never by attrition", and the argument is the owner's.
   Read as evidence: the decoders are young (first published this April to July), from two
   organisations whose claims of exactness are their own; Tier C is still RFC 0009 section 3.5's
   media player's worth of work — a second confined worker, a demuxer, frames across the wire, a
   clock, a presentation surface in three windows — and the hand-off already plays every viable clip
   on the reader's own player. `u3d` 0.1.0 is a partial reader, unfuzzed, with one release; Tier D
   also needs §13.6.4's view model and a mesh renderer. `doc/questions/Q632` recommends an RFC that
   prices Tier C against these crates before anything is built, and keeping Tier D excluded with its
   trigger restated so that it can be met by something other than a crate's existence.

6. **XFA holds on §K.1's own permission, and the owner has said "later".** Nothing about the three
   rows has decayed: §K.2's consistency rules still make the `AcroForm` the form of a hybrid file and
   the per-document sentence is built (ADR 1932). What blocks "later" is unchanged since RFC 0009
   section 4: the XFA 3.3 specification (403 today), a layout engine in the middle of the authoring
   line, FormCalc, and the launch path that `/NeedsRendering` puts the engine on. The one Rust XFA
   stack is AGPL-3.0 and `deny.toml` refuses it. `doc/questions/Q633` asks when "later" is and
   recommends that its first step be the specification in `doc/` and an RFC, not code.

7. **Clause 14's layout rows: four keep on the standard's condition, one is a scope choice, and a
   half-sentence in all five has decayed.** §14.8.3.2, §14.8.5.4.1, §14.8.5.4.6 and §14.8.5.4.7 each
   carry a `shall` addressed to whoever lays the content out or defines the attribute object, and no
   writer here defines one (`grep -rn '/O /Layout\|"Layout"' crates/pdf-transform crates/pdf-syntax`
   names none). But each note says this program "exports to no format", and `viewer-core`'s
   accessibility tree is an export of the structure tree to the platform that already reads
   §14.8.5.4.2's, §14.8.5.4.3's and §14.8.5.4.5's attributes. Table 380's inline attributes are what
   that export lacks — a text decoration a sighted reader sees as a drawn rule, a baseline shift that
   is a subscript, ruby — and `accesskit` 0.24.1, already in the tree, carries `TextDecoration` and
   `VerticalOffset`. `CLAUDE.md` has clause 14 in scope "as far as accessibility needs it", so whether
   it needs these is the owner's reading: `doc/questions/Q634`. A reflowing view, which is what the
   other four rows' attributes serve, is a layout engine and a bad idea under the authoring exclusion.

8. **The deprecated families hold on the standard's own recommendation.** Web capture's eighteen
   rows, the four prepress rows (§14.11.4, §14.11.6.1, §14.11.6.3, §14.11.7) and §14.2 rest on
   §14.10.1's, §14.11's and §14.2's deprecation and §3.15's definition: ignore it. The writers were
   checked against the condition: `split` and `merge` leave `/SpiderInfo`, `/IDS`, `/URLS` and
   `/DPartRoot` behind by name, `optimize --linearize` carries `/DPartRoot` as written, no writer
   emits `/ProcSet`, and the archive conversion removes `/OPI` for ISO 19005-2's reason. Building any
   of them would be writing or acting on bookkeeping §3.15 says a processor should ignore — the bad
   idea in this half. §14.12.4.2 keeps for the same shape: its audience is a production workflow.

9. **§10.6 holds, and its four notes owe one sentence the owner asked for.** The condition is
   continuous-tone output, and every raster this program transmits is eight-bit (RFC 0009 section
   2.4). A254 adopted question 8 "with its one flipping route named in the row", and none of
   §10.6.1–§10.6.4 names it: this program itself writing a bilevel or few-level spool. A note edit,
   no status.

10. **§13.5 is kept on the wrong ground.** Its note rests on the clause 13 entry; the clause itself
    says processors are not required to support alternate presentations, is deprecated, and defines
    no slideshow format a processor could execute. The outcome stands; the note should say the
    clause's own reason the day it is touched.

## 4. The candidate list

Ordered by value over cost. Each is a ledger slot for a coming batch; the status each ends at is the
one its work makes true.

| # | rows | the work | size | blocker |
|---|---|---|---|---|
| 1 | §14.11.2.2 | the guidelines in the print preview, Table 397's style and the program's defaults; `inapplicable` → `implemented` | a round | none (A254) |
| 2 | §13.2.3–§13.2.3.3, §13.2.4, §13.2.4.1, §13.2.4.3, §13.2.7.2.2, then the thirteen of finding 2 | read against `media.rs`; `out-of-scope` → `implemented` for the in-scope half, §13.2's note with them | a clause round | none |
| 3 | §12.6.4.10, §12.5.6.17 | a movie's `/F` through `Event::Media` at `HandOverMedia` | a round | Q631's confirmation |
| 4 | §10.6.1–§10.6.4 | the flipping route's sentence | minutes, inside any ledger slot | none (A254) |
| 5 | §14.8.5.4.4 | Table 380's decoration, baseline shift and ruby into the accessibility tree through `accesskit` | a round | Q634 |
| 6 | §12.6.4.9, §13.3 | a sound with `/F` handed over; embedded samples per Q631's answer | a round | Q631 |
| 7 | §13.7.2.3.2 | rich media assets in the attachments list | a round | Q631 |
| 8 | §13.2.6.2, §13.2.6.3, §13.2.7.5, §13.7's video and sound | an RFC pricing Tier C against section 1.3's crates | a round, then a stream | Q632 |
| 9 | §13.6's 27 rows, §12.6.4.16 | Tier D: a U3D reader and a mesh renderer | a stream | Q632 |
| 10 | K, K.1, K.2 | the specification, then an RFC | a stream | Q633 |

**Kept, with the sentence that keeps them**: §14.10's eighteen rows, the four deprecated prepress
rows and §14.2 (finding 8); §14.12.4.2 (a production workflow's); §14.8.3.2, §14.8.5.4.1,
§14.8.5.4.6 and §14.8.5.4.7 (a `shall` on a layout process, finding 7); §13.5 (finding 10);
§12.6.4.18 (a handler and a script call, Flash having no player); §13.1 and §13.2.1 (the family's
general text).
