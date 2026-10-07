# 1623 — Two premises that had expired, and six rows that moved

Session 1393. Status: **accepted**. Amends ADR 0098 (its decision on `/Order`) and ADRs 1122 and
1197 (their refusal to lay rich text formatting out); none is edited.
Context: ADR 1622 (the reading this came out of); `doc/questions/A72`; ADRs 0098, 1122, 1197, 1240;
`doc/third-party-data.md`'s XFDF section; ISO 32000-2 §7.10.2 (Table 39), §12.5.6.2 (Table 172),
§12.5.6.6 (Table 177), §12.7.4.3 (Table 228), §12.7.5.3 (Table 231), §12.7.8.3.2 (Table 249), §K.1.

## 1. §7.10.2: a choice is no longer a reason to decline

ADR 0098 left Table 39's `/Order 3` interpolated linearly because "[v]alid values shall be 1 and 3,
specifying linear and cubic spline interpolation" names a cubic spline and states none, and choosing
one would invent an algorithm. Two things answered that after it was written. `CLAUDE.md` principle 5
says that where the standard defines nothing a round makes a deliberate choice and documents it as
one, and `doc/questions/A72`'s bound — a choice is permitted where the clause names the kind of mark
and withholds only a quantity — is the owner's word for the shape. Here the clause names the kind,
a cubic spline, and withholds which; a linear ramp is no cubic spline at all. So the row is `partial`,
owed a cubic spline at a choice written down, with the clause's "If Size is less than 4, cubic spline
interpolation is not possible and Order 3 shall be ignored if specified" kept. The count is still 0
of 956 corpus functions and the crawl has never been asked, so a census is the build's first step;
it ranks the work and does not decide it. `doc/todo/65` bucket 6.

## 2. Rich text: the exclusion never reached it, and the text is held

ADR 1122 refused to lay a rich text value's formatting out on two grounds: XFA is `CLAUDE.md`'s
closed exclusion, and without its text the only source would be another reader's output. ADR 1197
answered the first — §K.1's permission is about schema-driven page generation, not an AcroForm entry
whose format the standard defines by pointing at another document — and kept the refusal on the
second, the formatting being stated in a specification this tree did not hold. That had stopped
being true: `doc/third-party-data.md` already recorded that the PDF Association hosts XFA 3.3 among
ISO 32000-2's normative references. Its direct link refuses a script, and the Internet Archive's
copy of 2026-08-19 of `https://pdfa.org/norm-refs/XFA-3_3.pdf` is the document — *XML Forms
Architecture (XFA) Specification Version 3.3*, January 2012, SHA-256
`a3344e7ef0b0da445bcce4323e689e646b31b64ecf1c318a8fc98f6b5edca01e` — now kept at
`/home/AI/specs/XFA-3_3.pdf`. Chapter 27 is its Rich Text Reference: XHTML elements with a restricted
set of CSS2 properties, a stated minimum a processor supports, and the rule that markup it does not
understand is ignored. The preface's Intellectual Property section permits writing software that
reads content in that architecture and displays it. It is cited by section and never quoted, and it
owes a section in `doc/third-party-data.md` before a round builds on it.

So the departure's premise is gone and five rows owe one build: §12.7.4.3 and §12.7.5.3 (a field's
formatting), §12.7.8.3.2 (Table 249's `/RV`, not imported because nothing would apply it),
§12.5.6.6 (Table 177's `/RC` and `/DS`) and §12.5.6.2 (Table 172's `/RC` in the popup). The last two
were `implemented` on the same refuted ground, which is why they move with the three. Their headings
— §12.5, §12.5.6, §12.7, §12.7.4, §12.7.5, §12.7.8 and §12.7.8.3 — derive `partial` (ADR 1599), and
§7.10 does for §7.10.2. `doc/todo/65` bucket 4 holds the five as one build.

## 3. What is left for the round that builds it

Code comments in `pdf-model` still give the exclusion as the reason — `form.rs`, `forms_data.rs`,
`popup.rs`, `appearance.rs`, and the owed sentence `xfdf.rs` prints for `<value-richtext>` — and are
the build's to correct with the code they describe.
