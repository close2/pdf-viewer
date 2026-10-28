//! The worker's realms for a form's scripts: a script of a form's, `FormCalc` or JavaScript, run
//! against questions the host answers (ADRs 2524, 2545).
//!
//! The interpreters are `pdf_formcalc` and `pdf_script::xfa`'s [`FormRealm`], neither of which holds
//! a form; the [`Host`] of the one and the [`FormHost`] of the other is here one conversation.
//! A path the script names, a value it reads and a value it assigns are each a frame to the host
//! and a frame back, on the pipes the JavaScript realm uses, so the worker's profile (ADR 1608)
//! is unchanged: no descriptor, no file, nothing but the two pipes. **The form never crosses**:
//! the worker learns of it what the script asks, one container at a time, and the host's `Form`
//! applies its own limits to each assignment (ADR 2507), so a script cannot grow the form from
//! here.
//!
//! A `FormCalc` run is held to the steps the host named and to `pdf_formcalc`'s other ceilings
//! ([`Budget::CALCULATION`]); a JavaScript run to the steps the host named and to the budget of a
//! field's event ([`ScriptBudget::FIELD_EVENT`]: its wall clock, its loop, recursion and stack
//! limits and its guards on what an argument can make large), in one realm kept for the worker's
//! life so that the form's globals persist (XFA 3.3 page 414). The host holds the conversation to
//! a deadline and to a count of questions ([`MAX_QUESTIONS`]) from its side, which is the side
//! that is trusted, and the activation's sequence to its own clock, whichever the language.

use std::cell::{Cell, RefCell};
use std::io::Write;
use std::rc::Rc;

use pdf_formcalc::parse::Separator;
use pdf_formcalc::{Budget, Host, Occurrence, Segment, Value, run_counted};
use pdf_model::xfa::{FormLanguage, Occurrence as Wanted, Scalar, Step};
use pdf_script::Budget as ScriptBudget;
use pdf_script::xfa::{FormHost, FormRealm};

use super::{Incoming, Pipes, read_frame, write_frame};
use crate::form_wire::{
    Done, FormAnswer, FormQuestion, FormRun, MAX_NAME_BYTES, MAX_PATH_STEPS, MAX_VALUE_BYTES,
    decode_answer, decode_form_run, encode_question,
};

/// Runs one form run to its end and writes the frame that says how it ended.
///
/// # Errors
///
/// A pipe's: a worker that cannot write to its host has nobody to refuse to.
pub(super) fn serve_run(
    pipes: &Rc<RefCell<Pipes>>,
    realm: &mut Option<FormRealm>,
    payload: &[u8],
) -> Result<(), std::io::Error> {
    let done = match decode_form_run(payload) {
        Ok(run) => match run.language {
            FormLanguage::FormCalc => execute(pipes, &run),
            FormLanguage::JavaScript => execute_javascript(pipes, realm, &run),
        },
        Err(error) => Done {
            steps: 0,
            outcome: Err(format!("the form run does not decode: {error}")),
        },
    };
    let (kind, bytes) = encode_question(&FormQuestion::Done(done));
    let mut pipes = pipes.borrow_mut();
    write_frame(&mut pipes.output, kind, &bytes)?;
    pipes.output.flush()
}

/// The `FormCalc` script, run.
fn execute(pipes: &Rc<RefCell<Pipes>>, run: &FormRun) -> Done {
    let mut host = Conversation::new(pipes);
    let budget = Budget {
        steps: Budget::CALCULATION.steps.min(run.steps_left),
        ..Budget::CALCULATION
    };
    let (outcome, steps) = run_counted(&run.script, &mut host, budget);
    // A value the host could not tell the script made the script's reading of the form wrong, so
    // whatever it computed from a null it was handed in that value's place is not an outcome.
    let failure = host
        .broken
        .borrow()
        .clone()
        .or_else(|| host.unreadable.borrow().clone());
    let outcome = match (failure, outcome) {
        (Some(sentence), _) => Err(sentence),
        (None, Ok(value)) => Ok(scalar(value)),
        (None, Err(stop)) => Err(stop.to_string()),
    };
    Done { steps, outcome }
}

/// The JavaScript script, run in the worker's form realm, which is made at the first such run.
fn execute_javascript(
    pipes: &Rc<RefCell<Pipes>>,
    realm: &mut Option<FormRealm>,
    run: &FormRun,
) -> Done {
    if realm.is_none() {
        match FormRealm::new(ScriptBudget::FIELD_EVENT) {
            Ok(made) => *realm = Some(made),
            Err(sentence) => {
                return Done {
                    steps: 0,
                    outcome: Err(sentence),
                };
            }
        }
    }
    let Some(realm) = realm.as_mut() else {
        return Done {
            steps: 0,
            outcome: Err("the form's realm could not be constructed".to_owned()),
        };
    };
    let host = Rc::new(Conversation::new(pipes));
    let shared: Rc<dyn FormHost> = Rc::clone(&host) as Rc<dyn FormHost>;
    let ran = realm.run(&run.script, run.node, shared, run.steps_left);
    // A pipe that broke made every later answer a refusal the script may have caught, so the run
    // is the pipe's failure whatever the script made of it.
    let broken = host.broken.borrow().clone();
    Done {
        steps: ran.steps,
        outcome: match broken {
            Some(sentence) => Err(sentence),
            None => ran.outcome,
        },
    }
}

/// A script's value as the form holds one.
fn scalar(value: Value) -> Scalar {
    match value {
        Value::Null => Scalar::Null,
        Value::Number(n) => Scalar::Number(n),
        Value::String(text) => Scalar::Text(text),
    }
}

/// A form seen only through the host's answers.
struct Conversation {
    /// The pipes the JavaScript realm shares.
    pipes: Rc<RefCell<Pipes>>,
    /// Why the pipes can be used no more, once they cannot: an answer that is not the one asked
    /// for leaves the next frame's place unknown, so nothing more is asked.
    broken: RefCell<Option<String>>,
    /// The sentence the host gave for a value it would not tell, which [`Host::value`] cannot
    /// return.
    unreadable: RefCell<Option<String>>,
    /// How many questions have been put.
    questions: Cell<usize>,
}

impl Conversation {
    /// A conversation on `pipes` that has put no question.
    fn new(pipes: &Rc<RefCell<Pipes>>) -> Self {
        Self {
            pipes: Rc::clone(pipes),
            broken: RefCell::new(None),
            unreadable: RefCell::new(None),
            questions: Cell::new(0),
        }
    }

    /// Puts `question` and reads the answer.
    fn ask(&self, question: &FormQuestion) -> Result<FormAnswer, String> {
        if let Some(why) = self.broken.borrow().as_ref() {
            return Err(why.clone());
        }
        self.questions.set(self.questions.get().saturating_add(1));
        let broken = |why: String| {
            *self.broken.borrow_mut() = Some(why.clone());
            Err(why)
        };
        let Ok(mut pipes) = self.pipes.try_borrow_mut() else {
            return broken("the pipes to the host are in use".to_owned());
        };
        let (kind, bytes) = encode_question(question);
        if let Err(error) = write_frame(&mut pipes.output, kind, &bytes) {
            return broken(format!("the host cannot be asked: {error}"));
        }
        match read_frame(&mut pipes.input) {
            Ok(Some(Incoming::Frame { kind, payload })) => match decode_answer(kind, &payload) {
                Ok(answer) => Ok(answer),
                Err(error) => broken(format!("the host's answer does not decode: {error}")),
            },
            Ok(Some(Incoming::TooLarge { length })) => broken(format!(
                "the host's answer of {length} bytes is past what is read"
            )),
            Ok(None) => broken("the host closed its end".to_owned()),
            Err(error) => broken(format!("the host's answer cannot be read: {error}")),
        }
    }

    /// An answer of a kind that was not asked for, which is a worker and a host out of step.
    fn unexpected(&self, asked: &str) -> String {
        let why = format!("the host answered a question about {asked} with another kind of answer");
        *self.broken.borrow_mut() = Some(why.clone());
        why
    }
}

/// The JavaScript realm's questions, which are the `FormCalc` interpreter's with one more: a path
/// from a container the host named (XFA 3.3 page 107). A path or a name past what the wire carries
/// is refused here, before it is put, so that a frame the host would not read is never written.
impl FormHost for Conversation {
    fn resolve(&self, from: Option<u32>, path: &[Step]) -> Result<Vec<u32>, String> {
        if path.len() > MAX_PATH_STEPS {
            return Err(format!("a reference of more than {MAX_PATH_STEPS} names"));
        }
        if path.iter().any(|step| step.name.len() > MAX_NAME_BYTES) {
            return Err(format!("a name of more than {MAX_NAME_BYTES} bytes"));
        }
        let question = match from {
            None => FormQuestion::Resolve(path.to_vec()),
            Some(from) => FormQuestion::ResolveFrom(from, path.to_vec()),
        };
        match self.ask(&question)? {
            FormAnswer::Nodes(nodes) => nodes,
            _ => Err(self.unexpected("a path")),
        }
    }

    fn value(&self, node: u32) -> Result<Scalar, String> {
        match self.ask(&FormQuestion::Read(node))? {
            FormAnswer::Value(value) => value,
            _ => Err(self.unexpected("a value")),
        }
    }

    fn assign(&self, node: u32, value: Scalar) -> Result<(), String> {
        if let Scalar::Text(text) = &value
            && text.len() > MAX_VALUE_BYTES
        {
            return Err(format!(
                "a value of {} bytes, past the {MAX_VALUE_BYTES} a form is told",
                text.len()
            ));
        }
        match self.ask(&FormQuestion::Write(node, value))? {
            FormAnswer::Ack(done) => done,
            _ => Err(self.unexpected("an assignment")),
        }
    }
}

impl Host for Conversation {
    type Node = u32;

    fn resolve(&self, path: &[Segment]) -> Result<Vec<u32>, String> {
        if path.len() > MAX_PATH_STEPS {
            return Err(format!("a reference of more than {MAX_PATH_STEPS} names"));
        }
        let mut steps = Vec::with_capacity(path.len());
        for segment in path {
            match segment.separator {
                Separator::First | Separator::Dot => {}
                Separator::DotDot => {
                    return Err("the descendant separator `..` (page 104)".to_owned());
                }
                Separator::DotHash => return Err("the class separator `.#` (page 96)".to_owned()),
            }
            if segment.name.len() > MAX_NAME_BYTES {
                return Err(format!("a name of more than {MAX_NAME_BYTES} bytes"));
            }
            steps.push(Step {
                name: segment.name.clone(),
                occurrence: match segment.occurrence {
                    Occurrence::Same => Wanted::Same,
                    Occurrence::Every => Wanted::Every,
                    Occurrence::At(n) => Wanted::At(n),
                    Occurrence::Relative(n) => Wanted::Relative(n),
                },
            });
        }
        match self.ask(&FormQuestion::Resolve(steps))? {
            FormAnswer::Nodes(nodes) => nodes,
            _ => Err(self.unexpected("a path")),
        }
    }

    fn value(&self, node: &u32) -> Value {
        match self.ask(&FormQuestion::Read(*node)) {
            // A refusal recorded in `broken` by `ask` reads as null, as a null does.
            Ok(FormAnswer::Value(Ok(Scalar::Null))) | Err(_) => Value::Null,
            Ok(FormAnswer::Value(Ok(Scalar::Number(n)))) => Value::Number(n),
            Ok(FormAnswer::Value(Ok(Scalar::Text(text)))) => Value::String(text),
            Ok(FormAnswer::Value(Err(sentence))) => {
                self.unreadable.borrow_mut().get_or_insert(sentence);
                Value::Null
            }
            Ok(_) => {
                self.unexpected("a value");
                Value::Null
            }
        }
    }

    fn assign(&mut self, node: &u32, value: Value) -> Result<(), String> {
        match self.ask(&FormQuestion::Write(*node, scalar(value)))? {
            FormAnswer::Ack(done) => done,
            _ => Err(self.unexpected("an assignment")),
        }
    }
}
