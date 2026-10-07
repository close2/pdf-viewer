//! What crosses to a script worker and back, as bytes (ADR 1609).
//!
//! `confined-transport` carries a kind byte and a length and knows nothing of either; this module is
//! what the kind means and what the payload holds. **A run carries the one script it needs, once**:
//! the first run of a script brings its text and the index the host gave it, and every later run of
//! the same script names the index alone — so a keystroke script fired per key crosses once per
//! worker rather than once per key. The request beside it is `pdf_script::wire`'s own encoding,
//! written with its script empty, so the event's fields cross exactly as the engine reads them and
//! this module never restates them.
//!
//! Every integer is little-endian, a string is a `u32` byte length and that many bytes of UTF-8,
//! and a run begins with a version byte, as `pdf_script::wire` does, so that a host and a worker
//! built from different trees refuse each other rather than misread each other. A length is read
//! only against the bytes that remain, so a hostile one asks for nothing.

use pdf_script::wire::{
    self as script_wire, decode_outcome, decode_question, decode_request, encode_outcome,
    encode_question,
};
use pdf_script::{Outcome, Question, Request};

/// The eight bytes a script worker's greeting begins with.
///
/// Different from every other worker's, so that a host started on the wrong program refuses it at
/// its first nine bytes.
pub const MAGIC: &[u8; 8] = b"PDFSCW01";

/// The kind of a run: host to worker.
pub const FRAME_RUN: u8 = 1;
/// The kind of an outcome: worker to host, `pdf_script::wire`'s encoding as the payload.
pub const FRAME_OUTCOME: u8 = 2;
/// The kind of a refusal: worker to host, one sentence of UTF-8 as the payload.
pub const FRAME_REFUSED: u8 = 3;
/// The kind of a question: worker to host, `pdf_script::wire`'s encoding of the question a script
/// put as the payload. The script is held in the worker until a [`FRAME_ANSWER`] comes, and the
/// worker's reply to that is the run's outcome (ADR 1627).
pub const FRAME_QUESTION: u8 = 4;
/// The kind of an answer: host to worker, `pdf_script::wire`'s encoding of the answer.
pub const FRAME_ANSWER: u8 = 5;

/// The first byte of a run.
pub const VERSION: u8 = 1;

/// Longest script, in bytes, a run carries.
///
/// `pdf_model::view`'s own bound on a script it hands a runner, restated here because the worker
/// is the side that must not believe the host's bound held: a megabyte is not a field script.
pub const MAX_SCRIPT_BYTES: usize = 1 << 20;

/// Largest run a worker reads, in bytes.
///
/// One script at [`MAX_SCRIPT_BYTES`] and the event's strings beside it: a field's value, the
/// change a keystroke makes and the field's name, each a line of text. Three mebibytes is the
/// script and two more for those, and it is what [`crate::worker`] refuses past without reading
/// it into memory (ADR 1609).
pub const MAX_RUN_BYTES: usize = 3 << 20;

/// Longest refusal sentence either side keeps, in bytes.
pub const MAX_SENTENCE_BYTES: usize = 4096;

/// One run: the script the worker does not hold yet, if any, which script to run, and the request.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    /// A script crossing for the first time: its index and its text.
    pub new_script: Option<(u32, String)>,
    /// The index of the script to run, which [`Self::new_script`] or an earlier run gave it.
    pub script: u32,
    /// The trigger, the field and the event, with an empty script: the worker supplies the text.
    pub request: Request,
}

/// What a worker answers a run, or an answer, with.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    /// The run's outcome.
    Outcome(Outcome),
    /// The run was not made, and the sentence saying why.
    Refused(String),
    /// The run's script put a question and is held until it is answered.
    Asked(Question),
}

/// Why bytes are not a run or a reply this module wrote.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WireError {
    /// The bytes end inside a field.
    #[error("the message ends inside a field")]
    Truncated,
    /// The version byte is not [`VERSION`].
    #[error("the message is version {0}, and this reads version {VERSION}")]
    Version(u8),
    /// A tag has a value no encoding uses.
    #[error("the message's {0} has a value no encoding uses")]
    Invalid(&'static str),
    /// A string is not UTF-8.
    #[error("a string in the message is not UTF-8")]
    Utf8,
    /// A script is longer than [`MAX_SCRIPT_BYTES`].
    #[error("a script of {0} bytes is past the {MAX_SCRIPT_BYTES} a run carries")]
    ScriptTooLong(usize),
    /// The request carries a script of its own, which a run never does.
    #[error("the request carries its own script, and a run names its script by index")]
    ScriptInRequest,
    /// Bytes remain after the message.
    #[error("{0} bytes follow the message")]
    Trailing(usize),
    /// The kind is not one a worker answers with.
    #[error("a frame of kind {0} is not a worker's answer")]
    Kind(u8),
    /// The request or the outcome inside is not `pdf_script::wire`'s.
    #[error("{0}")]
    Script(#[from] script_wire::WireError),
}

/// Encodes a run. The request's script is written empty whatever it holds.
#[must_use]
pub fn encode_run(run: &Run) -> Vec<u8> {
    let mut out = vec![VERSION];
    match &run.new_script {
        None => out.push(0),
        Some((index, text)) => {
            out.push(1);
            out.extend_from_slice(&index.to_le_bytes());
            put_str(&mut out, text);
        }
    }
    out.extend_from_slice(&run.script.to_le_bytes());
    let request = Request {
        script: String::new(),
        ..run.request.clone()
    };
    let encoded = script_wire::encode_request(&request);
    put_len(&mut out, encoded.len());
    out.extend_from_slice(&encoded);
    out
}

/// Decodes a run.
///
/// # Errors
///
/// [`WireError`] for bytes that are not exactly one run of this version, a script past
/// [`MAX_SCRIPT_BYTES`], or a request that carries a script of its own.
pub fn decode_run(bytes: &[u8]) -> Result<Run, WireError> {
    let mut reader = Reader { rest: bytes };
    match reader.u8()? {
        VERSION => {}
        other => return Err(WireError::Version(other)),
    }
    let new_script = match reader.u8()? {
        0 => None,
        1 => {
            let index = reader.u32()?;
            let text = reader.string()?;
            if text.len() > MAX_SCRIPT_BYTES {
                return Err(WireError::ScriptTooLong(text.len()));
            }
            Some((index, text))
        }
        _ => return Err(WireError::Invalid("new script")),
    };
    let script = reader.u32()?;
    let length = reader.length()?;
    let request = decode_request(reader.take(length)?)?;
    if !request.script.is_empty() {
        return Err(WireError::ScriptInRequest);
    }
    if !reader.rest.is_empty() {
        return Err(WireError::Trailing(reader.rest.len()));
    }
    Ok(Run {
        new_script,
        script,
        request,
    })
}

/// Encodes a reply as the kind it is framed under and its payload.
#[must_use]
pub fn encode_reply(reply: &Reply) -> (u8, Vec<u8>) {
    match reply {
        Reply::Outcome(outcome) => (FRAME_OUTCOME, encode_outcome(outcome)),
        Reply::Refused(sentence) => (
            FRAME_REFUSED,
            cut(sentence, MAX_SENTENCE_BYTES).as_bytes().to_vec(),
        ),
        Reply::Asked(question) => (FRAME_QUESTION, encode_question(question)),
    }
}

/// Decodes a reply from the kind it was framed under and its payload.
///
/// # Errors
///
/// [`WireError`] for a kind no worker answers with, an outcome `pdf_script::wire` does not read, or
/// a sentence that is not UTF-8 or is past [`MAX_SENTENCE_BYTES`].
pub fn decode_reply(kind: u8, bytes: &[u8]) -> Result<Reply, WireError> {
    match kind {
        FRAME_OUTCOME => Ok(Reply::Outcome(decode_outcome(bytes)?)),
        FRAME_REFUSED => {
            if bytes.len() > MAX_SENTENCE_BYTES {
                return Err(WireError::Trailing(
                    bytes.len().saturating_sub(MAX_SENTENCE_BYTES),
                ));
            }
            let sentence = std::str::from_utf8(bytes).map_err(|_| WireError::Utf8)?;
            Ok(Reply::Refused(sentence.to_owned()))
        }
        FRAME_QUESTION => Ok(Reply::Asked(decode_question(bytes)?)),
        other => Err(WireError::Kind(other)),
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

/// Writes a count, which no run this crate builds takes past `u32`.
fn put_len(out: &mut Vec<u8>, length: usize) {
    out.extend_from_slice(&u32::try_from(length).unwrap_or(u32::MAX).to_le_bytes());
}

/// Writes a string, cut at a character boundary rather than written with a length that lies.
fn put_str(out: &mut Vec<u8>, text: &str) {
    let kept = cut(text, u32::MAX as usize);
    put_len(out, kept.len());
    out.extend_from_slice(kept.as_bytes());
}

/// Reads a run from the front.
struct Reader<'a> {
    /// What has not been read.
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    /// The next `count` bytes.
    fn take(&mut self, count: usize) -> Result<&'a [u8], WireError> {
        if self.rest.len() < count {
            return Err(WireError::Truncated);
        }
        let (taken, rest) = self.rest.split_at(count);
        self.rest = rest;
        Ok(taken)
    }

    /// One byte.
    fn u8(&mut self) -> Result<u8, WireError> {
        self.take(1)?.first().copied().ok_or(WireError::Truncated)
    }

    /// A `u32`.
    fn u32(&mut self) -> Result<u32, WireError> {
        let bytes: [u8; 4] = self.take(4)?.try_into().map_err(|_| WireError::Truncated)?;
        Ok(u32::from_le_bytes(bytes))
    }

    /// A length, as a count of bytes that must still be there.
    fn length(&mut self) -> Result<usize, WireError> {
        usize::try_from(self.u32()?).map_err(|_| WireError::Truncated)
    }

    /// A string.
    fn string(&mut self) -> Result<String, WireError> {
        let length = self.length()?;
        let bytes = self.take(length)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| WireError::Utf8)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use pdf_model::aform::Trigger;
    use pdf_model::view::{ScriptEvent, ScriptSite};
    use pdf_script::{Ending, Outcome, Request};

    use super::{
        FRAME_OUTCOME, FRAME_REFUSED, Reply, Run, VERSION, WireError, decode_reply, decode_run,
        encode_reply, encode_run,
    };

    /// A format request on `Total`, with an empty script, as a run carries one.
    pub(crate) fn request() -> Request {
        let event = ScriptEvent {
            site: ScriptSite::Field(Trigger::Format),
            field: "Total",
            label: "",
            script: "",
            value: "12",
            change: "",
            selection: (0, 2),
            will_commit: false,
            source: "",
            fields: &[],
            page: 0,
            pages: 1,
            commit_key: None,
            field_full: false,
            change_ex: "",
            dirty: false,
            document: None,
        };
        Request::of(&event, 1_704_465_015_000, 3600)
    }

    fn run() -> Run {
        Run {
            new_script: Some((7, "event.value = 'é';".to_owned())),
            script: 7,
            request: request(),
        }
    }

    #[test]
    fn a_run_and_both_replies_come_back_as_they_went() {
        assert_eq!(decode_run(&encode_run(&run())), Ok(run()));
        let named = Run {
            new_script: None,
            ..run()
        };
        assert_eq!(decode_run(&encode_run(&named)), Ok(named));
        for reply in [
            Reply::Outcome(Outcome::unchanged(Ending::Finished)),
            Reply::Refused("not held".to_owned()),
        ] {
            let (kind, bytes) = encode_reply(&reply);
            assert_eq!(decode_reply(kind, &bytes), Ok(reply));
        }
    }

    /// The script crosses beside the request, never inside it: an encoder handed a request that
    /// holds one writes it empty, and a decoder handed one that holds one refuses it.
    #[test]
    fn a_request_never_carries_its_script() {
        let mut holding = run();
        holding.request.script = "event.rc = false;".to_owned();
        let decoded = decode_run(&encode_run(&holding)).expect("decodes");
        assert!(decoded.request.script.is_empty());

        let request = pdf_script::wire::encode_request(&holding.request);
        let mut bytes = vec![VERSION, 0, 0, 0, 0, 0];
        bytes.extend_from_slice(&u32::try_from(request.len()).expect("small").to_le_bytes());
        bytes.extend_from_slice(&request);
        assert_eq!(decode_run(&bytes), Err(WireError::ScriptInRequest));
    }

    #[test]
    fn every_cut_of_a_run_is_refused_and_nothing_follows_one() {
        let bytes = encode_run(&run());
        for end in 0..bytes.len() {
            assert!(decode_run(bytes.get(..end).expect("a prefix")).is_err());
        }
        let mut longer = bytes;
        longer.push(0);
        assert_eq!(decode_run(&longer), Err(WireError::Trailing(1)));
    }

    #[test]
    fn a_kind_no_worker_answers_with_is_refused() {
        assert_eq!(decode_reply(1, &[]), Err(WireError::Kind(1)));
        assert_eq!(decode_reply(FRAME_REFUSED, &[0xFF]), Err(WireError::Utf8));
        assert!(decode_reply(FRAME_OUTCOME, &[]).is_err());
    }
}
