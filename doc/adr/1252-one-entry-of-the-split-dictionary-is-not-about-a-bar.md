# ADR 1252 — One entry of the collection split dictionary is not about a bar

Status: accepted, 2026-09-22. Session 1207, the batch's host-UI round.

Table 158's `/Direction` `N` is obeyed in all three windows; its `H`, `V` and `/Position` stay
departed, and Table 157's `/Colors` with them — but on a reason that survives the fact ADR 1168's
did not. Amends ADR 1168 section 2's last paragraph; the property that ADR states, and everything
it decided about `/Sort` and `/Layout`, stands.

## 1. The premise that was false

ADR 1168 put `/Colors` and `/Split` on the platform's side of its own property and wrote that both
"describe a widget none of the three windows has". Checked against the tree, the second half of
that sentence is wrong: `viewer-gtk` has held a `gtk4::Paned` between the panels and the page since
before that ADR was written, and `viewer-qt` a `QSplitter`. Two of the three windows have exactly
the widget the clause names.

A second claim of that round's has also gone stale in both ledger rows: they say `/Colors` and
`/Split` are "named by `unsupported_presentation` rather than dropped in silence", and neither
entry was ever mentioned by that function or by any other. They are named now
(`panel::unused_furniture`), which is what those sentences claimed.

## 2. What `N` actually says, and why it is not furniture

§12.3.5.1 introduces the splitter in a `may`: "the available display area **may** be divided by a
splitter bar into two areas; one area containing a display of the navigation controls of the
collection … and one area containing a preview of the initial or currently selected document of the
collection." Table 158 then states the bar's initial view. Two of its `/Direction` values describe
where that bar goes. The third does not:

> N indicates that the window is not split. The entire window region shall be dedicated to the file
> navigation view.

That is a `shall` about **what the window shows**, and it holds whether or not a processor has a
bar to put anywhere. A window with no splitter can obey it; this program's windows now do. The
panel takes the window's region, the files tab is selected, and the page's half is taken out rather
than hidden — `viewer-gtk` removes the `GtkPaned`'s end child, which is the move `apply_chrome`
already makes in the other direction and for the same reason, and `viewer-qt` hides the
`QSplitter`'s second widget. Both are put back for any document that asks for anything else, so
nothing survives a second file. `viewer-ui` draws its own sidebar and takes a width.

**Not while `/View` is `H`.** That value says "[t]he collection view shall be initially hidden", and
a view a document asked to hide is not one the window is dedicated to; the two entries would
otherwise ask for opposite things at once. Table 153's own "No splitter shall be used if the View
key has a value of H or C" is met by construction, since this program never places a splitter from
what a document said.

## 3. Why `H`, `V` and `/Position` are still a departure

Not because the widget is missing — it is not. Because of what the two areas *are*. §12.3.5.1's
second area is "a preview of the initial or currently selected document of the collection"; this
program's second area is the **container's own pages**, which ADR 0202 decided and §7.6.7's
unencrypted wrapper is the argument for: a wrapper's whole purpose is a page saying the payload is
encrypted, and a window that replaced it with a preview of an attachment would hide the sentence
that document exists to show. A percentage whose subject is the other pair of areas is not a
percentage about this one, and setting it would be obeying the number while disobeying the sentence
it belongs to.

`/Colors` is unchanged and needs no new reason: it is "a suggested set of colours for use by a
collection layout" and the only sentence about using them is a NOTE — "[i]t is recommended that a
layout use the colours provided" — which states no requirement. Each window paints in its own
platform's colours.

## 4. What is said out loud

`panel::unused_furniture` names both where a document states them, in one sentence for all three
windows on ADR 0711's reason: an entry read and silently unused is indistinguishable from one
nobody read. `N` is **not** in that sentence, because it is obeyed.
