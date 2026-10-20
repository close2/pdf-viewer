//! The suite's process split: a confined worker runs the program, and the broker that started it
//! is the program's whole world (ADR 2140, `doc/questions/A02`).
//!
//! # What crosses, and why it is the command line
//!
//! A [`Plan`](crate::Plan) is data, but it is not *small* data: `archive`'s carries a
//! configuration's derivations, supplies and preservations, its fonts, its profile and its tool
//! outputs, and a [`Report`](crate::Report) carries a conversion report. Writing a second encoding
//! of both for the wire would be a second statement of every verb's vocabulary. **The plan's wire
//! form is the command line that states it**: `quorra-transform`'s `plan` is a pure function of
//! the arguments and of the bytes of the files they name, so the worker is sent the arguments and
//! builds the very plan the in-process program builds, and asks for each file the moment the
//! program would have opened it. The report never crosses at all: the worker prints it, and what it
//! prints crosses as bytes.
//!
//! # The world, as a trait
//!
//! [`World`] is everything the program reaches outside its arguments: the files the command line
//! names, the documents, a password descriptor, the terminal, standard output and error, the
//! outputs, the one external program a configuration may name, and a file beside a document. The
//! in-process route hands the program a world that is this machine. The confined route hands it
//! [`Forwarded`], which asks the broker for each of those, and the broker answers each request
//! from *its* world — through [`Fences`], which is the reason the split is worth having: a worker
//! that was subverted by the document it parsed can ask, and the broker gives it only what the
//! person's command line already named.
//!
//! # Frames
//!
//! On `confined-transport`'s wire (ADR 0812): a nine-byte header, a payload, and a descriptor
//! beside a document's answer. The worker leads; the broker answers the requests that want an
//! answer and carries out the rest. Fonts a drawing thread asks for go through the transport's
//! own resource port, which takes the same lock as everything here.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::Sinks;
use crate::images::ImageFile;
use crate::pattern::Pattern;
use crate::tool::{Bounds, Delivery, Tool, ToolOutcome, ToolRequest, ToolResult};

/// The name of the worker program, **without the platform's executable suffix**.
///
/// A separate executable rather than this one re-invoked with a flag, for the reason
/// `pdf_sandbox::WORKER_PROGRAM` gives: what it links is reachable only from a `main` whose first
/// statements give away the ability to do anything but read and derive from a document.
pub const WORKER_PROGRAM: &str = "quorra-transform-worker";

/// Environment variable naming the worker program explicitly, for a binary not installed beside
/// it.
pub const WORKER_PATH_VARIABLE: &str = "QUORRA_TRANSFORM_WORKER";

/// The greeting's eight bytes, which say the program on the other end is this one.
pub const MAGIC: &[u8; 8] = b"QTRANSF1";

/// How a file the command line names is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// `std::fs::read`: a font program, an ICC profile, a configuration's text.
    Bytes,
    /// `pdf_syntax::read_file`: the room asked for first, for a payload written into a document.
    Whole,
}

/// Everything the transform program reaches beyond its arguments.
///
/// Two implementations: this machine, in `quorra-transform`, and [`Forwarded`], in the worker.
/// The program is written once against this, which is what makes the two routes one program.
pub trait World {
    /// Reads a file the command line names.
    ///
    /// # Errors
    ///
    /// The reading's own.
    fn read(&mut self, path: &Path, how: Reading) -> std::io::Result<Vec<u8>>;

    /// Opens a document the command line names, on disk where it can be.
    ///
    /// # Errors
    ///
    /// The opening's own.
    fn document(&mut self, path: &Path) -> std::io::Result<pdf_syntax::FileBytes>;

    /// One line from an open descriptor, the line ending kept, as `--password-fd` reads it.
    ///
    /// # Errors
    ///
    /// The reading's own.
    fn password(&mut self, descriptor: u32) -> std::io::Result<String>;

    /// Whether standard input is a terminal, which is whether there is anybody to ask.
    fn is_terminal(&mut self) -> bool;

    /// One line from standard input: the answer to a question just put on standard error.
    ///
    /// # Errors
    ///
    /// The reading's own.
    fn answer(&mut self) -> std::io::Result<String>;

    /// Writes to standard output.
    fn print(&mut self, text: &str);

    /// Writes to standard error.
    fn say(&mut self, text: &str);

    /// The outputs of one pass: files at the names the plan expands, or standard output.
    fn sinks(&self, to_stdout: bool) -> Box<dyn Sinks>;

    /// Runs the external program a configuration names, through `executor::execute`.
    ///
    /// # Errors
    ///
    /// The executor's refusal, in its own words.
    fn execute(&mut self, request: &ToolRequest) -> Result<ToolResult, String>;

    /// Reads the file a document's stream names beside the document (ADR 1155's rule).
    ///
    /// # Errors
    ///
    /// The reading's own.
    fn read_beside(&mut self, path: &Path) -> std::io::Result<Vec<u8>>;
}

/// The frame kinds, worker to broker.
mod ask {
    /// Bytes for standard output; no answer.
    pub(super) const OUT: u8 = 0x10;
    /// Bytes for standard error; no answer.
    pub(super) const ERR: u8 = 0x11;
    /// A named file, by [`super::Reading`] and path.
    pub(super) const READ: u8 = 0x12;
    /// A document, by path, and whether a descriptor may answer it.
    pub(super) const DOCUMENT: u8 = 0x13;
    /// A password descriptor's line.
    pub(super) const PASSWORD: u8 = 0x14;
    /// Whether standard input is a terminal.
    pub(super) const TERMINAL: u8 = 0x15;
    /// A line from standard input.
    pub(super) const ANSWER: u8 = 0x16;
    /// A tool request.
    pub(super) const EXECUTE: u8 = 0x17;
    /// A file beside the document.
    pub(super) const BESIDE: u8 = 0x18;
    /// A pass's outputs begin; no answer.
    pub(super) const PASS: u8 = 0x20;
    /// A pass's outputs end; no answer.
    pub(super) const PASS_END: u8 = 0x21;
    /// An output opened by name.
    pub(super) const SINK_OPEN: u8 = 0x22;
    /// Bytes written to an output.
    pub(super) const SINK_WRITE: u8 = 0x23;
    /// An output flushed.
    pub(super) const SINK_FLUSH: u8 = 0x24;
    /// An output closed; no answer.
    pub(super) const SINK_CLOSE: u8 = 0x25;
    /// The run is over, with its exit status; no answer.
    pub(super) const EXIT: u8 = 0x2F;
}

/// The frame kinds, broker to worker.
mod answer {
    /// The arguments, once, first.
    pub(super) const BEGIN: u8 = 0x01;
    /// Done, nothing to carry.
    pub(super) const OK: u8 = 0x80;
    /// Bytes.
    pub(super) const BYTES: u8 = 0x81;
    /// An `std::io::Error`, as [`super::encode_error`] wrote it.
    pub(super) const ERROR: u8 = 0x82;
    /// A document's length, its descriptor beside the header.
    pub(super) const DESCRIPTOR: u8 = 0x83;
    /// An output's number.
    pub(super) const SINK: u8 = 0x84;
    /// A tool's result.
    pub(super) const RESULT: u8 = 0x85;
    /// A refusal, in words.
    pub(super) const REFUSED: u8 = 0x86;
}

/// A payload being written.
#[derive(Debug, Default)]
struct Encoder(Vec<u8>);

impl Encoder {
    fn byte(mut self, value: u8) -> Self {
        self.0.push(value);
        self
    }

    fn number(mut self, value: u64) -> Self {
        self.0.extend_from_slice(&value.to_be_bytes());
        self
    }

    fn bytes(self, value: &[u8]) -> Self {
        let mut out = self.number(u64::try_from(value.len()).unwrap_or(u64::MAX));
        out.0.extend_from_slice(value);
        out
    }

    fn text(self, value: &str) -> Self {
        self.bytes(value.as_bytes())
    }

    fn path(self, value: &Path) -> Self {
        self.bytes(value.as_os_str().as_encoded_bytes())
    }

    fn texts(self, values: &[String]) -> Self {
        values.iter().fold(
            self.number(u64::try_from(values.len()).unwrap_or(u64::MAX)),
            |out, value| out.text(value),
        )
    }
}

/// A payload being read. Every read is checked, because the bytes are the other process's.
#[derive(Debug)]
struct Decoder<'a>(&'a [u8]);

impl<'a> Decoder<'a> {
    fn byte(&mut self) -> Option<u8> {
        let (first, rest) = self.0.split_first()?;
        self.0 = rest;
        Some(*first)
    }

    fn number(&mut self) -> Option<u64> {
        let (head, rest) = self.0.split_first_chunk::<8>()?;
        self.0 = rest;
        Some(u64::from_be_bytes(*head))
    }

    fn bytes(&mut self) -> Option<&'a [u8]> {
        let length = usize::try_from(self.number()?).ok()?;
        let (head, rest) = self.0.split_at_checked(length)?;
        self.0 = rest;
        Some(head)
    }

    fn text(&mut self) -> Option<String> {
        String::from_utf8(self.bytes()?.to_vec()).ok()
    }

    fn path(&mut self) -> Option<PathBuf> {
        // The bytes are what `as_encoded_bytes` wrote on this same platform, in a process built
        // from the same tree; text is all a command line on it holds, so a path that is not text
        // is refused rather than reinterpreted.
        self.text().map(PathBuf::from)
    }

    fn texts(&mut self) -> Option<Vec<String>> {
        let count = usize::try_from(self.number()?).ok()?;
        // Bounded by what is left: every text costs at least its eight length bytes.
        if count > self.0.len() / 8 {
            return None;
        }
        (0..count).map(|_| self.text()).collect()
    }

    fn finished(&self) -> bool {
        self.0.is_empty()
    }
}

/// The `ErrorKind`s a reading can come back with, as a byte each.
const KINDS: &[std::io::ErrorKind] = &[
    std::io::ErrorKind::Other,
    std::io::ErrorKind::NotFound,
    std::io::ErrorKind::PermissionDenied,
    std::io::ErrorKind::AlreadyExists,
    std::io::ErrorKind::InvalidInput,
    std::io::ErrorKind::InvalidData,
    std::io::ErrorKind::UnexpectedEof,
    std::io::ErrorKind::OutOfMemory,
    std::io::ErrorKind::IsADirectory,
    std::io::ErrorKind::Unsupported,
    std::io::ErrorKind::WriteZero,
    std::io::ErrorKind::BrokenPipe,
];

/// An error, so that the other side's `Display` of it is this side's, byte for byte: an operating
/// system's error by its number, which `from_raw_os_error` turns back into the same sentence, and
/// any other by its kind and its words.
fn encode_error(error: &std::io::Error) -> Vec<u8> {
    let kind = KINDS
        .iter()
        .position(|kind| *kind == error.kind())
        .and_then(|at| u8::try_from(at).ok())
        .unwrap_or(0);
    let encoder = Encoder::default().byte(kind);
    match error.raw_os_error() {
        Some(code) => encoder.byte(1).number(u64::from(code.cast_unsigned())),
        None => encoder.byte(0).text(&error.to_string()),
    }
    .0
}

/// The error [`encode_error`] wrote.
fn decode_error(payload: &[u8]) -> std::io::Error {
    let mut decoder = Decoder(payload);
    let kind = decoder
        .byte()
        .and_then(|at| KINDS.get(usize::from(at)))
        .copied()
        .unwrap_or(std::io::ErrorKind::Other);
    match decoder.byte() {
        Some(1) => decoder
            .number()
            .and_then(|code| u32::try_from(code).ok())
            .map_or_else(
                || std::io::Error::other("the broker sent an error this build cannot read"),
                |code| std::io::Error::from_raw_os_error(code.cast_signed()),
            ),
        _ => std::io::Error::new(
            kind,
            decoder
                .text()
                .unwrap_or_else(|| String::from("the broker sent an error this build cannot read")),
        ),
    }
}

/// A tool request, worker to broker.
fn encode_request(request: &ToolRequest) -> Vec<u8> {
    Encoder::default()
        .text(&request.id)
        .text(&request.site)
        .text(&request.tool)
        .path(&request.program)
        .texts(&request.args)
        .bytes(&request.input)
        .text(&request.expects)
        .number(request.bounds.timeout.as_secs())
        .number(u64::from(request.bounds.timeout.subsec_nanos()))
        .number(request.bounds.output_limit)
        .byte(match request.delivery {
            Delivery::Stdout => 0,
            Delivery::OutputDirectory => 1,
        })
        .text(&request.subject)
        .0
}

/// The request [`encode_request`] wrote.
fn decode_request(payload: &[u8]) -> Option<ToolRequest> {
    let mut decoder = Decoder(payload);
    let request = ToolRequest {
        id: decoder.text()?,
        site: decoder.text()?,
        tool: decoder.text()?,
        program: decoder.path()?,
        args: decoder.texts()?,
        input: Arc::from(decoder.bytes()?),
        expects: decoder.text()?,
        bounds: Bounds {
            timeout: Duration::new(decoder.number()?, u32::try_from(decoder.number()?).ok()?),
            output_limit: decoder.number()?,
        },
        delivery: match decoder.byte()? {
            0 => Delivery::Stdout,
            1 => Delivery::OutputDirectory,
            _ => return None,
        },
        subject: decoder.text()?,
    };
    decoder.finished().then_some(request)
}

/// A tool's result, broker to worker.
fn encode_result(result: &ToolResult) -> Vec<u8> {
    let encoder = Encoder::default().text(&result.id);
    let encoder = match &result.outcome {
        ToolOutcome::Produced => encoder.byte(0),
        ToolOutcome::Declined => encoder.byte(1),
        ToolOutcome::Failed(why) => encoder.byte(2).text(why),
    };
    encoder
        .bytes(&result.output)
        .text(&result.stderr)
        .path(&result.program)
        .text(&result.digest)
        .0
}

/// The result [`encode_result`] wrote.
fn decode_result(payload: &[u8]) -> Option<ToolResult> {
    let mut decoder = Decoder(payload);
    let id = decoder.text()?;
    let outcome = match decoder.byte()? {
        0 => ToolOutcome::Produced,
        1 => ToolOutcome::Declined,
        2 => ToolOutcome::Failed(decoder.text()?),
        _ => return None,
    };
    let result = ToolResult {
        id,
        outcome,
        output: Arc::from(decoder.bytes()?),
        stderr: decoder.text()?,
        program: decoder.path()?,
        digest: decoder.text()?,
    };
    decoder.finished().then_some(result)
}

/// The worker's world: every request is a frame to the broker, under the transport's one lock.
///
/// **Only after `confined_transport::link::requests_go_to_the_host` was called**, which the
/// worker does before its lockdown; until then every request is refused by the transport.
#[derive(Debug)]
pub struct Forwarded {
    /// The largest answer this process will hold, in bytes.
    most: usize,
    /// How many documents arrived as descriptors, which the confinement's `RLIMIT_NOFILE` bounds.
    descriptors: usize,
}

/// How many documents may arrive as descriptors.
///
/// `RLIMIT_NOFILE` is eight in the confinement (ADR 0812): three standard streams, the socket's
/// own end, and room for four documents. A fifth arrives as its bytes, which is the same document
/// to every reader of it — `pdf_syntax::FileBytes` is one type for both.
const DOCUMENT_DESCRIPTORS: usize = 4;

impl Forwarded {
    /// A world over the armed link, holding no answer longer than `most` bytes.
    #[must_use]
    pub fn new(most: usize) -> Self {
        Self {
            most,
            descriptors: 0,
        }
    }

    /// The program's arguments: the broker's first frame, read off the worker's standard input,
    /// and no longer than `most` bytes — a command line is the person's, but its length is still a
    /// number this process allocates from.
    ///
    /// # Errors
    ///
    /// Where the frame is not the first frame this build defines.
    pub fn arguments(
        input: &mut impl confined_transport::link::Source,
        most: usize,
    ) -> std::io::Result<Vec<String>> {
        let mut header = [0u8; confined_transport::frame::HEADER_LEN];
        let mut unclaimed = Vec::new();
        input.fill(&mut header, &mut unclaimed)?;
        let refused = || std::io::Error::other("the broker's first frame is not its arguments");
        let (kind, length) = confined_transport::frame::parse_header(header).ok_or_else(refused)?;
        if kind != answer::BEGIN || length > most {
            return Err(refused());
        }
        let mut payload = vec![0u8; length];
        input.fill(&mut payload, &mut unclaimed)?;
        // A descriptor nobody asked for is leaked rather than dropped: closing one asks `fcntl`
        // first in a debug build, and `fcntl` is off the allow-list (trap 32, ADR 0880).
        for arrived in unclaimed {
            std::mem::forget(arrived);
        }
        let mut decoder = Decoder(&payload);
        decoder
            .texts()
            .filter(|_| decoder.finished())
            .ok_or_else(refused)
    }

    /// Asks the broker, and reads its answer.
    fn ask(
        &self,
        kind: u8,
        payload: &[u8],
    ) -> std::io::Result<(
        u8,
        Vec<u8>,
        Vec<confined_transport::link::ReceivedDescriptor>,
    )> {
        confined_transport::link::exchange_with_the_host(kind, payload, self.most)
    }

    /// Asks for bytes, or the error that stood in for them.
    fn bytes(&self, kind: u8, payload: &[u8]) -> std::io::Result<Vec<u8>> {
        let (answered, bytes, descriptors) = self.ask(kind, payload)?;
        forget(descriptors);
        match answered {
            answer::BYTES => Ok(bytes),
            answer::ERROR => Err(decode_error(&bytes)),
            _ => Err(unexpected()),
        }
    }

    /// Tells the broker something that wants no answer.
    fn tell(kind: u8, payload: &[u8]) -> std::io::Result<()> {
        confined_transport::link::send_to_the_host(kind, payload)
    }
}

/// A descriptor nobody asked for, leaked for [`Forwarded::arguments`]'s reason.
fn forget(descriptors: Vec<confined_transport::link::ReceivedDescriptor>) {
    for arrived in descriptors {
        std::mem::forget(arrived);
    }
}

/// The error for an answer of a kind the request does not take.
fn unexpected() -> std::io::Error {
    std::io::Error::other("the broker answered with a frame this request does not take")
}

impl World for Forwarded {
    fn read(&mut self, path: &Path, how: Reading) -> std::io::Result<Vec<u8>> {
        let how = match how {
            Reading::Bytes => 0,
            Reading::Whole => 1,
        };
        self.bytes(ask::READ, &Encoder::default().byte(how).path(path).0)
    }

    fn document(&mut self, path: &Path) -> std::io::Result<pdf_syntax::FileBytes> {
        let may = u8::from(self.descriptors < DOCUMENT_DESCRIPTORS);
        let (answered, payload, mut descriptors) =
            self.ask(ask::DOCUMENT, &Encoder::default().byte(may).path(path).0)?;
        match answered {
            answer::DESCRIPTOR => {
                let length = Decoder(&payload).number().ok_or_else(unexpected)?;
                if descriptors.is_empty() {
                    return Err(std::io::Error::other(
                        "the broker said the document was on disk and sent no descriptor with it",
                    ));
                }
                let descriptor = descriptors.remove(0);
                forget(descriptors);
                self.descriptors = self.descriptors.saturating_add(1);
                pdf_syntax::FileBytes::from_handle(std::fs::File::from(descriptor), length)
            }
            answer::BYTES => {
                forget(descriptors);
                Ok(pdf_syntax::FileBytes::from(payload))
            }
            answer::ERROR => {
                forget(descriptors);
                Err(decode_error(&payload))
            }
            _ => {
                forget(descriptors);
                Err(unexpected())
            }
        }
    }

    fn password(&mut self, descriptor: u32) -> std::io::Result<String> {
        let bytes = self.bytes(
            ask::PASSWORD,
            &Encoder::default().number(u64::from(descriptor)).0,
        )?;
        String::from_utf8(bytes).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "stream did not contain valid UTF-8",
            )
        })
    }

    fn is_terminal(&mut self) -> bool {
        // A broker that cannot be asked has nobody to ask either.
        self.bytes(ask::TERMINAL, &[])
            .is_ok_and(|answered| answered == [1])
    }

    fn answer(&mut self) -> std::io::Result<String> {
        let bytes = self.bytes(ask::ANSWER, &[])?;
        String::from_utf8(bytes).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "stream did not contain valid UTF-8",
            )
        })
    }

    fn print(&mut self, text: &str) {
        // **The one failure here with nowhere to go**: the broker that would print it is the
        // party that has gone, and the next request the program makes fails by name. The worker
        // still exits, because the broker's end of the socket closing ends every later exchange.
        let _ = Self::tell(ask::OUT, text.as_bytes());
    }

    fn say(&mut self, text: &str) {
        // As [`Forwarded::print`]: the broker is where this would be reported, and it is gone.
        let _ = Self::tell(ask::ERR, text.as_bytes());
    }

    fn sinks(&self, to_stdout: bool) -> Box<dyn Sinks> {
        Box::new(ForwardedSinks::begin(self.most, to_stdout))
    }

    fn execute(&mut self, request: &ToolRequest) -> Result<ToolResult, String> {
        let (answered, payload, descriptors) = self
            .ask(ask::EXECUTE, &encode_request(request))
            .map_err(|error| error.to_string())?;
        forget(descriptors);
        match answered {
            answer::RESULT => decode_result(&payload).ok_or_else(|| {
                String::from("the broker sent a tool result this build cannot read")
            }),
            answer::REFUSED => Err(Decoder(&payload).text().unwrap_or_else(|| {
                String::from("the broker refused the tool, in words this build cannot read")
            })),
            _ => Err(unexpected().to_string()),
        }
    }

    fn read_beside(&mut self, path: &Path) -> std::io::Result<Vec<u8>> {
        self.bytes(ask::BESIDE, &Encoder::default().path(path).0)
    }
}

/// One pass's outputs, opened by the broker.
#[derive(Debug)]
struct ForwardedSinks {
    /// The largest answer this process will hold.
    most: usize,
}

impl ForwardedSinks {
    /// Tells the broker a pass begins, which is when it makes the pass's own sinks.
    fn begin(most: usize, to_stdout: bool) -> Self {
        // A pass the broker was not told of is refused at its first output, by name.
        let _ = Forwarded::tell(ask::PASS, &[u8::from(to_stdout)]);
        Self { most }
    }
}

impl Drop for ForwardedSinks {
    fn drop(&mut self) {
        // Every writer borrowed these sinks, so every writer has been dropped — closed — first.
        let _ = Forwarded::tell(ask::PASS_END, &[]);
    }
}

/// How much one output gathers before it crosses: a frame per write would be a round trip per
/// PNG chunk, and the bytes are the same however they are cut.
const CROSSING: usize = 64 << 10;

impl Sinks for ForwardedSinks {
    fn open(&self, name: &str) -> std::io::Result<Box<dyn Write + Send + '_>> {
        let (answered, payload, descriptors) = confined_transport::link::exchange_with_the_host(
            ask::SINK_OPEN,
            &Encoder::default().text(name).0,
            self.most,
        )?;
        forget(descriptors);
        match answered {
            answer::SINK => {
                let id = Decoder(&payload).number().ok_or_else(unexpected)?;
                Ok(Box::new(std::io::BufWriter::with_capacity(
                    CROSSING,
                    ForwardedWriter {
                        id,
                        most: self.most,
                    },
                )))
            }
            answer::ERROR => Err(decode_error(&payload)),
            _ => Err(unexpected()),
        }
    }
}

/// One output, written by the broker.
#[derive(Debug)]
struct ForwardedWriter {
    /// The broker's number for it.
    id: u64,
    /// The largest answer this process will hold.
    most: usize,
}

impl ForwardedWriter {
    /// One request about this output, answered by nothing but an error.
    fn exchange(&self, kind: u8, payload: &[u8]) -> std::io::Result<()> {
        let (answered, bytes, descriptors) =
            confined_transport::link::exchange_with_the_host(kind, payload, self.most)?;
        forget(descriptors);
        match answered {
            answer::OK => Ok(()),
            answer::ERROR => Err(decode_error(&bytes)),
            _ => Err(unexpected()),
        }
    }
}

impl Write for ForwardedWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.exchange(
            ask::SINK_WRITE,
            &Encoder::default().number(self.id).bytes(data).0,
        )?;
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.exchange(ask::SINK_FLUSH, &Encoder::default().number(self.id).0)
    }
}

impl Drop for ForwardedWriter {
    fn drop(&mut self) {
        // As a file's own `drop`: a close has nobody to report to. The broker drops its writer,
        // which is what the in-process route's `BufWriter` does at the same moment.
        let _ = Forwarded::tell(ask::SINK_CLOSE, &Encoder::default().number(self.id).0);
    }
}

/// What a broker gives a worker, decided from the person's command line before the worker runs.
///
/// **Every request a worker makes is answered through one of these, and each is a fence**: a
/// worker the document subverted can still ask, and what it is given is only what the person
/// already named. The binary builds them from its own reading of the arguments, the same grammar
/// the worker reads them by.
#[derive(Debug, Default)]
pub struct Fences {
    /// The files the command line names for reading: the documents, `--attach`, `--font`,
    /// `--config` and `--output-intent-profile`.
    pub named: Vec<PathBuf>,
    /// The descriptors the command line names: `--password-fd` and the two encryption ones.
    pub descriptors: Vec<u32>,
    /// What `-o` admits as an output name.
    pub outputs: Vec<Pattern>,
    /// Whether `-o -` sends the one output to standard output.
    pub to_stdout: bool,
    /// The directory of the document a `--resolve-external-data` run reads beside, where it
    /// asked to.
    pub beside: Option<PathBuf>,
    /// Each remedy the configuration names that runs a program — `derive`, and the `preserve`
    /// that fetches what a stream keeps outside the file: its site and its tool.
    pub tools: Vec<(String, Tool)>,
    /// The target a configuration's `{target}` is filled with.
    pub target: String,
}

impl Fences {
    /// Whether `path` is a file the command line names.
    fn names(&self, path: &Path) -> bool {
        self.named.iter().any(|named| named == path)
    }

    /// Whether `name` is an output name `-o` admits, as the verbs derive one from it.
    ///
    /// **Three shapes, and `images` writes the two beyond the first**: an expansion of the pattern;
    /// under `--native`, an expansion with the codec's extension appended, because the caller cannot
    /// know which file form an image will take; and a mask beside an image, `images::mask_name`'s
    /// image name with its extension replaced by the mask's.
    fn admits(&self, name: &str) -> bool {
        let expansion =
            |candidate: &str| self.outputs.iter().any(|pattern| pattern.admits(candidate));
        if self.to_stdout || expansion(name) {
            return true;
        }
        let extensions = every_image_file().map(ImageFile::extension);
        extensions.iter().any(|extension| {
            let Some(stem) = name.strip_suffix(extension) else {
                return false;
            };
            let Some(stem) = stem.strip_suffix('.') else {
                return false;
            };
            expansion(stem)
                || (extension.starts_with("mask.")
                    && self.outputs.iter().any(|pattern| pattern.admits_stem(stem)))
        })
    }

    /// Whether `path` is one plain file name in the document's own directory — ADR 1155's rule,
    /// checked again on this side of the boundary.
    fn beside(&self, path: &Path) -> bool {
        let Some(directory) = &self.beside else {
            return false;
        };
        let Some(name) = path.file_name() else {
            return false;
        };
        path.parent() == Some(directory.as_path()) && directory.join(name) == path
    }

    /// Whether `request` is, field for field, what the configuration's tool at its site asks for.
    ///
    /// The arguments are filled as `archive::remedies::request_for` fills them, `{site}` and
    /// `{target}`; nothing else in a request may differ from the operator's own declaration, so a
    /// worker can run only the programs the operator named, with the arguments the operator wrote.
    fn admits_tool(&self, request: &ToolRequest) -> bool {
        self.tools.iter().any(|(site, tool)| {
            *site == request.site
                && tool.name == request.tool
                && tool.program == request.program
                && tool.expects == request.expects
                && tool.bounds == request.bounds
                && tool.delivery() == request.delivery
                && tool
                    .args
                    .iter()
                    .map(|argument| {
                        argument
                            .replace("{site}", site)
                            .replace("{target}", &self.target)
                    })
                    .eq(request.args.iter().cloned())
        })
    }
}

/// Every file form an image is written in, so that a form added to [`ImageFile`] is a compile error
/// here until the fence admits its extension.
fn every_image_file() -> [ImageFile; 8] {
    let every = [
        ImageFile::Png,
        ImageFile::Ppm,
        ImageFile::Pgm,
        ImageFile::Jpeg,
        ImageFile::JpegEncoded,
        ImageFile::Jp2,
        ImageFile::Mask,
        ImageFile::MaskPgm,
    ];
    for file in every {
        match file {
            ImageFile::Png
            | ImageFile::Ppm
            | ImageFile::Pgm
            | ImageFile::Jpeg
            | ImageFile::JpegEncoded
            | ImageFile::Jp2
            | ImageFile::Mask
            | ImageFile::MaskPgm => {}
        }
    }
    every
}

/// Why a broker stopped serving before its worker said how the run ended.
#[derive(Debug, thiserror::Error)]
pub enum Stopped {
    /// The transport failed: the worker died, or the socket did.
    #[error(transparent)]
    Transport(#[from] confined_transport::TransportError),
    /// The worker sent a frame this build does not define.
    #[error("the confined worker sent a frame of kind {0:#04x}, which this build does not define")]
    Unrecognised(u8),
}

/// Serves a worker the program `arguments` describes, from `world`, through `fences`, until it
/// says how the run ended; and answers with its exit status.
///
/// # Errors
///
/// [`Stopped`] where the worker ended without saying.
pub fn serve(
    host: &mut confined_transport::Host,
    world: &mut dyn World,
    fences: &Fences,
    arguments: &[String],
) -> Result<u8, Stopped> {
    host.send(answer::BEGIN, &Encoder::default().texts(arguments).0, None)?;
    let mut broker = Broker {
        host,
        world,
        fences,
    };
    loop {
        let (kind, payload) = broker.host.receive()?;
        match kind {
            ask::EXIT => return Ok(payload.first().copied().unwrap_or(2)),
            ask::PASS => broker.pass()?,
            other => broker.common(other, &payload)?,
        }
    }
}

/// The broker's half of one run.
struct Broker<'a> {
    /// The worker.
    host: &'a mut confined_transport::Host,
    /// This machine.
    world: &'a mut dyn World,
    /// What the command line named.
    fences: &'a Fences,
}

impl Broker<'_> {
    /// One pass: the outputs it opens, until it ends.
    fn pass(&mut self) -> Result<(), Stopped> {
        // The broker's own reading of `-o -`, not the worker's word for it.
        let sinks = self.world.sinks(self.fences.to_stdout);
        let mut writers: Vec<Option<Box<dyn Write + Send + '_>>> = Vec::new();
        loop {
            let (kind, payload) = self.host.receive()?;
            match kind {
                ask::PASS_END => return Ok(()),
                ask::SINK_OPEN => {
                    let answer = match Decoder(&payload).text() {
                        Some(name) if self.fences.admits(&name) => {
                            sinks.open(&name).map(|writer| {
                                writers.push(Some(writer));
                                writers.len().saturating_sub(1)
                            })
                        }
                        Some(name) => Err(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied,
                            format!(
                                "the confined worker asked to write {name:?}, which -o does not \
                                 name"
                            ),
                        )),
                        None => Err(unexpected()),
                    };
                    match answer {
                        Ok(id) => self.host.send(
                            answer::SINK,
                            &Encoder::default()
                                .number(u64::try_from(id).unwrap_or(u64::MAX))
                                .0,
                            None,
                        )?,
                        Err(error) => self.host.send(answer::ERROR, &encode_error(&error), None)?,
                    }
                }
                ask::SINK_WRITE | ask::SINK_FLUSH | ask::SINK_CLOSE => {
                    let mut decoder = Decoder(&payload);
                    let writer = decoder
                        .number()
                        .and_then(|id| usize::try_from(id).ok())
                        .and_then(|id| writers.get_mut(id));
                    let Some(writer) = writer else {
                        return Err(Stopped::Unrecognised(kind));
                    };
                    if kind == ask::SINK_CLOSE {
                        // Dropped, as the in-process route drops its writer: a `BufWriter` flushes
                        // on the way out and has nobody to tell if that fails.
                        writer.take();
                        continue;
                    }
                    let done = match (writer.as_mut(), kind) {
                        (None, _) => Err(std::io::Error::other("this output was closed")),
                        (Some(writer), ask::SINK_WRITE) => match decoder.bytes() {
                            Some(data) => writer.write_all(data),
                            None => Err(unexpected()),
                        },
                        (Some(writer), _) => writer.flush(),
                    };
                    match done {
                        Ok(()) => self.host.send(answer::OK, &[], None)?,
                        Err(error) => self.host.send(answer::ERROR, &encode_error(&error), None)?,
                    }
                }
                other => self.common(other, &payload)?,
            }
        }
    }

    /// Every request that is not about one pass's outputs.
    fn common(&mut self, kind: u8, payload: &[u8]) -> Result<(), Stopped> {
        let mut decoder = Decoder(payload);
        let refused = |path: &Path, what: &str| {
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!(
                    "the confined worker asked for {}, which is not {what}",
                    path.display()
                ),
            )
        };
        let reply = match kind {
            ask::OUT => {
                self.world.print(&String::from_utf8_lossy(payload));
                return Ok(());
            }
            ask::ERR => {
                self.world.say(&String::from_utf8_lossy(payload));
                return Ok(());
            }
            ask::READ => {
                let how = match decoder.byte() {
                    Some(0) => Some(Reading::Bytes),
                    Some(1) => Some(Reading::Whole),
                    _ => None,
                };
                match (how, decoder.path()) {
                    (Some(how), Some(path)) if self.fences.names(&path) => {
                        self.world.read(&path, how)
                    }
                    (Some(_), Some(path)) => Err(refused(&path, "a file the command line names")),
                    _ => Err(unexpected()),
                }
            }
            ask::DOCUMENT => {
                let may = decoder.byte() == Some(1);
                return match decoder.path() {
                    Some(path) if self.fences.names(&path) => self.document(&path, may),
                    Some(path) => self.error(&refused(&path, "a document the command line names")),
                    None => self.error(&unexpected()),
                };
            }
            ask::PASSWORD => match decoder.number().and_then(|fd| u32::try_from(fd).ok()) {
                Some(fd) if self.fences.descriptors.contains(&fd) => {
                    self.world.password(fd).map(String::into_bytes)
                }
                Some(fd) => Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!(
                        "the confined worker asked for descriptor {fd}, which the command line \
                         does not name"
                    ),
                )),
                None => Err(unexpected()),
            },
            ask::TERMINAL => Ok(vec![u8::from(self.world.is_terminal())]),
            ask::ANSWER => self.world.answer().map(String::into_bytes),
            ask::BESIDE => match decoder.path() {
                Some(path) if self.fences.beside(&path) => self.world.read_beside(&path),
                Some(path) => Err(refused(
                    &path,
                    "a plain file name beside the document, read under --resolve-external-data",
                )),
                None => Err(unexpected()),
            },
            ask::EXECUTE => return self.execute(payload),
            other => return Err(Stopped::Unrecognised(other)),
        };
        match reply {
            Ok(bytes) => self.host.send(answer::BYTES, &bytes, None)?,
            Err(error) => self.error(&error)?,
        }
        Ok(())
    }

    /// A tool run, where the configuration declares it (ADR 2141).
    fn execute(&mut self, payload: &[u8]) -> Result<(), Stopped> {
        let (kind, reply) = match decode_request(payload) {
            Some(request) if self.fences.admits_tool(&request) => {
                match self.world.execute(&request) {
                    Ok(result) => (answer::RESULT, encode_result(&result)),
                    Err(why) => (answer::REFUSED, Encoder::default().text(&why).0),
                }
            }
            Some(request) => (
                answer::REFUSED,
                Encoder::default()
                    .text(&format!(
                        "the confined worker asked to run {} as the tool {:?} at {:?}, \
                             which is not what the configuration declares for that site",
                        request.program.display(),
                        request.tool,
                        request.site
                    ))
                    .0,
            ),
            None => (
                answer::REFUSED,
                Encoder::default()
                    .text("the confined worker sent a tool request this build cannot read")
                    .0,
            ),
        };
        self.host.send(kind, &reply, None)?;
        Ok(())
    }

    /// A document: its descriptor where it is on disk and the worker has room for one, else its
    /// bytes.
    fn document(&mut self, path: &Path, may: bool) -> Result<(), Stopped> {
        let bytes = match self.world.document(path) {
            Ok(bytes) => bytes,
            Err(error) => return self.error(&error),
        };
        if may && let Some(descriptor) = bytes.descriptor() {
            let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
            self.host.send(
                answer::DESCRIPTOR,
                &Encoder::default().number(length).0,
                Some(descriptor),
            )?;
            return Ok(());
        }
        let whole = bytes.read(0..bytes.len());
        self.host.send(answer::BYTES, &whole, None)?;
        Ok(())
    }

    /// An error, as the answer to the request just read.
    fn error(&mut self, error: &std::io::Error) -> Result<(), Stopped> {
        self.host.send(answer::ERROR, &encode_error(error), None)?;
        Ok(())
    }
}

/// Tells the broker how the run ended, which is the worker's last frame.
///
/// # Errors
///
/// Where the broker is gone.
pub fn exit(code: u8) -> std::io::Result<()> {
    Forwarded::tell(ask::EXIT, &[code])
}

#[cfg(test)]
mod tests {
    use super::{Fences, decode_error, decode_request, decode_result, encode_error};
    use super::{encode_request, encode_result};
    use crate::tool::{Bounds, Delivery, Tool, ToolOutcome, ToolRequest, ToolResult};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;

    /// An error crosses as the sentence the other side would have printed (ADR 2140).
    #[test]
    fn an_error_crosses_as_its_own_sentence() {
        for error in [
            std::io::Error::from_raw_os_error(2),
            std::io::Error::new(std::io::ErrorKind::OutOfMemory, "no room for 9 bytes"),
            std::io::Error::from(std::io::ErrorKind::IsADirectory),
        ] {
            let crossed = decode_error(&encode_error(&error));
            assert_eq!(crossed.to_string(), error.to_string());
            assert_eq!(crossed.kind(), error.kind());
        }
    }

    fn tool() -> Tool {
        Tool {
            name: String::from("convert"),
            program: PathBuf::from("/usr/bin/convert"),
            args: vec![String::from("--site={site}"), String::from("--to={target}")],
            expects: String::from("application/pdf"),
            bounds: Bounds {
                timeout: Duration::from_millis(1500),
                output_limit: 1 << 20,
            },
        }
    }

    fn request(program: &str) -> ToolRequest {
        ToolRequest {
            id: String::from("embedded-file/12/0"),
            site: String::from("embedded-file"),
            tool: String::from("convert"),
            program: PathBuf::from(program),
            args: vec![
                String::from("--site=embedded-file"),
                String::from("--to=3b"),
            ],
            input: Arc::from(&b"%PDF-1.7"[..]),
            expects: String::from("application/pdf"),
            bounds: tool().bounds,
            delivery: Delivery::Stdout,
            subject: String::from("a.xml"),
        }
    }

    /// A request and a result cross whole, and a worker can run only what the operator declared.
    #[test]
    fn a_tool_crosses_whole_and_only_the_declared_one_is_admitted() {
        let asked = request("/usr/bin/convert");
        assert_eq!(decode_request(&encode_request(&asked)), Some(asked.clone()));
        let result = ToolResult {
            id: asked.id.clone(),
            outcome: ToolOutcome::Failed(String::from("exit 3")),
            output: Arc::from(&b"out"[..]),
            stderr: String::from("warning"),
            program: asked.program.clone(),
            digest: String::from("ab"),
        };
        assert_eq!(decode_result(&encode_result(&result)), Some(result));

        let fences = Fences {
            tools: vec![(String::from("embedded-file"), tool())],
            target: String::from("3b"),
            ..Fences::default()
        };
        assert!(fences.admits_tool(&asked));
        assert!(!fences.admits_tool(&request("/bin/sh")));
        let mut other_site = asked.clone();
        other_site.site = String::from("elsewhere");
        assert!(!fences.admits_tool(&other_site));
        let mut other_args = asked;
        other_args.args.push(String::from("-rf"));
        assert!(!fences.admits_tool(&other_args));
    }

    /// An output is a name `-o` expands to, or one `images` derives from such a name, and nothing
    /// else.
    #[test]
    fn an_output_is_a_name_the_pattern_or_the_image_verb_derives() {
        let fences = Fences {
            outputs: vec!["out/img-%d.png".parse().expect("a pattern")],
            ..Fences::default()
        };
        for admitted in ["out/img-3.png", "out/img-3.png.jpg", "out/img-3.mask.png"] {
            assert!(fences.admits(admitted), "{admitted}");
        }
        for refused in [
            "out/img-3.sh",
            "img-3.png",
            "out/img-3.mask.sh",
            "out/../img-3.png",
        ] {
            assert!(!fences.admits(refused), "{refused}");
        }
        let native = Fences {
            outputs: vec!["img-%d".parse().expect("a pattern")],
            ..Fences::default()
        };
        for admitted in ["img-1", "img-1.jp2", "img-1.mask.pgm"] {
            assert!(native.admits(admitted), "{admitted}");
        }
        assert!(!native.admits("img-1.exe"));
    }

    /// A file beside the document is one name in its directory and nothing further.
    #[test]
    fn beside_the_document_is_one_name_in_its_directory() {
        let fences = Fences {
            beside: Some(PathBuf::from("docs")),
            named: vec![PathBuf::from("docs/in.pdf")],
            ..Fences::default()
        };
        assert!(fences.beside(Path::new("docs/data.bin")));
        assert!(!fences.beside(Path::new("docs/sub/data.bin")));
        assert!(!fences.beside(Path::new("/etc/passwd")));
        assert!(!fences.beside(Path::new("docs/../secret")));
        assert!(fences.names(Path::new("docs/in.pdf")));
        assert!(!fences.names(Path::new("docs/other.pdf")));
    }
}
