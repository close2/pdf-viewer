# ADR 0214 — A tree a screen reader can read, and the runtime it costs

Status: accepted, 2026-08-07 (session 376). Closes the fifth and last of the five items
`doc/HANDOVER.md` §0 listed as blocked on the `viewer-core` boundary.

## Context

`Query::AccessibilityTree` has answered since the hundred-and-forty-ninth session with §14.7's
elements, §14.9's spoken form of each and the quadrilaterals they cover (ADR 0134). **Nothing
consumed it for two hundred and twenty-seven sessions.** "Speak a page" was the one thing
`doc/HANDOVER.md`'s "Where we are" still listed as missing, and it was a *host* rather than a
vocabulary.

`CLAUDE.md` names AccessKit with AT-SPI as this project's accessibility layer. Two things had to
be decided in writing before the first line, and a third turned up while writing it.

## Decision 1 — take `accesskit` and `accesskit_unix`, and confine the runtime to one crate

`accesskit_unix` reaches AT-SPI through `zbus`, which is asynchronous, and `CLAUDE.md` forbids an
async runtime "unless something genuinely requires one". This one does: AT-SPI *is* D-Bus, there
is no synchronous D-Bus client worth having, and writing one would be a second implementation of
the protocol beside the one every desktop already uses.

The four questions ADR 0014 asks of a dependency, and the fifth this project's startup rules add.

**Which crates.** `accesskit` 0.24.1 and `accesskit_unix` 0.22.1, both MIT OR Apache-2.0. They
bring **61 packages** this tree did not have, every one of them MIT, Apache-2.0, Zlib or
Unlicense-or-MIT; `cargo deny check` is clean on all four checks with no new exception in
`deny.toml`. The alternative considered was `accesskit_winit`, which would have added a *second*
copy of winit's version constraints to satisfy for no benefit — this host already owns its
window's events and needs only the adapter.

**What `memchr` does here, because ADR 0186 refused a crate over it.** That ADR declined
`quick-xml` because `memchr`'s SIMD paths are its `unsafe` and taking it would have put `memchr`
into a shipped viewer for the first time. `memchr` appears in this graph too — through `winnow`,
`toml_parser` and `proc-macro-crate` — and **it is a build dependency of a proc macro**:
`cargo tree -e normal` shows it nowhere. The rule ADR 0186 stated is intact and this dependency
does not test it.

**What it does on the launch path — measured, before and after.** Under `Xvfb` on this machine's
software adapter, `pdf-viewer --trace doc/PDF20_AN001-BPC.pdf`, nine launches of each binary
alternated on an otherwise idle machine:

| | before | after |
|---|---|---|
| first present, median | **113.8 ms** | **114.1 ms** |
| first present, mean | 113.6 | 115.2 |
| first present, min | 104.3 | 106.6 |

and the timeline's own steps, medians over the same nine launches each, which is the number that
matters because it says *where* any cost would be:

| step | before | after |
|---|---|---|
| arguments | 0.022 | 0.024 |
| chrome fonts | 1.470 | 1.555 |
| event loop | 24.802 | 25.047 |
| window | 0.159 | 0.146 |
| graphics instance | 3.814 | 2.720 |
| graphics device | 15.483 | 15.450 |
| document joined | 4.575 | 5.175 |
| first present | 62.857 | 64.383 |

**No step moved.** The executable grew 2.5 MB (16.96 to 19.47), which is the only mechanism by
which this could have cost anything, and 0.3 ms of median is what it costs.

**Both measurements were taken twice and the first pair is worth recording as a lesson about the
instrument.** An earlier set of twelve launches each, taken while the corpus gates were running on
the same machine, put the medians at 134.2 and 135.2 with the minima 20 ms apart — a difference
that vanished on a quiet machine, on a binary whose changed code cannot execute before the
timeline closes. `doc/HANDOVER.md`'s "Measuring" says this about wall clocks and it is still true:
the *steps* are the robust statistic here, because a step is bounded by what it does and a total is
bounded by whatever else the machine is doing.

**The reason there is nothing to find is structural, and it is the fifth question.** `Bridge::new`
is called from `redraw_requested`, **after `Launch::arrived` has closed the timeline**. Nothing
about D-Bus, no thread, no connection, exists until the first frame is on the screen. Beyond that
line the design is `accesskit_unix`'s own and it is the right one: creating an adapter spawns one
thread, that thread connects to the session bus, and every adapter stays *inactive* — publishing
nothing and asking this program for nothing — until `org.a11y.Status.IsEnabled` says an assistive
technology is present. Publishing a page costs a lock and a clone when nobody is listening.

**What confines it.** `viewer-accessibility` is the only crate that may name `accesskit_unix`, and
its manifest makes that dependency `cfg(target_os = "linux")`. Nothing on the render path, the
parse path or the launch path depends on the crate at all. The half that maps §14.8.4 onto
AccessKit is platform-free and is compiled and tested on all three targets.

**What the other two platforms say.** AccessKit has macOS and Windows adapters and this program
does not use them, because nothing here can test one. That is named rather than absent:
`Bridge::shortfall` answers with a sentence `pdf-viewer` prints in its first lines, exactly as
`pdf_sandbox::Confinement::shortfall` does for the two platforms with no kernel confinement (ADR
0194). Both cross-target checks build under `-D warnings`.

## Decision 2 — the mapping, and the fourteen places two vocabularies do not line up

§14.8.4 defines forty-one standard structure types; `accesskit::Role` has about two hundred, from
ARIA, the DPub module and three desktop trees. The mapping is mostly mechanical and it is a
mapping between two vocabularies, so `viewer_accessibility::role`'s module documentation writes
out **every place a distinction is lost** rather than defaulting it to `Role::Unknown`. The table
is there and is not repeated here; the three decisions in it worth stating are:

- **`Part` becomes `Group`, not `DocPart`.** The DPub `doc-part` is "a major thematic section",
  which asserts the hierarchy §14.8.4.4 says a `Part` is explicitly *without*: "[e]ncloses a
  grouping of structure elements without consideration for their hierarchy."
- **`Form` becomes `Group`, not `Role::Form`.** §14.8.4.7.2 makes the `Form` structure type **one
  widget annotation**, so the AccessKit role of the same name would be wrong twice: it means a
  container of fields, and this is a field. What it should become is the widget's own control
  role, which needs §12.7's field type behind §14.7.5.3's `/OBJR`; that is what `doc/todo/31` now
  owes.
- **A type §14.8.4 does not define becomes `Group` with a description naming it.** The role map
  has already been applied by then, so §14.8.4.1's requirement — "[a]ll structure elements
  occurring within a tagged PDF document shall have a type matching one of those defined as a
  Standard Structure Type, or a role map providing a mapping from the non-standard type to a
  Standard Structure Type" — has been unmet by the *document*. A person whose reader says "group"
  is owed the reason, and dropping the name would have hidden which side the defect is on.

**One rule came out of the library rather than the standard.** `accesskit_consumer`'s
`common_filter`, which every AccessKit platform adapter applies, removes a `GenericContainer` or a
`TextRun` node and lifts its children — which is exactly what `NonStruct` asks for in its own
words ("it should not be interpreted or exported to other document formats. Its descendants shall
be processed") and is also a way to lose text, because a node's name goes with the node. So the
mapping takes a second input, whether the element has anything of its own to say, and a type that
would be filtered out becomes a `Label` when it does. **`Artifact` is the one exception and it is
deliberate**: §14.8.2.2.1 makes an artifact content that is on the page and is not the document's,
and a running head read aloud on every page is what tagging exists to prevent.

## Decision 3 — what this program refused is in the tree

A page with an unreported gap is one thing; a page whose text is not drawn at all is another, and
the person who cannot see the page is the one person for whom "the title bar says one item was not
drawn" is no answer. `Query::Reports` already answers with the current page's refusals in the
words `viewer_core::report` chose for a person, and they cross as a `Role::Status` group — AT-SPI's
`StatusBar`: advisory, findable, and not an alert that interrupts — with one node per item. A page
with nothing to report grows no such group.

**And an untagged page says so.** 885 of the corpus's 974 documents state no structure tree, and
what crosses then is one node saying that in this program's own words. Deliberately **not** an
invented structure over the text layer: reading order is precisely what §14.7 exists to state, and
a reader that guessed one would be presenting a guess where a person is entitled to the author's
answer.

## What the round found in `viewer-core`, which was not on anybody's list

Three things, each a claim the code did not keep.

**§14.7.3's role map was not applied, against a `shall`.** ISO 32000-2 §14.7.3:

> A structure type shall always be mapped to its corresponding name in the role map, if there is
> one, even if the original name is one of the standard types.

`pdf_model::structure::Tree::role` has done this since the seventy-eighth session, transitively and
through §14.8.6.2's namespace maps. `viewer_core::accessibility` read the raw `/S` past it, on a
sentence in ADR 0134 and in two ledger rows: "[t]he role is handed over as the document states it
rather than mapped through the role map: a host that knows its platform's vocabulary is better
placed to map `H1` or `TD` than this tree is." **That is two claims wearing one coat**, which is
the shape `doc/HANDOVER.md` names under "a reason that names an architecture". §14.7.3's role map
is the *file's* statement about its own names; §14.8.4's set onto a platform's roles is the host's.
Only the second was ever the host's.

**The answer was the whole file's structure, not the page's.** ADR 0134 said "an element belonging
to another page is skipped rather than answered with, because a screen reader is being told what is
on the screen", and the code filtered only §14.7.5.2's marked-content children by page. Every
*element* in the document was answered with, bounded at 8192 — so a thousand-page tagged document
handed a screen reader thousands of empty nodes. An element is now kept when it, or something below
it, names a content item on the page being asked about, through §14.7.5.2's sequences or §14.7.5.3's
object references, and the parent links are repaired.

**An element spoke its whole subtree.** `AccessibilityNode::name` was the text of everything the
element enclosed, so a paragraph and the span inside it both said the span's words. That is the
opposite of what a platform tree wants — text belongs to the node that carries it — and it was
built that way to make §14.9.3's `/Alt` mean something. It does not need to be: `/Alt` *replaces*
the element, so the accumulated text is never the thing to say. `name` is now the element's own
content items, `quads` are still everything it encloses (a focus ring goes round the whole cell),
and a new `substituted` field says which of the two kinds of name this is — so a host stops
descending where the author has already spoken for the subtree.

## How far it was verified, and by what

**End to end, on a real bus, in this environment.** `dbus-run-session` for a session bus,
`at-spi-bus-launcher` for `org.a11y.Bus` (whose `IsEnabled` is already true), `at-spi2-registryd`
for the registry, `Xvfb` for a display, and `busctl` walking `org.a11y.atspi.Accessible` from the
registry's root — a real client, not this program's own types. `doc/PDF20_AN001-BPC.pdf` page one
comes back as:

```text
[Application] 'pdf-viewer'
  [Frame] 'PDF 2.0 Application Note 001: Usage of Black Point Compensation — Cover — page 1 of 5 …'
    [DocumentFrame] 'PDF 2.0 Application Note 001: Usage of Black Point Compensation'
      [Panel] 'page Cover (1 of 5)'
        [DocumentFrame] ''
          [Image] 'PDF Association logo'
          [Paragraph] 'A ppl ication Note'
          [Paragraph] '\nPDF 2 .0 A pplication Note 0 01: \nB lack  Point  Compensation'
          [Paragraph] '\n2018-09'
          [Paragraph] 'PDF TWG'
          [Paragraph] ''
            [Label] '\n© 2018 PDF A ssociation – '
            [Link] 'pdfa.org'
          [Paragraph] '\nThis work is licensed under CC-BY-4.0 '
            [Image] 'Creative Commons'
```

One press of the right arrow through `xdotool` turns it into `[Panel] 'page Copyright (2 of 5)'`
with page two's paragraphs under it — §12.4.2's own label, not a number. And
`issue13316_reduced.pdf`, whose font "has no outline for any of the 5 code(s) the page shows
through it, so the text it states is not drawn", comes back as an untagged page that says so with a
`[StatusBar] '1 thing(s) on this page were not drawn as the document specifies'` beside it carrying
the reason.

**The first end-to-end run found a defect no unit test could have.** Every `Label` node arrived on
the bus with an empty name. `accesskit_consumer::Node::label_comes_from_value` is
`self.role() == Role::Label` and nothing else, so the one role this mapping uses for text that is
only text is the one whose *label* an assistive technology never reads — its name comes from its
`value`. `tree::say` is that rule and `a_static_text_node_puts_its_text_in_the_value` is the test.
A gate that stopped at the `TreeUpdate` would have passed.

**What is not verified here.** No screen reader was run: Orca is not installed on this machine, and
a real reading is a person's judgement rather than a byte comparison. What a person on a desktop
should run is `orca` beside `pdf-viewer` and listen; what a developer can run without one is the
recipe above, which is what `doc/HANDOVER.md`'s "Verify it" now carries.

## What is left, and it is written down rather than implied

`doc/todo/31` keeps four things: the `TH` header cell's axis, which §14.8.4.8.3 leaves to the
`Scope` attribute; the `Form` element's control role, which needs the widget behind its `/OBJR`;
AT-SPI's `Text` interface, so that a screen reader can move through a paragraph by word and
character rather than hear it whole; and the actions an assistive technology may request, which
this tree declares none of and reports by name if one arrives anyway.

## The lesson

**A sentence that explains why something is the host's job should be split into one claim per
mapping before it is believed.** ADR 0134's refusal to map the role was right about the platform
vocabulary and wrong about the file's own role map, and it stood for two hundred and twenty-seven
sessions in an ADR and two ledger rows because the two were never separated. It is the same shape
as ADR 0168's `NoZoom`/`NoRotate` and the same remedy: split a refusal into one claim per entry
before believing it.
