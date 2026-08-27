# ADR 0707 — The arrangement two windows could not see, and the files all three lost

Status: accepted, 2026-08-25. Session 772, a general-improvement round choosing its own subject.

§12.3.5's `shall` reaches all three windows, and a defect found by reading §12.3.5.2 beside the code
is fixed in the two places this tree had written it.

## 1. Why this subject

`tools/state.sh quick` prints, per host, which of `viewer_core`'s thirty-one questions it asks, and
its own reading calls exactly one of the gaps **"a debt, with the sharpest clause here"**:

> `Query::Collection` — "[i]f this dictionary is present in a PDF document, the interactive PDF
> processor shall present the document as a portable collection" (§12.3.5) — a shall addressed to a
> viewer. `viewer-ui` shows it and the two native hosts do not.

That is the spec-driven track naming its own next item, in a `shall` addressed by name to an
interactive PDF processor, and it had a second witness in this tree's own prose:
`viewer_host::panel::attachment_rows`'s doc comment said the arrangement "is a different answer
… that this host does not yet ask". A documented gap, a clause, and no dependency on any other
round's files.

It also answers `CLAUDE.md`'s "the map is not the territory" head on. **The item on the list was the
smaller half.** Reading §12.3.5.2 in `doc/md/` to write the mapping turned up a defect that was on
no list, that no gate could see, and that `viewer-ui` had carried since ADR 0202 built the panel —
section 3.

## 2. What the two native hosts now do

`viewer_host::panel::collection_rows` is the mapping, toolkit-free and beside the other five, so the
GTK window and the Qt window present a collection identically and are checked without a display.
Both hosts' `Tab::Files` arm asks `Query::Collection` first and falls back to `attachment_rows`,
because **a collection is these same files arranged rather than a second population of them** — the
same reason ADR 0202 made it the files tab in `viewer-ui` rather than a seventh panel.

What a row now carries:

- §12.3.5.2's folder tree, each file under the folder its name-tree key names, and the row still
  carrying the *tree's* key — folder tag and all — in its `RowAction::Extract`, because that is what
  `Command::Extract` names a file by;
- Table 155's visible fields in `/O` order as the row's detail, `/V false` dropping a column
  whatever its `/O` says, and a field stating no `/O` sorting after those that do;
- and §12.3.5.1's `/D` as `PanelRow::emphasis`, a new flag on the shared row.

**`emphasis` is the same shape as `note` and for the same reason.** The clause states no appearance
for "the document that shall be initially presented in the user interface", so each toolkit says it
its own way — Adwaita's `heading` class, a bold `Qt::FontRole`, `viewer-ui`'s bold face — while
*which row it is* stays one decision in one place. A rule each host applied to `RowAction::Extract`
would be three rules, and three rules is how two hosts stop agreeing.

**The container's own pages stay on the screen**, unchanged from ADR 0202 and restated in the new
code: the clause says to present the document as a collection and does not say instead of what, and
§7.6.7's unencrypted wrapper is the case that settles it.

## 3. The defect: a file the panel dropped

Two sentences of §12.3.5.2, neither of which this tree obeyed:

> When folders are used, all files in the EmbeddedFiles name tree (see "Table 32 -Entries in the
> name dictionary") shall be treated as members of the folder structure by an interactive PDF
> processor.

> If no folder structure is specified, interactive PDF processors should show all files in the
> collection in a flat list.

`viewer_ui::chrome::collection_rows` listed the files whose key names *no* folder, then walked
`/Folders` listing the files whose key names each folder it found. Two populations fell between:

- **a collection with no `/Folders` at all** — the walk never happens, so only the untagged files
  were drawn and every key of the form `<n>name` vanished;
- **a key naming a folder identifier the tree does not state** — `<3>report.pdf` in a document whose
  `/Folders` states folders 1 and 9 — which is neither untagged nor reached by the walk.

In both cases the file was in the document, in the answer, and not on the screen. **That is trap 5's
shape rather than a presentation question**: a panel drawing fewer files than the document embeds
looks exactly like a document that embeds fewer files, and nothing said otherwise.

The second case needs a choice, and it is recorded as one. Such a key *conforms* to the clause's
naming rules, so the clause's own "[f]iles in the EmbeddedFiles name tree that do not conform to
these rules shall be treated as associated with the root folder" does not reach it. What it
contradicts is "[t]he value shall correspond to a folder ID", which is a requirement on the
**producer**, and for a producer that breaks it the clause states no remedy. The root is the one
place in the structure that admits a file no folder claims, so that is where it goes — chosen to
keep the `shall` above rather than to invent an arrangement, and written down as a choice.

The first case needs no choice: the clause states the answer, and it is a flat list of all of them.

**Fixed in both copies.** `viewer-ui` had the defect and the new `viewer-host` mapping would have
been written with it, because it was written by reading the working code before reading the clause.

## 4. Why the mapping is written twice, and it is not an oversight

`viewer_ui::chrome` builds its own rows for every panel whose row is a *widget* — an expander, a
switch, a miniature — and shares `viewer_host::panel`'s rows for the two that are plain text
(§12.4.3's threads, §14.3.3's information). Outline, layers and attachments are already written in
both places on that division. A collection row is a `viewer-ui` `Row` with a depth and a style and a
`viewer_host::PanelRow` with children, and unifying them would mean either giving `shared_row` a
depth and an `Act::Extract` it has no other caller for, or giving `PanelRow` `viewer-ui`'s
`bold: detail.is_some()` rule, which means something else entirely.

So the division stands and the cost is paid where it is visible: both doc comments state the
clause's rules, both name the other, and section 3 is the standing evidence that this cost is real —
one copy had a defect for four hundred and twenty rounds and the sweep that would find it does not
exist. A round that unifies the two row types is welcome; this one did not, and says so.

## 5. What was measured, and what was not

No pixel of a **page** moves: nothing here is in `pdf-render`, `pdf-model` or either rasteriser, and
the change→gate map puts all four touched crates in its "no corpus gate" row.

Both fixes were **calibrated against the defect** before they were believed (trap 13). With the
placement rule reverted to what it was, `every_embedded_file_is_shown_whatever_its_key_names` and
`the_document_a_collection_opens_on_is_the_row_set_apart` fail in `viewer-host`, and
`no_embedded_file_is_lost_because_its_key_names_a_folder_that_is_not_there` fails in `viewer-ui`;
with the fix, all three pass. The folder-tree test carries its own calibration in its body — it
asserts what `attachment_rows` produces for the same files, which is the flat list both native
hosts drew until this round.

**The fixture is written in the test**, because not one of the 974 pdf.js documents states a
`/Collection` and the one that does is under `doc/corpora/`, which no gate may depend on. A test
that skipped itself where that submodule is absent would leave §12.3.5's `shall` ungated on this
machine and on CI — trap 8's converse.

## 6. What is still owed

Table 153's `/View T` tile mode, `/Sort`'s order, `/Colors`, `/Split` and §12.3.6's `/Navigator`
layouts are read, carried across every boundary, and presented by nobody: three windows now draw one
presentation of a collection where the clause describes several. That is what keeps §12.3.5 and
§12.3.5.1 `partial`, and it is unchanged by this round.
