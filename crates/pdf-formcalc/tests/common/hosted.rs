//! The host of a `FormCalc` script over `pdf-model`'s form: the adapter ADR 2482 puts in the
//! program that runs scripts, not in either crate, because `pdf-model` does not depend on this
//! one and this one depends on nothing. The test of a hybrid form and the census example both
//! include this file, so that what the census counts is what the test proves.

use pdf_formcalc::parse::Separator;
use pdf_formcalc::{Budget, Host, Occurrence, Segment, Stop, Value, run};
use pdf_model::xfa::{Calculation, Form, Occurrence as Wanted, Scalar, Step};

/// A form and the container whose script runs.
struct Hosted<'f, 'a> {
    form: &'f mut Form<'a>,
    here: usize,
}

impl Host for Hosted<'_, '_> {
    type Node = usize;

    fn resolve(&self, path: &[Segment]) -> Result<Vec<usize>, String> {
        let mut steps = Vec::with_capacity(path.len());
        for segment in path {
            match segment.separator {
                Separator::First | Separator::Dot => {}
                Separator::DotDot => {
                    return Err("the descendant separator `..` (page 94)".to_owned());
                }
                Separator::DotHash => return Err("the class separator `.#` (page 94)".to_owned()),
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
        self.form.resolve(self.here, &steps)
    }

    fn value(&self, node: &usize) -> Value {
        match self.form.value(*node) {
            Scalar::Null => Value::Null,
            Scalar::Number(n) => Value::Number(n),
            Scalar::Text(text) => Value::String(text),
        }
    }

    fn assign(&mut self, node: &usize, value: Value) -> Result<(), String> {
        self.form.set(*node, scalar(value))
    }
}

fn scalar(value: Value) -> Scalar {
    match value {
        Value::Null => Scalar::Null,
        Value::Number(n) => Scalar::Number(n),
        Value::String(text) => Scalar::Text(text),
    }
}

/// Runs one `FormCalc` calculation against the form and, where its container takes a value, makes
/// the script's value the container's (XFA 3.3 page 379).
///
/// # Errors
///
/// The script's [`Stop`].
pub(crate) fn calculate(form: &mut Form<'_>, calculation: &Calculation) -> Result<Value, Stop> {
    let mut host = Hosted {
        form,
        here: calculation.node,
    };
    let value = run(&calculation.script, &mut host, Budget::CALCULATION)?;
    if calculation.sets_value {
        host.form
            .set(calculation.node, scalar(value.clone()))
            .map_err(Stop::Exception)?;
    }
    Ok(value)
}
