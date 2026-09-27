//! The confined side of the boundary.
//!
//! The whole of the worker's life: confine itself, say so, then answer decode requests
//! until its input closes. It opens nothing, connects to nothing, and forks nothing —
//! after [`crate::lockdown::apply`] it could not do any of those if it tried, which is the
//! point.
//!
//! # Order matters here more than anywhere else in this crate
//!
//! Lockdown comes first, before a single byte of a request is read. A worker that read its
//! first image and *then* confined itself would have handled untrusted input unconfined —
//! which is the entire failure this crate exists to prevent, and it would look exactly like
//! working code.

use std::io::Write as _;

use crate::{Decoded, decode, protocol};

/// Runs the worker to completion.
///
/// Reads requests from standard input and writes responses to standard output, which are
/// the pipes the parent connected. Returns when the parent closes its end.
///
/// # Errors
///
/// Returns an error if the process could not be confined, or if a pipe failed. A confinement
/// failure returns *before* the greeting, so the parent sees a worker that never identified
/// itself rather than one it can trust.
pub fn serve() -> Result<(), std::io::Error> {
    // The one argument a worker takes is read before lockdown, as everything the parent says
    // at start is: it sizes the ceiling the lockdown installs (ADR 1333).
    let whole = whole_samples(std::env::args().skip(1))?;
    let confinement = match whole {
        Some(samples) => {
            crate::lockdown::apply_for_decoder_within(decode::address_space_for(samples))
        }
        None => crate::lockdown::apply(),
    }
    .map_err(std::io::Error::other)?;
    let admitted = whole.unwrap_or(decode::MAX_SAMPLES);

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    output.write_all(&protocol::encode_handshake(confinement))?;
    output.flush()?;

    while let Some(wire) = protocol::read_request(&mut input)? {
        let response = match protocol::typed_request(&wire) {
            Some(request) => answer(&request, admitted),
            None => protocol::encode_error(&format!("request kind {} is not defined", wire.kind)),
        };
        output.write_all(&response)?;
        output.flush()?;
    }
    Ok(())
}

/// Decodes one request into an encoded response.
///
/// A refusal is a response, not an error: an image this cannot decode is an ordinary event
/// that the page reports and draws around, and treating it as a transport failure would
/// throw away a healthy worker on every malformed image in a corpus.
///
/// `admitted` is the most samples a full-resolution JPEG 2000 decode may produce here: what this
/// worker's ceiling was sized for, whatever a request asks.
fn answer(request: &protocol::Request<'_>, admitted: u64) -> Vec<u8> {
    let decoded = match request {
        protocol::Request::Jbig2 { data, globals } => {
            decode::jbig2(data, globals).map(Decoded::Bilevel)
        }
        protocol::Request::Jpx { data, indices } => {
            decode::jpx(data, *indices).map(Decoded::Raster)
        }
        protocol::Request::JpxWhole {
            data,
            indices,
            samples,
        } => decode::jpx_whole(data, *indices, (*samples).min(admitted)).map(Decoded::Raster),
        protocol::Request::Ccitt { data, parameters } => {
            decode::ccitt(data, *parameters).map(Decoded::Bilevel)
        }
    };
    match decoded {
        Ok(decoded) => protocol::encode_response(&decoded),
        Err(detail) => protocol::encode_error(&detail),
    }
}

/// The argument a worker for full-resolution decodes is started with: `--whole <samples>`, or
/// nothing for the ordinary worker ([`crate::Sandbox::whole`], ADR 1333).
///
/// # Errors
///
/// An argument this program does not take, or a count that is not a number: a worker started
/// wrongly stops before confining itself, so the parent sees one that never greeted it.
fn whole_samples(mut arguments: impl Iterator<Item = String>) -> std::io::Result<Option<u64>> {
    match (arguments.next(), arguments.next(), arguments.next()) {
        (None, _, _) => Ok(None),
        (Some(flag), Some(count), None) if flag == WHOLE_ARGUMENT => count
            .parse()
            .map(Some)
            .map_err(|_| std::io::Error::other(format!("{WHOLE_ARGUMENT} {count}: not a count"))),
        _ => Err(std::io::Error::other(
            "the worker takes no argument but --whole <samples>",
        )),
    }
}

/// The flag [`whole_samples`] reads, and [`crate::Sandbox::whole`] passes.
pub(crate) const WHOLE_ARGUMENT: &str = "--whole";

#[cfg(test)]
mod tests {
    use super::{WHOLE_ARGUMENT, whole_samples};

    fn arguments(words: &[&str]) -> impl Iterator<Item = String> {
        words
            .iter()
            .map(|word| (*word).to_owned())
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// No argument is the ordinary worker; the one flag with a count is a whole-resolution
    /// worker; anything else stops the worker before it confines itself or greets its parent.
    #[test]
    fn a_worker_takes_its_budget_or_nothing() {
        assert_eq!(whole_samples(arguments(&[])).ok(), Some(None));
        assert_eq!(
            whole_samples(arguments(&[WHOLE_ARGUMENT, "134217728"])).ok(),
            Some(Some(1 << 27))
        );
        assert!(whole_samples(arguments(&[WHOLE_ARGUMENT, "many"])).is_err());
        assert!(whole_samples(arguments(&[WHOLE_ARGUMENT])).is_err());
        assert!(whole_samples(arguments(&["--other", "1"])).is_err());
        assert!(whole_samples(arguments(&[WHOLE_ARGUMENT, "1", "2"])).is_err());
    }
}
