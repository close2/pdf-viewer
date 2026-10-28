//! The content-stream loop: reading one stream's tokens and acting on each operator.

use pdf_model::content::reader::ContentReader;
use pdf_syntax::Dictionary;

use super::{
    Context, DefaultSpace, DeviceFamily, Extent, MAX_NESTING, Observation, Operands, Operator,
    Origin, PropertyList, Selected, State, Step, Walk, next_step,
};

impl Walk<'_> {
    /// Runs one content stream, in the state it inherits.
    ///
    /// ISO 32000-2 §8.10.1 starts a form `XObject` in the graphics state in force at `Do`, so
    /// the inherited state is a copy and what the form does to it does not return.
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
        // §14.6.2's property list, where the operator being assembled was given one written
        // into its operands rather than named through the resources.
        let mut property: Option<Dictionary> = None;
        // Where this stream found the marked-content stack. A stream that opens more sequences
        // than it closes leaves them here, and one that closes more would otherwise close its
        // caller's; §14.6.1 makes neither a thing a conforming stream does, so the depth is
        // restored on the way out rather than the imbalance reported.
        let base = self.marks.len();
        loop {
            if self.budget == 0 {
                self.survey.stopped_short = true;
                self.marks.truncate(base);
                return;
            }
            self.budget = self.budget.saturating_sub(1);
            let step = next_step(reader);
            let names_it = matches!(&step, Step::Name(name) if name == b"ActualText");
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
                    self.note_integer(value, page);
                    numbers.integer(value);
                }
                Step::Real(value) => {
                    self.note_real(value, page);
                    numbers.real(value);
                }
                Step::Text(text) => {
                    self.note_length(text.len(), page, Extent::String);
                    // Bounded by the same budget the kept strings are, so a stream of literals
                    // between two operators cannot grow this vector without one.
                    if text.len() <= self.shown_budget {
                        strings.push(text);
                    }
                }
                Step::Unlisted(word) => {
                    if let Some(ledger) = &mut self.ledger {
                        ledger.operator(&word);
                    }
                    self.note_unlisted_operator(&word, page, origin);
                    names.clear();
                    strings.clear();
                    numbers = Numbers::default();
                    property = None;
                }
                Step::Operator(operator) => {
                    if let Some(ledger) = &mut self.ledger {
                        ledger.operator(operator.label().as_bytes());
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
                    self.apply(
                        &operator,
                        &mut state,
                        &mut stack,
                        &Operands {
                            names: &names,
                            strings: &strings,
                            integer: numbers.last_integer,
                            nonzero: numbers.nonzero,
                            property: property.as_ref(),
                        },
                        &context,
                        reader,
                    );
                    names.clear();
                    strings.clear();
                    numbers = Numbers::default();
                    property = None;
                }
            }
        }
    }

    /// One operator, against the state it changes.
    fn apply(
        &mut self,
        operator: &Operator,
        state: &mut State,
        stack: &mut Vec<State>,
        operands: &Operands<'_>,
        context: &Context<'_>,
        reader: &mut ContentReader<'_>,
    ) {
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
            Operator::Show {
                starts_line,
                sets_spacing,
            } => {
                if sets_spacing {
                    // `aw ac string "` sets both before it shows; one flag for both operands can
                    // only over-report a spacing, which keeps the font refused.
                    state.set_spacing(true, operands.nonzero);
                    state.set_spacing(false, operands.nonzero);
                }
                self.show(state, operands, starts_line, context);
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
            Operator::Other => {}
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
}
