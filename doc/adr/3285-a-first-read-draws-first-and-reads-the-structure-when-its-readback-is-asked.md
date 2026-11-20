# 3285 — A first read draws first, and reads the structure when its readback is asked

Session 2146, a measure round in the `lane3` worktree. Status: **accepted** and built. Context: ADR 3261 (an
ancestor's statements read once a page), ADR 3262 (the bands moved for §14.9.4's walk, EC3's memory figure left
outside), ADR 2976 (the outermost `/ActualText` covers what its element encloses), ADR 1855 (a page's first read);
`CLAUDE.md` principle 2's "Nothing eager"; `crates/pdf-model/src/view/settings.rs`
(`ViewState::set_structure_deferred`), `crates/pdf-model/src/content/marked.rs` (`Interpreter::accessibility`),
`crates/viewer-core/src/open.rs` (`interpret_to_draw`, `first_read`, `Readback`, `OnScreen::readback`). Clauses: ISO 32000-2 §14.9.2,
§14.9.3, §14.9.4, §14.9.5.

**What page one needs.** §14.9.4 says what `/ActualText` is for:

> The ActualText value shall be used as a replacement, not a description, for the content, providing text that is
> equivalent to what a person would see when viewing the content. The value of ActualText is a character
> substitution for the content enclosed by the structure element or marked-content sequence.

A character substitution changes the characters a page reads back as — what is copied, searched, selected and
spoken — and no mark: the glyphs drawn are the content's, which the substitution stands *for*. §14.9.3's `/Alt` and
§14.9.5's `/E` are spoken in place of what they describe, and §14.9.2's `/Lang` is the language text is spoken in;
none of the four is read by anything that paints. So drawing page one does not need them. What reading them from
a structure element costs is the element itself — its object stream expanded — and, since ADR 2976 made the
outermost `/ActualText` win, `/P` to the structure root, whose top-level elements on ISO 32000-2's own file list
thousands of kids each. In the gate's EC3 profile the marked-content operators spent 46.2 M of the 169.1 M
instruction first frame there (`Document::get_key`, 43.4 M over 657 calls), and the parsed ancestors and their
expanded streams stay in the document's object cache, which is what held `peak_anon_mib` past its band (ADR 3262).

**The decision.** A reading may be asked to leave them: `ViewState::set_structure_deferred(true)`. Under it,
`Interpreter::accessibility` still reads the sequence's own property list — it is in hand, and it is where
§14.9.3's own example states `/Alt` — but where an entry is still missing and the sequence's `/MCID` names an
entry of the parent tree, it does not fetch the element and records that it did not:
`Interpretation::structure_deferred`. Divided pieces and the replacement's checkpoint carry the record. Where it is
set, the interpretation's readback (`text`, `text_layer`, `marked`, `artifacts`, `objects`, `described`,
`language`) is provisional, and interpreting the page again with the setting off is the readback.

`viewer-core` reads a page it is about to draw under a copy of the view with the setting on: `interpret_to_draw`,
which `Viewer::interpret_arranged` calls for every arranged page not yet read — page one at launch among them —
and `first_read`, for a page a host reads out on its own thread. `interpret`, which a search and a print call,
keeps the open view. The page's `Interpreted` keeps the readback in a `Readback` beside a
`OnceLock`; every reader of text, placements or spans asks `OnScreen::readback` (or `Open::readback`), which answers
the reading's own where nothing was deferred and otherwise interprets the page once more under the open view,
keeping only the readback. A deferred read keeps no replacement (a re-placement would carry the provisional
readback) and is not put in the search's readback cache; a search takes a page on the screen's readback from its
reading — settling it, once, for the selection and the accessibility tree too — rather than reading the page
again, so `doc/todo/49`'s one reading per page showing holds, its second half now paid when text is first asked. The next whole read of the page — a zoom of a `NoZoom`
page, a view change — is an ordinary one.

**What it costs.** (a) The second reading: a tagged page's first question about its text — a pointer over the
page asks `text_at`, a search, a selection, a screen reader — pays one more interpretation of that page, off the
launch path; a host may settle it on idle after the first present, which nothing does yet. The readback cache's
`an_edit_forgets_every_page_the_search_had_read` now finds no page held after an edit on its tagged document,
where the page showing used to be put back by the read that draws it. (b) A deferred page that
names an element holds two readbacks once settled. (c) The display list's byte bound (ADR 1507) charges the
readback as it grows, so a page within the width of its `/ActualText` of the bound could be drawn by the deferring
reading and refused by the whole one; the drawing is the deferring reading's, as for every page whose first read
this is. (d) An object stream damaged where only the structure reaches is said as a loss (§7.5.7) with the
next page read's reports rather than with page one's. Untagged pages pay nothing: no element is named, so
nothing is deferred, and the readback is the reading's own.

**What it does not change.** Every reading that is not a first read — `pdf_model::interpret` and its kin, search,
print, the corpus walks, the oracle — runs with the setting off, so the readback every census compares is computed
exactly as before; the selection census's `Selection::All` against `Interpretation::text` is the check that the
settled readback is that one.
