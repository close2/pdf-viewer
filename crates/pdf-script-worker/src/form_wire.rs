//! What a script of an XFA form's crosses as, in either language: the script out, the form's
//! questions back, the answer to each, and the run's end (ADRs 2524, 2545).
//!
//! A document's JavaScript run is a request and an outcome ([`crate::wire`]); a form's run is a
//! conversation, because the form it reads and writes is the host's and the worker holds none of
//! it. The host sends [`FormRun`], naming the script's language, and then answers each
//! [`FormQuestion`] the interpreter puts — a path to resolve from the script's container (XFA 3.3
//! pages 94 and 114) or from a container it was told of (page 107), a value to read, a value to
//! assign — until the worker says [`FormQuestion::Done`]. A container crosses as the number the host's `Form` gave it, which the
//! worker can neither forge into a container nor use for anything but a question back to the host,
//! so the worker learns of the form no more than the script reads.
//!
//! Every length on the wire is read against the bytes that remain and every count against a
//! constant before anything is allocated from it, as [`crate::wire`] does, because the worker's
//! side of the pipe is the one a hostile document's script runs on.

use pdf_model::xfa::{FormLanguage, Occurrence, Scalar, Step};

use crate::wire::{MAX_SCRIPT_BYTES, Reader, WireError, put_len, put_str};

/// The kind of a form run: host to worker.
pub const FRAME_FORM_RUN: u8 = 6;
/// The kind of a run's end: worker to host.
pub const FRAME_FORM_DONE: u8 = 7;
/// The kind of a path to resolve: worker to host.
pub const FRAME_FORM_RESOLVE: u8 = 8;
/// The kind of a value to read: worker to host.
pub const FRAME_FORM_READ: u8 = 9;
/// The kind of a value to assign: worker to host.
pub const FRAME_FORM_WRITE: u8 = 10;
/// The kind of the answer to a path: host to worker.
pub const FRAME_FORM_NODES: u8 = 11;
/// The kind of the answer to a read: host to worker.
pub const FRAME_FORM_VALUE: u8 = 12;
/// The kind of the answer to an assignment: host to worker.
pub const FRAME_FORM_ACK: u8 = 13;
/// The kind of a path to resolve from a container the worker was told of: worker to host.
pub const FRAME_FORM_RESOLVE_FROM: u8 = 14;

/// Most names one path carries, which is far past a reference any form writes: the parser holds a
/// script to 64 levels of nesting (ADR 2482) and a SOM expression is a chain of names.
pub const MAX_PATH_STEPS: usize = 64;

/// Longest name in a path, in bytes.
pub const MAX_NAME_BYTES: usize = 4096;

/// Most containers one path may resolve to. A host whose form resolves a path to more answers with
/// a sentence instead, so that an answer fits the frame a worker reads.
pub const MAX_NODES: usize = 65_536;

/// Longest text, in bytes, a value read or assigned carries: the interpreter's own ceiling on a
/// string (`pdf_formcalc::Budget::CALCULATION`), restated because the worker must not believe the
/// host's bound held.
pub const MAX_VALUE_BYTES: usize = 1 << 20;

/// Most questions one run may put before the host stops answering them.
///
/// A script's own step budget bounds this too, but a compromised worker does not keep its budget.
/// Ten thousand is far past what a calculation asks: the longest in ADR 2482's census is one
/// script of 76 `Replace` calls, a few hundred questions. Past it the host answers each question
/// with a sentence, and the script ends with the error that sentence is.
pub const MAX_QUESTIONS: usize = 10_000;

/// One run: where the script belongs, what it may spend, and its text.
#[derive(Debug, Clone, PartialEq)]
pub struct FormRun {
    /// The container the script belongs to, as the host's `Form` numbers it.
    pub node: u32,
    /// The language the script is in.
    pub language: FormLanguage,
    /// How many steps the script may take (page 380's means of ending a seemingly infinite loop).
    pub steps_left: u64,
    /// The script.
    pub script: String,
}

/// What the interpreter puts to the host, or how the run ended.
#[derive(Debug, Clone, PartialEq)]
pub enum FormQuestion {
    /// The containers a path names, relative to the run's own container.
    Resolve(Vec<Step>),
    /// The containers a path names below a container the host named: a JavaScript container
    /// object's `resolveNode` and its children (XFA 3.3 pages 90 and 107).
    ResolveFrom(u32, Vec<Step>),
    /// The value a container holds.
    Read(u32),
    /// Sets the value a container holds.
    Write(u32, Scalar),
    /// The run ended.
    Done(Done),
}

/// How a run ended.
#[derive(Debug, Clone, PartialEq)]
pub struct Done {
    /// How many steps it took.
    pub steps: u64,
    /// The value it computed, or the sentence saying why it computed none.
    pub outcome: Result<Scalar, String>,
}

/// The host's answer to a [`FormQuestion`] that is not [`FormQuestion::Done`].
#[derive(Debug, Clone, PartialEq)]
pub enum FormAnswer {
    /// The containers a path names, or the sentence saying why it names none.
    Nodes(Result<Vec<u32>, String>),
    /// The value a container holds, or the sentence saying why it can be told none.
    Value(Result<Scalar, String>),
    /// Whether an assignment was made.
    Ack(Result<(), String>),
}

/// Encodes a form run.
#[must_use]
pub fn encode_form_run(run: &FormRun) -> Vec<u8> {
    let mut out = vec![crate::wire::VERSION];
    out.extend_from_slice(&run.node.to_le_bytes());
    out.push(match run.language {
        FormLanguage::FormCalc => 0,
        FormLanguage::JavaScript => 1,
    });
    out.extend_from_slice(&run.steps_left.to_le_bytes());
    put_str(&mut out, &run.script);
    out
}

/// Decodes a form run.
///
/// # Errors
///
/// [`WireError`] for bytes that are not exactly one form run of this version, a language byte
/// that names neither language, or a script past [`MAX_SCRIPT_BYTES`].
pub fn decode_form_run(bytes: &[u8]) -> Result<FormRun, WireError> {
    let mut reader = Reader { rest: bytes };
    match reader.u8()? {
        crate::wire::VERSION => {}
        other => return Err(WireError::Version(other)),
    }
    let node = reader.u32()?;
    let language = match reader.u8()? {
        0 => FormLanguage::FormCalc,
        1 => FormLanguage::JavaScript,
        _ => return Err(WireError::Invalid("language")),
    };
    let steps_left = reader.u64()?;
    let script = reader.string()?;
    if script.len() > MAX_SCRIPT_BYTES {
        return Err(WireError::ScriptTooLong(script.len()));
    }
    reader.end()?;
    Ok(FormRun {
        node,
        language,
        steps_left,
        script,
    })
}

/// Encodes a question as the kind it is framed under and its payload.
#[must_use]
pub fn encode_question(question: &FormQuestion) -> (u8, Vec<u8>) {
    let mut out = Vec::new();
    match question {
        FormQuestion::Resolve(path) => {
            put_path(&mut out, path);
            (FRAME_FORM_RESOLVE, out)
        }
        FormQuestion::ResolveFrom(from, path) => {
            out.extend_from_slice(&from.to_le_bytes());
            put_path(&mut out, path);
            (FRAME_FORM_RESOLVE_FROM, out)
        }
        FormQuestion::Read(node) => {
            out.extend_from_slice(&node.to_le_bytes());
            (FRAME_FORM_READ, out)
        }
        FormQuestion::Write(node, value) => {
            out.extend_from_slice(&node.to_le_bytes());
            put_scalar(&mut out, value);
            (FRAME_FORM_WRITE, out)
        }
        FormQuestion::Done(done) => {
            out.extend_from_slice(&done.steps.to_le_bytes());
            put_result(&mut out, &done.outcome, put_scalar);
            (FRAME_FORM_DONE, out)
        }
    }
}

/// Decodes a question from the kind it was framed under and its payload.
///
/// # Errors
///
/// [`WireError`] for a kind no worker puts a question under, a path past [`MAX_PATH_STEPS`] or a
/// name past [`MAX_NAME_BYTES`], a text past [`MAX_VALUE_BYTES`], or bytes that end early or run on.
pub fn decode_question(kind: u8, bytes: &[u8]) -> Result<FormQuestion, WireError> {
    let mut reader = Reader { rest: bytes };
    let question = match kind {
        FRAME_FORM_RESOLVE => FormQuestion::Resolve(read_path(&mut reader)?),
        FRAME_FORM_RESOLVE_FROM => {
            let from = reader.u32()?;
            FormQuestion::ResolveFrom(from, read_path(&mut reader)?)
        }
        FRAME_FORM_READ => FormQuestion::Read(reader.u32()?),
        FRAME_FORM_WRITE => {
            let node = reader.u32()?;
            FormQuestion::Write(node, read_scalar(&mut reader)?)
        }
        FRAME_FORM_DONE => {
            let steps = reader.u64()?;
            let outcome = read_result(&mut reader, read_scalar)?;
            FormQuestion::Done(Done { steps, outcome })
        }
        other => return Err(WireError::Kind(other)),
    };
    reader.end()?;
    Ok(question)
}

/// Encodes an answer as the kind it is framed under and its payload.
#[must_use]
pub fn encode_answer(answer: &FormAnswer) -> (u8, Vec<u8>) {
    let mut out = Vec::new();
    match answer {
        FormAnswer::Nodes(nodes) => {
            put_result(&mut out, nodes, |out, nodes| {
                put_len(out, nodes.len());
                for node in nodes {
                    out.extend_from_slice(&node.to_le_bytes());
                }
            });
            (FRAME_FORM_NODES, out)
        }
        FormAnswer::Value(value) => {
            put_result(&mut out, value, put_scalar);
            (FRAME_FORM_VALUE, out)
        }
        FormAnswer::Ack(done) => {
            put_result(&mut out, done, |_, ()| {});
            (FRAME_FORM_ACK, out)
        }
    }
}

/// Decodes an answer from the kind it was framed under and its payload.
///
/// # Errors
///
/// [`WireError`] for a kind no host answers with, a list past [`MAX_NODES`], a text past
/// [`MAX_VALUE_BYTES`], or bytes that end early or run on.
pub fn decode_answer(kind: u8, bytes: &[u8]) -> Result<FormAnswer, WireError> {
    let mut reader = Reader { rest: bytes };
    let answer = match kind {
        FRAME_FORM_NODES => FormAnswer::Nodes(read_result(&mut reader, |reader| {
            let count = reader.length()?;
            if count > MAX_NODES {
                return Err(WireError::Invalid("node count"));
            }
            // The count is checked against the bytes that remain before it reserves anything.
            if reader.rest.len() < count.saturating_mul(4) {
                return Err(WireError::Truncated);
            }
            let mut nodes = Vec::with_capacity(count);
            for _ in 0..count {
                nodes.push(reader.u32()?);
            }
            Ok(nodes)
        })?),
        FRAME_FORM_VALUE => FormAnswer::Value(read_result(&mut reader, read_scalar)?),
        FRAME_FORM_ACK => FormAnswer::Ack(read_result(&mut reader, |_| Ok(()))?),
        other => return Err(WireError::Kind(other)),
    };
    reader.end()?;
    Ok(answer)
}

/// Writes a path as its count and each step's name and occurrence.
fn put_path(out: &mut Vec<u8>, path: &[Step]) {
    put_len(out, path.len());
    for step in path {
        put_str(out, &step.name);
        put_occurrence(out, step.occurrence);
    }
}

/// Reads what [`put_path`] wrote, the count held to [`MAX_PATH_STEPS`] before anything is reserved
/// and each name to [`MAX_NAME_BYTES`].
fn read_path(reader: &mut Reader<'_>) -> Result<Vec<Step>, WireError> {
    let count = reader.length()?;
    if count > MAX_PATH_STEPS {
        return Err(WireError::Invalid("path length"));
    }
    let mut path = Vec::with_capacity(count);
    for _ in 0..count {
        let name = reader.string()?;
        if name.len() > MAX_NAME_BYTES {
            return Err(WireError::Invalid("name length"));
        }
        path.push(Step {
            name,
            occurrence: read_occurrence(reader)?,
        });
    }
    Ok(path)
}

/// Writes `Ok` as a zero byte and its payload, `Err` as a one and a sentence cut to the longest a
/// refusal keeps.
fn put_result<T>(
    out: &mut Vec<u8>,
    result: &Result<T, String>,
    put: impl FnOnce(&mut Vec<u8>, &T),
) {
    match result {
        Ok(value) => {
            out.push(0);
            put(out, value);
        }
        Err(sentence) => {
            out.push(1);
            put_str(out, cut(sentence, crate::wire::MAX_SENTENCE_BYTES));
        }
    }
}

/// Reads what [`put_result`] wrote.
fn read_result<T>(
    reader: &mut Reader<'_>,
    read: impl FnOnce(&mut Reader<'_>) -> Result<T, WireError>,
) -> Result<Result<T, String>, WireError> {
    match reader.u8()? {
        0 => Ok(Ok(read(reader)?)),
        1 => {
            let sentence = reader.string()?;
            if sentence.len() > crate::wire::MAX_SENTENCE_BYTES {
                return Err(WireError::Invalid("sentence length"));
            }
            Ok(Err(sentence))
        }
        _ => Err(WireError::Invalid("result")),
    }
}

/// `text` cut to at most `bytes` bytes, at a character boundary.
fn cut(text: &str, bytes: usize) -> &str {
    let mut end = text.len().min(bytes);
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    text.get(..end).unwrap_or_default()
}

fn put_scalar(out: &mut Vec<u8>, value: &Scalar) {
    match value {
        Scalar::Null => out.push(0),
        Scalar::Number(n) => {
            out.push(1);
            out.extend_from_slice(&n.to_bits().to_le_bytes());
        }
        Scalar::Text(text) => {
            out.push(2);
            put_str(out, text);
        }
    }
}

fn read_scalar(reader: &mut Reader<'_>) -> Result<Scalar, WireError> {
    match reader.u8()? {
        0 => Ok(Scalar::Null),
        1 => Ok(Scalar::Number(f64::from_bits(reader.u64()?))),
        2 => {
            let text = reader.string()?;
            if text.len() > MAX_VALUE_BYTES {
                return Err(WireError::Invalid("text length"));
            }
            Ok(Scalar::Text(text))
        }
        _ => Err(WireError::Invalid("value")),
    }
}

fn put_occurrence(out: &mut Vec<u8>, occurrence: Occurrence) {
    let (tag, n) = match occurrence {
        Occurrence::Same => (0u8, 0i64),
        Occurrence::Every => (1, 0),
        Occurrence::At(n) => (2, n),
        Occurrence::Relative(n) => (3, n),
    };
    out.push(tag);
    out.extend_from_slice(&n.to_le_bytes());
}

fn read_occurrence(reader: &mut Reader<'_>) -> Result<Occurrence, WireError> {
    let tag = reader.u8()?;
    let n = reader.i64()?;
    match tag {
        0 => Ok(Occurrence::Same),
        1 => Ok(Occurrence::Every),
        2 => Ok(Occurrence::At(n)),
        3 => Ok(Occurrence::Relative(n)),
        _ => Err(WireError::Invalid("occurrence")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> FormRun {
        FormRun {
            node: 7,
            language: FormLanguage::FormCalc,
            steps_left: 99,
            script: "$ = 1 + 2".to_owned(),
        }
    }

    fn javascript_run() -> FormRun {
        FormRun {
            node: 8,
            language: FormLanguage::JavaScript,
            steps_left: 1,
            script: "Units.rawValue * Price.rawValue".to_owned(),
        }
    }

    fn questions() -> Vec<FormQuestion> {
        vec![
            FormQuestion::Resolve(vec![
                Step {
                    name: "$".to_owned(),
                    occurrence: Occurrence::Same,
                },
                Step {
                    name: "Row".to_owned(),
                    occurrence: Occurrence::Every,
                },
                Step {
                    name: "Cell".to_owned(),
                    occurrence: Occurrence::At(-3),
                },
                Step {
                    name: "é".to_owned(),
                    occurrence: Occurrence::Relative(2),
                },
            ]),
            FormQuestion::ResolveFrom(
                9,
                vec![Step {
                    name: "Cell".to_owned(),
                    occurrence: Occurrence::Every,
                }],
            ),
            FormQuestion::ResolveFrom(u32::MAX, Vec::new()),
            FormQuestion::Read(4),
            FormQuestion::Write(4, Scalar::Text("x".to_owned())),
            FormQuestion::Write(5, Scalar::Number(f64::NEG_INFINITY)),
            FormQuestion::Write(6, Scalar::Null),
            FormQuestion::Done(Done {
                steps: 12,
                outcome: Ok(Scalar::Number(3.5)),
            }),
            FormQuestion::Done(Done {
                steps: 0,
                outcome: Err("refused".to_owned()),
            }),
        ]
    }

    fn answers() -> Vec<FormAnswer> {
        vec![
            FormAnswer::Nodes(Ok(vec![1, 2, 3])),
            FormAnswer::Nodes(Ok(Vec::new())),
            FormAnswer::Nodes(Err("no such name".to_owned())),
            FormAnswer::Value(Ok(Scalar::Text("ü".to_owned()))),
            FormAnswer::Value(Err("too long".to_owned())),
            FormAnswer::Ack(Ok(())),
            FormAnswer::Ack(Err("no value".to_owned())),
        ]
    }

    #[test]
    fn a_run_a_question_and_an_answer_come_back_as_they_went() {
        assert_eq!(decode_form_run(&encode_form_run(&run())), Ok(run()));
        assert_eq!(
            decode_form_run(&encode_form_run(&javascript_run())),
            Ok(javascript_run())
        );
        for question in questions() {
            let (kind, bytes) = encode_question(&question);
            assert_eq!(decode_question(kind, &bytes), Ok(question));
        }
        for answer in answers() {
            let (kind, bytes) = encode_answer(&answer);
            assert_eq!(decode_answer(kind, &bytes), Ok(answer));
        }
    }

    #[test]
    fn every_cut_of_a_message_is_refused_and_nothing_may_follow_one() {
        let mut messages = vec![
            (0, encode_form_run(&run())),
            (0, encode_form_run(&javascript_run())),
        ];
        messages.extend(questions().iter().map(encode_question));
        messages.extend(answers().iter().map(encode_answer));
        for (kind, bytes) in messages {
            for end in 0..bytes.len() {
                let prefix = bytes.get(..end).expect("a prefix");
                let refused = match kind {
                    0 => decode_form_run(prefix).is_err(),
                    FRAME_FORM_RESOLVE..=FRAME_FORM_WRITE
                    | FRAME_FORM_DONE
                    | FRAME_FORM_RESOLVE_FROM => decode_question(kind, prefix).is_err(),
                    _ => decode_answer(kind, prefix).is_err(),
                };
                assert!(refused, "kind {kind} cut at {end}");
            }
            let mut longer = bytes;
            longer.push(0);
            let refused = match kind {
                0 => decode_form_run(&longer).is_err(),
                FRAME_FORM_RESOLVE..=FRAME_FORM_WRITE
                | FRAME_FORM_DONE
                | FRAME_FORM_RESOLVE_FROM => decode_question(kind, &longer).is_err(),
                _ => decode_answer(kind, &longer).is_err(),
            };
            assert!(refused, "kind {kind} with a byte after it");
        }
    }

    /// A count on the wire is a claim: a path of 65 steps, a name past its bound, a node list past
    /// its bound and a text past its bound are each refused before anything is allocated from them.
    #[test]
    fn the_bounds_are_tested_at_the_bound() {
        let step = |name: &str| Step {
            name: name.to_owned(),
            occurrence: Occurrence::Same,
        };
        let path = |count: usize| FormQuestion::Resolve((0..count).map(|_| step("a")).collect());
        let (kind, bytes) = encode_question(&path(MAX_PATH_STEPS));
        assert!(decode_question(kind, &bytes).is_ok());
        let (kind, bytes) = encode_question(&path(MAX_PATH_STEPS + 1));
        assert_eq!(
            decode_question(kind, &bytes),
            Err(WireError::Invalid("path length"))
        );
        let from =
            |count: usize| FormQuestion::ResolveFrom(3, (0..count).map(|_| step("a")).collect());
        let (kind, bytes) = encode_question(&from(MAX_PATH_STEPS));
        assert!(decode_question(kind, &bytes).is_ok());
        let (kind, bytes) = encode_question(&from(MAX_PATH_STEPS + 1));
        assert_eq!(
            decode_question(kind, &bytes),
            Err(WireError::Invalid("path length"))
        );

        // A language byte that names neither language is refused.
        let mut bytes = encode_form_run(&run());
        if let Some(language) = bytes.get_mut(5) {
            *language = 2;
        }
        assert_eq!(decode_form_run(&bytes), Err(WireError::Invalid("language")));

        let name = |bytes: usize| FormQuestion::Resolve(vec![step(&"n".repeat(bytes))]);
        let (kind, ok) = encode_question(&name(MAX_NAME_BYTES));
        assert!(decode_question(kind, &ok).is_ok());
        let (kind, over) = encode_question(&name(MAX_NAME_BYTES + 1));
        assert_eq!(
            decode_question(kind, &over),
            Err(WireError::Invalid("name length"))
        );

        let text = |bytes: usize| FormQuestion::Write(0, Scalar::Text("t".repeat(bytes)));
        let (kind, ok) = encode_question(&text(MAX_VALUE_BYTES));
        assert!(decode_question(kind, &ok).is_ok());
        let (kind, over) = encode_question(&text(MAX_VALUE_BYTES + 1));
        assert_eq!(
            decode_question(kind, &over),
            Err(WireError::Invalid("text length"))
        );

        let nodes = |count: usize| FormAnswer::Nodes(Ok(vec![0; count]));
        let (kind, ok) = encode_answer(&nodes(MAX_NODES));
        assert!(decode_answer(kind, &ok).is_ok());
        let (kind, over) = encode_answer(&nodes(MAX_NODES + 1));
        assert_eq!(
            decode_answer(kind, &over),
            Err(WireError::Invalid("node count"))
        );

        // A count of a billion over no bytes reserves nothing and is refused as short.
        let mut claim = vec![0u8];
        claim.extend_from_slice(&1_000_000_000u32.to_le_bytes());
        assert!(decode_answer(FRAME_FORM_NODES, &claim).is_err());
    }

    #[test]
    fn a_kind_nobody_sends_is_refused() {
        assert_eq!(decode_question(1, &[]), Err(WireError::Kind(1)));
        assert_eq!(
            decode_answer(FRAME_FORM_RESOLVE, &[]),
            Err(WireError::Kind(8))
        );
    }
}
