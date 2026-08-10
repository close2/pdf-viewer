//! One byte range of one remote object, read through `curl`.
//!
//! # Why a subprocess rather than a crate
//!
//! Reading an HTTPS object needs a TLS stack, a certificate store and an HTTP client. This
//! workspace has none of those and should not gain three for a developer tool: `doc/todo/03`
//! asks for a fetcher, not for a network layer inside a PDF viewer. `tools/pdfref` already
//! drives `pdftoppm`, `mutool` and `gs` as subprocesses for exactly this reason, and the
//! platform's `curl` is better maintained than anything this project would pin.
//!
//! What it costs is written down: the tool does not run where `curl` is absent, and it says
//! so ([`crate::Error::NoTransport`]) rather than failing obscurely.

use std::io::Read as _;
use std::process::{Command, Stdio};

use crate::Error;

/// How many bytes the object at `url` holds, asked with `HEAD` so that no body is
/// transferred.
///
/// The tail of a ZIP is where its directory is, and a suffix range is the one thing HTTP's
/// range syntax expresses that this crate's inclusive-pair signature does not — so the length
/// is asked for outright instead, which also puts the archive's size in front of the caller
/// before anything else happens.
///
/// # Errors
///
/// As [`fetch_range`], plus [`Error::Transport`] when the answer carries no `Content-Length`.
pub fn length(url: &str) -> Result<u64, Error> {
    let output = Command::new("curl")
        .arg("--silent")
        .arg("--show-error")
        .arg("--fail")
        .arg("--location")
        .arg("--max-redirs")
        .arg("3")
        .arg("--head")
        .arg(url)
        .output()
        .map_err(Error::NoTransport)?;
    let complaint = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if !output.status.success() {
        return Err(Error::Transport {
            url: url.to_owned(),
            first: 0,
            last: 0,
            reason: if complaint.is_empty() {
                format!("curl exited with {}", output.status)
            } else {
                complaint
            },
        });
    }
    // A redirect chain prints one header block per hop, so the *last* content length is the
    // object's. Header names are case-insensitive (RFC 9110 section 5.1) and servers differ.
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<u64>().ok())?
        })
        .next_back()
        .ok_or_else(|| Error::Transport {
            url: url.to_owned(),
            first: 0,
            last: 0,
            reason: "the answer carried no Content-Length, so the archive's tail cannot be \
                     addressed"
                .to_owned(),
        })
}

/// Reads bytes `first..=last` of `url`.
///
/// The range is inclusive at both ends, which is HTTP's convention rather than Rust's, so
/// that what this function is asked for and what appears in the request are the same numbers.
///
/// # Errors
///
/// [`Error::NoTransport`] when `curl` cannot be started, [`Error::Transport`] when it runs and
/// fails, and [`Error::ShortRange`] when the server answers with a different number of bytes
/// than were asked for — which is the check that stops a truncated transfer from being parsed
/// as a truncated archive.
pub fn fetch_range(url: &str, first: u64, last: u64) -> Result<Vec<u8>, Error> {
    let want = last.saturating_sub(first).saturating_add(1);
    // `--fail` turns an HTTP error status into a non-zero exit rather than a body of HTML
    // that would otherwise be parsed as ZIP. `--silent --show-error` keeps the progress meter
    // off stdout and diagnostics on stderr, which is where this reads them from.
    //
    // **`--max-filesize` is the load-bearing one.** A server that ignores `Range` answers 200
    // with the whole object, and against a 1.3 GB archive that is exactly the accident this
    // tool exists to prevent — the length check below would notice it only after the gigabyte
    // had crossed the wire. This aborts on the announced content length instead, before the
    // body starts. The slack covers a redirect's own small body.
    let mut child = Command::new("curl")
        .arg("--silent")
        .arg("--show-error")
        .arg("--fail")
        .arg("--location")
        .arg("--max-redirs")
        .arg("3")
        .arg("--max-filesize")
        .arg(want.saturating_add(4096).to_string())
        .arg("--range")
        .arg(format!("{first}-{last}"))
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(Error::NoTransport)?;

    // Read stdout before waiting: a pipe this large would otherwise fill and deadlock.
    let mut body = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        out.read_to_end(&mut body).map_err(Error::NoTransport)?;
    }
    let mut complaint = String::new();
    if let Some(mut err) = child.stderr.take() {
        let mut raw = Vec::new();
        err.read_to_end(&mut raw).map_err(Error::NoTransport)?;
        String::from_utf8_lossy(&raw)
            .trim()
            .clone_into(&mut complaint);
    }
    let status = child.wait().map_err(Error::NoTransport)?;

    if !status.success() {
        return Err(Error::Transport {
            url: url.to_owned(),
            first,
            last,
            reason: if complaint.is_empty() {
                format!("curl exited with {status}")
            } else {
                complaint
            },
        });
    }
    // The second half of the same guard: a partial answer, or a full one small enough to slip
    // under `--max-filesize`, is an error rather than a surprise.
    if u64::try_from(body.len()).unwrap_or(u64::MAX) != want {
        return Err(Error::ShortRange {
            url: url.to_owned(),
            want,
            got: body.len(),
        });
    }
    Ok(body)
}
