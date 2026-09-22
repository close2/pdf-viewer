# 1227 — A file a document names is a level of its own, and a page for paper is a second pair of boxes

Session 1195. Status: **accepted**.
Context: `crates/pdf-model/src/action.rs` (`Action::GoToR`, `RemoteGoTo`, `remote_go_to`,
`TargetRoot`, `refused`), `crates/pdf-model/src/view.rs` (`Request::Remote`),
`crates/pdf-model/src/page.rs` (`Boundaries`, `Page::print_box`, `Page::print_clip_box`,
`Page::render_for_printing`), `crates/viewer-core/src/{command.rs,interact.rs,open.rs,viewer.rs}`,
`crates/viewer-host/src/policy.rs` (`RemoteDocuments`, `REMOTE_DOCUMENTS`, `Remote`, `remote`,
`asked_to_open_remote`, `remote_note`, `remote_declined`),
`crates/viewer-confined/src/protocol.rs`, `crates/viewer-ffi/src/kinds.rs`, the four windows,
`doc/conformance/ledger.toml` §12.6.4.3 and §12.2.
Builds: ADR 1155 (a link is a program this machine starts, and `file` is outside its schemes),
ADR 1079 (the policy in one function), ADR 1062 (a host-supplied answer about this machine),
ADR 1101 (a file matched rather than asked for), ADR 1190 (what the fourth window owes),
ADR 1180 and ADR 1204 (the print operation and where matrix B stops being the identity),
ADR 1203, ADR 1145 (`/HideMenubar`), `doc/ui-boundary.md` rule 2, `CLAUDE.md` principle 3.
Clauses: ISO 32000-2 §12.6.4.3 (Table 203), §12.3.2.2, §12.3.2.3, §12.6.4.4 (Table 204), §7.11.2.2,
§7.11.5, §12.7.6.4, §14.7.2 (Table 354), §12.2 (Table 147), §14.11.2.

Two decisions, taken together because one round took them, and separate everywhere else. ADR 1186
is the precedent for the shape.

## 1. §12.6.4.3 was refused for a filesystem the program has had since ADR 0244

The refusal read *"GoToR: a destination in another file, which this reader has no filesystem to
open"*, and the ledger row had already found half of the error: `pdf-model` has no filesystem and
is handed bytes, but **this program** has had one in every host since ADR 0244, and
`viewer_host::read_import` has answered the identical question for §12.7.6.4's import-data action
since then — a file a *document* names, admitted only as a single path component beside the open
document.

So nothing was missing except the wiring, and the wiring already existed in the shape §12.6.4.4
uses: the name crosses as `Event::NeedsFile`, the bytes come back as `Command::Supply`, and the
destination is read in the document that arrived. `Purpose::RemoteDocument` is the third value of
that vocabulary and it needed no new message on the boundary — the eleventh round of hosts in a
row that has not.

What the clause itself decides, and what it leaves open:

- **`/D` is read in the remote document and nowhere else.** §12.3.2.2's NOTE is explicit — "the
  page parameter specifies an integer page number within the remote document instead of a page
  object in the current document" — so an explicit destination resolved in the source would name
  an object of the wrong file. `RemoteGoTo::destination` is therefore carried unresolved and
  `RemoteGoTo::page_in` reads it against the document a host supplied.
- **`/SD` is read too, and it is not out of reach.** The row had it down as unread "for the same
  reason the action is refused", because its first element is a structure element ID "in the remote
  document". Once that document is open the identifier is a name-tree lookup this tree has had for
  a long time: `structure::Tree::element_by_id` through §14.7.2's `/IDTree`, then
  §12.3.2.3's own algorithm in `destination::structure_element_page`. Table 203's precedence is a
  `should`, so an identifier the remote document does not hold falls back to `/D` rather than
  refusing.
- **`/NewWindow` is obeyed as written, and the clause is why.** Not one of its three sentences
  carries a `shall`. The one with force is the `false` case — "the destination document replaces
  the current document in the same window" — and that is what happens. The absent case is referred
  to "the interactive PDF processor's preference", and this program's preference is one document in
  one view, which is the answer Table 204's identical entry has had since §12.6.4.4 was built. A
  `true` is said out loud rather than passed over, because a person who asked for a second window
  and got one window is owed the difference (trap 5). **This is not a claim that a second window
  would be wrong**; it is a claim that the clause does not require one, and a round that builds
  tabs or a second `DocumentId` changes what this program's preference *is* without re-arguing the
  clause.

## 2. The level is its own value, and ADR 1155 is the argument for splitting it

`--remote-documents=` takes `refuse`, `ask`, `warn` and `open` — `Links`'s four words, in `Links`'s
direction, because the permissive end is again the one where something a document named is acted
on. The question this ADR settles is why it is not `--links=` itself.

ADR 1155 put `file` outside `LINK_SCHEMES` on exactly this ground: "[o]pening a file a *document*
named is §12.7.6.4's hazard one clause over, where `read_import` answers it with a directory a
**person** supplied, and nothing here may be looser than that." That sentence divides the two
subjects already. One value for both would make each of two sentences mean the other:

- a reader who set `--links=open` said their browser may be started on a URL a document chose, and
  said nothing about which PDFs beside their document may be parsed by this program;
- a reader who set `--links=refuse` said they want no other program started, and did not say they
  may not follow a cross-reference inside their own set of documents.

**The path rule comes before the level, and that order is the decision.** `viewer_host::remote`
resolves the name with `resolve_import` first: a name that is not a single path component beside
the open document is refused at *every* level, including `open`. What the level decides is the one
act that is left — opening a file that has already passed the narrowest policy this program has.
A `/F` in §7.11.5's URL form falls to the same rule and is said by name; fetching it would need a
network `CLAUDE.md` principle 3 gives this program none of.

**The default is `ask`**, on ADR 1155's own two-part test applied to this act: a document never
reaches a file on this disk by itself, and a reader is not refused by their own viewer the jump the
clause describes. A window with no dialogue cannot put that question, so `quorra-confined` is
pinned to `refuse` — and it now *answers* `Command::Supply` with no bytes, which it did not before:
the worker was left holding an action nobody would ever answer for, and the click reported nothing
at all. That was a real defect on the `/GoToE` path too, and it is fixed for both.

## 3. §12.2's print pair: one more pair of boxes, selected where the purpose is stated

Table 147 states four boundary entries and this tree carried two. `/PrintArea` is "[t]he name of
the page boundary representing the area of a page that shall be rendered when printing the
document" against `/ViewArea`'s same sentence about the screen, and `/PrintClip` against
`/ViewClip` likewise — so one page holds both answers at once and which of them is drawn is decided
by what the output is for.

`Pages` now carries a `Boundaries` of two pairs instead of one tuple, `build_page` picks all four
with the closure it already had, and `Page::print_box` and `Page::print_clip_box` stand beside
`display_box` and `clip_box`. **The selection is `viewer_core::open::page`'s**, which is the one
place in that crate where a page is built, and its condition is the `Purpose::Print` the print
operation itself stated through `ViewState` — never an inference, which is what ADR 1173 fixed for
that value. `pdf-transform`'s named-box render writes the same two rectangles directly and is the
precedent for the shape.

The ledger row's standing objection — that a second pair would be "a capability that reaches the
crate and never reaches the program" — expired when ADR 1180 gave this program a print operation,
and the gate says so rather than the ADR: `print_preferences.rs` prints the same two documents it
shows, and the one stating `/PrintArea /MediaBox` lays a 400-unit square onto paper where the
silent control lays its 360-unit crop box.

## 4. `/PrintScaling`'s second sentence, re-read rather than carried forward

The row said the sentence — "[i]f the print dialogue is suppressed and its parameters are provided
from some other source, this entry nevertheless shall be honoured" — had "no path in this tree to
be owed against", on the ground that every window's print goes through a dialogue. **That ground is
incomplete**: `quorra_print` and `quorra_print_page` are a print with no dialogue at all, whose
sheet and resolution come from a C caller.

Read against the entry, that path honours it, and this is the reasoning a later round should not
have to redo. `quorra_print` applies **no page scaling**. That is `/PrintScaling /None` exactly.
It is also this library's stated default for `AppDefault`, which the entry defines as "the
interactive PDF processor's default print scaling" and nowhere requires to differ from `None`. And
it could not honestly be anything else: `quorra_print`'s `media` argument is the sheet *in the
page's own user space*, so the caller has already chosen the placement and there is no paper size
here to fit a page to. A caller that wants one states §12.5.6.22's matrix B itself through
`quorra_print_placed`, with Table 147's value answered to it by `quorra_preference` and Table 148's
`/Enforce` reported by `Defaults::enforcement_note` under `CLAUDE.md`'s restriction rule.

So §12.2 takes `departed` rather than `implemented`, and for one entry that has nothing to do with
printing: `/HideMenubar`, read, answered in words and deliberately not obeyed, because the only
menu bar these windows have is the one holding the reader's own restriction levels (ADR 1145).

## 5. What this does not close

No window opens a second document beside the first. The core has held a `BTreeMap<DocumentId,
Open>` and `Command::Focus` since it existed, and every host hard-codes one id; a round that gives
one of them tabs gets `/NewWindow true` almost for free, and section 1 says what it would have to
re-state rather than re-argue. Nothing here touches §12.6.4.7's `Thread` action, whose `/F` is
still refused — it names a bead in another file, and what a jump there needs is the same supply
path applied to a different table.
