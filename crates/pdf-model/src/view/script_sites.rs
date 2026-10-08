//! The scripts a document runs outside a field's own four triggers: Table 32's name tree and the
//! catalog's `/OpenAction` when the document is opened, Table 198's page events, Table 197's
//! annotation events (RFC 0008 sections 6.5 and 6.6, ADR 1602), and Table 200's events of the
//! document as a whole, which a host marks around a close, a save and a print (ADR 1614).
//!
//! **Only the ECMAScript actions are run here.** A host already performs every other action of the
//! same chains — `crate::action::for_annotation` and `crate::action::for_page` read them and
//! [`ViewState::perform_all`] performs them, `JavaScript` among them a refusal, since §12.6.4.17's
//! row is the owner's to move (`doc/questions/Q286`) — so each method here walks the same chain for
//! its scripts alone and hands them to the runner, and a host calls both at the event it raises:
//! [`ViewState::run_annotation_scripts`] at a widget's pointer and focus events, and
//! [`ViewState::run_page_scripts`] as the page shown changes (ADR 1750). Where no runner is
//! supplied nothing runs and the open says, once, how many document-level scripts went unrun.
//!
//! **The open sequence runs after the first present.** §12.6.4.17 says of the name tree: "When the
//! document is opened, all of the actions in this name tree shall be executed, defining ECMAScript
//! functions for use by other scripts in the document." *When opened* is not *before the first
//! frame*, and `CLAUDE.md`'s principle 2 puts nothing eager on the launch path — so a host calls
//! [`ViewState::run_open_scripts`] once page one is on the screen (RFC 0008 section 6.6), and a
//! field a format changes repaints as a `NeedAppearances` rewrite does. The cost to
//! time-to-first-page is nothing, by construction.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use pdf_syntax::{Dictionary, Document, Object, ObjectId, tree};

use super::ViewState;
use super::script_model::{DocumentTrigger, ScriptSite};
use super::scripts::{MAX_SEQUENCE_TIME, ScriptEvent, script_text};
use crate::action::{PageTrigger, Trigger as AnnotationTrigger};

/// Most document-level scripts the open sequence runs.
///
/// Table 32's tree is the document's, so its size is too; the census's largest library is a few
/// dozen entries, and a tree of thousands is a document using the open as a loop.
const MAX_LIBRARY: usize = 1024;

/// Most actions one chain's walk visits, the same bound `crate::action` reads a chain under.
const MAX_CHAIN: usize = 256;

/// What a host's request to run an event's scripts did (ADR 1762).
///
/// The two answers a host acts on separately: whether the runner was handed the event's scripts,
/// so that the action path's refusal of the same chain's ECMAScript actions is not said as well
/// (ADR 1752), and whether what a page draws may have changed, so that the page is interpreted
/// again only then — a cursor crossing a widget whose `/E` script only logs or asks a question
/// changes nothing a page draws.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScriptsRan {
    /// How many scripts the runner was handed: 0 where the level is `off`, where the chain holds
    /// none, and before the open sequence has run.
    pub handed: usize,
    /// Whether a script's edits can have changed what a page draws ([`ScriptEdit::redraws`],
    /// a value written over a different one, or a walk of `/CO` that may write one).
    ///
    /// [`ScriptEdit::redraws`]: super::ScriptEdit::redraws
    pub changed: bool,
}

/// One sequence's tally while it runs.
#[derive(Debug, Clone, Copy, Default)]
struct Ran {
    /// Scripts handed to the runner.
    handed: usize,
    /// Whether a value changed, which walks `/CO`.
    values: bool,
    /// Whether a script asked for `/CO` to be walked.
    calculate: bool,
    /// Whether what a page draws may have changed.
    drawn: bool,
}

impl Ran {
    /// One more script run, and what applying its edits did.
    fn count(&mut self, applied: super::scripts::Applied) {
        self.handed = self.handed.saturating_add(1);
        self.values |= applied.values;
        self.calculate |= applied.calculate;
        self.drawn |= applied.drawn;
    }

    /// Another sequence's tally added to this one.
    fn add(&mut self, other: Self) {
        self.handed = self.handed.saturating_add(other.handed);
        self.values |= other.values;
        self.calculate |= other.calculate;
        self.drawn |= other.drawn;
    }
}

impl ViewState {
    /// Runs the open sequence of RFC 0008 section 6.5 step 1, after the first present: every
    /// entry of Table 32's `/JavaScript` name tree in the tree's order, then the catalog's
    /// `/OpenAction` where it is a script, then page `page`'s Table 198 `/O` and its annotations'
    /// Table 197 `/PO` and `/PV` ([`Self::run_page_scripts`]), then every format a runner runs.
    ///
    /// Answers how many scripts were handed to the runner. **What a host calls once page one has
    /// been presented**, with the page it presented; calling it again runs the sequence again.
    /// §12.11.1's requirements are a host's to evaluate first, as it does today. Every script is
    /// held to its runner's budget and the sequence as a whole to [`MAX_SEQUENCE_TIME`] — RFC
    /// 0008 section 6.8's open-sequence deadline — after which the rest are reported as not run
    /// (ADR 1602).
    pub fn run_open_scripts(&mut self, document: &Document, page: usize) -> usize {
        self.scripting.opened = true;
        let library = library(document);
        if self.runner.0.is_none() {
            if !library.is_empty() {
                self.report(format!(
                    "the document carries {} document-level script(s) in Table 32's /JavaScript \
                     name tree, and none was run: no host has supplied a runner for scripts",
                    library.len()
                ));
            }
            return 0;
        }
        // The page shown, and no other: this runs as the document opens (ADR 1653 section 4).
        let table = super::field_table_on_page(document, page);
        let started = Instant::now();
        let mut handed = 0_usize;
        let mut changed = false;
        let mut calculate = false;
        for (index, (label, script)) in library.iter().enumerate() {
            if started.elapsed() > MAX_SEQUENCE_TIME {
                self.report(format!(
                    "the open sequence was stopped at document-level script {} of {}: it ran \
                     longer than its budget of {} ms (ADR 1602)",
                    index.saturating_add(1),
                    library.len(),
                    MAX_SEQUENCE_TIME.as_millis()
                ));
                return handed;
            }
            let event = ScriptEvent {
                label,
                script,
                page,
                ..ScriptEvent::at(ScriptSite::Library, "")
            };
            if let Some((result, applied)) = self.run_event(document, &table, event) {
                handed = handed.saturating_add(1);
                changed |= applied.values;
                calculate |= applied.calculate;
                self.report_each(
                    &format!("the document-level script {label:?}"),
                    result.report,
                );
            }
        }
        for script in open_action_scripts(document) {
            let event = ScriptEvent {
                script: &script,
                page,
                ..ScriptEvent::at(ScriptSite::OpenAction, "")
            };
            if let Some((result, applied)) = self.run_event(document, &table, event) {
                handed = handed.saturating_add(1);
                changed |= applied.values;
                calculate |= applied.calculate;
                self.report_each("the document's open action", result.report);
            }
        }
        let ran = self.page_scripts(document, &table, page, PageTrigger::Open);
        handed = handed.saturating_add(ran.handed);
        self.after_scripts(
            document,
            &table,
            changed || ran.values,
            calculate || ran.calculate,
        );
        self.refresh_formatted(document, &table);
        handed
    }

    /// Runs Table 198's `/O` or `/C` of page `page` and its annotations' matching Table 197
    /// events, in the order the two tables state, and answers how many scripts were handed over
    /// and whether what a page draws may have changed ([`ScriptsRan`]).
    ///
    /// Table 197's `/PO` "shall be executed after the O action in the page's additional - actions
    /// dictionary", so an open runs the page's script first and then each annotation's `/PO` and
    /// `/PV`; its `/PC` "shall be executed before the C action", so a close runs each annotation's
    /// `/PC` and `/PI` first. **The request a host raises as the page shown changes** — the
    /// leaving page's close, then the arriving page's open — beside the non-script actions it
    /// already performs; an annotation's four page events are run here rather than through
    /// [`Self::run_annotation_scripts`], so a host that raises them per annotation for their other
    /// actions does not run their scripts twice (ADRs 1602, 1750). The page visible is the page
    /// shown: Table 197 lets more than one page be visible, and a host that shows one page at a
    /// time raises the pair with the open and the close, as every window here does.
    ///
    /// Before [`Self::run_open_scripts`] has run nothing is run: an open is the open sequence's,
    /// which runs the page shown when it runs, and a close says it was not run
    /// ([`Self::before_open`]).
    pub fn run_page_scripts(
        &mut self,
        document: &Document,
        page: usize,
        trigger: PageTrigger,
    ) -> ScriptsRan {
        if self.runner.0.is_none() {
            return ScriptsRan::default();
        }
        // Cheap where the turn holds no script, which is nearly every turn: the fields are read
        // only once one is found.
        let held = page_turn_scripts(document, page, trigger);
        if held == 0 {
            return ScriptsRan::default();
        }
        if !self.scripting.opened {
            if trigger == PageTrigger::Close {
                self.before_open(held, &format!("page {}'s close", page.saturating_add(1)));
            }
            return ScriptsRan::default();
        }
        // The page turned to or from, and no other (ADR 1653 section 4).
        let table = super::field_table_on_page(document, page);
        let ran = self.page_scripts(document, &table, page, trigger);
        self.finish(document, &table, ran)
    }

    /// Runs one of Table 197's events' scripts on one annotation, and answers how many were handed
    /// over and whether what a page draws may have changed ([`ScriptsRan`]).
    ///
    /// The chain is the one `crate::action::for_annotation` reads — `/U`'s with Table 197's
    /// precedence, the annotation's `/A` where it has one — and its ECMAScript actions are run in
    /// the chain's order. What a host calls beside performing the same event's other actions
    /// (ADR 1602). A widget's event has its field as `event.target`; Adobe's reference does not
    /// listen to `event.rc` at any of these, and neither does this.
    ///
    /// **The request a host raises at a widget's pointer and focus events** — `/E`, `/X`, `/D`,
    /// `/U`, `/Fo`, `/Bl` — beside performing the same event's other actions (ADR 1750). Cheap
    /// where the chain holds no script, which is nearly every event a cursor crossing a page
    /// raises: the field tree is walked only once a script is found. Table 197's four page events
    /// are [`Self::run_page_scripts`]'s, and one raised here runs its scripts as any other does.
    /// Before [`Self::run_open_scripts`] has run nothing is run, and a chain that holds scripts
    /// says so ([`Self::before_open`]).
    pub fn run_annotation_scripts(
        &mut self,
        document: &Document,
        annotation: ObjectId,
        trigger: AnnotationTrigger,
    ) -> ScriptsRan {
        if self.runner.0.is_none() {
            return ScriptsRan::default();
        }
        let Some(dictionary) = document.get(annotation).as_dict().cloned() else {
            return ScriptsRan::default();
        };
        let scripts = annotation_chain(document, &dictionary, trigger)
            .map(|entry| scripts_in(document, &entry))
            .unwrap_or_default();
        if scripts.is_empty() {
            return ScriptsRan::default();
        }
        if !self.scripting.opened {
            self.before_open(
                scripts.len(),
                &format!(
                    "the /{} event of the annotation of object {}",
                    trigger.key(),
                    annotation.number
                ),
            );
            return ScriptsRan::default();
        }
        let table = super::widgets_by_field_name(document);
        let page = page_of(document, annotation, &dictionary);
        let ran = self.annotation_scripts(document, &table, annotation, &scripts, trigger, page);
        self.finish(document, &table, ran)
    }

    /// What a host's request ends with once its scripts have run: `/CO` walked where they changed
    /// a value or asked for it, the formats refreshed, and the answer.
    fn finish(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        ran: Ran,
    ) -> ScriptsRan {
        if ran.handed > 0 {
            self.after_scripts(document, table, ran.values, ran.calculate);
            self.refresh_formatted(document, table);
        }
        ScriptsRan {
            handed: ran.handed,
            changed: ran.drawn,
        }
    }

    /// Runs Table 200's script for one moment of the document as a whole, and answers how many
    /// scripts were handed over.
    ///
    /// **What a host calls at each of the five moments** (ADR 1614): [`DocumentTrigger::WillClose`]
    /// before it lets a document go, [`DocumentTrigger::WillSave`] before
    /// [`ViewState::save`] writes §7.5.6's update and [`DocumentTrigger::DidSave`] once it has —
    /// so what a will-save script writes into a field is in the file — and
    /// [`DocumentTrigger::WillPrint`] and [`DocumentTrigger::DidPrint`] at the start and the end of
    /// a print operation. `page` is the zero-based page the host shows, `this.pageNum`.
    ///
    /// **A trigger here is a notification, never a question.** Table 200 says of each entry only
    /// when its action "shall be performed" — `/WC` "before closing a document", `/WS` "before
    /// saving a document" — and gives the action no part in whether the operation happens; so a
    /// script that sets `event.rc` false is reported, and the close, the save or the print goes
    /// ahead. A document that could keep its reader from closing or saving it would be a
    /// restriction no level could turn off, which `CLAUDE.md` principle 3 forbids.
    ///
    /// The value column makes every entry "[a]n ECMAScript action", so an action of another type
    /// in the chain is not performed, and is reported. The chain is held to its runner's budget per
    /// script and to [`MAX_SEQUENCE_TIME`] as a whole; with no runner supplied, the scripts are
    /// reported as not run, once.
    pub fn run_document_scripts(
        &mut self,
        document: &Document,
        trigger: DocumentTrigger,
        page: usize,
    ) -> usize {
        let key = trigger.key();
        let subject = format!("the document's /{key} script");
        let Some(entry) = document_entry(document, trigger) else {
            return 0;
        };
        let (scripts, others) = chain(document, &entry);
        if others > 0 {
            self.report(format!(
                "the catalog's /AA /{key} chain holds {others} action(s) that are not \
                 ECMAScript, which Table 200 does not admit there, and none was performed"
            ));
        }
        if scripts.is_empty() {
            return 0;
        }
        if self.runner.0.is_none() {
            self.report(format!(
                "{subject} was not run, before or after the {} it marks: no host has supplied a \
                 runner for scripts",
                trigger.operation()
            ));
            return 0;
        }
        let table = super::widgets_by_field_name(document);
        let started = Instant::now();
        let (mut handed, mut changed, mut calculate) = (0_usize, false, false);
        for (index, script) in scripts.iter().enumerate() {
            if started.elapsed() > MAX_SEQUENCE_TIME {
                self.report(format!(
                    "{subject} chain was stopped at script {} of {}: it ran longer than its \
                     budget of {} ms (ADR 1614)",
                    index.saturating_add(1),
                    scripts.len(),
                    MAX_SEQUENCE_TIME.as_millis()
                ));
                break;
            }
            let event = ScriptEvent {
                script,
                page,
                ..ScriptEvent::at(ScriptSite::Document(trigger), "")
            };
            let Some((result, applied)) = self.run_event(document, &table, event) else {
                continue;
            };
            handed = handed.saturating_add(1);
            changed |= applied.values;
            calculate |= applied.calculate;
            let refused = !result.rc;
            self.report_each(&subject, result.report);
            if refused {
                let operation = trigger.operation();
                self.report(if trigger.before() {
                    format!(
                        "{subject} set event.rc false, and the {operation} goes ahead: Table 200 \
                         performs the script before the {operation} and gives it no say in \
                         whether the {operation} happens (ADR 1614)"
                    )
                } else {
                    format!(
                        "{subject} set event.rc false after the {operation}, which nothing \
                         listens to (ADR 1614)"
                    )
                });
            }
        }
        if handed > 0 {
            self.after_scripts(document, &table, changed, calculate);
            self.refresh_formatted(document, &table);
        }
        handed
    }

    /// Says that a request a host raised held scripts and ran none, because the open sequence has
    /// not run (ADR 1750).
    ///
    /// Table 32's name tree is executed "[w]hen the document is opened … defining ECMAScript
    /// functions for use by other scripts in the document", so a script run before it would meet a
    /// library that does not exist yet, and a person would read the `ReferenceError` as the
    /// form's. A host runs the sequence after its first present (RFC 0008 section 6.6), so only a
    /// host that raises an event before it, or never presents, reaches this.
    fn before_open(&mut self, scripts: usize, subject: &str) {
        if scripts == 0 {
            return;
        }
        self.report(format!(
            "{subject} holds {scripts} script(s), and none was run: the document's open sequence, \
             which defines the functions its scripts call, has not run yet (ADR 1750)"
        ));
    }

    /// The page's scripts and its annotations' for one of Table 198's events.
    fn page_scripts(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        page: usize,
        trigger: PageTrigger,
    ) -> Ran {
        let Some(found) = crate::page::Pages::new(document).get(page) else {
            return Ran::default();
        };
        let annotations = page_annotations(document, &found.dict);
        let page_scripts = page_entry_scripts(document, &found.dict, trigger);
        let mut total = Ran::default();
        let run_page = |view: &mut Self| {
            let mut ran = Ran::default();
            for script in &page_scripts {
                let event = ScriptEvent {
                    script,
                    page,
                    ..ScriptEvent::at(ScriptSite::Page(trigger), "")
                };
                if let Some((result, applied)) = view.run_event(document, table, event) {
                    ran.count(applied);
                    view.report_each(
                        &format!(
                            "page {}'s {} script",
                            page.saturating_add(1),
                            page_noun(trigger)
                        ),
                        result.report,
                    );
                }
            }
            ran
        };
        if trigger == PageTrigger::Open {
            total.add(run_page(self));
        }
        // Each annotation's pair together, the order a host raises them in for their other
        // actions: Table 197 orders `/PO` and `/PC` against the page's own entry and states no
        // order between one annotation's events and another's.
        for annotation in annotations {
            let Some(dictionary) = document.get(annotation).as_dict().cloned() else {
                continue;
            };
            for each in annotation_triggers(trigger) {
                let scripts = annotation_chain(document, &dictionary, each)
                    .map(|entry| scripts_in(document, &entry))
                    .unwrap_or_default();
                if scripts.is_empty() {
                    continue;
                }
                total.add(
                    self.annotation_scripts(document, table, annotation, &scripts, each, page),
                );
            }
        }
        if trigger == PageTrigger::Close {
            total.add(run_page(self));
        }
        total
    }

    /// One annotation's scripts for one of Table 197's events, `scripts` being its chain's.
    fn annotation_scripts(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        annotation: ObjectId,
        scripts: &[String],
        trigger: AnnotationTrigger,
        page: usize,
    ) -> Ran {
        let field = table
            .iter()
            .find(|(_, widgets)| widgets.contains(&annotation))
            .map(|(name, _)| name.clone())
            .unwrap_or_default();
        let value = if field.is_empty() {
            String::new()
        } else {
            self.text_of(document, annotation).unwrap_or_default()
        };
        let mut ran = Ran::default();
        for script in scripts {
            let event = ScriptEvent {
                script,
                value: &value,
                page,
                ..ScriptEvent::at(ScriptSite::Annotation(trigger), &field)
            };
            if let Some((result, applied)) = self.run_event(document, table, event) {
                ran.count(applied);
                let subject = if field.is_empty() {
                    format!("the annotation of object {}", annotation.number)
                } else {
                    field.clone()
                };
                self.report_each(
                    &format!("{subject}'s /{} script", trigger.key()),
                    result.report,
                );
            }
        }
        ran
    }

    /// Walks `/CO` where a sequence's scripts changed a value or asked for it.
    ///
    /// Table 224's order is walked "when the value of any field changes", and a script's change is
    /// a change; `calculateNow` asks for the same walk whether or not anything changed.
    fn after_scripts(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        changed: bool,
        calculate: bool,
    ) {
        if changed || calculate {
            self.recalculate_scripts(document, table, "");
        }
    }
}

/// The zero-based page an annotation is on, `this.pageNum` at its events: the page Table 166's
/// `/P` names, or where it names none the page whose `/Annots` lists the annotation (ADR 1700).
///
/// `/P` is Optional for every subtype but a screen annotation, so a page's own `/Annots` is the
/// statement every annotation has; the walk is made only at an annotation's event, a person's act.
/// Page one where neither names a page.
fn page_of(document: &Document, id: ObjectId, annotation: &Dictionary) -> usize {
    let pages = crate::page::Pages::new(document);
    if let Some(page) = annotation.get("P").and_then(Object::as_reference) {
        return pages.index_of(page).unwrap_or_default();
    }
    let mut ordered: Vec<(usize, ObjectId)> = pages
        .indices()
        .into_iter()
        .map(|(page, index)| (index, page))
        .collect();
    ordered.sort_unstable();
    ordered
        .into_iter()
        .find(|(_, page)| {
            document.get(*page).as_dict().is_some_and(|page| {
                document
                    .get_key(page, "Annots")
                    .as_array()
                    .unwrap_or_default()
                    .iter()
                    .any(|entry| entry.as_reference() == Some(id))
            })
        })
        .map_or(0, |(index, _)| index)
}

/// How many scripts a page turn's request would hand over: the page's own and its annotations' for
/// one of Table 198's events.
fn page_turn_scripts(document: &Document, page: usize, trigger: PageTrigger) -> usize {
    let Some(found) = crate::page::Pages::new(document).get(page) else {
        return 0;
    };
    let own = page_entry_scripts(document, &found.dict, trigger).len();
    page_annotations(document, &found.dict)
        .into_iter()
        .filter_map(|annotation| document.get(annotation).as_dict().cloned())
        .flat_map(|dictionary| {
            annotation_triggers(trigger).map(|each| {
                annotation_chain(document, &dictionary, each)
                    .map_or(0, |entry| scripts_in(document, &entry).len())
            })
        })
        .fold(own, usize::saturating_add)
}

/// The annotations a page's `/Annots` lists, by object.
fn page_annotations(document: &Document, page: &Dictionary) -> Vec<ObjectId> {
    match document.get_key(page, "Annots") {
        Object::Array(items) => items.iter().filter_map(Object::as_reference).collect(),
        _ => Vec::new(),
    }
}

/// The scripts of a page's own Table 198 entry for one event.
///
/// `/AA` is not one of §7.7.3.4's inheritable entries, so it is read from the page's own
/// dictionary, as `crate::action::for_page` reads it.
fn page_entry_scripts(document: &Document, page: &Dictionary, trigger: PageTrigger) -> Vec<String> {
    let additional = document.get_key(page, "AA");
    additional
        .as_dict()
        .and_then(|additional| additional.get(page_key(trigger)).cloned())
        .map(|entry| scripts_in(document, &entry))
        .unwrap_or_default()
}

/// Table 197's two events of each annotation that ride with one of Table 198's: an opened page is
/// the page shown, so it is opened and becomes visible; a closed one is closed and is no longer
/// visible (ADR 1750).
fn annotation_triggers(trigger: PageTrigger) -> [AnnotationTrigger; 2] {
    match trigger {
        PageTrigger::Open => [AnnotationTrigger::PageOpen, AnnotationTrigger::PageVisible],
        PageTrigger::Close => [
            AnnotationTrigger::PageClose,
            AnnotationTrigger::PageInvisible,
        ],
    }
}

/// The key of a page's additional-actions dictionary that states one of Table 198's events.
fn page_key(trigger: PageTrigger) -> &'static str {
    match trigger {
        PageTrigger::Open => "O",
        PageTrigger::Close => "C",
    }
}

/// How a page's event names itself in a report.
fn page_noun(trigger: PageTrigger) -> String {
    format!("/{}", page_key(trigger))
}

/// The action entry one of Table 197's events states on an annotation, `/U`'s precedence applied.
///
/// "For backward compatibility, the A entry in an annotation dictionary, if present, takes
/// precedence over this entry" — the rule `crate::action::for_annotation` applies to the same
/// annotation, applied here to the same entry.
fn annotation_chain(
    document: &Document,
    annotation: &Dictionary,
    trigger: AnnotationTrigger,
) -> Option<Object> {
    if trigger == AnnotationTrigger::Up
        && let Some(stated) = annotation.get("A")
        && !matches!(document.resolve(stated), Object::Null)
    {
        return Some(stated.clone());
    }
    let additional = document.get_key(annotation, "AA");
    additional.as_dict()?.get(trigger.key()).cloned()
}

/// Every entry of Table 32's `/JavaScript` name tree, in the tree's order, with each entry's
/// scripts joined in its chain's order.
///
/// §7.9.6 orders a name tree's keys, so the walk's order is the tree's; an entry whose action is
/// not ECMAScript, or whose text cannot be read, contributes nothing.
fn library(document: &Document) -> Vec<(String, String)> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    let names = document.get_key(&catalog, "Names");
    let Some(names) = names.as_dict() else {
        return Vec::new();
    };
    let root = document.get_key(names, "JavaScript");
    let Some(root) = root.as_dict() else {
        return Vec::new();
    };
    tree::name_pairs(root, &|object| document.resolve(object))
        .into_iter()
        .take(MAX_LIBRARY)
        .filter_map(|(name, action)| {
            let scripts = scripts_in(document, &action);
            (!scripts.is_empty()).then(|| (pdf_syntax::text_string(&name), scripts.join("\n")))
        })
        .collect()
}

/// The catalog's `/OpenAction` chain's scripts, where it is an action rather than a destination.
fn open_action_scripts(document: &Document) -> Vec<String> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    match catalog.get("OpenAction") {
        Some(entry) if document.resolve(entry).as_dict().is_some() => scripts_in(document, entry),
        _ => Vec::new(),
    }
}

/// The catalog's Table 200 entry for one trigger, where its additional-actions dictionary states
/// one.
///
/// Table 29 makes the catalog's `/AA` "[a]n additional-actions dictionary defining the actions
/// that shall be taken in response to various trigger events affecting the document as a whole".
fn document_entry(document: &Document, trigger: DocumentTrigger) -> Option<Object> {
    let catalog = document.catalog().ok()?;
    let additional = document.get_key(&catalog, "AA");
    additional.as_dict()?.get(trigger.key()).cloned()
}

/// The text of every ECMAScript action in one chain, in §12.6.2's execution order — an action,
/// then its `/Next` subtree, then the next sibling — each action dictionary visited once.
fn scripts_in(document: &Document, entry: &Object) -> Vec<String> {
    chain(document, entry).0
}

/// [`scripts_in`], with how many actions of the chain are of another type, or are ECMAScript whose
/// text cannot be read.
fn chain(document: &Document, entry: &Object) -> (Vec<String>, usize) {
    let mut walked = Walked::default();
    walk(document, entry, &mut walked);
    (walked.scripts, walked.others)
}

/// What one chain's walk has found so far.
#[derive(Debug, Default)]
struct Walked {
    /// Each script's text, in order.
    scripts: Vec<String>,
    /// How many actions were not a script whose text was read.
    others: usize,
    /// Every action dictionary visited, by object.
    seen: BTreeSet<ObjectId>,
    /// How many entries the walk has visited, held to [`MAX_CHAIN`].
    visited: usize,
}

/// Appends one action's script and its `/Next` subtree's.
fn walk(document: &Document, entry: &Object, walked: &mut Walked) {
    if walked.visited >= MAX_CHAIN {
        return;
    }
    walked.visited = walked.visited.saturating_add(1);
    if let Object::Reference(id) = entry
        && !walked.seen.insert(*id)
    {
        return;
    }
    let resolved = document.resolve(entry);
    let Some(action) = resolved.as_dict() else {
        return;
    };
    match script_text(document, action) {
        Some(script) => walked.scripts.push(script),
        None => walked.others = walked.others.saturating_add(1),
    }
    let next = action.get("Next").cloned().unwrap_or(Object::Null);
    match document.resolve(&next) {
        Object::Array(items) => {
            for item in &items {
                walk(document, item, walked);
            }
        }
        Object::Dictionary(_) => walk(document, &next, walked),
        _ => {}
    }
}
