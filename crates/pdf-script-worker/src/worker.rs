//! The confined side: confine, greet, then answer each run until the host closes its end.
//!
//! **Lockdown comes first**, before a byte of a script is read, for `viewer_confined::worker`'s
//! reason: a worker that read its first script and then confined itself would have run hostile
//! input unconfined, and that would look exactly like working code.
//!
//! **Nothing is asked of the machine before it**, which is what makes this worker's profile the
//! narrowest of the three: no thread pool is sized, no address space is read from `/proc` and no
//! font port is armed. The largest message it reads is a constant ([`MAX_RUN_BYTES`]) rather than
//! a budget derived from what the process already occupies, because its ceiling was itself sized
//! against that constant (ADR 1609).
//!
//! **Its frames are read with `read`, never `recvmsg`.** A descriptor sent beside a frame on a
//! socket is delivered only to a `recvmsg` that asks for it; to `read` the kernel discards it and
//! closes its copy, so a host — or anything that has taken one over — cannot hand this process a
//! file. `recvmsg` is off its allow-list, and that is the reason it can be.

use std::collections::BTreeMap;
use std::io::{Read, Write as _};

use confined_transport::{frame, greeting};
use pdf_sandbox::lockdown::{self, Profile};
use pdf_script::{Budget, Realm};

use crate::wire::{FRAME_RUN, MAGIC, MAX_RUN_BYTES, Reply, decode_run, encode_reply};

/// Most scripts one worker holds.
///
/// A form's scripts are one per field and trigger; the census's largest form has 1 418 fields, and
/// a worker is replaced, holding nothing, whenever it is killed. Past this the run is refused by
/// name rather than the table grown.
const MAX_HELD_SCRIPTS: usize = 8192;

/// Most bytes of script text one worker holds.
///
/// Counted against the address-space ceiling beside the engine's own allocations (ADR 1609): eight
/// mebibytes is eight of the largest script a run may carry, and some thousands of real ones.
const MAX_HELD_BYTES: usize = 8 << 20;

/// Runs the script worker to completion.
///
/// Confines this process with [`Profile::Script`], writes the greeting, then reads runs from
/// standard input and writes replies to standard output until the host closes its end.
///
/// # Errors
///
/// The confinement's, before the greeting — so the host sees a worker that never identified itself
/// rather than one it can trust — and a pipe's.
pub fn serve() -> Result<(), std::io::Error> {
    let confinement = lockdown::apply_for(Profile::Script).map_err(std::io::Error::other)?;

    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    output.write_all(&greeting::encode(MAGIC, confinement))?;
    output.flush()?;

    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut held = Held::default();
    let mut realm = None;
    while let Some(incoming) = read_frame(&mut input)? {
        let reply = match incoming {
            Incoming::Frame { kind, payload } => answer(&mut held, &mut realm, kind, &payload),
            Incoming::TooLarge { length } => Reply::Refused(format!(
                "a run of {length} bytes is past the {MAX_RUN_BYTES} this worker reads"
            )),
        };
        let (kind, bytes) = encode_reply(&reply);
        output.write_all(&frame::header(kind, bytes.len()))?;
        output.write_all(&bytes)?;
        output.flush()?;
    }
    Ok(())
}

/// One frame, or the fact that it was past [`MAX_RUN_BYTES`].
#[derive(Debug)]
enum Incoming {
    /// A frame, whole.
    Frame {
        /// Its kind.
        kind: u8,
        /// Its bytes.
        payload: Vec<u8>,
    },
    /// A frame too large to read, whose bytes have been read past so that the next frame begins
    /// where the host thinks it does.
    TooLarge {
        /// What the header said.
        length: usize,
    },
}

/// Reads one frame, or `None` at the end of input.
///
/// The length is checked before anything is allocated from it: it is the host's claim, and a
/// worker whose host has been taken over is the case the bound is for.
fn read_frame(input: &mut impl Read) -> Result<Option<Incoming>, std::io::Error> {
    let mut header = [0u8; frame::HEADER_LEN];
    match input.read_exact(&mut header) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let Some((kind, length)) = frame::parse_header(header) else {
        return Err(std::io::Error::other(
            "a frame whose length this build does not read",
        ));
    };
    if length > MAX_RUN_BYTES {
        let skipped = std::io::copy(
            &mut input.take(u64::try_from(length).unwrap_or(u64::MAX)),
            &mut std::io::sink(),
        )?;
        if skipped < u64::try_from(length).unwrap_or(u64::MAX) {
            return Ok(None);
        }
        return Ok(Some(Incoming::TooLarge { length }));
    }
    let mut payload = vec![0u8; length];
    input.read_exact(&mut payload)?;
    Ok(Some(Incoming::Frame { kind, payload }))
}

/// The scripts this worker holds, by the index the host gave each.
#[derive(Debug, Default)]
struct Held {
    /// Each script's text.
    scripts: BTreeMap<u32, String>,
    /// Their bytes, together.
    bytes: usize,
}

impl Held {
    /// Holds `text` under `index`, replacing what the index held; or the sentence saying why not.
    fn hold(&mut self, index: u32, text: String) -> Result<(), String> {
        let replaced = self.scripts.get(&index).map_or(0, String::len);
        let bytes = self
            .bytes
            .saturating_sub(replaced)
            .saturating_add(text.len());
        if bytes > MAX_HELD_BYTES {
            return Err(format!(
                "this worker holds {} bytes of scripts and the script given index {index} would \
                 take it past {MAX_HELD_BYTES}",
                self.bytes
            ));
        }
        if replaced == 0
            && !self.scripts.contains_key(&index)
            && self.scripts.len() >= MAX_HELD_SCRIPTS
        {
            return Err(format!(
                "this worker holds {MAX_HELD_SCRIPTS} scripts, which is as many as it holds"
            ));
        }
        self.scripts.insert(index, text);
        self.bytes = bytes;
        Ok(())
    }
}

/// Answers one frame, in the document's realm, which the first run constructs.
///
/// One realm for the worker's whole life, because a worker serves one document: what that
/// document's library defined at the open is what a field's script calls later (ADR 1602). A
/// refusal is a reply rather than an error: a host that sent something this worker does not run
/// keeps its worker, and only a broken pipe ends one.
fn answer(held: &mut Held, realm: &mut Option<Realm>, kind: u8, payload: &[u8]) -> Reply {
    if kind != FRAME_RUN {
        return Reply::Refused(format!(
            "a frame of kind {kind} is not one this worker reads"
        ));
    }
    let run = match decode_run(payload) {
        Ok(run) => run,
        Err(error) => return Reply::Refused(format!("the run does not decode: {error}")),
    };
    if let Some((index, text)) = run.new_script
        && let Err(sentence) = held.hold(index, text)
    {
        return Reply::Refused(sentence);
    }
    let Some(text) = held.scripts.get(&run.script) else {
        return Reply::Refused(format!(
            "the run names script {}, which this worker does not hold",
            run.script
        ));
    };
    let request = pdf_script::Request {
        script: text.clone(),
        ..run.request
    };
    if realm.is_none() {
        match Realm::new(Budget::FIELD_EVENT) {
            Ok(constructed) => *realm = Some(constructed),
            Err(why) => return Reply::Refused(why),
        }
    }
    match realm.as_mut() {
        Some(realm) => Reply::Outcome(realm.run(&request)),
        None => Reply::Refused("the realm could not be constructed".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use pdf_script::Ending;

    use super::{Held, Incoming, MAX_HELD_BYTES, answer, read_frame};
    use crate::wire::tests::request;
    use crate::wire::{FRAME_RUN, MAX_RUN_BYTES, Reply, Run, encode_run};

    /// A script crosses once and is run by its index after that, in this process — the part of
    /// the worker that is not its confinement.
    #[test]
    fn a_script_held_once_is_run_by_its_index() {
        let mut held = Held::default();
        let first = Run {
            new_script: Some((0, "event.value = '<' + event.value + '>';".to_owned())),
            script: 0,
            request: request(),
        };
        let mut realm = None;
        let Reply::Outcome(outcome) = answer(&mut held, &mut realm, FRAME_RUN, &encode_run(&first))
        else {
            panic!("the first run is answered with an outcome");
        };
        assert_eq!(outcome.value.as_deref(), Some("<12>"));
        let again = Run {
            new_script: None,
            ..first
        };
        let Reply::Outcome(outcome) = answer(&mut held, &mut realm, FRAME_RUN, &encode_run(&again))
        else {
            panic!("the second run is answered with an outcome");
        };
        assert_eq!(outcome.ending, Ending::Finished);
        assert_eq!(outcome.value.as_deref(), Some("<12>"));
    }

    #[test]
    fn a_run_naming_a_script_not_held_is_refused_by_name() {
        let run = Run {
            new_script: None,
            script: 3,
            request: request(),
        };
        let reply = answer(
            &mut Held::default(),
            &mut None,
            FRAME_RUN,
            &encode_run(&run),
        );
        assert!(
            matches!(&reply, Reply::Refused(sentence) if sentence.contains("script 3")),
            "{reply:?}"
        );
    }

    #[test]
    fn the_held_scripts_are_bounded_in_bytes() {
        let mut held = Held::default();
        let script = "x".repeat(1 << 20);
        for index in 0..8 {
            held.hold(index, script.clone()).expect("eight fit");
        }
        assert_eq!(held.bytes, MAX_HELD_BYTES);
        assert!(held.hold(8, script.clone()).is_err());
        held.hold(0, "y".to_owned())
            .expect("a replacement frees its bytes");
        assert!(held.hold(8, "z".repeat(1 << 19)).is_ok());
    }

    /// A frame past the bound is read past rather than read in, and the next frame is still found.
    #[test]
    fn a_frame_past_the_bound_is_read_past_and_the_next_is_found() {
        let length = MAX_RUN_BYTES + 1;
        let mut bytes = confined_transport::frame::header(FRAME_RUN, length).to_vec();
        bytes.resize(bytes.len() + length, 0);
        bytes.extend_from_slice(&confined_transport::frame::header(FRAME_RUN, 1));
        bytes.push(9);
        let mut input = bytes.as_slice();
        assert!(matches!(
            read_frame(&mut input).expect("reads"),
            Some(Incoming::TooLarge { length: told }) if told == length
        ));
        assert!(matches!(
            read_frame(&mut input).expect("reads"),
            Some(Incoming::Frame { kind: FRAME_RUN, payload }) if payload == [9]
        ));
        assert!(read_frame(&mut input).expect("reads").is_none());
    }
}
