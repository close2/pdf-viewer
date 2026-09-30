# The interface's own font, and the text it cannot set

Status: **most of it was never a font question**, and the four-hundred-and-ninety-first session
closed that half. A *script* this binary does not carry is set from a face the machine offers (ADR
1382); what is open is a machine that offers none, which is a decision the project owner has not been
asked for — and the demand for it is four documents.
Priority: 27
Corpus: **13 documents** still state something in a panel this program cannot set, out of 54 that
did. What remains is Hebrew, Thai and CJK, plus one malformed file's U+FFFD.
Clauses: §9.6.2.2, §9.6.5, §12.3.3, §12.5.6.14, §14.3.3
Code: `crates/viewer-ui/src/chrome.rs` (`Chrome::set`, `Chrome::text`, `Chrome::width`),
`crates/pdf-font/src/loading.rs` (`LoadedFont::character_glyph`),
`crates/pdf-model/examples/interface_font_census.rs`,
`crates/viewer-ui/examples/chrome_coverage.rs`

## What it is

Everything this program draws for itself is set in §9.6.2.2's fourteen, compiled into the binary
(ADR 0133) — which is what makes an interface reproduce on a machine with no fonts installed, and
is the right default. A character those faces do not state is drawn as a **box**, 0.6 em wide,
which advances and is counted, so five characters of Japanese are five boxes rather than an empty
row and §12.5.6.14's popup says *how many* under its note (ADR 0195, ADR 0191).

## What is answered

**The coverage question was mostly an encoding question, and it is closed** (ADR 0326). A panel
asked the face for a character *code*, and a simple font's codes are one byte — so the route
reached §9.6.5.2's `StandardEncoding` and stopped at 149 characters, while the compiled-in
Helvetica is Liberation Sans and states 668. The interface was drawing a box for `é`. A character
with no code is now looked up in the face by character (`LoadedFont::character_glyph`), and only
what the face does not state at all is a box.

**The demand is measured**, by `pdf-model --example interface_font_census`, which opens every
corpus document and asks the seven populations a program draws from one — §12.3.3's outline
titles, §8.11.4.3's layer names, §7.11.4's file names, §14.3.3's `/Info`, §14.3.2's XMP, §12.4.2's
page labels and §12.5.6.14's popup text — with each character asked of *both* routes. It is
deliberately not routed through `Chrome`, which is the code under test;
`viewer-ui --example chrome_coverage` is that other question and is kept.

Of the 54 documents whose panels lost a character, **41 lose nothing at all now**. 130 of the 144
characters recovered are Latin-1 Supplement — so the commonest thing this interface could not set
was a French or German word, not a foreign script.

## What is not answered: a script the binary does not carry

Thirteen documents, and the census names every one of them. By script: 213 characters of Hebrew,
all of them in `issue14046.pdf`; 81 of Thai, all in `issue13211.pdf`; 85 of Japanese and Chinese
over six documents, the largest `issue2884_reduced.pdf`; and 77 of U+FFFD, mostly in
`bug1146106.pdf` — which is a *report about the file* rather than a coverage gap, because it
writes its text strings as UTF-16 little-endian and §7.9.2.2 admits no such encoding — with the
remainder in fuzzed files.

Two answers remain, and neither is obviously right at this size:

1. **Compile in a face with the coverage** — a licence question, a size question (a CJK face is
   megabytes against the standard fourteen's 804 KB) and a decision the project owner has not been
   asked for. A CJK face buys six documents in 964 and a Hebrew or Thai one buys a single document
   apiece, which is a worse trade than when this file was written and priced them against 74.
2. **Ask the host.** A native host draws these rows in a `QTreeView` or a `GtkListView` with the
   platform's own font stack, which has the coverage and did not have to ship it — so on the hosts
   `doc/todo/30` is about, this whole item is `viewer-ui`'s alone. That is an argument for not
   spending a megabyte on it here.

**`quorra` now asks the machine for a character the compiled-in faces do not state** (ADR 1382),
the same covering search a substituted composite font uses, and draws the box only where the machine
offers no face. Every character the fourteen state is still drawn from the binary, so ADR 0133's
argument holds for all of them; what differs from machine to machine is only what was a box on every
machine. `Chrome::compiled_in_only` is the chrome with no machine face behind it. So on a machine
with the faces, the thirteen documents above lose nothing in `quorra`'s panels either, and the two
answers above are now about a machine that has none.

**The search runs on a thread of its own, never on the thread that draws** (ADR 1406):
`viewer_host::machine_faces` answers a character from the faces already found or queues it, the
character is the box until the answer lands, and the window is woken to draw it. Measured cold on
this machine, the catalogue walk is a second, and a document opening on a panel holding one such
character had put it in front of first present. **`quorra-qt` takes the same answer**: Qt falls
back by family, eleven faces here share `Droid Sans`, and the strip and the panels drew boxes until
the host handed Qt the file the search names (ADRs 1406, 1418). `quorra-gtk` needs nothing: Pango
draws it.

**A right-to-left label is joined and ordered in `quorra` too** (ADR 1417): `Chrome::laid_out`
passes every line of chrome through `pdf_font::shaping::Label` — the Unicode Standard's joining,
then UAX #9 with the label as one paragraph whose direction is its own first strong character's —
and asks for each displayed form as it asks for any character. Pango and Qt draw the same forms in
the same order from the stored text, which is why the shared rows carry it unshaped.

**A word is set in one face, and a wrapped line in its paragraph's direction** (ADR 1430). The
characters of a word that the compiled-in faces lack are asked of the machine together
(`MachineFaces::ask_word`), so a cursive word no longer changes face where the first face found
lacks one of its forms; only where no single face states the word is each character asked alone.
The search ranks the faces that qualify by the style asked for before their repertoire
(`pdf_font::substitute::installed_covering_styled`), so a word is not set in an extra-light face
beside regular ones. And a paragraph `quorra` wraps — a popup's text, a card's sentence — is resolved
by UAX #9 as one paragraph and laid out a line at a time (`Label::line_of`), so a line that begins
with a word of the other direction still reads in its paragraph's. `quorra-qt` registers its faces
from the same search; `quorra-qt` hands none to Qt before the first frame is on the screen (trap 70,
ADR 1429).
