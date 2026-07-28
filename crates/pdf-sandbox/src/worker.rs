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
    let confinement = crate::lockdown::apply().map_err(std::io::Error::other)?;

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    output.write_all(&protocol::encode_handshake(confinement))?;
    output.flush()?;

    while let Some(wire) = protocol::read_request(&mut input)? {
        let response = match protocol::typed_request(&wire) {
            Some(request) => answer(&request),
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
fn answer(request: &protocol::Request<'_>) -> Vec<u8> {
    let decoded = match request {
        protocol::Request::Jbig2 { data, globals } => {
            decode::jbig2(data, globals).map(Decoded::Bilevel)
        }
        protocol::Request::Jpx { data, indices } => {
            decode::jpx(data, *indices).map(Decoded::Raster)
        }
    };
    match decoded {
        Ok(decoded) => protocol::encode_response(&decoded),
        Err(detail) => protocol::encode_error(&detail),
    }
}
