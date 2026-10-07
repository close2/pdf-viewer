//! The scripts a document runs outside a field's own four triggers: Table 32's name tree and the
//! catalog's `/OpenAction` when the document is opened, Table 198's page events, and Table 197's
//! annotation events (RFC 0008 sections 6.5 and 6.6, ADR 1602).
//!
//! **Only the ECMAScript actions are run here.** A host already performs every other action of the
//! same chains — `crate::action::for_annotation` and `crate::action::for_page` read them and
//! [`ViewState::perform_all`] performs them — so each method here walks the same chain for its
//! scripts alone and hands them to the runner, and a host calls both. Where no runner is supplied
//! nothing runs and the open says, once, how many document-level scripts went unrun.
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
use super::script_model::ScriptSite;
use super::scripts::{MAX_SEQUENCE_TIME, ScriptEvent, script_text};
use crate::action::{PageTrigger, Trigger as AnnotationTrigger};

/// Most document-level scripts the open sequence runs.
///
/// Table 32's tree is the document's, so its size is too; the census's largest library is a few
/// dozen entries, and a tree of thousands is a document using the open as a loop.
const MAX_LIBRARY: usize = 1024;

/// Most actions one chain's walk visits, the same bound `crate::action` reads a chain under.
const MAX_CHAIN: usize = 256;

impl ViewState {
    /// Runs the open sequence of RFC 0008 section 6.5 step 1, after the first present: every
    /// entry of Table 32's `/JavaScript` name tree in the tree's order, then the catalog's
    /// `/OpenAction` where it is a script, then page `page`'s Table 198 `/O` and its annotations'
    /// Table 197 `/PO` ([`Self::run_page_scripts`]), then every format a runner runs.
    ///
    /// Answers how many scripts were handed to the runner. **What a host calls once page one has
    /// been presented**, with the page it presented; calling it again runs the sequence again.
    /// §12.11.1's requirements are a host's to evaluate first, as it does today. Every script is
    /// held to its runner's budget and the sequence as a whole to [`MAX_SEQUENCE_TIME`] — RFC
    /// 0008 section 6.8's open-sequence deadline — after which the rest are reported as not run
    /// (ADR 1602).
    pub fn run_open_scripts(&mut self, document: &Document, page: usize) -> usize {
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
        let table = super::widgets_by_field_name(document);
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
        let (ran, page_changed, page_calculate) =
            self.page_scripts(document, &table, page, PageTrigger::Open);
        handed = handed.saturating_add(ran);
        self.after_scripts(
            document,
            &table,
            changed || page_changed,
            calculate || page_calculate,
        );
        self.refresh_formatted(document, &table);
        handed
    }

    /// Runs Table 198's `/O` or `/C` of page `page` and its annotations' matching Table 197 event,
    /// in the order the two tables state, and answers how many scripts were handed over.
    ///
    /// Table 197's `/PO` "shall be executed after the O action in the page's additional - actions
    /// dictionary", so an open runs the page's script first and then each annotation's; its `/PC`
    /// "shall be executed before the C action", so a close runs each annotation's first. What a
    /// host calls on a page turn — the leaving page's close, then the arriving page's open —
    /// beside the non-script actions it already performs; an annotation's `/PO` and `/PC` are run
    /// here rather than through [`Self::run_annotation_scripts`] (ADR 1602).
    pub fn run_page_scripts(
        &mut self,
        document: &Document,
        page: usize,
        trigger: PageTrigger,
    ) -> usize {
        if self.runner.0.is_none() {
            return 0;
        }
        let table = super::widgets_by_field_name(document);
        let (ran, changed, calculate) = self.page_scripts(document, &table, page, trigger);
        if ran > 0 {
            self.after_scripts(document, &table, changed, calculate);
            self.refresh_formatted(document, &table);
        }
        ran
    }

    /// Runs one of Table 197's events' scripts on one annotation, and answers how many were handed
    /// over.
    ///
    /// The chain is the one `crate::action::for_annotation` reads — `/U`'s with Table 197's
    /// precedence, the annotation's `/A` where it has one — and its ECMAScript actions are run in
    /// the chain's order. What a host calls beside performing the same event's other actions
    /// (ADR 1602). A widget's event has its field as `event.target`; Adobe's reference does not
    /// listen to `event.rc` at any of these, and neither does this.
    pub fn run_annotation_scripts(
        &mut self,
        document: &Document,
        annotation: ObjectId,
        trigger: AnnotationTrigger,
    ) -> usize {
        if self.runner.0.is_none() {
            return 0;
        }
        let Some(dictionary) = document.get(annotation).as_dict().cloned() else {
            return 0;
        };
        let table = super::widgets_by_field_name(document);
        let page = page_of(document, &dictionary);
        let (ran, changed, calculate) =
            self.annotation_scripts(document, &table, annotation, &dictionary, trigger, page);
        if ran > 0 {
            self.after_scripts(document, &table, changed, calculate);
            self.refresh_formatted(document, &table);
        }
        ran
    }

    /// The page's scripts and its annotations' for one of Table 198's events.
    fn page_scripts(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        page: usize,
        trigger: PageTrigger,
    ) -> (usize, bool, bool) {
        let Some(found) = crate::page::Pages::new(document).get(page) else {
            return (0, false, false);
        };
        let annotations: Vec<ObjectId> = match document.get_key(&found.dict, "Annots") {
            Object::Array(items) => items.iter().filter_map(Object::as_reference).collect(),
            _ => Vec::new(),
        };
        let page_scripts = {
            let additional = document.get_key(&found.dict, "AA");
            let key = match trigger {
                PageTrigger::Open => "O",
                PageTrigger::Close => "C",
            };
            additional
                .as_dict()
                .and_then(|additional| additional.get(key).cloned())
                .map(|entry| scripts_in(document, &entry))
                .unwrap_or_default()
        };
        let annotation_trigger = match trigger {
            PageTrigger::Open => AnnotationTrigger::PageOpen,
            PageTrigger::Close => AnnotationTrigger::PageClose,
        };
        let mut total = (0_usize, false, false);
        let mut add = |ran: (usize, bool, bool)| {
            total = (
                total.0.saturating_add(ran.0),
                total.1 | ran.1,
                total.2 | ran.2,
            );
        };
        let run_page = |view: &mut Self| {
            let mut ran = (0_usize, false, false);
            for script in &page_scripts {
                let event = ScriptEvent {
                    script,
                    page,
                    ..ScriptEvent::at(ScriptSite::Page(trigger), "")
                };
                if let Some((result, applied)) = view.run_event(document, table, event) {
                    ran = (
                        ran.0.saturating_add(1),
                        ran.1 | applied.values,
                        ran.2 | applied.calculate,
                    );
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
            add(run_page(self));
        }
        for annotation in annotations {
            let Some(dictionary) = document.get(annotation).as_dict().cloned() else {
                continue;
            };
            add(self.annotation_scripts(
                document,
                table,
                annotation,
                &dictionary,
                annotation_trigger,
                page,
            ));
        }
        if trigger == PageTrigger::Close {
            add(run_page(self));
        }
        total
    }

    /// One annotation's scripts for one of Table 197's events.
    fn annotation_scripts(
        &mut self,
        document: &Document,
        table: &BTreeMap<String, Vec<ObjectId>>,
        annotation: ObjectId,
        dictionary: &Dictionary,
        trigger: AnnotationTrigger,
        page: usize,
    ) -> (usize, bool, bool) {
        let scripts = annotation_chain(document, dictionary, trigger)
            .map(|entry| scripts_in(document, &entry))
            .unwrap_or_default();
        if scripts.is_empty() {
            return (0, false, false);
        }
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
        let mut ran = (0_usize, false, false);
        for script in &scripts {
            let event = ScriptEvent {
                script,
                value: &value,
                page,
                ..ScriptEvent::at(ScriptSite::Annotation(trigger), &field)
            };
            if let Some((result, applied)) = self.run_event(document, table, event) {
                ran = (
                    ran.0.saturating_add(1),
                    ran.1 | applied.values,
                    ran.2 | applied.calculate,
                );
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

/// The zero-based page an annotation's Table 166 `/P` names, or page one where it names none.
fn page_of(document: &Document, annotation: &Dictionary) -> usize {
    let Some(page) = annotation.get("P").and_then(Object::as_reference) else {
        return 0;
    };
    crate::page::Pages::new(document)
        .index_of(page)
        .unwrap_or_default()
}

/// How a page's event names itself in a report.
fn page_noun(trigger: PageTrigger) -> &'static str {
    match trigger {
        PageTrigger::Open => "/O",
        PageTrigger::Close => "/C",
    }
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

/// The text of every ECMAScript action in one chain, in §12.6.2's execution order — an action,
/// then its `/Next` subtree, then the next sibling — each action dictionary visited once.
fn scripts_in(document: &Document, entry: &Object) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut visited = 0_usize;
    walk(document, entry, &mut out, &mut seen, &mut visited);
    out
}

/// Appends one action's script and its `/Next` subtree's.
fn walk(
    document: &Document,
    entry: &Object,
    out: &mut Vec<String>,
    seen: &mut BTreeSet<ObjectId>,
    visited: &mut usize,
) {
    if *visited >= MAX_CHAIN {
        return;
    }
    *visited = visited.saturating_add(1);
    if let Object::Reference(id) = entry
        && !seen.insert(*id)
    {
        return;
    }
    let resolved = document.resolve(entry);
    let Some(action) = resolved.as_dict() else {
        return;
    };
    if let Some(script) = script_text(document, action) {
        out.push(script);
    }
    let next = action.get("Next").cloned().unwrap_or(Object::Null);
    match document.resolve(&next) {
        Object::Array(items) => {
            for item in &items {
                walk(document, item, out, seen, visited);
            }
        }
        Object::Dictionary(_) => walk(document, &next, out, seen, visited),
        _ => {}
    }
}
