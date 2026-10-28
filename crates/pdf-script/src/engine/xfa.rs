//! An XFA form's JavaScript: one script of a container's, run against the containers a host
//! answers for (XFA 3.3 pages 106–107 and 412–415; ADR 2545).
//!
//! **The form never enters the realm.** What a script names is a question to a [`FormHost`] — the
//! containers a path names, a container's value, an assignment — as it is for `FormCalc` (ADR
//! 2524), so the realm holds a number for each container it was told of and nothing else of the
//! form.
//!
//! **The object model is the subset a calculation or a value event reaches**, and every other
//! member is refused by name rather than read as `undefined` (trap 5):
//!
//! - `this` is the script's container (page 415), and a *naked reference* (page 413) — a name the
//!   script does not declare — is the container SOM's search finds from it (page 114). A name the
//!   engine itself defines (`Math`, `Number`, `parseInt` …) is the engine's, which is this module's
//!   choice where the page states no precedence; a name of the script's own is its own unless a
//!   container answers it, and then assigning to it is refused, since a container is set through
//!   its `rawValue` and never by its name (page 106).
//! - A container's `rawValue` (page 106) is read and assigned; `isNull` is read; a name that is a
//!   child of it is that child (page 90's basic references); `resolveNode` and `resolveNodes`
//!   (page 107) take a SOM expression in a string. Anything else on a container — `value`, which is
//!   a node and not its value (page 107's note), `formattedValue`, `presence` — is refused by name.
//! - `xfa` and its synonym `$xfa` (page 414) offer `resolveNode` and `resolveNodes` and refuse
//!   every other member — the DOMs, `xfa.host`, `xfa.event` — by name. A qualified reference other
//!   than `$` (`$form`, `$data`) is the host's to refuse, which says so in its own sentence.
//!
//! **The value of a calculation** (page 379 evaluates the script as a function and assigns its
//! return value) is the script's completion value, which for a script
//! that ends in an expression is that expression's, as `FormCalc`'s is (page 1050); a script whose
//! top level returns is evaluated as a function body and its return value taken. `undefined` and
//! `null` are the null value, a boolean is 1 or 0, and an object is refused.
//!
//! **The budget is the document realm's** ([`Budget`]): the engine's cost units sliced against the
//! steps and the wall clock, its loop, recursion and stack limits, and the guards on what an
//! argument can make large (ADR 1590), so a runaway script is stopped in the worker; past what a
//! script was handed the run reports itself over, which ends the activation it belongs to (page
//! 380). Global variables persist from one run to the next, which page 414 asks for.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::pin;
use std::rc::Rc;
use std::task::{Context as TaskContext, Poll, Waker};
use std::time::{Duration, Instant};

use boa_engine::module::IdleModuleLoader;
use boa_engine::object::builtins::JsProxy;
use boa_engine::optimizer::OptimizerOptions;
use boa_engine::property::{Attribute, PropertyKey};
use boa_engine::{
    Context, JsError, JsNativeError, JsObject, JsResult, JsString, JsSymbol, JsValue,
    NativeFunction, Script, Source, js_string,
};
use pdf_model::xfa::{Occurrence, Scalar, Step};

use super::{Hooks, SLICE, State, ending_of, too_deep};
use crate::{Budget, Ending, Nobody};

/// The most steps a SOM expression in a string may have, as a `FormCalc` path may (ADR 2524).
pub const MAX_PATH_STEPS: usize = 64;
/// The most bytes one name of such an expression may have.
pub const MAX_NAME_BYTES: usize = 4096;
/// The most containers one run may hold an object for, its own included: each is a proxy and its
/// target in the engine's heap, about a kilobyte in all, and the worker's address space is held to
/// tens of megabytes (ADR 1608). A list does not count until an item of it is taken.
pub const MAX_NODES_HELD: usize = 4096;
/// The most container numbers the lists of one run may hold between them.
pub const MAX_LISTED: usize = 1 << 20;

/// The global name the scope of naked references is held under. It is the engine's own name, so
/// it is never asked about.
const SCOPE: &str = "__xfa_naked_references__";

/// The containers of a form, as a host answers for them.
pub trait FormHost {
    /// The containers `path` names: from `from`, a container this host named, where it is given,
    /// and else from the script's own container by page 114's search.
    ///
    /// # Errors
    ///
    /// The sentence naming the part of SOM the host does not follow.
    fn resolve(&self, from: Option<u32>, path: &[Step]) -> Result<Vec<u32>, String>;

    /// The value the container holds.
    ///
    /// # Errors
    ///
    /// The sentence saying why the host will not tell it.
    fn value(&self, node: u32) -> Result<Scalar, String>;

    /// Sets the container's value.
    ///
    /// # Errors
    ///
    /// The sentence saying why the container takes no value.
    fn assign(&self, node: u32, value: Scalar) -> Result<(), String>;
}

/// What one run of a form's script came to.
#[derive(Debug, Clone, PartialEq)]
pub struct FormRan {
    /// The engine's cost units the run spent, or one past what it was handed where a budget ended
    /// it.
    pub steps: u64,
    /// The script's value, or the sentence saying why it has none.
    pub outcome: Result<Scalar, String>,
}

/// A realm for a form's JavaScript, kept for the form's life so that its globals persist (page
/// 414). Not `Send`, for [`super::Realm`]'s reason.
#[derive(Debug)]
pub struct FormRealm {
    /// The context every run evaluates in.
    context: Context,
    /// The object the global code's `this` is; its prototype is each run's container.
    this: JsObject,
    /// The budget every run is held to.
    budget: Budget,
    /// The key a container object's number is read under, which no script is handed.
    key: JsSymbol,
    /// How long a run waited on a person, which is always nothing here: a form's script asks no
    /// question.
    waited: Rc<Cell<Duration>>,
}

/// What a run keeps in its context's host data: the host, and what it was told.
struct Run {
    host: Rc<dyn FormHost>,
    /// What each naked reference named, asked once a run.
    names: RefCell<BTreeMap<String, Option<u32>>>,
    /// The object handed out for each container.
    nodes: RefCell<BTreeMap<u32, JsObject>>,
    /// The containers each list holds.
    lists: RefCell<Vec<Vec<u32>>>,
    /// How many container numbers the lists hold between them.
    listed: Cell<usize>,
    /// The first sentence the object model refused with, which the run's outcome carries even
    /// where the script caught it: what the script computed rests on a reading it was refused.
    refused: RefCell<Option<String>>,
    /// The key a container object's number is read under; no script holds it.
    key: JsSymbol,
}

impl FormRealm {
    /// A realm with the form's object model installed.
    ///
    /// # Errors
    ///
    /// The sentence saying what could not be installed, where the engine refuses a property on a
    /// context just constructed — which it does not.
    pub fn new(budget: Budget) -> Result<Self, String> {
        let slot = Rc::new(RefCell::new(None));
        let hooks = Hooks {
            utc_offset_seconds: Rc::new(Cell::new(0)),
            buffer_bytes: budget.buffer_bytes,
            global_this: Some(Rc::clone(&slot)),
        };
        // Trap 118: the loader that reads no file system, as the document's realm has.
        let mut context = Context::builder()
            .host_hooks(Rc::new(hooks))
            .module_loader(Rc::new(IdleModuleLoader))
            .build()
            .map_err(|error| format!("the engine could not be constructed: {error}"))?;
        let limits = context.runtime_limits_mut();
        limits.set_loop_iteration_limit(budget.loop_iterations);
        limits.set_recursion_limit(usize::try_from(budget.recursion).unwrap_or(usize::MAX));
        limits.set_stack_size_limit(usize::try_from(budget.stack).unwrap_or(usize::MAX));
        context.set_optimizer_options(OptimizerOptions::empty());
        let waited = Rc::new(Cell::new(Duration::ZERO));
        context.insert_data(State::new(budget, Rc::new(Nobody), Rc::clone(&waited)));
        let installed = install(&mut context);
        installed
            .map_err(|error| format!("the form's object model could not be installed: {error}"))?;
        let this = slot
            .borrow_mut()
            .take()
            .ok_or_else(|| "the engine made no object for the global code's this".to_owned())?;
        let key = JsSymbol::new(Some(js_string!("container")))
            .ok_or_else(|| "the engine has no symbol left to make".to_owned())?;
        let builtins: BTreeSet<String> = context
            .global_object()
            .own_property_keys(&mut context)
            .map_err(|error| format!("the engine's own names could not be listed: {error}"))?
            .into_iter()
            .filter_map(|key| match key {
                PropertyKey::String(name) => Some(name.to_std_string_escaped()),
                PropertyKey::Index(index) => Some(index.get().to_string()),
                PropertyKey::Symbol(_) => None,
            })
            .collect();
        context.insert_data(Builtins(builtins));
        Ok(Self {
            context,
            this,
            budget,
            key,
            waited,
        })
    }

    /// Runs `script` as the script of the container `here`, asking `host` what it names, within
    /// `steps_left` of the engine's cost units and the realm's budget.
    pub fn run(
        &mut self,
        script: &str,
        here: u32,
        host: Rc<dyn FormHost>,
        steps_left: u64,
    ) -> FormRan {
        let over = steps_left.saturating_add(1);
        let refused = |steps: u64, sentence: String| FormRan {
            steps,
            outcome: Err(sentence),
        };
        if let Some(exceeded) = too_deep(script, &self.budget) {
            return refused(0, format!("the script is not run: {}", exceeded.sentence()));
        }
        self.context.insert_data(Run {
            host,
            names: RefCell::new(BTreeMap::new()),
            nodes: RefCell::new(BTreeMap::new()),
            lists: RefCell::new(Vec::new()),
            listed: Cell::new(0),
            refused: RefCell::new(None),
            key: self.key.clone(),
        });
        let ran = self.evaluate(script, here, steps_left);
        let recorded = State::take(&self.context);
        let first_refusal = self
            .context
            .get_data::<Run>()
            .and_then(|run| run.refused.borrow_mut().take());
        // Nothing of this run's is kept for the next but the globals the script made.
        let _ = self.context.remove_data::<Run>();
        let _ = self.this.set_prototype(None);
        match (ran, recorded.exceeded, first_refusal) {
            (_, Some(exceeded), _) => refused(
                over,
                format!("the script was stopped: {}", exceeded.sentence()),
            ),
            (Err((steps, sentence)), None, _) => refused(steps, sentence),
            (Ok(_), None, Some(sentence)) => refused(0, sentence),
            (Ok((steps, value)), None, None) => FormRan {
                steps,
                outcome: value,
            },
        }
    }

    /// Parses and evaluates the run, sliced against the steps and the wall clock.
    fn evaluate(&mut self, script: &str, here: u32, steps_left: u64) -> Evaluated {
        let over = steps_left.saturating_add(1);
        let container =
            node_object(here, &mut self.context).map_err(|error| (0, sentence(&error)))?;
        let _ = self.this.set_prototype(Some(container));
        let parsed = parse(
            &format!("with ({SCOPE}) {{\n{script}\n}}"),
            &mut self.context,
        )
        .or_else(|first| {
            // Page 379's function: a script whose top level returns is a function's body.
            parse(
                &format!("with ({SCOPE}) {{ (function () {{\n{script}\n}}).call(this) }}"),
                &mut self.context,
            )
            .map_err(|_| first)
        })
        .map_err(|error| (0, format!("the script does not parse: {error}")))?;
        let budget = Budget {
            steps: self.budget.steps.min(steps_left),
            ..self.budget
        };
        self.waited.set(Duration::ZERO);
        let started = Instant::now();
        let mut spent = 0_u64;
        let mut waker = TaskContext::from_waker(Waker::noop());
        let completion = {
            let mut evaluation = pin!(parsed.evaluate_async_with_budget(&mut self.context, SLICE));
            loop {
                match evaluation.as_mut().poll(&mut waker) {
                    Poll::Ready(Ok(value)) => break value,
                    Poll::Ready(Err(error)) => {
                        return Err(match ending_of(&error, &budget) {
                            Ending::Exceeded(exceeded) => (
                                over,
                                format!("the script was stopped: {}", exceeded.sentence()),
                            ),
                            _ => (spent, format!("the script threw: {}", sentence(&error))),
                        });
                    }
                    Poll::Pending => {
                        spent = spent.saturating_add(u64::from(SLICE));
                        if spent > budget.steps {
                            return Err((
                                over,
                                format!(
                                    "the script was stopped: {}",
                                    crate::Exceeded::Steps(budget.steps).sentence()
                                ),
                            ));
                        }
                        if started.elapsed() > budget.wall {
                            return Err((
                                over,
                                format!(
                                    "the script was stopped: {}",
                                    crate::Exceeded::Wall(budget.wall).sentence()
                                ),
                            ));
                        }
                    }
                }
            }
        };
        Ok((spent, scalar(&completion, &mut self.context)))
    }
}

/// How an evaluation ended: the steps it spent and the script's value, or the steps to report and
/// the sentence saying why it stopped.
type Evaluated = Result<(u64, Result<Scalar, String>), (u64, String)>;

/// The names the engine defines before any script runs, which are never a container's, kept
/// where a trap can read them.
struct Builtins(BTreeSet<String>);

/// A script's text, parsed.
fn parse(source: &str, context: &mut Context) -> JsResult<Script> {
    Script::parse(Source::from_bytes(source.as_bytes()), None, context)
}

/// What an error says, in a sentence.
fn sentence(error: &JsError) -> String {
    error
        .as_opaque()
        .map_or_else(|| error.to_string(), |value| value.display().to_string())
}

/// A script's value as the form holds one.
fn scalar(value: &JsValue, context: &mut Context) -> Result<Scalar, String> {
    if value.is_undefined() || value.is_null() {
        return Ok(Scalar::Null);
    }
    if let Some(flag) = value.as_boolean() {
        return Ok(Scalar::Number(if flag { 1.0 } else { 0.0 }));
    }
    if let Some(number) = value.as_number() {
        return Ok(Scalar::Number(number));
    }
    if value.is_string() || value.is_bigint() {
        return value
            .to_string(context)
            .map(|text| Scalar::Text(text.to_std_string_escaped()))
            .map_err(|error| sentence(&error));
    }
    Err(
        "the value is an object, which no container holds; a container's value is its rawValue \
         (XFA 3.3 page 106)"
            .to_owned(),
    )
}

/// A container's value as the script sees it.
fn js_value(value: Scalar) -> JsValue {
    match value {
        Scalar::Null => JsValue::null(),
        Scalar::Number(number) => JsValue::from(number),
        Scalar::Text(text) => JsValue::from(JsString::from(text.as_str())),
    }
}

/// The run's data, which a trap is only called inside.
fn run_of(context: &Context) -> JsResult<&Run> {
    context.get_data::<Run>().ok_or_else(|| {
        JsNativeError::typ()
            .with_message("no form script is running")
            .into()
    })
}

/// Refuses with `sentence`, which the run's outcome then carries.
fn refuse(sentence: String, context: &Context) -> JsError {
    if let Some(run) = context.get_data::<Run>() {
        run.refused
            .borrow_mut()
            .get_or_insert_with(|| sentence.clone());
    }
    JsNativeError::typ().with_message(sentence).into()
}

/// A property key as a name, or `None` for a symbol.
fn key_name(key: &JsValue) -> Option<String> {
    key.as_string().map(|name| name.to_std_string_escaped())
}

/// Installs the scope of naked references and `xfa`.
fn install(context: &mut Context) -> JsResult<()> {
    super::guard::install(context)?;
    let scope = JsProxy::builder(JsObject::with_null_proto())
        .has(scope_has)
        .get(scope_get)
        .set(scope_set)
        .build(context)?;
    context.register_global_property(js_string!(SCOPE), scope, Attribute::empty())?;
    let xfa = JsProxy::builder(JsObject::with_null_proto())
        .get(xfa_get)
        .set(xfa_set)
        .build(context)?;
    context.register_global_property(js_string!("xfa"), xfa.clone(), Attribute::READONLY)?;
    context.register_global_property(js_string!("$xfa"), xfa, Attribute::READONLY)?;
    Ok(())
}

/// The container a naked reference names, asked once a run, or `None`.
fn naked(name: &str, context: &Context) -> JsResult<Option<u32>> {
    if context
        .get_data::<Builtins>()
        .is_some_and(|builtins| builtins.0.contains(name))
    {
        return Ok(None);
    }
    let run = run_of(context)?;
    if let Some(known) = run.names.borrow().get(name) {
        return Ok(*known);
    }
    if name.len() > MAX_NAME_BYTES {
        return Ok(None);
    }
    let path = [Step {
        name: name.to_owned(),
        occurrence: Occurrence::Same,
    }];
    let found = run
        .host
        .resolve(None, &path)
        .map_err(|why| refuse(format!("{name}: {why}"), context))?;
    let first = found.first().copied();
    run.names.borrow_mut().insert(name.to_owned(), first);
    Ok(first)
}

/// `has` of the scope: whether a name is a container's.
fn scope_has(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let Some(name) = args.get(1).and_then(key_name) else {
        return Ok(false.into());
    };
    Ok(naked(&name, context)?.is_some().into())
}

/// `get` of the scope: the container a name is.
fn scope_get(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let Some(name) = args.get(1).and_then(key_name) else {
        return Ok(JsValue::undefined());
    };
    match naked(&name, context)? {
        Some(node) => Ok(node_object(node, context)?.into()),
        None => Ok(JsValue::undefined()),
    }
}

/// `set` of the scope: a container is never assigned by its name.
fn scope_set(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let name = args.get(1).and_then(key_name).unwrap_or_default();
    Err(refuse(
        format!(
            "{name} names a container of the form, which a script sets through its rawValue \
             (XFA 3.3 page 106) and not by assigning to its name"
        ),
        context,
    ))
}

/// The object a container is, one per container a run.
fn node_object(node: u32, context: &mut Context) -> JsResult<JsObject> {
    let run = run_of(context)?;
    if let Some(held) = run.nodes.borrow().get(&node) {
        return Ok(held.clone());
    }
    if run.nodes.borrow().len() >= MAX_NODES_HELD {
        return Err(refuse(
            format!("a run that names more than {MAX_NODES_HELD} containers"),
            context,
        ));
    }
    let key = run.key.clone();
    let target = JsObject::with_null_proto();
    target.set(key, JsValue::from(node), false, context)?;
    let proxy: JsObject = JsProxy::builder(target)
        .get(node_get)
        .set(node_set)
        .build(context)?
        .into();
    run_of(context)?
        .nodes
        .borrow_mut()
        .insert(node, proxy.clone());
    Ok(proxy)
}

/// The container number a container object's target holds.
fn number_of(target: &JsValue, context: &mut Context) -> JsResult<u32> {
    let key = run_of(context)?.key.clone();
    let held = target
        .as_object()
        .map(|object| object.get(key, context))
        .transpose()?;
    number_of_value(held.as_ref()).ok_or_else(|| {
        JsNativeError::typ()
            .with_message("not a container of the form")
            .into()
    })
}

/// The container `this` is, for a method of one.
fn this_container(this: &JsValue, context: &mut Context) -> JsResult<u32> {
    let key = run_of(context)?.key.clone();
    let held = this
        .as_object()
        .map(|object| object.get(key, context))
        .transpose()?;
    number_of_value(held.as_ref()).ok_or_else(|| {
        refuse(
            "a container's method called on something that is not one".to_owned(),
            context,
        )
    })
}

/// A number read back as a container's.
#[expect(
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the number is checked to be a whole number within u32's range before the cast, so the \
              exact comparison is the test and the cast loses nothing"
)]
fn number_of_value(value: Option<&JsValue>) -> Option<u32> {
    let number = value?.as_number()?;
    let whole = number.trunc() == number && (0.0..=f64::from(u32::MAX)).contains(&number);
    whole.then_some(number as u32)
}

/// What a container offers besides its children.
const OFFERED: &str = "rawValue, isNull, resolveNode and resolveNodes";

/// `get` of a container.
fn node_get(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    // A trap is called on its handler, and the target is its first argument.
    let target = &args.first().cloned().unwrap_or_default();
    let key = args.get(1).cloned().unwrap_or_default();
    let Some(name) = key_name(&key) else {
        // The key a container's number is read under answers it; any other symbol is nothing.
        if let Some(symbol) = key.as_symbol()
            && symbol == run_of(context)?.key
        {
            return Ok(JsValue::from(number_of(target, context)?));
        }
        return Ok(JsValue::undefined());
    };
    let node = number_of(target, context)?;
    let host = Rc::clone(&run_of(context)?.host);
    match name.as_str() {
        "rawValue" => host
            .value(node)
            .map(js_value)
            .map_err(|why| refuse(format!("rawValue: {why}"), context)),
        "isNull" => host
            .value(node)
            .map(|value| JsValue::from(value == Scalar::Null))
            .map_err(|why| refuse(format!("isNull: {why}"), context)),
        "resolveNode" => Ok(method(context, "resolveNode", resolve_node).into()),
        "resolveNodes" => Ok(method(context, "resolveNodes", resolve_nodes).into()),
        _ => {
            let child = child(&host, node, &name, context)?;
            match child {
                Some(child) => Ok(node_object(child, context)?.into()),
                None => Err(refuse(
                    format!(
                        "{name}: neither a child of the container nor a property this reader's \
                         object model offers a form's JavaScript ({OFFERED})"
                    ),
                    context,
                )),
            }
        }
    }
}

/// The child of `node` called `name`, the first of several (page 115).
fn child(
    host: &Rc<dyn FormHost>,
    node: u32,
    name: &str,
    context: &Context,
) -> JsResult<Option<u32>> {
    if name.len() > MAX_NAME_BYTES {
        return Ok(None);
    }
    let path = [Step {
        name: name.to_owned(),
        occurrence: Occurrence::Same,
    }];
    host.resolve(Some(node), &path)
        .map(|found| found.first().copied())
        .map_err(|why| refuse(format!("{name}: {why}"), context))
}

/// `set` of a container: only its `rawValue`.
fn node_set(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    // A trap is called on its handler, and the target is its first argument.
    let target = &args.first().cloned().unwrap_or_default();
    let name = args.get(1).and_then(key_name).unwrap_or_default();
    if name != "rawValue" {
        return Err(refuse(
            format!(
                "{name}: a property a form's JavaScript may not set here; rawValue is the one it may"
            ),
            context,
        ));
    }
    let node = number_of(target, context)?;
    let value = args.get(2).cloned().unwrap_or_default();
    let value =
        scalar(&value, context).map_err(|why| refuse(format!("rawValue: {why}"), context))?;
    let host = Rc::clone(&run_of(context)?.host);
    host.assign(node, value)
        .map_err(|why| refuse(format!("rawValue: {why}"), context))?;
    Ok(true.into())
}

/// A native method under `name`.
fn method(
    context: &mut Context,
    name: &str,
    body: fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue>,
) -> JsObject {
    super::bridge::function(context, name, NativeFunction::from_fn_ptr(body))
}

/// `get` of `xfa`.
fn xfa_get(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let Some(name) = args.get(1).and_then(key_name) else {
        return Ok(JsValue::undefined());
    };
    match name.as_str() {
        "resolveNode" => Ok(method(context, "resolveNode", resolve_node).into()),
        "resolveNodes" => Ok(method(context, "resolveNodes", resolve_nodes).into()),
        _ => Err(refuse(
            format!(
                "xfa.{name}: not in the object model this reader offers a form's JavaScript, \
                 which is resolveNode, resolveNodes and the form's own containers"
            ),
            context,
        )),
    }
}

/// `set` of `xfa`.
fn xfa_set(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let name = args.get(1).and_then(key_name).unwrap_or_default();
    Err(refuse(
        format!("xfa.{name}: the xfa object is not a script's to set"),
        context,
    ))
}

/// What a `resolveNode` or `resolveNodes` call names: from the container it was called on, or from
/// the script's own for `xfa`'s.
fn resolved(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<Vec<u32>> {
    let from = match this.as_object() {
        Some(object) if this_is_xfa(&object, context) => None,
        _ => Some(this_container(this, context)?),
    };
    let text = args
        .first()
        .and_then(JsValue::as_string)
        .map(|text| text.to_std_string_escaped())
        .ok_or_else(|| {
            refuse(
                "resolveNode takes a SOM expression in a string (XFA 3.3 page 107)".to_owned(),
                context,
            )
        })?;
    let path = som(&text).map_err(|why| refuse(format!("{text}: {why}"), context))?;
    let host = Rc::clone(&run_of(context)?.host);
    host.resolve(from, &path)
        .map_err(|why| refuse(format!("{text}: {why}"), context))
}

/// Whether `object` is `xfa`.
fn this_is_xfa(object: &JsObject, context: &mut Context) -> bool {
    context
        .global_object()
        .get(js_string!("xfa"), context)
        .ok()
        .and_then(|value| value.as_object())
        .is_some_and(|xfa| JsObject::equals(&xfa, object))
}

/// `resolveNode`: the one container a SOM expression names, or null (page 107).
fn resolve_node(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let found = resolved(this, args, context)?;
    match found.as_slice() {
        [] => Ok(JsValue::null()),
        [one] => Ok(node_object(*one, context)?.into()),
        many => Err(refuse(
            format!(
                "the expression names {} containers; resolveNodes is the method for a list (XFA \
                 3.3 page 107)",
                many.len()
            ),
            context,
        )),
    }
}

/// `resolveNodes`: the containers a SOM expression names, in document order, as a list with a
/// `length` and an `item` (page 107).
fn resolve_nodes(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let found = resolved(this, args, context)?;
    let run = run_of(context)?;
    let listed = run.listed.get().saturating_add(found.len());
    if listed > MAX_LISTED {
        return Err(refuse(
            format!("lists of more than {MAX_LISTED} containers in one run"),
            context,
        ));
    }
    run.listed.set(listed);
    let length = found.len();
    let index = {
        let mut lists = run.lists.borrow_mut();
        lists.push(found);
        lists.len().saturating_sub(1)
    };
    let list = JsObject::with_object_proto(context.intrinsics());
    let (Ok(length), Ok(index)) = (u32::try_from(length), u32::try_from(index)) else {
        return Err(refuse(
            "a list past what a number names".to_owned(),
            context,
        ));
    };
    list.set(js_string!("length"), JsValue::from(length), false, context)?;
    let key = run_of(context)?.key.clone();
    list.set(key, JsValue::from(index), false, context)?;
    let item = method(context, "item", list_item);
    list.set(js_string!("item"), item, false, context)?;
    Ok(list.into())
}

/// A list's `item(i)`: its `i`th container, or null past its end.
fn list_item(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let index = this_container(this, context)?;
    let at = args
        .first()
        .cloned()
        .unwrap_or_default()
        .to_number(context)?;
    let node = {
        let run = run_of(context)?;
        let lists = run.lists.borrow();
        let list = usize::try_from(index)
            .ok()
            .and_then(|index| lists.get(index));
        number_of_value(Some(&JsValue::from(at)))
            .and_then(|at| usize::try_from(at).ok())
            .and_then(|at| list.and_then(|list| list.get(at).copied()))
    };
    match node {
        Some(node) => Ok(node_object(node, context)?.into()),
        None => Ok(JsValue::null()),
    }
}

/// A SOM expression in a string, as the steps a host resolves: names joined by `.`, each with an
/// optional `[n]`, `[*]`, `[+n]` or `[-n]`, a leading `$` for the script's own container (pages
/// 94–96 and 114–115).
///
/// # Errors
///
/// The sentence naming the part of SOM this reader does not take in a string: the descendant
/// separator `..` (page 104), the class separator `.#` (page 96), a predicate (page 105) and an
/// escaped character, each refused by name, and an expression past the bounds.
pub fn som(text: &str) -> Result<Vec<Step>, String> {
    if text.contains("..") {
        return Err("the descendant separator `..` (page 104)".to_owned());
    }
    if text.contains(".#") || text.starts_with('#') {
        return Err("the class separator `.#` (page 96)".to_owned());
    }
    if text.contains(".[") || text.contains(".(") {
        return Err("a predicate (page 105)".to_owned());
    }
    if text.contains('\\') {
        return Err("an escaped character (page 105)".to_owned());
    }
    let mut steps = Vec::new();
    for part in text.split('.') {
        if steps.len() >= MAX_PATH_STEPS {
            return Err(format!("an expression of more than {MAX_PATH_STEPS} names"));
        }
        let (name, occurrence) = match part.split_once('[') {
            None => (part, Occurrence::Same),
            Some((name, index)) => {
                let index = index
                    .strip_suffix(']')
                    .ok_or_else(|| format!("the index of {name} is not closed"))?;
                (name, occurrence_of(index.trim())?)
            }
        };
        let name = name.trim();
        if name.is_empty() {
            return Err("an empty name".to_owned());
        }
        if name.len() > MAX_NAME_BYTES {
            return Err(format!("a name of more than {MAX_NAME_BYTES} bytes"));
        }
        steps.push(Step {
            name: name.to_owned(),
            occurrence,
        });
    }
    Ok(steps)
}

/// An index of a SOM step (page 94).
fn occurrence_of(index: &str) -> Result<Occurrence, String> {
    if index == "*" {
        return Ok(Occurrence::Every);
    }
    let parsed = |digits: &str| {
        digits
            .parse::<i64>()
            .map_err(|_| format!("the index [{index}] is not a number"))
    };
    if let Some(by) = index.strip_prefix('+') {
        return Ok(Occurrence::Relative(parsed(by)?));
    }
    if let Some(by) = index.strip_prefix('-') {
        return Ok(Occurrence::Relative(parsed(by)?.saturating_neg()));
    }
    Ok(Occurrence::At(parsed(index)?))
}
