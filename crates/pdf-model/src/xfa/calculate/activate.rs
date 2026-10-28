//! What runs, and in what order, when a stimulus reaches a form: XFA 3.3's activation of the
//! automation objects (pages 379–380 and 404–407), as a work list that ends under a budget
//! (ADRs 2506, 2507).
//!
//! This module reads no script language. A [`Runner`] — written by the program that runs scripts,
//! because this crate does not depend on an interpreter (ADR 2482), and the shape of the runner
//! `pdf-script` is given (ADR 1591) — runs one script's text
//! against the [`Form`] it is handed and says what it computed and how many steps it took.
//!
//! **The order** (page 379, restated by the rules of pages 404–407):
//!
//! - A person's entry ([`Stimulus::Entry`]): the field's `change` events, then the calculations
//!   that depend on the value, then the validations. Page 391 says a `change` event is the
//!   person's own and a calculation or a merge does not fire it.
//! - The completion of a merge ([`Stimulus::Merge`]): every calculation, then the validations,
//!   then the `initialize` events. Page 379's general order puts events first; page 407's third
//!   rule is the specific one for this stimulus and puts the `initialize` events last, so the
//!   specific rule is the one followed (ADR 2506).
//! - Value calculations come before property calculations, and among the property calculations
//!   the inner subform before the outer one (page 405, rule 1, items 4 and 5). Among value
//!   calculations the order is the template's own, except that a calculation comes after the
//!   ones whose containers it reads. Pages 404–405 leave the order of one event's scripts
//!   undefined and report that Adobe's processor runs them in document order, which is the
//!   choice made here for events.
//!
//! **The cascade** (pages 379–380): a calculation is run again when a container it names has
//! changed. What it names is what [`Form::resolve`] returned while it ran, so the dependency
//! follows the path the script took. A subform counts as changed whenever a container inside it
//! does (page 379), so a change wakes the calculation of every subform around it. A calculation
//! never wakes itself, because a script that reads its own value is the circular reference page
//! 380 leaves to the form's creator, and two calculations that wake each other for ever are
//! ended by [`Limits`], the means of terminating a seemingly infinite loop that page 380
//! recommends a processor have.
//!
//! **The end.** An activation that exhausts a limit changes nothing: every value it calculated
//! is dropped and the report says so, as a script run that did not finish changes nothing in
//! `pdf-script` (ADR 1603 section 3; ADR 2507 decides the limits). The work list is a set taken smallest first and the loop is
//! bounded by [`Limits::activations`], so a hostile template costs time and never the call
//! stack.

use std::collections::{BTreeMap, BTreeSet};

use super::{Calculation, Form, Scalar};
use crate::xfa::datasets::Change;
use crate::xfa::template::{Language, Override};
use crate::xfa::{Resource, Template};

/// What a [`Runner`] says of one script.
#[derive(Debug, Clone, PartialEq)]
pub struct Ran {
    /// How many steps the script took. More than the `steps_left` it was handed says it ran into
    /// that ceiling, and the cascade ends there (page 380).
    pub steps: u64,
    /// The value the script computed, or the sentence saying why it computed none. A script
    /// refused part-way has still made the assignments it made before the refusal.
    pub outcome: Result<Scalar, String>,
}

/// The language of a script a [`Runner`] is handed: the two a `script` element's `contentType`
/// names (XFA 3.3 pages 106 and 412), a script in any other being refused before a runner sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormLanguage {
    /// Chapter 25's language, `application/x-formcalc` or no `contentType`.
    FormCalc,
    /// `application/x-javascript` (page 106), reaching the form through its `rawValue` and its
    /// SOM expressions.
    JavaScript,
}

impl FormLanguage {
    /// The language a script was written in, where a runner is handed it.
    #[must_use]
    pub fn of(language: &Language) -> Option<Self> {
        match language {
            Language::FormCalc => Some(Self::FormCalc),
            Language::JavaScript => Some(Self::JavaScript),
            Language::Other(_) => None,
        }
    }

    /// The language's name in a sentence.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::FormCalc => "FormCalc",
            Self::JavaScript => "JavaScript",
        }
    }
}

/// Runs one script of a form's, in either language page 412 names.
pub trait Runner {
    /// Runs `script`, written in `language`, as the script of the container `node` against
    /// `form`, within `steps_left` steps. The script's assignments go through [`Form::set`].
    fn run(
        &mut self,
        form: &mut Form<'_>,
        node: usize,
        language: FormLanguage,
        script: &str,
        steps_left: u64,
    ) -> Ran;
}

/// The ceilings of one activation, which no single script's budget sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// How many scripts may be run, the discovery of what each reads included.
    pub activations: usize,
    /// How many steps the scripts may take between them.
    pub steps: u64,
    /// How many container names the calculations may have named between them, the dependencies
    /// the cascade keeps: a script that names a thousand rows is a form, and a million names in
    /// all is a template built to hold memory (principle 3).
    pub names: usize,
    /// How many bytes of text the scripts may assign between them, an overwrite charged as much as
    /// a first assignment, the discovery runs included: a calculation that builds a megabyte
    /// string and assigns it to every field of a template is memory that no step count sees
    /// (principle 3).
    pub bytes: u64,
}

impl Limits {
    /// Ten thousand runs, ten million steps and 64 MiB of assigned text (ADR 2507): the
    /// census's largest form carries at most a few dozen calculation scripts (ADR 2482), a cascade
    /// settles in a few runs per calculation, and each script is held to a hundred thousand steps
    /// of its own, so this is two orders past what a form needs and a fraction of a second of
    /// work at most.
    pub const DEFAULT: Self = Self {
        activations: 10_000,
        steps: 10_000_000,
        names: 1_000_000,
        bytes: 64 << 20,
    };
}

/// What reached the form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stimulus<'a> {
    /// Data binding finished (page 379): every calculation is activated.
    Merge,
    /// A person entered values into these fields, named by their PDF field names.
    Entry(&'a [String]),
}

/// How an activation ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// No calculation was left to run.
    Settled,
    /// The named limit was reached and the activation's edits were dropped: `activations`,
    /// `steps`, `names` or `bytes`, each a field of [`Limits`].
    Budget(&'static str),
}

/// A field a person entered a value into whose calculation says the entry is the host's to
/// present (XFA 3.3 pages 641–642): the `error` message, or the `warning`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overridden {
    /// The field, by its PDF field name.
    pub field: String,
    /// What the calculation's `override` says: [`Override::Error`] or [`Override::Warning`].
    pub mode: Override,
}

/// What an activation did: the edits the scripts made, beside a person's, and what it refused.
#[derive(Debug, Clone, PartialEq)]
pub struct Activation {
    /// The fields whose value a script changed, by PDF field name, in document order. Empty
    /// where [`Self::ended`] is a budget.
    pub edits: Vec<(String, Scalar)>,
    /// Every sentence the activation owes a report: each script refused or stopped, by the field
    /// it belongs to, and the limit it ended on.
    pub report: Vec<String>,
    /// How it ended.
    pub ended: Ended,
    /// How many scripts were run.
    pub activations: usize,
    /// The messages of the validations the form fails when they run, after the calculations have
    /// settled and, on a merge, before the `initialize` events: one `Vec` per message the person
    /// is shown ([`Resource::before_save`]).
    pub validations: Vec<Vec<String>>,
    /// The fields of a person's entry whose calculation has `override` `error` or `warning`, in
    /// the order entered: the host presents the message page 642 asks for, an error for the first
    /// and a warning for the second. Empty where [`Self::ended`] is a budget, which keeps neither
    /// the calculated values nor the replacement of the entry.
    pub overridden: Vec<Overridden>,
}

impl Activation {
    /// The activation as the outcome of a script run: each calculated value an edit a person could
    /// have made, so that it reaches the `/V` and the `datasets` packet the way a typed one does
    /// (RFC 0008 section 6.4; ADR 2396).
    #[must_use]
    pub fn result(&self) -> crate::view::ScriptResult {
        crate::view::ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits: self
                .edits
                .iter()
                .map(|(field, value)| crate::view::ScriptEdit::Value {
                    field: field.clone(),
                    value: text(value),
                })
                .collect(),
            report: self.report.clone(),
        }
    }
}

/// A number as text, with at most eleven fractional digits and no trailing zeros: the precision
/// chapter 25 displays a result at (page 1062), which a field takes as a person would type it.
fn text(value: &Scalar) -> String {
    match value {
        Scalar::Null => String::new(),
        Scalar::Text(text) => text.clone(),
        Scalar::Number(n) => {
            let written = format!("{n:.11}");
            let trimmed = written.trim_end_matches('0').trim_end_matches('.');
            if trimmed == "-0" || trimmed.is_empty() {
                "0".to_owned()
            } else {
                trimmed.to_owned()
            }
        }
    }
}

/// The most sentences one activation reports; past it the rest are counted, so that a template
/// of thousands of refused scripts costs a counter and not its own size in strings.
const MAX_REPORT: usize = 256;

/// The most characters one reported sentence keeps.
const MAX_SENTENCE: usize = 512;

impl Resource {
    /// Activates the automation objects a `stimulus` reaches, in page 379's order, and says what
    /// they changed. `fields` are every field of the form by its PDF name with the value its
    /// `/V` carries, a person's entry included.
    ///
    /// Only `FormCalc` runs; a script in another language is refused by name (RFC 0011 section 5's
    /// step 5). The validations of the settled form are in the result, run as a save runs them.
    #[must_use]
    pub fn activate(
        &self,
        fields: &[(String, Change)],
        stimulus: Stimulus<'_>,
        runner: &mut dyn Runner,
        limits: Limits,
    ) -> Activation {
        let mut form = self.form(fields);
        form.limit_text(limits.bytes);
        let initial = form.values.clone();
        let mut session = Session::new(&mut form, runner, limits, &self.template);
        match stimulus {
            Stimulus::Entry(names) => {
                let typed = session.typed(names);
                session.fire("change", Some(&typed));
                session.settle(Seeds::Entered(&typed));
            }
            Stimulus::Merge => session.settle(Seeds::All),
        }
        let mut validations = Vec::new();
        if session.ended.is_none() {
            validations = self.before_save(&session.after(fields, &initial));
        }
        if matches!(stimulus, Stimulus::Merge) {
            session.fire("initialize", None);
            session.settle(Seeds::Changed);
        }
        session.finish(&initial, validations)
    }
}

/// What starts a pass of the work list.
#[derive(Clone, Copy)]
enum Seeds<'a> {
    /// Every calculation.
    All,
    /// The calculations that depend on what changed, and the calculations of the containers a
    /// person entered values into that a person may not override.
    Entered(&'a [usize]),
    /// The calculations that depend on what changed.
    Changed,
}

/// One activation in progress.
struct Session<'f, 'a, 'r> {
    form: &'f mut Form<'a>,
    template: &'a Template,
    runner: &'r mut dyn Runner,
    calculations: Vec<Calculation>,
    /// What each calculation (by position) named at its last run.
    named: Vec<BTreeSet<usize>>,
    /// The subform calculations by the container they belong to.
    properties: BTreeMap<usize, usize>,
    /// The inverse of `named`: the calculations that name each container.
    readers_of: BTreeMap<usize, BTreeSet<usize>>,
    /// How many names `named` holds in all.
    edges: usize,
    /// The most names `edges` may reach ([`Limits::names`]).
    max_names: usize,
    /// Whether discovery has filled `named` and `order`.
    known: bool,
    /// Each calculation's place in the order its inputs come before it, by position.
    order: Vec<usize>,
    activations_left: usize,
    steps_left: u64,
    run: usize,
    report: Vec<String>,
    reported: BTreeSet<String>,
    unreported: usize,
    ended: Option<&'static str>,
    /// The entries whose `override` the host presents.
    overridden: Vec<Overridden>,
}

impl<'f, 'a, 'r> Session<'f, 'a, 'r> {
    fn new(
        form: &'f mut Form<'a>,
        runner: &'r mut dyn Runner,
        limits: Limits,
        template: &'a Template,
    ) -> Self {
        let calculations = form.calculations();
        let named = vec![BTreeSet::new(); calculations.len()];
        let properties = calculations
            .iter()
            .enumerate()
            .filter(|(_, calculation)| !calculation.sets_value)
            .map(|(index, calculation)| (calculation.node, index))
            .collect();
        Self {
            form,
            template,
            runner,
            calculations,
            named,
            properties,
            readers_of: BTreeMap::new(),
            edges: 0,
            max_names: limits.names,
            known: false,
            order: Vec::new(),
            activations_left: limits.activations,
            steps_left: limits.steps,
            run: 0,
            report: Vec::new(),
            reported: BTreeSet::new(),
            unreported: 0,
            ended: None,
            overridden: Vec::new(),
        }
    }

    /// The containers of the PDF field names a person entered values into.
    fn typed(&mut self, names: &[String]) -> Vec<usize> {
        let mut typed = Vec::new();
        for name in names {
            match self.form.named(name) {
                Some(node) => typed.push(node),
                None => self.say(format!("{name}: the form's field list does not name it")),
            }
        }
        typed
    }

    /// Records a sentence once.
    fn say(&mut self, mut sentence: String) {
        if self.report.len() >= MAX_REPORT {
            self.unreported = self.unreported.saturating_add(1);
            return;
        }
        // A name or a script's own message may be as long as the template, so a sentence is cut
        // where it is kept; the set that dedupes holds no more than the report does.
        if let Some((at, _)) = sentence.char_indices().nth(MAX_SENTENCE) {
            sentence.truncate(at);
        }
        if self.reported.insert(sentence.clone()) {
            self.report.push(sentence);
        }
    }

    /// What the container is called in a sentence.
    fn label(&self, node: usize) -> String {
        self.form
            .names
            .get(node)
            .cloned()
            .flatten()
            .or_else(|| self.template.get(node).and_then(|c| c.name.clone()))
            .unwrap_or_else(|| "an unnamed container".to_owned())
    }

    /// Runs `script` on `form` (the session's or a scratch copy of it) if a limit allows, and
    /// charges what it took. `Err` is the sentence for a script that ran no further.
    fn execute(
        &mut self,
        scratch: Option<&mut Form<'a>>,
        node: usize,
        language: &Language,
        script: &str,
    ) -> Result<Scalar, String> {
        let language = match (FormLanguage::of(language), language) {
            (Some(known), _) => known,
            (None, Language::Other(other)) => {
                return Err(format!("its script is in {other}, which no step runs"));
            }
            (None, _) => return Err("its script is in a language no step runs".to_owned()),
        };
        if self.activations_left == 0 {
            self.ended.get_or_insert("activations");
            return Err("the activation ran out of runs".to_owned());
        }
        if self.steps_left == 0 {
            self.ended.get_or_insert("steps");
            return Err("the activation ran out of steps".to_owned());
        }
        self.activations_left = self.activations_left.saturating_sub(1);
        self.run = self.run.saturating_add(1);
        let handed = self.steps_left;
        let form = match scratch {
            Some(form) => form,
            None => &mut *self.form,
        };
        let ran = self.runner.run(form, node, language, script, handed);
        if form.take_text_exhausted() {
            self.ended.get_or_insert("bytes");
        }
        self.steps_left = handed.saturating_sub(ran.steps);
        if ran.steps > handed {
            self.ended.get_or_insert("steps");
        }
        ran.outcome
    }

    /// Fires the `activity` events of the containers `only` names, or of every container, in
    /// document order (page 404's order for scripts the standard leaves unordered).
    fn fire(&mut self, activity: &str, only: Option<&[usize]>) {
        let nodes: Vec<usize> = match only {
            Some(only) => only.to_vec(),
            None => (0..self.template.container_count()).collect(),
        };
        for node in nodes {
            let Some(container) = self.template.get(node) else {
                continue;
            };
            let scripts: Vec<_> = container
                .events
                .iter()
                .filter(|event| event.own && event.activity == activity)
                .map(|event| event.script.clone())
                .collect();
            for script in scripts {
                if self.ended.is_some() {
                    return;
                }
                if let Err(why) = self.execute(None, node, &script.language, &script.text) {
                    let label = self.label(node);
                    self.say(format!("{label}: its {activity} event: {why}"));
                }
            }
        }
    }

    /// The position in `calculations` of the container's calculation.
    fn index_of(&self, node: usize) -> Option<usize> {
        self.calculations.iter().position(|c| c.node == node)
    }

    /// Records what the calculation at `index` named when it last ran, keeping `readers_of` its
    /// inverse. A calculation does not wake itself (page 380's circular reference is the form
    /// creator's to prevent), so its own container is left out.
    fn record(&mut self, index: usize, mut named: BTreeSet<usize>) {
        let Some(calculation) = self.calculations.get(index) else {
            return;
        };
        named.remove(&calculation.node);
        let old = self
            .named
            .get_mut(index)
            .map(std::mem::take)
            .unwrap_or_default();
        self.edges = self.edges.saturating_sub(old.len());
        for node in &old {
            if let Some(readers) = self.readers_of.get_mut(node) {
                readers.remove(&index);
            }
        }
        self.edges = self.edges.saturating_add(named.len());
        for &node in &named {
            self.readers_of.entry(node).or_default().insert(index);
        }
        if let Some(slot) = self.named.get_mut(index) {
            *slot = named;
        }
        if self.edges > self.max_names {
            self.ended.get_or_insert("names");
        }
    }

    /// How deep the container is: the count of containers above it.
    fn depth(&self, node: usize) -> usize {
        let mut depth = 0_usize;
        let mut at = self.template.get(node).and_then(|c| c.parent);
        while let Some(parent) = at {
            depth = depth.saturating_add(1);
            if depth > self.template.container_count() {
                break;
            }
            at = self.template.get(parent).and_then(|c| c.parent);
        }
        depth
    }

    /// The rank of a calculation in the work list: value calculations before property
    /// calculations, inner before outer among those, then the template's order.
    fn rank(&self, index: usize) -> (bool, usize, usize) {
        match self.calculations.get(index) {
            Some(calculation) if calculation.sets_value => (
                false,
                self.order.get(index).copied().unwrap_or(index),
                index,
            ),
            Some(calculation) => (
                true,
                usize::MAX.saturating_sub(self.depth(calculation.node)),
                index,
            ),
            None => (true, usize::MAX, index),
        }
    }

    /// Learns what every calculation names, by running each once on a copy of the form, because
    /// a person's entry has to find the calculations that read it before any of them has run.
    fn discover(&mut self) {
        self.known = true;
        let mut scratch = self.form.clone();
        let room = scratch.text_room();
        for index in 0..self.calculations.len() {
            let Some(calculation) = self.calculations.get(index).cloned() else {
                continue;
            };
            if calculation.override_mode == Override::Disabled || self.ended.is_some() {
                continue;
            }
            scratch.take_resolved();
            // Whether it ran or was refused, what it resolved before stopping is what it reads.
            let outcome = self.execute(
                Some(&mut scratch),
                calculation.node,
                &calculation.language,
                &calculation.script,
            );
            if let (Ok(value), true) = (outcome, calculation.sets_value) {
                let _ = scratch.set(calculation.node, value);
                if scratch.take_text_exhausted() {
                    self.ended.get_or_insert("bytes");
                }
            }
            let named = scratch.take_resolved();
            self.record(index, named);
        }
        // What the copy stored is charged to the form the activation keeps.
        self.form.limit_text(
            self.form
                .text_room()
                .saturating_sub(room.saturating_sub(scratch.text_room())),
        );
        self.order = self.inputs_first();
    }

    /// The position of each calculation in an order that runs the calculations a calculation reads
    /// before it: Kahn's algorithm over the names they hold, taking the template's order among
    /// those that are ready, and where only a cycle is left, the first of it in the template's
    /// order. The cascade of pages 379–380 needs no order to be right, but it needs this one to be
    /// quick: run in the template's order, a chain whose links come in reverse runs each link
    /// again for every link after it.
    fn inputs_first(&self) -> Vec<usize> {
        let count = self.calculations.len();
        let mut before: Vec<usize> = vec![0; count];
        let mut after: Vec<Vec<usize>> = vec![Vec::new(); count];
        for (index, calculation) in self.calculations.iter().enumerate() {
            if !calculation.sets_value {
                continue;
            }
            let Some(readers) = self.readers_of.get(&calculation.node) else {
                continue;
            };
            for &reader in readers {
                let reads_value = self.calculations.get(reader).is_some_and(|c| c.sets_value);
                if reader != index
                    && reads_value
                    && let (Some(count), Some(list)) =
                        (before.get_mut(reader), after.get_mut(index))
                {
                    *count = count.saturating_add(1);
                    list.push(reader);
                }
            }
        }
        let mut ready: BTreeSet<usize> = (0..count).filter(|&i| before[i] == 0).collect();
        let mut placed = vec![usize::MAX; count];
        let mut next = 0_usize;
        let mut waiting: BTreeSet<usize> = (0..count).filter(|&i| before[i] != 0).collect();
        while next < count {
            let Some(index) = ready.pop_first().or_else(|| waiting.pop_first()) else {
                break;
            };
            waiting.remove(&index);
            if let Some(slot) = placed.get_mut(index) {
                *slot = next;
            }
            next = next.saturating_add(1);
            for &reader in after.get(index).map_or(&[][..], Vec::as_slice) {
                if placed.get(reader) != Some(&usize::MAX) {
                    continue;
                }
                if let Some(count) = before.get_mut(reader) {
                    *count = count.saturating_sub(1);
                    if *count == 0 && waiting.remove(&reader) {
                        ready.insert(reader);
                    }
                }
            }
        }
        placed
    }

    /// What the `override` of each calculation of a field a person entered into does (pages
    /// 641–642): `error` and `ignore` put the calculation back on the work list, so the calculated
    /// value replaces the person's; `warning` leaves the person's value standing. The messages the
    /// page asks to be presented are the host's: [`Activation::overridden`] names the fields they
    /// are owed for, and the report says so in words.
    fn entered_overrides(&mut self, typed: &[usize], wanted: &mut BTreeSet<usize>) {
        for &node in typed {
            let Some(index) = self.index_of(node) else {
                continue;
            };
            let mode = self.calculations.get(index).map(|c| c.override_mode);
            let label = self.label(node);
            if let (Some(mode @ (Override::Error | Override::Warning)), Some(field)) =
                (mode, self.form.names.get(node).cloned().flatten())
            {
                self.overridden.push(Overridden { field, mode });
            }
            match mode {
                Some(Override::Error) => {
                    wanted.insert(index);
                    self.say(format!(
                        "{label}: its calculation has override error, so the value \
                         entered is replaced by the calculated one; the error message \
                         page 642 asks to be presented is the host's to show"
                    ));
                }
                Some(Override::Ignore) => {
                    wanted.insert(index);
                }
                Some(Override::Warning) => self.say(format!(
                    "{label}: its calculation has override warning, so the value entered \
                     stands; the warning page 642 asks to be presented, with its choice \
                     of dismissing or overriding, is the host's to show"
                )),
                Some(Override::Disabled) | None => {}
            }
        }
    }

    /// Runs the work list from `seeds` until nothing is left to run or a limit ends it.
    fn settle(&mut self, seeds: Seeds<'_>) {
        if self.ended.is_some() {
            return;
        }
        let mut wanted: BTreeSet<usize> = BTreeSet::new();
        match seeds {
            Seeds::All => {
                self.discover();
                wanted.extend(0..self.calculations.len());
            }
            Seeds::Entered(typed) => {
                if !self.known {
                    self.discover();
                }
                self.entered_overrides(typed, &mut wanted);
                let mut changed: BTreeSet<usize> = self.form.take_changed();
                changed.extend(typed.iter().copied());
                wanted.extend(self.readers(&changed, None));
            }
            Seeds::Changed => {
                let changed = self.form.take_changed();
                if !self.known && !changed.is_empty() {
                    self.discover();
                }
                wanted.extend(self.readers(&changed, None));
            }
        }
        wanted.retain(|&index| {
            self.calculations
                .get(index)
                .is_some_and(|c| c.override_mode != Override::Disabled)
        });
        let mut queue: BTreeSet<(bool, usize, usize)> =
            wanted.iter().map(|&index| self.rank(index)).collect();
        let mut queued = wanted;
        while let Some((_, _, index)) = queue.pop_first() {
            queued.remove(&index);
            if self.ended.is_some() {
                return;
            }
            let Some(calculation) = self.calculations.get(index).cloned() else {
                continue;
            };
            self.form.take_resolved();
            let _ = self.form.take_changed();
            let outcome = self.execute(
                None,
                calculation.node,
                &calculation.language,
                &calculation.script,
            );
            let named = self.form.take_resolved();
            match outcome {
                Ok(value) if calculation.sets_value => {
                    let stored = self.form.set(calculation.node, value);
                    if self.form.take_text_exhausted() {
                        self.ended.get_or_insert("bytes");
                    }
                    if let Err(why) = stored {
                        let label = self.label(calculation.node);
                        self.say(format!("{label}: {why}"));
                    }
                }
                Err(why) if self.ended.is_none() => {
                    let label = self.label(calculation.node);
                    self.say(format!("{label}: its calculation: {why}"));
                }
                Ok(_) | Err(_) => {}
            }
            self.record(index, named);
            let changed = self.form.take_changed();
            let woken: BTreeSet<usize> = self
                .readers(&changed, Some(index))
                .into_iter()
                .filter(|j| !queued.contains(j))
                .filter(|&j| {
                    self.calculations
                        .get(j)
                        .is_some_and(|c| c.override_mode != Override::Disabled)
                })
                .collect();
            queue.extend(woken.iter().map(|&index| self.rank(index)));
            queued.extend(woken);
        }
    }

    /// The calculations a change to `changed` wakes, other than `except`: those that named one of
    /// them, and a subform's, which counts as changed whenever a container inside it does
    /// (page 379).
    fn readers(&self, changed: &BTreeSet<usize>, except: Option<usize>) -> Vec<usize> {
        let mut woken: BTreeSet<usize> = changed
            .iter()
            .filter_map(|node| self.readers_of.get(node))
            .flatten()
            .copied()
            .collect();
        let mut enclosing = BTreeSet::new();
        for &node in changed {
            let mut at = self.template.get(node).and_then(|c| c.parent);
            while let Some(parent) = at {
                if !enclosing.insert(parent) {
                    break;
                }
                at = self.template.get(parent).and_then(|c| c.parent);
            }
        }
        woken.extend(
            enclosing
                .iter()
                .filter_map(|node| self.properties.get(node)),
        );
        woken.retain(|&index| Some(index) != except);
        woken.into_iter().collect()
    }

    /// The form's fields after the edits so far, as the validations of the save read them.
    fn after(&self, fields: &[(String, Change)], initial: &[Scalar]) -> Vec<(String, Change)> {
        fields
            .iter()
            .map(|(name, change)| {
                let node = self.form.named(name);
                let current = node.map(|node| self.form.value(node));
                match (node, current) {
                    (Some(node), Some(now)) if initial.get(node) != Some(&now) => {
                        (name.clone(), Change::Text(text(&now)))
                    }
                    _ => (name.clone(), change.clone()),
                }
            })
            .collect()
    }

    /// The activation's answer.
    fn finish(mut self, initial: &[Scalar], validations: Vec<Vec<String>>) -> Activation {
        if let Some(limit) = self.ended {
            let sentence = format!(
                "the activation ended on its {limit} limit after {} runs, and nothing it \
                 calculated is kept (page 380)",
                self.run
            );
            // The limit's sentence is the one a person most needs, so it is kept past the cap.
            self.report.push(sentence);
        }
        let mut edits = Vec::new();
        if self.ended.is_none() {
            for node in 0..self.form.assigned.len() {
                if !self.form.assigned.get(node).copied().unwrap_or(false)
                    || self.form.values.get(node) == initial.get(node)
                {
                    continue;
                }
                if let Some(name) = self.form.names.get(node).cloned().flatten() {
                    edits.push((name, self.form.value(node)));
                } else {
                    let label = self.label(node);
                    self.say(format!(
                        "{label}: a script set its value, and the form's field list does not \
                         name it, so the value has no field to go to"
                    ));
                }
            }
        }
        if self.unreported > 0 {
            let sentence = format!(
                "and {} more sentences, past the {MAX_REPORT} an activation reports",
                self.unreported
            );
            self.report.push(sentence);
        }
        Activation {
            edits,
            report: self.report,
            ended: self.ended.map_or(Ended::Settled, Ended::Budget),
            activations: self.run,
            validations: if self.ended.is_none() {
                validations
            } else {
                Vec::new()
            },
            overridden: if self.ended.is_none() {
                self.overridden
            } else {
                Vec::new()
            },
        }
    }
}
