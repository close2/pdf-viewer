//! Which process parses the document: a confined worker by default, this one where the person
//! says so (ADR 2140, `doc/questions/A02`).
//!
//! **The worker is the default, and the in-process route is a fallback a host selects, said
//! aloud.** `--in-process` is that selection; a run that reads no document — `--help`,
//! `--remedy-sites`, or a command line this program cannot read — has nothing to confine and runs
//! here without a word, because what it prints is decided by the arguments alone.

use std::path::{Path, PathBuf};

use pdf_transform::confined::{Fences, World};
use pdf_transform::pattern::Pattern;
use pdf_transform::profiles::Contribution;

use super::here::Here;
use super::{Arguments, Exit, configuration_text, input_spec, program};

/// Runs the program in the process its arguments call for, and answers with its exit status.
pub(super) fn route(given: Vec<String>) -> u8 {
    let mut here = Here;
    let Ok(arguments) = Arguments::parse(given.clone()) else {
        return program(&mut here, given);
    };
    if reads_no_document(&arguments) {
        return program(&mut here, given);
    }
    if arguments.switch("--in-process") {
        here.say(
            "note: --in-process: this run parses the document in this process, unconfined — \
             ISO 32000-2's bytes reach the interpreter and the decoders with this process's \
             filesystem and network (ADR 2140)\n",
        );
        return program(&mut here, given);
    }
    confined(&mut here, &arguments, &given)
}

/// Whether the run is decided by its arguments alone.
fn reads_no_document(arguments: &Arguments) -> bool {
    arguments.switch("--help")
        || arguments.switch("-h")
        || arguments.switch("--remedy-sites")
        || matches!(arguments.verb.as_str(), "" | "help" | "--help" | "-h")
}

/// The run in a confined worker, served from this machine through the fences the arguments draw.
#[cfg(unix)]
fn confined(here: &mut Here, arguments: &Arguments, given: &[String]) -> u8 {
    use pdf_transform::confined::{MAGIC, WORKER_PATH_VARIABLE, WORKER_PROGRAM};

    let worker =
        match confined_transport::program_beside_executable(WORKER_PROGRAM, WORKER_PATH_VARIABLE) {
            Ok(worker) => worker,
            Err(missing) => {
                here.say(&format!(
                "error: the confined worker `{WORKER_PROGRAM}` {missing} — build it with `cargo \
                 build -p pdf-transform --bins`, name it with ${WORKER_PATH_VARIABLE}, or run \
                 with --in-process, which parses the document in this process, unconfined\n"
            ));
                return Exit::Error.code();
            }
        };
    let fences = fences(here, arguments);
    let mut host = match confined_transport::Host::start(
        &worker,
        MAGIC,
        &confined_transport::Canceller::new(),
    ) {
        Ok(host) => host,
        Err(error) => {
            here.say(&format!("error: {error}\n"));
            return Exit::Error.code();
        }
    };
    // The faces a document names and does not embed come from this machine through the
    // transport's port, as `pdf-vfs`'s worker's do: the worker can name no path, and what it
    // gains is a descriptor this process opened for a description it sent (ADR 0880).
    host.offer(Box::new(|request| {
        pdf_font::provider::open_a_face(request)
            .map(|(identity, content)| confined_transport::Provided { identity, content })
    }));
    let confinement = host.confinement();
    if !confinement.is_enforced() {
        here.say(&format!(
            "note: the confined worker is not confined on this machine — {confinement:?} — so the \
             document is parsed with this user's filesystem and network after all\n"
        ));
    }
    match pdf_transform::confined::serve(&mut host, here, &fences, given) {
        Ok(code) => code,
        Err(stopped) => {
            here.say(&format!("error: {stopped}\n"));
            Exit::Error.code()
        }
    }
}

/// Off Unix there is no confined worker for this suite, and the run says so.
#[cfg(not(unix))]
fn confined(here: &mut Here, _arguments: &Arguments, given: &[String]) -> u8 {
    here.say(
        "note: this platform has no confined worker for the transform suite, so this run parses \
         the document in this process (doc/todo/35)\n",
    );
    program(here, given.to_vec())
}

/// What a worker may be given, read off the person's own command line.
///
/// The flags are the ones this program's grammar reads a file or a descriptor from; a flag added
/// there that reads one is added here too, or the worker is refused it by name — loudly, as a
/// path the command line does not name — which is the direction a mistake should fail in.
#[cfg(unix)]
fn fences(here: &mut Here, arguments: &Arguments) -> Fences {
    let mut named: Vec<PathBuf> = arguments
        .positional
        .iter()
        .map(|spec| input_spec(spec).0)
        .collect();
    for flag in ["--attach", "--config", "--output-intent-profile"] {
        named.extend(arguments.every(flag).into_iter().map(PathBuf::from));
    }
    named.extend(
        arguments
            .every("--font")
            .into_iter()
            .filter_map(|stated| stated.split_once('='))
            .map(|(_, path)| PathBuf::from(path)),
    );
    let descriptors = ["--password-fd", "--encrypt-owner-fd", "--encrypt-user-fd"]
        .into_iter()
        .flat_map(|flag| arguments.every(flag))
        .filter_map(|fd| fd.parse().ok())
        .collect();
    let output = arguments.value(&["-o", "--output"]);
    let mut outputs: Vec<Pattern> = Vec::new();
    if let Some(output) = output {
        outputs.extend(output.parse::<Pattern>().ok());
        if output.ends_with('/') {
            outputs.extend(format!("{output}%t").parse::<Pattern>().ok());
        }
    }
    let beside = if arguments.switch("--resolve-external-data") {
        arguments
            .positional
            .first()
            .map(|spec| input_spec(spec).0)
            .and_then(|path| path.parent().map(Path::to_path_buf))
    } else {
        None
    };
    let (tools, target) = tools(here, arguments);
    Fences {
        named,
        descriptors,
        outputs,
        to_stdout: output == Some("-"),
        beside,
        tools,
        target,
    }
}

/// The tools the configuration names, each with the site it answers: its `derive` remedies and its
/// `preserve` remedies that fetch what a stream keeps outside the file, the two kinds that run a
/// program. None where it cannot be read, which the worker then reports in its own words.
#[cfg(unix)]
fn tools(
    here: &mut Here,
    arguments: &Arguments,
) -> (Vec<(String, pdf_transform::tool::Tool)>, String) {
    let Some(target) = arguments
        .value(&["--to"])
        .and_then(pdf_archive::Target::parse)
    else {
        return (Vec::new(), String::new());
    };
    let read = match configuration_text(here, arguments) {
        Ok(Some((_, text))) => Contribution::read(&text, target).ok(),
        Ok(None) | Err(_) => None,
    };
    let tools = read.map_or_else(Vec::new, |read| {
        let derived = read
            .derivations
            .into_iter()
            .map(|derivation| (derivation.site, derivation.tool));
        let fetched = read
            .resolutions
            .into_iter()
            .map(|resolution| (resolution.site, resolution.tool));
        derived.chain(fetched).collect()
    });
    (tools, target.to_string())
}
