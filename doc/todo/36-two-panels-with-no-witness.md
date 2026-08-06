# One panel the standard asks for and no corpus document wants

Status: **§12.4.3's articles are drawn since the three-hundred-and-forty-seventh session** (ADR
0200). §12.3.5's collection is read, measured at zero, and owed anyway.
Priority: 36 — capability, and the last `partial` row whose blocker is a *panel*
Corpus: **0 documents** state a `/Collection`; **0** state an article thread
Clauses: §12.3.5, §12.3.5.1, §12.3.5.2, §12.3.6, §12.4.3
Code: `crates/pdf-model/src/collection.rs`, `crates/pdf-model/src/article.rs`,
`crates/viewer-ui/src/chrome.rs`

## Why they are together

`doc/todo/01`'s fifth sweep — every `pub fn` in `pdf-model`, grepped against the two host crates —
produced eleven functions from these two modules in the three-hundred-and-thirteenth session:
`all_folders`, `folder_of`, `embedded_file_keys`, `initial_document`, `preferred`, `ascending`,
`is_file_name`, `is_in_the_item` from one, `beads_on_page` and `page_array_agrees` from the other.
Both modules are complete readers with no caller, and both rows explain themselves the same way:

- §12.3.5: "`partial` for the sentence a viewer owes and this one cannot pay: '[i]f this dictionary
  is present in a PDF document, the interactive PDF processor shall present the document as a
  portable collection', which needs a file browser."
- §12.4.3: "`partial` for what is left, which is a *person's* way in: nothing lists a document's
  threads for a reader to pick one … That is a panel rather than a clause — the same shape §12.3.3's
  outline was before the hundred-and-sixty-sixth session."

**A sidebar arrived in the hundred-and-sixty-sixth session and neither row moved for forty-seven
rounds.** That is the capability shape the sweeps watch for, and the correction was that the
*blocker* was a tab rather than a toolkit.

**§12.4.3's tab is drawn** (ADR 0200): `Query::Articles` answers with the threads,
`viewer_ui::chrome`'s sixth tab lists them with their `/I` title and bead count, and a click sends
`Command::Activate` on the thread — the outline's own message, so `interact::activate_object`
composes §12.6.4.7's `ThreadJump` and the click lands on Table 163's `/R`. What §12.4.3 still owes
is `beads_on_page`, which answers *which article is under this point* and which no host asks.

## What keeps them here rather than in a defect band

**Neither has a witness.** Measured in the three-hundred-and-thirteenth session over all 974 corpus
documents: **not one states a `/Collection`**, and §12.4.3's own row records the article half —
2 catalogs carry a `/Threads` entry, one an empty array and one a null, and no page carries a `/B`.

So a panel built for either would be drawn against a fixture this project wrote, and the only gate
that could see it is `viewer-ui/tests/panel.rs`, which counts ink. That is not a reason to refuse —
trap 8 says a corpus finds what documents contain and not what the standard says, and clause 12's
display half is in scope without exclusions — it is a reason to say what a round taking one owes:

## What each would cost

**§12.3.5's collection.** The `shall` is "present the document as a portable collection", and the
data is all read: Table 153's schema as columns, `/Sort` as the order, `/View` as the initial mode,
§12.3.5.2's folder tree, §12.3.6's `Navigator::preferred` choosing among the layouts *this* viewer
can draw. The files tab already lists §7.11.4's embedded files flat; what it would gain is the
folder tree, the schema's columns, and the one rule that decides whether it is obeyed at all —
§12.3.5.1's three fallbacks for `/D`, which `Collection::initial_document` already answers as three
values. **One decision is not made**: whether a collection's *container* pages stay on the screen or
the panel replaces them. The clause says "present the document as a portable collection" and §7.6.7's
unencrypted wrapper is the case that argues for keeping the page — its whole purpose is a page
saying the payload is encrypted, and Table 153's `/View H` is how it says so.

**~~§12.4.3's articles~~ — done.** It cost what this file predicted: a `Query`, an `Answer`, a tab,
and `activate_object` learning one more shape. The prediction that turned out to matter most is the
last one — the jump is Table 163's `/R` on the first bead, "which §12.6.4's thread action already
composes" — because *composing that action* rather than writing a second jump is what kept one
behaviour in one place.

## The order it was taken in, and what the collection still costs

Articles were first because they are a third of the work and exercise the `Command::Activate` path
§12.3.3's outline already uses. What is left for the collection is the list above: the folder tree,
the schema's columns, §12.3.5.1's three fallbacks for `/D`, and the one decision nobody has made —
whether a collection's *container* pages stay on the screen or the panel replaces them. §7.6.7's
unencrypted wrapper is the case that argues for keeping the page.
