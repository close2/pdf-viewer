# 1297 — A window is what the document asked, and a word is one face

The HOST-UI round of batch forty-six. ADRs 1429 and 1430. No row changed status; §12.2's note was
rewritten as what is. No question was written.

**Census.** `accessibility_census.rs` and `selection_census.rs` now read `corpus_passwords.rs`
through a `#[path]` (ADR 1377). `issue21579.pdf` left `REFUSED_OPEN` (3 to 2). Two floors moved:
"untagged pages answering honestly" from 876 to 877 (tracked) and from 889 to 890 (whole). Every
other floor held with 0 slack. The selection census compares 967 readbacks, and `issue21579` is
among them. Both censuses exit 0 behind the lock.

**Table 147, driven under `Xvfb`.** Three fixtures (all six entries, none, `/DisplayDocTitle`
alone), plus a landscape page with `/FitWindow` and `/CenterWindow`.
- Before: toolbar and status hiding worked; `/HideMenubar` was kept, with ADR 1145's sentence.
  Neither native window obeyed `/FitWindow` (both stayed 1000×1100), `/CenterWindow` (both at
  0,0) or `/DisplayDocTitle` (both showed the file name). `quorra` claimed the others "obey all
  three".
- After: all three windows show the `dc:title`. `quorra` is 750×1000 at 325,100, `quorra-qt`
  1000×856 at 200,172 and `quorra-gtk` 1000×868; each viewport now equals the page drawn in it.
  `quorra-gtk` says it cannot centre. GTK needed its page `GtkFixed` moved out of measurement. A
  GTK form field still takes typing.

**Labels.** Urdu "کہانی" in an outline was drawn in two faces, per character. It is now drawn in
one face, and in regular weight: the first single-face answer was extra-light, which is why
`installed_covering_styled` exists. A popup paragraph "Notes on the chapter titled שלום עולם."
wraps; its second line keeps the full stop at the right. Each test fails with its fix planted out.

**Launch (release, `Xvfb`, load 6–11).** `quorra-qt`, before the hold, Chinese outline title: first
frame 131–265 ms, and in one run of four the face was handed to Qt before the first frame
(0.235 s against 0.265 s). After, 5 runs: 131–341 ms against Latin 54–156, and every face was
handed at or after the first frame. The Chinese gap remains with no face handed, so it is Qt's own
first paint. `quorra`: first present Latin 105–140, Urdu 114–123, Chinese 112–138 ms; first-frame
host step 0.1–0.2 ms.

**Gates.** fmt, clippy (6 crates), tests (6 crates), `conformance`, `launch_path` (26 banded, 0
outside), `accessibility_census` and `selection_census`: all exit 0.

**Left.** `installed_covering`, the page's own search, still ignores weight. `chrome_coverage`
opens with no password.
