//! Printing and archiving: the pages a host is handed to print and the conversion a person asked
//! for.

use super::layout::damage;
use super::{DocumentId, Standing, Viewer};
#[cfg(doc)]
use crate::Command;
use crate::command::Printing;
use crate::event::Event;
use crate::open::Open;

impl Viewer {
    /// [`Command::Print`]: §7.6.4.2's bit 3 asked as an operation, and the intent if it is granted.
    ///
    /// The two ends of §8.11.4.5's print operation. [`Printing::Start`] asks the policy and, on a
    /// yes, puts the focused document into print intent; [`Printing::Finish`] takes it back out,
    /// which is the clause's own requirement that the changes "persist only for the duration of
    /// the print operation". Finishing asks nothing: a reader who is allowed to start is not asked
    /// again for permission to stop, and a `Finish` with no job running is a host tidying up.
    pub(super) fn print(&mut self, printing: Printing, events: &mut Vec<Event>) {
        use pdf_model::restriction::Operation;

        let Some(id) = self.focused else { return };
        let sheet = match printing {
            Printing::Start(sheet) => sheet,
            // A sheet the person chose after the grant. §12.5.6.22's media is the only thing it
            // changes, so the pages that already carry a watermark against the old one are
            // superseded and nothing else is.
            Printing::Paper(sheet) => {
                let Some(open) = self.documents.get_mut(&id) else {
                    return;
                };
                if open.printing.is_none() {
                    return;
                }
                open.printing = Some(sheet);
                open.view.set_paper(paper_of(sheet));
                open.stale();
                events.push(damage(self.viewport));
                return;
            }
            Printing::Finish => {
                let Some(open) = self.documents.get_mut(&id) else {
                    return;
                };
                Self::end_printing(open);
                events.push(damage(self.viewport));
                return;
            }
        };
        match self.standing(id, Operation::Print, None, None) {
            Standing::Refuse(refused) => events.push(refused),
            Standing::Proceed => {
                let policy = self.restrictions;
                let Some(open) = self.documents.get_mut(&id) else {
                    return;
                };
                let under = policy.under(open.restrictions);
                if let Some(fidelity) = Self::faithfulness(id, open, under, sheet, events) {
                    Self::begin_printing(id, open, sheet, fidelity, events);
                }
            }
            Standing::Warn(notes) => {
                let policy = self.restrictions;
                let Some(open) = self.documents.get_mut(&id) else {
                    return;
                };
                let under = policy.under(open.restrictions);
                let granted = Self::faithfulness(id, open, under, sheet, events);
                if let Some(fidelity) = granted {
                    Self::begin_printing(id, open, sheet, fidelity, events);
                }
                events.push(Event::Warned {
                    document: id,
                    operation: Operation::Print,
                    notes,
                });
            }
            Standing::Ask(notes) => {
                let Some(open) = self.documents.get_mut(&id) else {
                    return;
                };
                open.asking = Some(crate::open::Held::Print(sheet));
                events.push(Event::Asking {
                    document: id,
                    operation: Operation::Print,
                    notes,
                });
            }
        }
        events.push(damage(self.viewport));
    }

    /// [`Command::Archive`]: Table 22's bit 11 asked as an operation, and the file if it is
    /// granted.
    ///
    /// The conversion is not run here: `pdf_transform` depends on this crate, so the grant
    /// carries the file and the host runs the verb (ADR 1954). What this crate owns is the one
    /// place the reader's level for assembling a document is asked, exactly as a print's bit 3
    /// is asked by [`Self::print`].
    pub(super) fn archive(&mut self, asked: crate::Archiving, events: &mut Vec<Event>) {
        use pdf_model::restriction::Operation;

        let Some(id) = self.focused else { return };
        let standing = self.standing(id, Operation::Assemble, None, None);
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };
        match standing {
            Standing::Refuse(refused) => events.push(refused),
            Standing::Proceed => events.push(Self::archiving(id, open, asked)),
            Standing::Warn(notes) => {
                events.push(Self::archiving(id, open, asked));
                events.push(Event::Warned {
                    document: id,
                    operation: Operation::Assemble,
                    notes,
                });
            }
            Standing::Ask(notes) => {
                open.asking = Some(crate::open::Held::Archive(asked));
                events.push(Event::Asking {
                    document: id,
                    operation: Operation::Assemble,
                    notes,
                });
            }
        }
    }

    /// The grant [`Self::archive`] and a `yes` to its question both send.
    pub(super) fn archiving(id: DocumentId, open: &Open, asked: crate::Archiving) -> Event {
        Event::Archiving {
            document: id,
            bytes: open.document.bytes().clone(),
            asked,
        }
    }

    /// §7.6.4.2's Table 22 bit 12, asked of a print bit 3 has already let through.
    ///
    /// `None` is the *ask* level: the question is out and the job waits on
    /// [`Command::Answer`], which is the only one of the four verdicts that does not start a
    /// print. The other three all start one, at the fidelity they decided:
    ///
    /// - **`Proceed`** — the document withholds nothing, or this reader said not to obey it.
    /// - **`Refuse`** — the document withholds bit 12 and this reader obeys it, so the job goes
    ///   ahead degraded and [`Event::Refused`] names the *fidelity* that was withheld. Table 22
    ///   is what makes a refusal and a grant arrive together: "[w]hen this bit is clear (and bit
    ///   3 is set), printing shall be limited to a low- level representation of the appearance,
    ///   possibly of degraded quality" — a limit on the job rather than the end of it.
    /// - **`Warn`** — the job is faithful and the document's reasons are said afterwards, which
    ///   is what that level means everywhere else in this crate.
    ///
    /// ADR 1203.
    pub(super) fn faithfulness(
        id: DocumentId,
        open: &mut Open,
        policy: crate::RestrictionPolicy,
        sheet: crate::Sheet,
        events: &mut Vec<Event>,
    ) -> Option<crate::Fidelity> {
        use pdf_model::restriction::Operation;

        let operation = Operation::PrintFaithfully;
        let standing = Self::verdict_of(
            id,
            policy.level(operation).level(),
            &open.document,
            operation,
            None,
            None,
        );
        match standing {
            Standing::Proceed => Some(crate::Fidelity::Faithful),
            Standing::Refuse(refused) => {
                events.push(refused);
                Some(crate::Fidelity::Degraded)
            }
            Standing::Warn(notes) => {
                events.push(Event::Warned {
                    document: id,
                    operation,
                    notes,
                });
                Some(crate::Fidelity::Faithful)
            }
            Standing::Ask(notes) => {
                open.asking = Some(crate::open::Held::PrintFaithfully(sheet));
                events.push(Event::Asking {
                    document: id,
                    operation,
                    notes,
                });
                None
            }
        }
    }

    /// Puts one document into print intent, and says so.
    ///
    /// Three statements about the *output* rather than about the document, which is why they go
    /// through `ViewState` and nowhere else (`CLAUDE.md`'s rule 1): §8.11.4.5's event, which
    /// re-applies the `Print` usage application dictionaries over the states the reader has
    /// chosen; §12.5.6.22's target media; and, through the first of those, §12.5.3's device.
    /// Every page already interpreted was interpreted for a screen, so all of them are superseded
    /// — which is what `Open::stale` means and why a print operation redraws the window.
    ///
    /// `fidelity` is Table 22 bit 12's answer, decided by [`Self::faithfulness`] before this is
    /// reached and carried on the grant so that a host knows what its dialogue may offer.
    pub(super) fn begin_printing(
        id: DocumentId,
        open: &mut Open,
        sheet: crate::Sheet,
        fidelity: crate::Fidelity,
        events: &mut Vec<Event>,
    ) {
        // Table 200's `/WP`, "before printing a document": before the intent is set, so that what
        // the script writes into a field is on the paper (ADR 1614).
        open.view.run_document_scripts(
            &open.document,
            pdf_model::view::DocumentTrigger::WillPrint,
            open.page_index,
        );
        open.printing = Some(sheet);
        open.view
            .set_purpose(pdf_model::optional_content::Purpose::Print);
        open.view.set_paper(paper_of(sheet));
        open.stale();
        // What the print will not do with the document, said before it prints and once per job:
        // before the grant, so that a host showing events in order shows it ahead of its dialogue.
        let notes = crate::notes::what_a_print_does_not_do(&open.document);
        if !notes.is_empty() {
            events.push(Event::Reported {
                document: id,
                page: None,
                notes,
            });
        }
        events.push(Event::Printing {
            document: id,
            pages: open.page_count,
            fidelity,
        });
    }

    /// Takes it back out — §8.11.4.5's "then all groups shall revert to their prior states".
    ///
    /// Idempotent, because a host that sends [`Printing::Finish`] twice, or once with no job
    /// running, has said nothing wrong.
    fn end_printing(open: &mut Open) {
        if open.printing.take().is_none() {
            return;
        }
        open.view
            .set_purpose(pdf_model::optional_content::Purpose::View);
        open.view.set_paper(None);
        // Table 200's `/DP`, "after printing a document": the operation is over once the intent
        // is taken back out (ADR 1614).
        open.view.run_document_scripts(
            &open.document,
            pdf_model::view::DocumentTrigger::DidPrint,
            open.page_index,
        );
        open.stale();
    }
}

/// One page of a print operation, interpreted for the sheet and targeted at the printer.
///
/// `None` for a page index the document has not got, and for a page whose marks will not fit the
/// target at the job's resolution — [`pdf_render::TargetSpec::for_page`]'s own refusal, which is
/// the same bound every frame is drawn under. A refusal here is silent in this crate and loud in
/// the host: a print job that got no page for page seven says so where the job is, and a report
/// invented here would fire on a condition no clause states (trap 11).
///
/// The view state is the document's own, which [`Viewer::begin_printing`] has already put into
/// print intent — so this function states nothing about §12.5.3 or §8.11.4.5 and could not: the
/// whole of the print reading is in `pdf_model`, asked of a state that says what the output is
/// for.
/// A sheet as §12.5.6.22 reads it: the media, and what B does to the page on it.
///
/// `None` is Table 193's dimensions that "are not known at the time of drawing", under which the
/// page's own media box stands in and there is no B to cancel — so the scale is dropped with the
/// rectangle rather than carried alone. ADR 1204.
fn paper_of(sheet: crate::Sheet) -> Option<pdf_model::view::TargetMedia> {
    sheet.media.map(|media| pdf_model::view::TargetMedia {
        media,
        page_scale: sheet.page_scale,
    })
}
