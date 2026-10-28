//! `FormCalc` evaluated: XFA 3.3 chapter 25's semantics (pages 1060–1077) over a [`Host`] that
//! answers for the form, under a [`Budget`], cited and paraphrased.
//!
//! Every expression of the grammar is evaluated — the operators with their null and promotion
//! rules, variables in their blocks, user-defined functions, `if`, the three loops, `break` and
//! `continue`. What is outside the subset ADR 2482 chose is refused by name with
//! [`Stop::Unsupported`], never approximated: a method call, `.*`, a variable that holds a
//! reference, and every built-in function but the four the census's calculations call —
//! `Sum` (page 1090), `HasValue` (page 1117), `Concat` (page 1122) and `Replace` (page 1132).

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::parse::{self, Accessor, Binary, Expr, For, Index, Separator, Unary};
use crate::value::Value;

/// The form a script reads and writes: the processing application's object model, as far as an
/// accessor reaches it.
pub trait Host {
    /// One object of the form.
    type Node: Clone;

    /// The objects `path` names, relative to the container the script belongs to as chapter 3's
    /// SOM resolves it; `$` is that container. A path that names nothing is an empty list.
    ///
    /// # Errors
    ///
    /// A sentence naming the part of the path this host does not resolve.
    fn resolve(&self, path: &[Segment]) -> Result<Vec<Self::Node>, String>;

    /// The value `node` holds.
    fn value(&self, node: &Self::Node) -> Value;

    /// Sets the value `node` holds.
    ///
    /// # Errors
    ///
    /// A sentence naming why the node takes no value.
    fn assign(&mut self, node: &Self::Node, value: Value) -> Result<(), String>;
}

/// One container of an accessor, its index evaluated.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    /// What joins it to the segment before.
    pub separator: Separator,
    /// Its name as written.
    pub name: String,
    /// Its occurrence.
    pub occurrence: Occurrence,
}

/// The occurrence a segment names (page 1067).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Occurrence {
    /// No index: the occurrence that bears the referencing object's own number.
    Same,
    /// `[*]`.
    Every,
    /// `[n]`, counted from 0.
    At(i64),
    /// `[+n]` or `[-n]`, relative to the referencing object's occurrence.
    Relative(i64),
}

/// Every ceiling one evaluation is held to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// How many expressions may be evaluated, loop iterations included: page 380 recommends that
    /// a processor have some means of ending a seemingly infinite loop, and this is it.
    pub steps: u64,
    /// How deep user-defined functions may call one another.
    pub calls: u32,
    /// The longest string, in bytes, any expression may produce.
    pub string_bytes: usize,
}

impl Budget {
    /// The budget of one `calculate` script: a hundred thousand steps is a thousand times the
    /// largest calculation of the census (ADR 2482), and a mebibyte of text is far past any value
    /// a field shows.
    pub const CALCULATION: Self = Self {
        steps: 100_000,
        calls: 16,
        string_bytes: 1 << 20,
    };
}

/// Why an evaluation stopped without a value.
#[derive(Debug, Clone, PartialEq)]
pub enum Stop {
    /// The script is not a `FormCalculation`.
    Parse(parse::ParseError),
    /// An error exception the language raises, which terminates the script (page 1062).
    Exception(String),
    /// A construct of the language this evaluator does not carry out, by name.
    Unsupported(String),
    /// A ceiling of the [`Budget`], by name.
    Exceeded(&'static str),
}

impl fmt::Display for Stop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "the script does not parse: {error}"),
            Self::Exception(why) => write!(f, "the script raised an error: {why}"),
            Self::Unsupported(what) => write!(f, "{what} is not evaluated by this program"),
            Self::Exceeded(what) => write!(f, "the script exceeded its {what} budget"),
        }
    }
}

impl std::error::Error for Stop {}

/// Every built-in function chapter 25 documents, with the page it is on, so that one outside the
/// subset is refused by its name and its page.
const BUILT_INS: [(&str, u32); 67] = [
    ("abs", 1081),
    ("avg", 1082),
    ("ceil", 1083),
    ("count", 1084),
    ("floor", 1085),
    ("max", 1086),
    ("min", 1087),
    ("mod", 1088),
    ("round", 1089),
    ("sum", 1090),
    ("date", 1091),
    ("date2num", 1092),
    ("datefmt", 1093),
    ("isodate2num", 1094),
    ("isotime2num", 1095),
    ("localdatefmt", 1096),
    ("localtimefmt", 1097),
    ("num2date", 1098),
    ("num2gmtime", 1099),
    ("num2time", 1100),
    ("time", 1101),
    ("time2num", 1102),
    ("timefmt", 1104),
    ("apr", 1105),
    ("cterm", 1106),
    ("fv", 1107),
    ("ipmt", 1108),
    ("npv", 1109),
    ("pmt", 1110),
    ("ppmt", 1111),
    ("pv", 1112),
    ("rate", 1113),
    ("term", 1114),
    ("choose", 1115),
    ("exists", 1116),
    ("hasvalue", 1117),
    ("oneof", 1118),
    ("within", 1119),
    ("at", 1120),
    ("concat", 1122),
    ("decode", 1123),
    ("encode", 1124),
    ("format", 1125),
    ("left", 1127),
    ("len", 1128),
    ("lower", 1129),
    ("ltrim", 1130),
    ("parse", 1131),
    ("replace", 1132),
    ("right", 1133),
    ("rtrim", 1134),
    ("space", 1135),
    ("str", 1136),
    ("stuff", 1137),
    ("substr", 1138),
    ("uuid", 1139),
    ("upper", 1140),
    ("wordnum", 1141),
    ("get", 1143),
    ("post", 1144),
    ("put", 1146),
    ("ref", 1147),
    ("unitvalue", 1148),
    ("unittype", 1149),
    ("eval", 1146),
    ("null", 1147),
    ("messagebox", 1076),
];

/// Page 1076's note names thirteen built-ins the chapter does not document.
const UNDOCUMENTED: [&str; 13] = [
    "acos", "asin", "atan", "cos", "deg2rad", "exp", "log", "pi", "pow", "rad2deg", "sin", "sqrt",
    "tan",
];

/// Parses and evaluates `source` against `host`, and returns the value of its expression list:
/// the value page 379 assigns to a `calculate` script's container.
///
/// # Errors
///
/// [`Stop`], whose variants are the reasons; the host saw every assignment made before it.
pub fn run<H: Host>(source: &str, host: &mut H, budget: Budget) -> Result<Value, Stop> {
    run_counted(source, host, budget).0
}

/// [`run`], and how many steps the evaluation took, whether it ended in a value or a stop.
///
/// A host that runs several scripts under one ceiling charges each against what is left of it:
/// XFA 3.3 page 380 recommends a means of ending a seemingly infinite loop, and a cascade of
/// calculations (page 379) is one such loop that no single script's [`Budget`] sees (ADR 2506).
/// A script that does not parse took no steps.
pub fn run_counted<H: Host>(
    source: &str,
    host: &mut H,
    budget: Budget,
) -> (Result<Value, Stop>, u64) {
    let script = match parse::parse(source) {
        Ok(script) => script,
        Err(error) => return (Err(Stop::Parse(error)), 0),
    };
    let mut evaluator = Evaluator {
        host,
        budget,
        steps: 0,
        calls: 0,
        scopes: vec![Scope::default()],
    };
    let result = match evaluator.list(&script) {
        Ok(value) => Ok(value),
        Err(Signal::Stop(stop)) => Err(stop),
        Err(Signal::Break | Signal::Continue) => Err(Stop::Exception(
            "break or continue outside a loop".to_owned(),
        )),
    };
    (result, evaluator.steps)
}

/// What unwinds an evaluation: the end of a loop iteration, or a stop.
enum Signal {
    Break,
    Continue,
    Stop(Stop),
}

impl From<Stop> for Signal {
    fn from(stop: Stop) -> Self {
        Self::Stop(stop)
    }
}

type Flow<T> = Result<T, Signal>;

fn exception<T>(why: impl Into<String>) -> Flow<T> {
    Err(Signal::Stop(Stop::Exception(why.into())))
}

fn unsupported<T>(what: impl Into<String>) -> Flow<T> {
    Err(Signal::Stop(Stop::Unsupported(what.into())))
}

/// A user-defined function: its parameters, its body, and how many scopes were open where it
/// was declared, which are the ones its body sees.
struct Function {
    parameters: Vec<String>,
    body: Vec<Expr>,
    depth: usize,
}

/// One block's variables and functions (page 1074).
#[derive(Default)]
struct Scope {
    variables: HashMap<String, Value>,
    /// By name in lower case: function names are case insensitive (page 1076).
    functions: HashMap<String, Rc<Function>>,
}

/// An argument as a function receives it: a value, or the set an `[*]` accessor names.
enum Operand {
    One(Value),
    Set(Vec<Value>),
}

struct Evaluator<'h, H: Host> {
    host: &'h mut H,
    budget: Budget,
    steps: u64,
    calls: u32,
    scopes: Vec<Scope>,
}

impl<H: Host> Evaluator<'_, H> {
    fn step(&mut self) -> Flow<()> {
        self.steps = self.steps.saturating_add(1);
        if self.steps > self.budget.steps {
            return Err(Signal::Stop(Stop::Exceeded("step")));
        }
        Ok(())
    }

    fn checked(&self, text: String) -> Flow<Value> {
        if text.len() > self.budget.string_bytes {
            return Err(Signal::Stop(Stop::Exceeded("string length")));
        }
        Ok(Value::String(text))
    }

    /// Page 1060: the value of an expression list is the value of its last expression. An empty
    /// list is taken as null, the value of no expression.
    fn list(&mut self, body: &[Expr]) -> Flow<Value> {
        let mut last = Value::Null;
        for each in body {
            last = self.expression(each)?;
        }
        Ok(last)
    }

    /// A list in a block of its own (page 1074's table).
    fn block(&mut self, list: &[Expr]) -> Flow<Value> {
        self.scopes.push(Scope::default());
        let value = self.list(list);
        self.scopes.pop();
        value
    }

    fn variable(&self, name: &str) -> Option<&Value> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.variables.get(name))
    }

    fn variable_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.scopes
            .iter_mut()
            .rev()
            .find_map(|scope| scope.variables.get_mut(name))
    }

    fn declare(&mut self, name: &str, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.variables.insert(name.to_owned(), value);
        }
    }

    fn expression(&mut self, expression: &Expr) -> Flow<Value> {
        self.step()?;
        match expression {
            Expr::Assign(target, value) => {
                let value = self.value(value)?;
                self.assign(target, value.clone())?;
                Ok(value)
            }
            // Page 1065: the variable exists, holding the empty string, before its value is
            // evaluated, and the declaration's value is the value assigned.
            Expr::Var(name, init) => {
                self.declare(name, Value::String(String::new()));
                let value = match init {
                    Some(init) => self.value(init)?,
                    None => Value::String(String::new()),
                };
                self.declare(name, value.clone());
                Ok(value)
            }
            Expr::Func(name, parameters, body) => {
                let depth = self.scopes.len();
                if let Some(scope) = self.scopes.last_mut() {
                    scope.functions.insert(
                        name.to_lowercase(),
                        Rc::new(Function {
                            parameters: parameters.clone(),
                            body: body.clone(),
                            depth,
                        }),
                    );
                }
                Ok(Value::Null)
            }
            Expr::If(arms, otherwise) => self.if_expression(arms, otherwise.as_deref()),
            Expr::While(condition, body) => self.while_expression(condition, body),
            Expr::For(each) => self.for_expression(each),
            Expr::Foreach(variable, arguments, body) => {
                self.foreach_expression(variable, arguments, body)
            }
            Expr::Block(body) => self.block(body),
            Expr::Break => Err(Signal::Break),
            Expr::Continue => Err(Signal::Continue),
            simple => self.value(simple),
        }
    }

    /// Page 1072: the first condition that promotes to true has its list evaluated; with none,
    /// the `else` list, and with no `else`, 0.
    fn if_expression(
        &mut self,
        arms: &[(Expr, Vec<Expr>)],
        otherwise: Option<&[Expr]>,
    ) -> Flow<Value> {
        for (condition, list) in arms {
            if self.value(condition)?.boolean() {
                return self.block(list);
            }
        }
        match otherwise {
            Some(list) => self.block(list),
            None => Ok(Value::Number(0.0)),
        }
    }

    /// Page 1073: the body's value, while the condition promotes to true. A loop never entered
    /// is taken as 0, the value page 1073 gives the `foreach` that never enters.
    fn while_expression(&mut self, condition: &Expr, body: &[Expr]) -> Flow<Value> {
        let mut last = Value::Number(0.0);
        while self.value(condition)?.boolean() {
            self.step()?;
            match self.block(body) {
                Ok(value) => last = value,
                Err(Signal::Break) => break,
                Err(Signal::Continue) => {}
                Err(stop) => return Err(stop),
            }
        }
        Ok(last)
    }

    /// Page 1072. The page says a `downto` loop's step is subtracted from the variable and that
    /// its default is -1; the step is read as a magnitude subtracted, the reading under which the
    /// default loop ends, so `downto` with no step counts down by one.
    fn for_expression(&mut self, each: &For) -> Flow<Value> {
        self.scopes.push(Scope::default());
        let result = self.for_body(each);
        self.scopes.pop();
        result
    }

    fn for_body(&mut self, each: &For) -> Flow<Value> {
        let start = self.value(&each.start)?.number();
        if each.declares || self.variable(&each.variable).is_none() {
            self.declare(&each.variable, Value::Number(start));
        } else if let Some(slot) = self.variable_mut(&each.variable) {
            *slot = Value::Number(start);
        }
        let mut last = Value::Number(0.0);
        loop {
            let current = self.variable(&each.variable).map_or(0.0, Value::number);
            let end = self.value(&each.end)?.number();
            if (each.down && current < end) || (!each.down && current > end) {
                break;
            }
            self.step()?;
            match self.block(&each.body) {
                Ok(value) => last = value,
                Err(Signal::Break) => break,
                Err(Signal::Continue) => {}
                Err(stop) => return Err(stop),
            }
            let step = match &each.step {
                Some(step) => self.value(step)?.number(),
                None => 1.0,
            };
            // The body may have assigned the variable; the step is taken from what it holds after the body ran.
            let current = self.variable(&each.variable).map_or(0.0, Value::number);
            let next = if each.down {
                current - step
            } else {
                current + step
            };
            if let Some(slot) = self.variable_mut(&each.variable) {
                *slot = Value::Number(next);
            }
        }
        Ok(last)
    }

    /// Page 1073: the argument list is evaluated once, and the body runs with the variable
    /// holding each of its values in turn.
    fn foreach_expression(
        &mut self,
        variable: &str,
        arguments: &[Expr],
        body: &[Expr],
    ) -> Flow<Value> {
        let mut values = Vec::new();
        for argument in arguments {
            match self.operand(argument)? {
                Operand::One(value) => values.push(value),
                Operand::Set(set) => values.extend(set),
            }
        }
        self.scopes.push(Scope::default());
        let mut last = Value::Number(0.0);
        for value in values {
            if let Err(signal) = self.step() {
                self.scopes.pop();
                return Err(signal);
            }
            self.declare(variable, value);
            match self.block(body) {
                Ok(value) => last = value,
                Err(Signal::Break) => break,
                Err(Signal::Continue) => {}
                Err(stop) => {
                    self.scopes.pop();
                    return Err(stop);
                }
            }
        }
        self.scopes.pop();
        Ok(last)
    }

    /// A simple expression's value, a set refused (page 1068: an `[*]` accessor is an operand
    /// only of the functions that take a set).
    fn value(&mut self, expression: &Expr) -> Flow<Value> {
        match self.operand(expression)? {
            Operand::One(value) => Ok(value),
            Operand::Set(_) => exception("an accessor naming every occurrence is no single value"),
        }
    }

    fn operand(&mut self, expression: &Expr) -> Flow<Operand> {
        self.step()?;
        let value = match expression {
            Expr::Number(n) => Value::Number(*n),
            Expr::String(text) => Value::String(text.clone()),
            Expr::Null => Value::Null,
            Expr::NotANumber(at) => {
                return exception(format!(
                    "the special number at byte {at} is not supported (page 1057)"
                ));
            }
            Expr::Accessor(accessor) => return self.read(accessor),
            Expr::Call(name, arguments, _) => self.call(name, arguments)?,
            Expr::Unary(operator, operand) => {
                let operand = self.value(operand)?;
                unary(*operator, &operand)?
            }
            Expr::Binary(operator, left, right) => {
                let left = self.value(left)?;
                let right = self.value(right)?;
                binary(*operator, &left, &right)?
            }
            other => self.expression(other)?,
        };
        Ok(Operand::One(value))
    }

    /// The single plain name an accessor is, if it is one: the only accessor a variable can be.
    fn plain_name(accessor: &Accessor) -> Option<&str> {
        match accessor.steps.as_slice() {
            [step] if step.index == Index::Same && step.call.is_none() && !accessor.every_child => {
                Some(&step.name)
            }
            _ => None,
        }
    }

    /// The segments of `accessor` with its indexes evaluated, or the construct refused.
    fn path(&mut self, accessor: &Accessor) -> Flow<Vec<parse::Step>> {
        if accessor.every_child {
            return unsupported("the `.*` accessor of every child (page 1067)");
        }
        if let Some(step) = accessor.steps.iter().find(|step| step.call.is_some()) {
            return unsupported(format!("the method call {}() (page 1076)", step.name));
        }
        if let Some(first) = accessor.steps.first()
            && accessor.steps.len() > 1
            && self.variable(&first.name).is_some()
        {
            return unsupported(format!(
                "an accessor through the variable {}, a reference (page 1068)",
                first.name
            ));
        }
        Ok(accessor.steps.clone())
    }

    fn segments(&mut self, steps: &[parse::Step]) -> Flow<Vec<Segment>> {
        let mut segments = Vec::with_capacity(steps.len());
        for step in steps {
            let occurrence = match &step.index {
                Index::Same => Occurrence::Same,
                Index::Every => Occurrence::Every,
                Index::At(at) => Occurrence::At(integral(self.value(at)?.number())),
                Index::Relative(back, by) => {
                    let by = integral(self.value(by)?.number());
                    Occurrence::Relative(if *back { by.saturating_neg() } else { by })
                }
            };
            segments.push(Segment {
                separator: step.separator,
                name: step.name.clone(),
                occurrence,
            });
        }
        Ok(segments)
    }

    fn nodes(&mut self, accessor: &Accessor) -> Flow<Vec<H::Node>> {
        let steps = self.path(accessor)?;
        let segments = self.segments(&steps)?;
        self.host.resolve(&segments).or_else(unsupported)
    }

    fn read(&mut self, accessor: &Accessor) -> Flow<Operand> {
        if let Some(name) = Self::plain_name(accessor)
            && let Some(value) = self.variable(name)
        {
            return Ok(Operand::One(value.clone()));
        }
        let every = accessor.steps.iter().any(|step| step.index == Index::Every);
        let nodes = self.nodes(accessor)?;
        if every {
            return Ok(Operand::Set(
                nodes.iter().map(|node| self.host.value(node)).collect(),
            ));
        }
        match nodes.first() {
            Some(node) => Ok(Operand::One(self.host.value(node))),
            None => exception(format!("accessor {} is unknown", spelled(accessor))),
        }
    }

    /// Page 1066: an assignment sets a variable where the accessor names one in scope, else the
    /// object the accessor names.
    fn assign(&mut self, target: &Accessor, value: Value) -> Flow<()> {
        if let Some(name) = Self::plain_name(target)
            && let Some(slot) = self.variable_mut(name)
        {
            *slot = value;
            return Ok(());
        }
        let nodes = self.nodes(target)?;
        let Some(node) = nodes.first() else {
            return exception(format!("accessor {} is unknown", spelled(target)));
        };
        if nodes.len() > 1 && target.steps.iter().any(|step| step.index == Index::Every) {
            return exception("an accessor naming every occurrence takes no single value");
        }
        self.host.assign(node, value).or_else(unsupported)
    }

    /// Page 1076: a built-in takes precedence over a user-defined function of the same name.
    fn call(&mut self, name: &str, arguments: &[Expr]) -> Flow<Value> {
        let lower = name.to_lowercase();
        match lower.as_str() {
            "sum" => return self.sum(arguments),
            "concat" => return self.concat(arguments),
            "replace" => return self.replace(arguments),
            "hasvalue" => return self.has_value(arguments),
            _ => {}
        }
        if let Some(&(_, page)) = BUILT_INS.iter().find(|(built_in, _)| *built_in == lower) {
            return unsupported(format!("the built-in function {name}() (page {page})"));
        }
        if UNDOCUMENTED.contains(&lower.as_str()) {
            return unsupported(format!(
                "the undocumented built-in function {name}() (page 1076)"
            ));
        }
        let function = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.functions.get(&lower))
            .cloned();
        match function {
            Some(function) => self.call_user(name, &function, arguments),
            None => exception(format!("function {name} is unknown")),
        }
    }

    /// Page 1066: the arguments are assigned to the parameters and the body's last value is
    /// returned. The body sees the scopes open where the function was declared, its parameters
    /// and what it declares itself — and not the caller's blocks, which a function declared
    /// outside them cannot name.
    fn call_user(&mut self, name: &str, function: &Function, arguments: &[Expr]) -> Flow<Value> {
        if arguments.len() != function.parameters.len() {
            return exception(format!(
                "function {name} takes {} argument(s) and was given {}",
                function.parameters.len(),
                arguments.len()
            ));
        }
        let mut values = Vec::with_capacity(arguments.len());
        for argument in arguments {
            values.push(self.value(argument)?);
        }
        if self.calls >= self.budget.calls {
            return Err(Signal::Stop(Stop::Exceeded("function call depth")));
        }
        self.calls = self.calls.saturating_add(1);
        let hidden = self.scopes.split_off(function.depth.min(self.scopes.len()));
        let mut frame = Scope::default();
        for (parameter, value) in function.parameters.iter().zip(values) {
            frame.variables.insert(parameter.clone(), value);
        }
        self.scopes.push(frame);
        let result = self.list(&function.body);
        self.scopes.truncate(function.depth.min(self.scopes.len()));
        self.scopes.extend(hidden);
        self.calls = self.calls.saturating_sub(1);
        match result {
            Err(Signal::Break | Signal::Continue) => exception("break or continue outside a loop"),
            other => other,
        }
    }

    fn flattened(&mut self, arguments: &[Expr]) -> Flow<Vec<Value>> {
        let mut values = Vec::new();
        for argument in arguments {
            match self.operand(argument)? {
                Operand::One(value) => values.push(value),
                Operand::Set(set) => values.extend(set),
            }
        }
        Ok(values)
    }

    /// Page 1090: the sum of the non-null arguments, an `[*]` accessor's occurrences among them,
    /// or null where every one is null.
    fn sum(&mut self, arguments: &[Expr]) -> Flow<Value> {
        if arguments.is_empty() {
            return exception("Sum() needs at least one argument (page 1076)");
        }
        let values = self.flattened(arguments)?;
        let present: Vec<f64> = values
            .iter()
            .filter(|v| !v.is_null())
            .map(Value::number)
            .collect();
        if present.is_empty() {
            return Ok(Value::Null);
        }
        finite(present.iter().sum())
    }

    /// Page 1122: the arguments promoted to strings and joined, or null where every one is null.
    fn concat(&mut self, arguments: &[Expr]) -> Flow<Value> {
        if arguments.is_empty() {
            return exception("Concat() needs at least one argument (page 1076)");
        }
        let values = self.flattened(arguments)?;
        if values.iter().all(Value::is_null) {
            return Ok(Value::Null);
        }
        let mut text = String::new();
        for value in &values {
            text.push_str(&value.text());
            if text.len() > self.budget.string_bytes {
                return Err(Signal::Stop(Stop::Exceeded("string length")));
            }
        }
        self.checked(text)
    }

    /// Page 1132: every occurrence of the second string in the first replaced by the third, the
    /// empty string where the third is omitted or null; null where either of the first two is.
    /// An empty second string occurs nowhere a replacement could be made, and the first is
    /// returned as it is — a reading, as the page does not say.
    fn replace(&mut self, arguments: &[Expr]) -> Flow<Value> {
        if !(2..=3).contains(&arguments.len()) {
            return exception(format!(
                "Replace() takes two or three arguments and was given {}",
                arguments.len()
            ));
        }
        let mut values = Vec::with_capacity(3);
        for argument in arguments {
            values.push(self.value(argument)?);
        }
        let (Some(source), Some(pattern)) = (values.first(), values.get(1)) else {
            return exception("Replace() needs two arguments");
        };
        if source.is_null() || pattern.is_null() {
            return Ok(Value::Null);
        }
        let (source, pattern) = (source.text(), pattern.text());
        if pattern.is_empty() {
            return self.checked(source);
        }
        let replacement = values.get(2).map(Value::text).unwrap_or_default();
        let count = source.matches(pattern.as_str()).count();
        let grown = source
            .len()
            .saturating_sub(count.saturating_mul(pattern.len()))
            .saturating_add(count.saturating_mul(replacement.len()));
        if grown > self.budget.string_bytes {
            return Err(Signal::Stop(Stop::Exceeded("string length")));
        }
        self.checked(source.replace(pattern.as_str(), &replacement))
    }

    /// Page 1117: 1 where the argument has a value that is neither null, empty, nor only white
    /// space; an accessor that names no object has none.
    fn has_value(&mut self, arguments: &[Expr]) -> Flow<Value> {
        let [argument] = arguments else {
            return exception(format!(
                "HasValue() takes one argument and was given {}",
                arguments.len()
            ));
        };
        let value = match argument {
            Expr::Accessor(accessor)
                if Self::plain_name(accessor).is_none_or(|name| self.variable(name).is_none()) =>
            {
                let nodes = self.nodes(accessor)?;
                match nodes.first() {
                    Some(node) => self.host.value(node),
                    None => Value::Null,
                }
            }
            other => self.value(other)?,
        };
        let present = !value.is_null() && !value.text().trim().is_empty();
        Ok(Value::Number(f64::from(u8::from(present))))
    }
}

/// An index promoted to an integer by truncation (page 1077); one past `i64` saturates.
#[expect(
    clippy::cast_possible_truncation,
    reason = "page 1077 truncates an integral argument's fraction, and `as` saturates out of range"
)]
fn integral(n: f64) -> i64 {
    n.trunc() as i64
}

/// Page 1062: an intermediate NaN or infinity raises an error exception.
fn finite(n: f64) -> Flow<Value> {
    if n.is_finite() {
        Ok(Value::Number(n))
    } else {
        exception("an intermediate result is not a finite number (page 1062)")
    }
}

fn boolean(b: bool) -> Value {
    Value::Number(f64::from(u8::from(b)))
}

/// Page 1064's unary operators: `-` and `+` keep null, `not` promotes it to false first.
fn unary(operator: Unary, operand: &Value) -> Flow<Value> {
    match operator {
        Unary::Not => Ok(boolean(!operand.boolean())),
        _ if operand.is_null() => Ok(Value::Null),
        Unary::Minus => finite(-operand.number()),
        Unary::Plus => finite(operand.number()),
    }
}

/// Pages 1062–1064's binary operators.
fn binary(operator: Binary, left: &Value, right: &Value) -> Flow<Value> {
    let both_null = left.is_null() && right.is_null();
    match operator {
        Binary::Equal | Binary::NotEqual => {
            let same = compare(left, right).is_some_and(std::cmp::Ordering::is_eq);
            Ok(boolean(same == (operator == Binary::Equal)))
        }
        Binary::Less | Binary::LessEqual | Binary::Greater | Binary::GreaterEqual => {
            // Page 1063: a null operand compares identically to another null under `<=` and
            // `>=` only, and differently otherwise; differently is read as false whatever the
            // operator, since no ordering between a null and a value is stated.
            if left.is_null() || right.is_null() {
                return Ok(boolean(
                    both_null && matches!(operator, Binary::LessEqual | Binary::GreaterEqual),
                ));
            }
            let Some(order) = compare(left, right) else {
                return Ok(boolean(false));
            };
            Ok(boolean(match operator {
                Binary::Less => order.is_lt(),
                Binary::LessEqual => order.is_le(),
                Binary::Greater => order.is_gt(),
                _ => order.is_ge(),
            }))
        }
        _ if both_null => Ok(Value::Null),
        Binary::Or => Ok(boolean(left.number() != 0.0 || right.number() != 0.0)),
        Binary::And => Ok(boolean(left.number() != 0.0 && right.number() != 0.0)),
        Binary::Add => finite(left.number() + right.number()),
        Binary::Subtract => finite(left.number() - right.number()),
        Binary::Multiply => finite(left.number() * right.number()),
        Binary::Divide => finite(left.number() / right.number()),
    }
}

/// Page 1063's comparison: nulls equal each other and nothing else; two strings compare as
/// strings; anything else compares as numbers.
///
/// **Strings compare by code point, and that departs from the page**, which asks for a
/// locale-sensitive lexicographic comparison: this program holds no collation. Equality is the
/// same under both, and no calculation of the census compares two strings (ADR 2482).
fn compare(left: &Value, right: &Value) -> Option<std::cmp::Ordering> {
    match (left, right) {
        (Value::Null, Value::Null) => Some(std::cmp::Ordering::Equal),
        (Value::Null, _) | (_, Value::Null) => None,
        (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
        _ => left.number().partial_cmp(&right.number()),
    }
}

fn spelled(accessor: &Accessor) -> String {
    let mut text = String::new();
    for step in &accessor.steps {
        text.push_str(match step.separator {
            Separator::First => "",
            Separator::Dot => ".",
            Separator::DotDot => "..",
            Separator::DotHash => ".#",
        });
        text.push_str(&step.name);
    }
    text
}
