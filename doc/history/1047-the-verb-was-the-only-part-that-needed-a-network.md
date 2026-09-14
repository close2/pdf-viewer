# 1047 — the verb was the only part that needed a network

Date: 2026-09-14. ADR 1062. Touched `pdf-model` (`submission.rs` new, `action.rs`, `view.rs`, `appearance.rs`), `viewer-core`,
`viewer-host`, `viewer-confined`, `viewer-ffi`, the three hosts, five §12.7 ledger rows. A predecessor with this contract died on a
quota leaving `submission.rs` untracked and unwired; this round read it against the clause, corrected it, finished it and gated it.

**§12.7.6.2 was `reported` on a reason true of one word.** "Upon invocation of a submit-form action, an interactive PDF processor
shall transmit the names and values of selected interactive form fields to a specified uniform resource locator (URL)" — and *which*
names and values, in *which* of four formats, to *which* URL by *which* method are each a function of the document, of Tables 239
and 240, and of what a person entered since the file opened: state this tree holds. Only *transmit* needs a socket, which principle
3 withholds. So `pdf_model::submission::compose` composes the request whole, `viewer_core::interact` hands it over as
`Event::Submit`, and `viewer_host::policy::may_submit` is the single place answering whether this machine sends it. **That is ADR
1062**, principle 3's own shape: the policy asked once where a host can supply it. The refusal it replaces could not have become an
*ask* — four words inside `action::refused`.

**Three of the four formats are written**, none inventing a mark: FDF is §12.7.8's structure this tree already reads, HTML Form
format is HTML 4.01 section 17.13.4's url-encoding, PDF is `ViewState::save`'s §7.5.6 update. Every selection rule the clause states
is applied — `NoExport` ahead of `/Fields`, Include/Exclude, `IncludeNoValueFields`, the push-button sentence — and **every Table
240 flag not applied is a sentence on `Submission::owed`** carried into `Event::Reported`: bits 7, 8, 10, 11, 14, and bit 4 against
a clear bit 3, which the table forbids. A submission quietly missing one would be trap 5 where it hides best: the body would still
be a valid FDF. Three defects in the inherited draft are corrected: a `/V` stream that would not decode became an empty value, a
file-select `/V` holding no text became an empty file specification, and the media type was `application/vnd.fdf` on a claim that
IANA lists it — IANA lists `application/fdf`, registered by ISO TC 171/SC 2, whose registration also settles the `%FDF-1.2` header.

**Table 240 bit 5 is why `viewer-core` changed shape.** Its coordinates come from "the field's widget annotation rectangle", and a
submit button is a `/Widget` — so `link::links` never sees it and the click arrives through Table 197's `/U`. `interact::trigger`
and `Viewer::raise` now carry the point; `compose` reads the widget's own `/Rect`, since §12.6.4.8's `/IsMap` measures from another
annotation.

**§12.7.5.3's residue narrowed from a flag to a bullet.** Of bit 21's three, the FDF one is now the file specification the clause
asks for and the XML one is inside the XFDF refusal; what is left is `multipart/form-data`'s file *contents*, a filesystem this
crate has none of, now named on `owed`. **XFDF stays declined for 1035's reason** and §12.7.4.3's transformed `Tm` was left as 1035
measured it. Rows: **§12.7.6.2 `reported` → `partial`** — which reads backwards and is not, since `reported` meant nothing was done
and `partial` means the remainder says so, by flag; §12.7, §12.7.5.3, §12.7.6 and §12.7.6.1 corrected where each said the submission
was refused.

Calibrated (trap 13): five planted defects in `compose` — `NoExport` dropped, the file-select arm removed, Include/Exclude inverted,
`IncludeNoValueFields` ignored, a stream refusal swallowed — each named by its own test and no other; two in `interact::perform` —
the notes dropped, the submission not handed over — each failing the headless test. The FDF assertions read the body back with
`FormsData::read`, since §7.3.4.3 lets a string be written two ways. No page moves.
