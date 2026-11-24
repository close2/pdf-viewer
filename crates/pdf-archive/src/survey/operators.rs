//! The content-stream loop: reading one stream's tokens and acting on each operator.

use pdf_model::content::grammar::{Grammar, Operand};
use pdf_model::content::reader::ContentReader;
use pdf_syntax::Dictionary;

use super::{
    Context, DefaultSpace, DeviceFamily, Extent, MAX_NESTING, Observation, Operands, Operator,
    Origin, PropertyList, Selected, State, Step, Walk, next_step,
};

/// What a step is as an operand of the grammar's operators, or `None` for one that is not an
/// operand.
const fn grammar_operand(step: &Step) -> Option<Operand> {
    Some(match step {
        Step::Name(_) => Operand::Name,
        Step::Integer(_) | Step::WideInteger(_) | Step::Real(_) => Operand::Number,
        Step::Text(_) => Operand::Text,
        Step::Dictionary => Operand::Dictionary,
        Step::Other => Operand::Other,
        Step::Operator(..) | Step::Unlisted(_) | Step::End => return None,
    })
}

impl Walk<'_> {
    /// Runs one content stream, in the state it inherits.
    ///
    /// ISO 32000-2 §8.10.1 starts a form `XObject` in the graphics state in force at `Do`, so
    /// the inherited state is a copy and what the form does to it does not return.
    #[expect(
        clippy::too_many_lines,
        reason = "one match over the walk's steps whose arms share the operands being gathered; \
                  splitting it would pass those six locals to every piece"
    )]
    pub(super) fn run(
        &mut self,
        reader: &mut ContentReader<'_>,
        resources: &Dictionary,
        page: usize,
        depth: u32,
        initial: &State,
        origin: Origin,
    ) {
        let mut state = initial.clone();
        // `TechNote 0010` A028: parts 2 and 3 are read as if a default colour space had to be
        // defined in the *explicitly associated* resources dictionary, and as if a processor
        // ignored what that dictionary does not define. Both readings are recorded here because
        // the resolution does not name ISO 19005-4; see [`DefaultSpace`].
        let explicit = origin.explicitly_associated;
        let defaults = [
            DefaultSpace::read(self.default_space(resources, DeviceFamily::Gray), explicit),
            DefaultSpace::read(self.default_space(resources, DeviceFamily::Rgb), explicit),
            DefaultSpace::read(self.default_space(resources, DeviceFamily::Cmyk), explicit),
        ];
        let mut stack: Vec<State> = Vec::new();
        let mut names: Vec<Vec<u8>> = Vec::new();
        // Every string operand since the last operator: `Tj` takes one, `TJ` takes the strings
        // of its array, and `\"` takes one after two numbers. Keeping all of them and letting
        // the operator decide is what makes those three the same case here.
        let mut strings: Vec<Vec<u8>> = Vec::new();
        let mut numbers = Numbers::default();
        let mut strings_complete = true;
        // A show that moved where the reader did not: [`Self::place_from`], ISO 32000-2 §9.4.4.
        let mut displaced: Option<super::MovingShow> = None;
        // §14.6.2's property list, where the operator being assembled was given one written
        // into its operands rather than named through the resources.
        let mut property: Option<Dictionary> = None;
        // Whether the last operand was a name with another operand before it: §14.6.2's second
        // form, `tag properties BDC`, names its property list in the last of two operands
        // whatever the tag was written as (ADR 3396).
        let mut operands_read = 0_usize;
        let mut last_was_name = false;
        // Where this stream found the marked-content stack. A stream that opens more sequences
        // than it closes leaves them here, and one that closes more would otherwise close its
        // caller's; §14.6.1 makes neither a thing a conforming stream does, so the depth is
        // restored on the way out rather than the imbalance reported.
        let base = self.marks.len();
        // §7.8.2's operands, §8.4.2's balance, §9.4.1's text object and §8.6.8's component count,
        // which the interpreter repairs where a stream breaks them and a validator that does not
        // interpret reads off the tokens (ADR 3541). What it finds goes with the stream's lexical
        // repairs, which the caller records at the stream's content site.
        let mut grammar = Grammar::default();
        loop {
            if self.budget == 0 {
                self.survey.stopped_short = true;
                self.marks.truncate(base);
                return;
            }
            self.budget = self.budget.saturating_sub(1);
            let step = next_step(reader);
            let names_it = matches!(&step, Step::Name(name) if name == b"ActualText");
            if matches!(
                step,
                Step::Name(_)
                    | Step::Integer(_)
                    | Step::WideInteger(_)
                    | Step::Real(_)
                    | Step::Text(_)
                    | Step::Dictionary
                    | Step::Other
            ) {
                operands_read = operands_read.saturating_add(1);
                last_was_name = matches!(step, Step::Name(_));
            }
            if let Some(operand) = grammar_operand(&step) {
                grammar.operand(operand);
            }
            match step {
                Step::End => {
                    self.marks.truncate(base);
                    return;
                }
                Step::Other => {}
                Step::Dictionary => {
                    let dict = pdf_model::content::reader::inline_dictionary(reader);
                    self.note_inline_dictionary(&dict, page);
                    property = Some(dict);
                }
                Step::Name(name) => {
                    self.note_length(name.len(), page, Extent::Name);
                    self.survey.names_actual_text |= names_it;
                    // At most two are ever wanted: `Tf` and `Do` take the first, and `scn`
                    // takes the last. Keeping two rather than every operand is what stops a
                    // stream of names from growing this vector without bound.
                    if names.len() < 2 {
                        names.push(name);
                    } else {
                        names[1] = name;
                    }
                }
                // The last integer, because `Tr` takes exactly one and it comes last.
                Step::Integer(value) => {
                    self.note_integer(i128::from(value), page);
                    numbers.integer(value);
                }
                Step::Real(value) => {
                    self.note_real(value, page);
                    numbers.real(value);
                }
                Step::WideInteger(value) => {
                    self.note_integer(value, page);
                    numbers.wide_integer();
                }
                Step::Text(text) => {
                    self.note_length(text.len(), page, Extent::String);
                    strings_complete &= self.keep_string(text, &mut strings);
                }
                Step::Unlisted(word) => {
                    // A run the drawing reader salvages is an operand to the operator it draws;
                    // anything else no operator summary lists ends the operands before it.
                    if pdf_model::content::grammar::drawn_as_a_number(&word) {
                        grammar.operand(Operand::Number);
                    } else {
                        grammar.boundary();
                    }
                    if let Some(ledger) = &mut self.ledger {
                        ledger.operator(&word);
                    }
                    self.note_unlisted_operator(&word, page, origin);
                    names.clear();
                    strings.clear();
                    strings_complete = true;
                    numbers = Numbers::default();
                    property = None;
                    operands_read = 0;
                    last_was_name = false;
                }
                Step::Operator(operator, signature) => {
                    if let Some(ledger) = &mut self.ledger {
                        ledger.operator(operator.label().as_bytes());
                    }
                    match signature.map(|signature| grammar.operator(signature)) {
                        Some(Some(class)) => reader.note_repair(class),
                        Some(None) => {}
                        None => grammar.boundary(),
                    }
                    let context = Context {
                        resources,
                        defaults,
                        page,
                        depth,
                        what: origin.what,
                        record: origin.record,
                        marks_base: base,
                    };
                    self.place_from(&operator, &state, &context, &mut displaced);
                    self.apply(
                        &operator,
                        &mut state,
                        &mut stack,
                        &Operands {
                            names: &names,
                            strings: &strings,
                            integer: numbers.last_integer,
                            nonzero: numbers.nonzero,
                            strings_complete,
                            property: property.as_ref(),
                            named_property: last_was_name && operands_read >= 2,
                        },
                        &context,
                        reader,
                    );
                    if let Operator::SetSpace { stroking } = &operator
                        && let Some(name) = names.first()
                    {
                        grammar.select_space(*stroking, self.space_components(name, &context));
                    }
                    displaced = displaced.or(self.displacing.take());
                    names.clear();
                    strings.clear();
                    strings_complete = true;
                    numbers = Numbers::default();
                    property = None;
                    operands_read = 0;
                    last_was_name = false;
                }
            }
        }
    }

    /// How many components the space a `CS` or `cs` names has, where the walk can count them:
    /// a family name, or the resources' entry, read without recording anything (the operator's
    /// own arm records what it selected).
    fn space_components(&self, name: &[u8], context: &Context<'_>) -> Option<usize> {
        if let Some(count) = pdf_model::content::grammar::components(
            self.document,
            &pdf_syntax::Object::Name(pdf_syntax::Name::new(name.to_vec())),
        ) {
            return Some(count);
        }
        let entry = self.peek("ColorSpace", name, context)?;
        pdf_model::content::grammar::components(self.document, &entry)
    }

    /// Keeps a string operand where the budget allows, and says whether it did.
    ///
    /// Bounded by the same budget the kept strings are, so a stream of literals between two
    /// operators cannot grow the vector without one.
    fn keep_string(&self, text: Vec<u8>, strings: &mut Vec<Vec<u8>>) -> bool {
        let kept = text.len() <= self.shown_budget;
        if kept {
            strings.push(text);
        }
        kept
    }

    /// Settles the pending show a spacing moved, for the operator about to run (`doc/adr/2746`).
    ///
    /// An operator that sets the text matrix again from the line matrix, or begins or ends a text
    /// object, drops it. A text-showing operator under a defined font places a glyph from the
    /// moved position, which makes the move a mark, so the show is recorded; under an undefined
    /// font both the reader and the empty font draw nothing and the span goes on.
    fn place_from(
        &mut self,
        operator: &Operator,
        state: &State,
        context: &Context<'_>,
        displaced: &mut Option<super::MovingShow>,
    ) {
        match operator {
            Operator::Position
            | Operator::Show {
                starts_line: true, ..
            } => *displaced = None,
            Operator::Show { .. } => {
                if displaced.is_some()
                    && state
                        .font
                        .as_ref()
                        .is_some_and(|font| self.peek("Font", font, context).is_some())
                    && let Some(show) = displaced.take()
                {
                    self.record_moving_show(show);
                }
            }
            _ => {}
        }
    }

    /// One operator, against the state it changes.
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per state-changing operator, each beside the state it changes"
    )]
    fn apply(
        &mut self,
        operator: &Operator,
        state: &mut State,
        stack: &mut Vec<State>,
        operands: &Operands<'_>,
        context: &Context<'_>,
        reader: &mut ContentReader<'_>,
    ) {
        // ISO 32000-2 Annex Q.2 tests the graphics state of each element "to be rendered", so a
        // transparent state marks the page where an operator renders under it, and a state that
        // `Q` restores before anything is painted marks nothing.
        let renders = matches!(
            *operator,
            Operator::Show { .. }
                | Operator::Invoke
                | Operator::Shade
                | Operator::Paint { .. }
                | Operator::InlineImage
        );
        if renders && state.transparency.any() {
            self.survey.transparent.insert(context.page);
        }
        match *operator {
            Operator::Save => {
                if stack.len() < MAX_NESTING {
                    stack.push(state.clone());
                }
                // After the push, so that the first `q` of a stream counts as one level deep.
                self.note_nesting(stack.len(), context.page);
            }
            Operator::Restore => {
                if let Some(previous) = stack.pop() {
                    *state = previous;
                }
            }
            Operator::SetFont => {
                if let Some(name) = operands.first() {
                    state.font = Some(name.clone());
                    state.font_set_by_gs = None;
                    // §9.3.1's `Tf` is where the font resource is selected; what this walk
                    // *judges* is recorded at the text-showing operator ([`Walk::select`]),
                    // and the ledger keeps both so that the interpreter's `Tf` has a twin.
                    if self.ledger.is_some() {
                        let found = self.peek("Font", name, context);
                        self.note_selection("Font", name, found.as_ref());
                    }
                }
            }
            Operator::SetRenderMode => {
                if let Some(mode) = operands.integer {
                    state.mode = mode;
                }
            }
            Operator::SetSpacing { word } => state.set_spacing(word, operands.nonzero),
            Operator::Show { sets_spacing, .. } => {
                if sets_spacing {
                    // `aw ac string "` sets both before it shows; one flag for both operands can
                    // only over-report a spacing, which keeps the font refused.
                    state.set_spacing(true, operands.nonzero);
                    state.set_spacing(false, operands.nonzero);
                }
                self.show(state, operands, context);
            }
            Operator::Invoke => {
                if let Some(name) = operands.first().cloned() {
                    self.invoke(&name, state, context);
                }
            }
            Operator::SetGraphicsState => {
                if let Some(name) = operands.first() {
                    self.graphics_state(name, state, context);
                }
            }
            Operator::SetSpace { stroking } => {
                if let Some(name) = operands.first().cloned() {
                    self.set_space(&name, stroking, state, context);
                }
            }
            Operator::SetColour { stroking } => {
                let selected = if stroking { state.stroke } else { state.fill };
                if selected == Selected::Pattern
                    && let Some(name) = operands.last()
                {
                    self.pattern(name, state, context);
                }
            }
            Operator::SetDeviceColour { family, stroking } => {
                self.set_device_colour(family, stroking, state, context);
            }
            Operator::Shade => {
                if let Some(name) = operands.first() {
                    self.shading_resource(name, state, context);
                }
            }
            Operator::SetRenderingIntent => {
                if let Some(name) = operands.first()
                    && self.first_time(Observation::Intent(context.page, name.clone()))
                {
                    self.survey
                        .rendering_intents
                        .push((context.page, name.clone()));
                }
            }
            Operator::Paint { fill, stroke } => self.paint(fill, stroke, state, context),
            Operator::InlineImage => self.inline_image(reader, context),
            Operator::BeginMarked => self.begin_marked(operands, context),
            Operator::EndMarked => {
                if let Some(top) = self.marks.last_mut()
                    && top.suppressed > 0
                {
                    top.suppressed = top.suppressed.saturating_sub(1);
                } else if self.marks.len() > context.marks_base {
                    self.marks.pop();
                }
            }
            Operator::MarkedPoint => {
                // A point encloses nothing, so its property list is read for §14.9.2's `/Lang`
                // and nothing is pushed.
                if let PropertyList::Read(list) = self.property_list(operands, context) {
                    self.note_language(&list, context.page);
                }
            }
            Operator::Position | Operator::Other => {}
        }
    }
}

/// The numeric operands standing before an operator, as far as the walk reads them.
#[derive(Default)]
struct Numbers {
    /// The last integer, because `Tr` takes exactly one and it comes last.
    last_integer: Option<i64>,
    /// Whether any number was other than zero.
    nonzero: bool,
}

impl Numbers {
    /// An integer operand.
    fn integer(&mut self, value: i64) {
        self.last_integer = Some(value);
        self.nonzero |= value != 0;
    }

    /// A real operand.
    fn real(&mut self, value: f64) {
        self.nonzero |= value != 0.0;
    }

    /// An integer operand too large for `i64`, which is never zero. It is no `Tr` mode either,
    /// so the last integer is left as it was, as it was when the lexer handed it over as a real.
    const fn wide_integer(&mut self) {
        self.nonzero = true;
    }
}
