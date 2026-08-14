# 522 — The two refusals an annex outlived

**Finding.** Annex O's last two reported parameters were asked `doc/habits.md`'s sweep question —
*has the capability the refusal names arrived?* — and both answered yes, neither in the way its own
sentence predicted. `highlight` said "no host has asked for one to draw": the **annex** asks, which
ADR 0316 established is a legitimate reason for a message, and this one passes `doc/ui-boundary.md`'s
test twice — no host sees the fragment, which arrives inside `Command::Open` undecoded, and no host
may redo the page-to-device arithmetic ADR 0118 keeps in one place. `fdf` said "fetching a URI is
the host's, and no host supplies one yet": the first half describes the boundary working rather than
a blocker, and the second stopped being true when `Event::NeedsFile` reached three hosts for
§12.7.6.4's import action. **`Parameter::unhonoured` now names none of Annex O's eleven**, and no
network was needed for the second: a relative URI is a file beside the document and
`viewer_host::resolve_import` was already the policy.

**Date.** 2026-08-14.
**ADR.** [0357](../adr/0357-the-two-refusals-an-annex-outlived.md).
**Touched.** `crates/pdf-model/src/fragment.rs` (`unhonoured` answers `None` for all eleven, `Fdf`'s
own documentation, the test that used to list the two refused), `crates/pdf-model/src/action.rs`
(`data_format`, one reading of the extension for the two clauses that ask),
`crates/viewer-core/src/{query,viewer,open}.rs` (`Query::Highlight`, `Answer::Highlighted`,
`Open::highlights`, `Open::highlight`, `Open::import_from`, the `NeedsFile` raised as a document
opens), `crates/viewer-core/tests/fragments.rs` (four tests added, one re-aimed),
`crates/viewer-confined/src/{lib,protocol}.rs` (the question and the answer on the wire, both ways),
`crates/viewer-ffi/` (`pdfv_highlight_quads`, the header line, the two counting tests),
`crates/viewer-ui/`, `crates/viewer-gtk/`, `crates/viewer-qt/` (the overlay in each host's own
colour), `doc/conformance/ledger.toml` (§O, §O.2.2, §12.7.6.4), `doc/ui-boundary.md`,
`doc/crate-map.md`, `doc/todo/39`, `doc/adr/0357-*` (new), this file.

## What the annex-o command said, before and after

```
carried out: Page NamedDestination StructureElement Comment EmbeddedFile Zoom View ViewRect Search
reported:    Highlight Fdf
```

```
carried out: Page NamedDestination StructureElement Comment EmbeddedFile Zoom View ViewRect Highlight Search Fdf
reported:
```

## The two readings, briefly

**`highlight` is not the selection and the annex is what says so** — its two neighbours that point a
person at something say *selected* and this one says *highlighted*, with a nature "implementation-
dependent" that the other two are not given. ADR 0310 had already re-derived that much from the
annex; what expired was the clause after it. The rectangle now lives on `Open` in default user space
against the page the fragment had selected when it named it, `Query::Highlight` maps it through the
one `device_quad`, and three hosts wash it in a colour of their own — which is what an
implementation-dependent nature means at a boundary where chrome crosses as geometry.

**`fdf` needed a host, not a network.** "The URI shall be either a relative or absolute URI", and a
relative one resolves against the document's own — the directory it is in, which
`viewer_host::resolve_import` already answers for §12.7.6.4. So the fragment sets an import in
flight, `Viewer::open` raises `Event::NeedsFile { purpose: ImportData }` after the first page's
events (the annex asks for `fdf` last "so that the document can open directly to the appropriate
view"), and the host supplies. XFDF is declined by name for want of an XML parser, which belongs to
§12.7.6.4's row and not to this annex's.

## Seen working, in a window

`doc/habits.md` names "a capability that reached the crate and never reached the program" as a shape
this project keeps being caught by, so both were driven under `Xvfb` against the release binary.

`pdf-viewer 'doc/PDF20_AN001-BPC.pdf#page=1&highlight=100,400,150,300'` draws a green wash from 100
to 400 across and 150 to 300 down from the page's **top** left corner — over the title, with the
glyphs still readable through the `Multiply` blend, and nowhere near where the same numbers measured
from the bottom would have put it.

`pdf-viewer 'form.pdf#fdf=answers.fdf'`, with a two-line FDF written beside a copy of
`form_two_pages.pdf`, prints *this URI's fragment asks for the form data in answers.fdf, which the
host is being asked for* and then *import-data: 1 field(s) from answers.fdf, into 1 widget(s)*, and
the window shows the text field reading **Ada Lovelace** — a URI's fragment, a host's policy, and
§12.7.8's reader, end to end.

## Two things worth keeping

**A refusal can be right about the standard and wrong about the tree in one sentence.** Both of
these were: the reading in front of the dash survived and the claim after it had expired. Splitting
a refusal at its conjunction is `doc/habits.md`'s third shape, and it caught both of these where
reading the whole sentence as one claim would have kept them.

**A function that answers nothing is kept on purpose.** `Parameter::unhonoured` has no refusal left
and stays, because the answer decays in the other direction too — and because `tools/state.sh
annex-o` reads its text. That is the round's one lint exemption, and its reason names the tool.
