# 1762 — The tail of Tier 1 bridged, and the runners answer whether a page changed

Status: accepted and **built**. Session 1463. Empties `surface::NOT_BRIDGED` of the 21 names ADR
1724 section 4 left; builds on ADR 1736 (a host tells the view state what only it knows), ADR 1700
(a script writes what a reader's edit reaches), ADR 1617 (a property is the entry its appearance is
built from), ADR 1750 (the two runners) and ADR 1752 (the windows' half).
Code: `crates/pdf-model/src/view/script_sites.rs` (`ScriptsRan`, `Ran`, `finish`),
`crates/pdf-model/src/view/script_model.rs` (`ScriptEdit::redraws`, `ScriptEdit::Console`,
`ConsoleCommand`, `ConsoleRequest`, `Keys`, `FontName`, `Property::LineWidth`/`TextSize`/`TextFont`,
`WidgetState::line_width`/`text_size`/`text_font`, `Layer::intent`, `PageState::words`,
`line_width_of`, `tf_of`, `base_font_of`, `intent_of`), `crates/pdf-model/src/view/scripts.rs`
(`Applied::drawn`, `set_keys`, `take_console_requests`, `read_words`, `page_words`, `spells_words`,
`rich_value_of`, `MAX_WORD_PAGES`, `MAX_WORD_TIME`), `crates/pdf-model/src/appearance.rs`
(`scripted_entries`' one `/DA`, `form_font`), `crates/pdf-script/src/engine/bridge.rs`,
`pages.rs` (the word pair), `members.rs` (`getIntent`), `util.rs` (section 6),
`crates/pdf-script/src/wire.rs` (version 13), `crates/viewer-core/src/{interact,viewer}.rs` (three
named hunks: `.handed` where a count was compared).
Tests: `crates/pdf-script/tests/keys_words_and_console.rs`, `tests/util_members.rs`,
`tests/hook.rs` (`a_widget_s_script_answers_whether_the_page_it_draws_changed`), `tests/wire.rs`.

## 1. The census first

`examples/javascript_census.rs` over the 90 763 files, with a measurement patch applied and
reversed (`scratchpad/r1463/census-members.patch`) that prints every `.name` a document's scripts
spell for the 21 names, 72 s behind the lock: `this.getPageNumWords` and `this.getPageNthWord` 5
documents each (all five the same `_FindWord` library), `event.shift` 3, `lineWidth` 3, `event.modifier`
2, `console.show` 2, `textSize` 2, `util.scand` 1, `console.clear` 1; `stringFromStream` once, on
`SOAP` (Tier 2), so not `util`'s; the other eleven none. ADR 1724 section 4's figures hold.

## 2. The two runners answer `ScriptsRan`

`run_annotation_scripts` and `run_page_scripts` answer `{ handed, changed }`. A host needs both:
`handed` drops the action path's refusal of the same chain (ADR 1752), and `changed` says whether
the page is to be interpreted again. `changed` is true where a script's edits can have changed what
a page draws — `ScriptEdit::redraws` (a property, a reset, a layer, an annotation, a choice,
`calculateNow`), a value written over a different one, or a walk of `/CO`; a focus, a turn, a view
change, a timer, a beep and a console request are a host's own requests. Sound in one direction by
construction: every way a script reaches the page is an edit, and every edit kind is classified.
`viewer-core` keeps its behaviour through `.handed` until the host round reads `.changed`.

## 3. The members a host's knowledge answers

- **`event.shift`, `modifier`, `keyDown`** read `Keys` a host tells the view state
  (`set_keys`), the shape `set_window_view` has. The reference names the modifier key for Windows
  and the Mac and none here; Control is the choice. `keyDown` is the reference's list-box and
  combo-box keystroke's alone, so it reads false at every other site.
- **`console.show`, `hide`, `clear`** are `ScriptEdit::Console`, held as `ConsoleRequest` until a
  host takes it, with the index of `script_reports` it applies at; a `clear` drops the lines its run
  logged before it. What a window's console draws is the host round's.

## 4. The members the file answers

- **`lineWidth`** is Table 168's `/W`, Table 166's `/Border` third element where no `/BS` (its note:
  `/BS` makes `/Border` ignored), 1 otherwise (§12.5.4); written as the widget's `/BS /W`.
- **`textSize`, `textFont`** are the `/DA`'s `Tf`. A write is one `Tf` after the producer's
  operators — the shape `textColor` already took (ADR 1578 section 2) — and every property that
  writes `/DA` now writes one string, so a colour and a size set together both stand. `textFont`
  reads the `/DR` font's `/BaseFont`; a write names a `/DR` font by key or `/BaseFont`, and one the
  form does not hold is reported and not written: §12.7.4.3 has `Tf` name a `/DR` resource, and
  adding one to `/DR` is a cost this ADR does not take. The reference's `font` constants are carried.
- **`OCG.getIntent`** is Table 96's `/Intent`, `View` by default. **The brief had it refused by
  name; no reason holds**: it reads the document, as `name` and `state` do, and RFC 0008 section
  4.2's table has no `OCG` row at all — the premise that the section's rows admit all 21 does not
  hold for this one, and the owner's rule (A193: as much of the surface as is secure and reasonable)
  decides it.

## 5. The word pair

A page's words are its interpretation's text cut at white space, the readback's own word gaps
(`content::text`, ADRs 1502, 1515) being this program's heuristic, since §14.8.2.6.2 leaves an
untagged page's words to heuristics and the reference defines none; `bStrip` trims every
non-alphanumeric character at either end. **Words are data told with the document**: a request is
handed across the process boundary before the script runs (ADR 1591), so the view state reads every
page's words once, when a handed script's text spells either name — the five documents spell them
in the open's library — at most 1 024 pages or 2 s, a page past either refused by name. Costs, each
accepted: a name built at run time is never spelled, so its call is refused with that sentence; the
one reading runs on the host's thread inside the run. Table 22's bit 5 is not consulted: the words
go to the document's own realm, which has no path out of the process.

## 6. `util`'s seven

Each a pure function of its arguments in `engine/util.rs`, over a public span reader and writer
`pdf_model::span` (`spans`, `markup`, `Span`) on the rich-text module's own parse (ADR 1634).

- **`scand`** reads through `aform::parse_date`, `AFParseDateEx`'s reader, so the reference's
  two-digit-year horizon (below 50 is the 2000s) comes with it; a local `Date` at the request's
  offset, or null. The numbered formats are `printd`'s inverses: 0 and 1 §7.9.4's date string, 2
  `yyyy/mm/dd HH:MM:ss`.
- **`crackURL`** splits by RFC 3986's generic syntax, components as written; `nPort` 80 or 443 by
  default and absent for `file`; `nURLType` 1 for a bracketed IPv6 literal; a missing or malformed
  URL, or a scheme other than the reference's three, is a `TypeError`.
- **`spansToXML`, `xmlToSpans`** write and read chapter 27's XHTML body; `textColor`'s `G` and
  `CMYK` reach RGB through §10.4.2.2 and §10.4.2.5; text opening no element is one span.
- **`streamFromString`, `stringFromStream`** carry a `ReadStream` whose `read` answers hex; UTF-8
  and UTF-16 (big-endian behind §7.9.2.2's byte order mark); Shift-JIS, BigFive, GBK and UHC are
  refused by name, since no dependency here decodes them; reading is bounded by the string budget,
  16 MiB and 4 096 `read` calls.
- **`iconStreamFromIcon`** is refused by name: no member the realm carries hands out an `Icon`,
  and both consumers of an Icon Stream are Tier 2.

## 7. `richValue` and `richChange`, read-only

RFC 0008 section 4.2 admits the pair read-only; the brief's "written as a committed rich value" is
not what A193 accepted. At a rich text field's events (Table 231 bit 26), `event.richValue` is the
field's `/RV` read into Span objects in the worker (section 6's reader), where the `/RV` holds the
characters the value does, and the value as the save writes it otherwise (ADR 1635); sizes are
measured against the reference's 12 points, the field's `/DS` not applied — a cost. `event.richChange`
is one span of `event.change`, styled as the span the selection starts in. A field that is not rich
text has neither, as in the reference.

## 8. What the host round takes up

Three named hunks are the windows': calling `set_keys` with the pointer's and keyboard's state,
showing `take_console_requests` in a console, and reading `ScriptsRan::changed` where `.handed` now
stands. Until then a window's script reads every key up and its console requests are held unread.
