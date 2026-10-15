# 1960 — Viability is judged for the hand-off, and a form XObject clip is the player's

Session 1562, slot 5 of batch eighty-nine. Status: **accepted, built**. Context: RFC 0009 §3.5 and §12
item 3; `doc/questions/A254`; ADR 1946 §3; ISO 32000-2 §12.6.4.14, §13.2.1, §13.2.2, §13.2.3,
§13.2.4.2, §13.2.4.3, §13.2.7.2.2, §14.9.2.4, Tables 218 and 277–294; `crates/pdf-model/src/media.rs`.

## 1. Whose viability

§13.2.2 says "the viewer shall determine the viability of the objects used", and viability is always
relative to a player. This program plays nothing. Its one player is the **hand-off**: whichever program
this machine starts for the clip's content type, given the file or its address and nothing else. So
`pdf_model::media::judge` reads the clause's rules against that player and against a `Processor` that
holds only what a host has said about its reader. Three consequences follow from the clause, and none
of them is chosen here:

- **An option a player would have to be told** fails in MH and is ignored in BE. That covers Table
  287's /BU, Table 289's /B and /E, and every entry of Tables 291 and 294. The clause's own words are
  that an MH option "shall be honoured; otherwise, the containing object shall be considered non-viable".
- **A criterion whose fact nobody stated** is one this processor cannot show to be met. That covers
  Table 279's /A, /C, /O, /S, /R, /D and /Z, a non-empty /V (no software name names this program),
  and a non-empty /P (this program fully supports no version of constructs it does not play). /L is
  matched against the host's `--interface-language`.
- **§13.2.7.2.2's algorithm for a player that no software identifier names.** Conditions (a) and (b)
  cannot hold. Condition (c) refuses the hand-off wherever a /MU is non-empty, and condition (d)
  refuses it where no /CT is stated.

The alternative was to call every clip non-viable because nothing plays. It was rejected because the
entry admits the hand-off as the thing a reader gets, and judging viability against no player at all
would have made §13.2.2 a sentence that always says no.

## 2. Two readings the tables force

- **Table 286's /TF.** The paragraph says a permissions dictionary "with unrecognised values" is
  non-viable, and the /TF row says "An unrecognised value shall be treated as (TEMPNEVER)". The row is
  the more specific of the two statements, so an unknown /TF is read as `TEMPNEVER`. Any other unknown
  entry in /P makes the clip non-viable.
- **Table 285's form XObject goes on the excluded list.** "If D references a form XObject, the
  associated player is implicitly the interactive PDF processor, and the form XObject shall be rendered
  as if it were any other data type". "Any other data type" is a played medium: it is presented in the
  screen annotation's rectangle under §13.2.5's /D duration and /F fit, which the next sentence of the
  clause applies to it "just as they do to a QuickTime movie". That presenting is the playing that
  `CLAUDE.md`'s entry excludes, whatever the bytes are. The report names the clip as this processor's
  to play, and nothing is handed over, because no other program could play it. The trigger that
  reopens this is the trigger the entry names for the playing.

## 3. The action

`Rendition` leaves `action::refused`. Table 218 is read into `Action::Rendition`, and §12.6.4.14's
precondition is checked first: the screen annotation's /P must be a page, and the annotation must be
in that page's /Annots. /OP 0 and 4 are judged. A stop, a pause and a resume each say why nothing
happens. A /JS beside an /OP is the clause's fallback case, and /OP is executed. The cost is one more
`Action` variant and one more `Request`. Their consumers were `view.rs` and `viewer_core::interact`
alone.
