# 1417 — A chrome label is joined and ordered, and reads in its own direction

Session 1290. Status: **accepted** and built.
Context: `crates/pdf-font/src/shaping/label.rs` (new: `Label`, `Glyph`), `crates/pdf-font/src/shaping/mod.rs`,
`crates/viewer-ui/src/chrome.rs` (`Chrome::laid_out`, `caret`, `width`, `text`, `without_a_code`,
`elide`, `FindBar::draw_labelled`), `crates/viewer-gtk/src/host.rs` (a test).
Builds on: ADR 1413 (UAX #9 by `unicode-bidi`), ADR 1414 (joining by the UCD's tables), ADR 1406
(the machine face asked off the drawing thread). Clauses: ISO 32000-2 §12.3.3, §12.4.2, §14.9.2.1,
Table 147, §7.9.2.2.1.

## 1. What each window drew

Fixtures: an outline whose titles are "الفصل 12: السلام" (a lam-alef, a number, a colon) and
"مقدمة (أ)", one whose titles are "פרק 3 שלום" and "מבוא (א)", and a file named `ملف.pdf`, driven
under `Xvfb` in all three windows. `quorra-gtk` (Pango) and `quorra-qt` (`QTextLayout`) drew every
label joined, in UAX #9's order, the brackets mirrored, the tab reading `pdf.ملف` — and each set
the label at the row's left edge. `quorra` drew the stored characters one after another, left to
right: every Arabic letter in its isolated form, the words reversed, `(أ)` unmirrored, and the
tab's Arabic letters unjoined before `.pdf`.

## 2. Decision

- **A chrome label goes through `pdf_font::shaping::Label`**: section 9.2's joining over the whole
  label in logical order, then UAX #9 with the label as one paragraph and rule L1 over it as one
  line, then L2 and L4. `Chrome::laid_out` is the one place a label becomes glyphs, so `width`,
  `text`, `caret` and `without_a_code` cannot disagree about it; each displayed character is then
  asked of the compiled-in face and, failing that, of `viewer_host::machine_faces` off the drawing
  thread, as any character is (ADR 1406). A presentation form is simply the character asked for.
- **The find bar's caret** stands at `Label::boundary` of the typed string's end, ADR 1413's
  convention: at the left end of a right-to-left string, after "12" typed into Arabic. The bar only
  appends and removes at the end, so no other offset is asked; nothing else in the chrome is edited.
- **An elided label is checked as drawn**: the cut is measured letter by letter in isolated forms,
  and the joined result with its `…` is measured again and shortened until it fits.

## 3. The direction is the label's own — a choice, recorded as one

The standard is silent here, and each place it might speak was read. §12.3.3's `/Title` is "[t]he
text that shall be displayed on the screen for this item": what, not which way. §7.9.2.2.1 makes a
text string Unicode — "UTF-16BE, UTF-8 and Unicode character encoding are described in The Unicode
Standard by the Unicode Consortium" — and that standard's UAX #9 is in ISO 32000-2's bibliography.
§14.9.2.1 does reach outline entries — "This applies to both content within content streams and any text strings, including text strings not included in the structure hierarchy such
as, for example, entries in metadata, outline entries and names for optional content groups" —
but what it applies is a natural language, and a BCP 47 tag is not a direction. Table 147's
`/Direction` "has no direct effect on the document's contents or page numbering". §14.8.2.5.3 is
about show strings in a content stream, which a label is not. A file name and this program's own
words are not the document's at all.

So **each label is one paragraph whose direction is found by rules P2 and P3 from its own first
strong character**, never the window's, the document's `/Lang` or `/Direction`. That is what UAX #9
does with no higher-level protocol, and it is what Pango and Qt did with the same strings, so the
three windows agree. **Every line is anchored at the left** whatever its direction, because this
window's layout is left to right and both toolkits did the same; a right-aligned row among
left-aligned ones would stop reading as one list. A popup's wrapped lines are each one label.

## 4. Measured

- `quorra --trace=frames,launch`, release, `Xvfb`, `llvmpipe`; "cold" is every file under
  `/usr/share/fonts` evicted with `posix_fadvise(DONTNEED)` first; three launches each, load 2 to 5.
  An outline panel open at launch (`/PageMode /UseOutlines`) with the two Arabic items:
  first present before 127.7 / 127.9 / 125.5 ms cold, 150.0 / 134.2 / 135.5 warm; after 121.4 /
  123.7 / 124.2 cold, 119.0 / 119.9 / 131.6 warm. The heaviest host step of any frame in four
  seconds: 0.5 ms before, 0.4 ms after. Two Latin items for scale: 92.8 to 138.4 ms before, 110.0 to
  155.7 after. No search is in front of first present: the forms are asked of ADR 1406's thread.
- `Label::new`, release, 10 000 runs: 0.87 µs for "Chapter 3", 2.66 µs for a 53-character Latin
  title, 2.47 µs for the Arabic one. A panel of forty rows costs about a tenth of a millisecond a
  frame for it, inside the noise of the host step above, so no Latin fast path is taken.
- Tests: `label::tests` (the lam-alef worked from the tables, a number in an Arabic label, the
  direction from the first strong character, a mirrored bracket, the caret); `chrome::tests`
  (a Hebrew label drawn as its letters reversed, the caret, an Arabic label's forms from the
  machine) — all three chrome tests fail with the reordering planted out.

## 5. Left

A face whose Arabic is reachable only through its `GSUB` is not chosen for a form (ADR 1414); a
label mixing faces per character may change face inside a word where the first face found lacks a
form; a popup's wrapped paragraph takes its direction per line rather than per paragraph.
