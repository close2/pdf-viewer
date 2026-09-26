//! §12.7.6.2's verb: the composed request sent, and what came back handed to the window.
//!
//! # What the clause asks, and what it leaves to the processor
//!
//! ISO 32000-2 §12.7.6.2:
//!
//! > Upon invocation of a submit-form action, an interactive PDF processor shall transmit the
//! > names and values of selected interactive form fields to a specified uniform resource locator
//! > (URL).
//!
//! Which names, which values, which format, which method and which URL are the document's, and
//! `pdf_model::submission::compose` has answered all of them before a host sees the request. What
//! is left is the transmission, and it is this module's: [`transmit`] sends the method, the media
//! type and the body exactly as composed, to the URL exactly as composed.
//!
//! **The clause states no duty about the answer, and that is a reading rather than an omission.**
//! Its only word on one is a NOTE — "Presumably, the URL is the address of a Web server that will
//! process them and send back a response" — and a NOTE states no requirement. The one `shall`
//! about a response is §12.7.8's, one clause over, on the format the server may answer in: Table
//! 246's `/Status` is "[a] status string that shall be displayed indicating the result of an
//! action, typically a submit-form action", and §12.7.8.1 says FDF "can be used when submitting
//! form data to a server, receiving the response, and incorporating it into the interactive form".
//! So an FDF answer is imported through the §12.7.8 reader the import-data action already uses,
//! whose `/Status` is displayed there; a PDF answer is a document and opens beside the one that
//! sent the form; and anything else is said in a sentence with its media type, because a person
//! who pressed a button is owed what the server said even where this program cannot show it
//! (ADR 1291).
//!
//! # Which thread, and why
//!
//! **A thread of its own per submission, named `submit-form`, and never the window's.** A request
//! waits on a network for as long as the server takes, so the event thread would stop drawing and
//! stop answering keys; and the drawing thread belongs to `crate::drawing`, whose whole contract is
//! that a draw can be taken back — a request already on the wire cannot. What crosses back is a
//! [`Returned`] on a channel, collected by the window on its own timer ([`Submitter::interval`]),
//! which is [`crate::drawing`]'s pull model for its reason: one of the three toolkits cannot be
//! pushed to from another thread.
//!
//! # What is bounded
//!
//! `CLAUDE.md` principle 3: a server is as untrusted as a document. The answer's body is read to
//! at most [`RESPONSE_LIMIT`] bytes and the whole exchange to [`TIMEOUT`], each a documented
//! choice rather than a number the standard states; redirects are **not** followed, because the
//! person was asked about one URL and a server that answers "go elsewhere" is saying something
//! the person should read rather than something this program should do for them.
//!
//! # What is checked before the TLS stack sees it
//!
//! **A string a document chose enters the TLS stack, so it is checked first.** Table 239's `/F`
//! is sent exactly as composed, and its host is what `rustls` writes into the handshake's server
//! name and what `rustls-webpki` matches the server's certificate against. `ring` itself is handed
//! none of it — its inputs are the keys, signatures and transcript the server's handshake brings —
//! but the owner's condition on admitting `ring` into this process (`doc/questions/A130`, ADR 1327)
//! is that only well-formed input is passed onward. [`check_url`] is that check, run by
//! [`transmit`] and by `crate::policy::may_submit` alike: rudimentary rather than a full URL
//! grammar, each bound written beside the constant that holds it, and each failure a
//! [`UrlRefusal`] that names the check.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use pdf_model::action::DataFormat;
use pdf_model::submission::{Method, Submission};
use viewer_core::DocumentId;

/// The longest a submission may take, from connecting to the last byte of the answer.
///
/// **A documented choice**: the standard states none. A minute is long enough for a server doing
/// real work on a form and short enough that a person who pressed a button learns something
/// happened to it before they have given up on it.
pub const TIMEOUT: Duration = Duration::from_mins(1);

/// The most of an answer's body this program reads.
///
/// **A documented choice, and principle 3's**: a server that answers without end is the network's
/// decompression bomb. 64 MiB holds any form's answer and any PDF a form would reasonably be
/// answered with; a larger one is refused by name rather than truncated, because half a document
/// is not a document.
pub const RESPONSE_LIMIT: u64 = 64 * 1024 * 1024;

/// The longest host name [`check_url`] passes on, in octets.
///
/// RFC 1035 section 2.3.4 bounds a domain name at 255 octets in the form it travels in on the
/// wire, a length octet before each label and a zero after the last; written out with dots and
/// without the root's trailing one, that is 253 characters.
pub const NAME_OCTETS: usize = 253;

/// The longest label of a host name [`check_url`] passes on, in octets — RFC 1035 section 2.3.4's
/// bound on one label.
pub const LABEL_OCTETS: usize = 63;

/// How often a window asks whether an answer has arrived, while one is outstanding.
pub const LOOK: Duration = Duration::from_millis(50);

/// Why a submission produced no answer at all.
#[derive(Debug, thiserror::Error)]
pub enum TransmitError {
    /// The scheme is not one this program sends a form to — `crate::policy::SUBMIT_SCHEMES`.
    ///
    /// Checked again here although `crate::policy::may_submit` refused it first, for
    /// `crate::policy::open_uri`'s reason: this is the function that actually reaches the network,
    /// so it is the last place the guarantee can be made.
    #[error("{0} names no scheme this reader sends a form to (http, https)")]
    Scheme(String),
    /// The URL failed one of [`check_url`]'s checks, so nothing of it reached the TLS stack.
    #[error("{url} was refused before anything was sent: {refusal}")]
    Url {
        /// The URL as composed.
        url: String,
        /// The check it failed.
        refusal: UrlRefusal,
    },
    /// This machine's trust store could not be read, so no server could be believed.
    #[error("this machine's certificate store could not be read: {0}")]
    TrustStore(String),
    /// The request did not complete: no connection, a TLS refusal, a timeout, an answer too large.
    #[error("{0}")]
    Network(String),
}

/// Which of [`check_url`]'s checks a URL failed, by name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UrlRefusal {
    /// No `//` and authority after the scheme, so the URL names no server (RFC 3986 section 3.2).
    #[error("the URL names no host after its scheme")]
    NoAuthority,
    /// User information before the host (`user@host`): a form's address has no use for it, and it
    /// is where a URL hides a host behind something that looks like one.
    #[error(
        "the URL carries user information (`@`) before its host, which this reader does not send"
    )]
    Userinfo,
    /// A `%` in the authority: a host is sent only as it is written, never decoded first.
    #[error(
        "the URL's host is percent-encoded (`%`), and this reader sends only a host written out"
    )]
    PercentEncoded,
    /// Nothing between the `//` and the port or path.
    #[error("the URL's host is empty")]
    EmptyHost,
    /// Longer than [`NAME_OCTETS`].
    #[error(
        "the host name is {0} octets long, past the {NAME_OCTETS} a domain name may have (RFC 1035 \
         section 2.3.4)"
    )]
    NameTooLong(usize),
    /// A label longer than [`LABEL_OCTETS`].
    #[error(
        "the host name has a label of {0} octets, past the {LABEL_OCTETS} a label may have (RFC \
         1035 section 2.3.4)"
    )]
    LabelTooLong(usize),
    /// Two dots together, or a dot first.
    #[error("the host name has an empty label")]
    EmptyLabel,
    /// A character that is not an ASCII letter, a digit or a hyphen, inside a label.
    #[error(
        "the host name holds {0:?}, which is not a letter, a digit, a hyphen or a dot (RFC 1035 \
         section 2.3.1)"
    )]
    Character(char),
    /// A bracketed host that does not parse as an IPv6 address.
    #[error("the bracketed host {0:?} is not an IPv6 address (RFC 3986 section 3.2.2)")]
    Ipv6(String),
    /// A port that is not one to five digits naming a number up to 65535.
    #[error("the port {0:?} is not a number from 0 to 65535 (RFC 3986 section 3.2.3)")]
    Port(String),
    /// A control character or white space in the path, query or fragment.
    #[error("the URL holds a control character or white space after its host")]
    Control,
}

/// Checks a submission's URL before any of it is handed to the HTTP client and the TLS stack.
///
/// Why this exists is the module's section *What is checked before the TLS stack sees it*: the
/// host is a string a document chose, and it enters `rustls` for the server name and for the
/// certificate's name check. What is checked, in this order:
///
/// - the authority: a `//` after the scheme, no user information (`@`) and no percent-encoding
///   (`%`) anywhere in it — which also refuses an IPv6 zone identifier, since RFC 6874 writes one
///   as `%25`;
/// - the host: either an IPv6 literal in brackets that `std::net::Ipv6Addr` parses, or a name of
///   at most [`NAME_OCTETS`] octets whose labels are non-empty, at most [`LABEL_OCTETS`] octets
///   and made of ASCII letters, digits and hyphens (RFC 1035 section 2.3.1's letters, digits and
///   hyphens, with a digit admitted first as RFC 1123 section 2.1 allows, so a dotted IPv4 address
///   passes as a name). One trailing dot, the root's, is admitted;
/// - the port, where one is written: one to five ASCII digits naming a `u16`;
/// - the rest: no control character and no white space.
///
/// **What is not checked, and why.** A hyphen first or last in a label, an all-digit name that is
/// not a real IPv4 address, and the path's and query's grammar: none of them is a string the TLS
/// stack parses — the path and query travel as application data inside the connection, and a host
/// of the wrong shape but the right characters fails at the resolver or at the certificate's name
/// check, both of which refuse by name already. An internationalised name is refused rather than
/// converted: a document that means one writes its ASCII form (RFC 5890), and this reader does
/// not choose a conversion on its behalf. The owner asked for rudimentary checks rather than a
/// grammar (`doc/questions/A130`), and a grammar is where a checker's own defects would live.
///
/// # Errors
///
/// The [`UrlRefusal`] naming the first check the URL failed.
pub fn check_url(url: &str) -> Result<(), UrlRefusal> {
    let after_scheme = url
        .split_once(':')
        .map_or("", |(_, after_scheme)| after_scheme);
    let rest = after_scheme
        .strip_prefix("//")
        .ok_or(UrlRefusal::NoAuthority)?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let remainder = rest.get(authority.len()..).unwrap_or_default();
    if authority.contains('@') {
        return Err(UrlRefusal::Userinfo);
    }
    if authority.contains('%') {
        return Err(UrlRefusal::PercentEncoded);
    }
    let port = if let Some(bracketed) = authority.strip_prefix('[') {
        let (literal, after) = bracketed
            .split_once(']')
            .ok_or_else(|| UrlRefusal::Ipv6(bracketed.to_owned()))?;
        literal
            .parse::<std::net::Ipv6Addr>()
            .map_err(|_| UrlRefusal::Ipv6(literal.to_owned()))?;
        if after.is_empty() {
            None
        } else {
            Some(
                after
                    .strip_prefix(':')
                    .ok_or_else(|| UrlRefusal::Port(after.to_owned()))?,
            )
        }
    } else {
        let (host, port) = match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        };
        check_name(host)?;
        port
    };
    if let Some(port) = port {
        // `u16::from_str` admits a leading `+`, which is no port RFC 3986 section 3.2.3 writes.
        let digits =
            !port.is_empty() && port.len() <= 5 && port.bytes().all(|b| b.is_ascii_digit());
        if !digits || port.parse::<u16>().is_err() {
            return Err(UrlRefusal::Port(port.to_owned()));
        }
    }
    if remainder
        .chars()
        .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(UrlRefusal::Control);
    }
    Ok(())
}

/// [`check_url`]'s checks on a host written as a name.
fn check_name(host: &str) -> Result<(), UrlRefusal> {
    if host.is_empty() {
        return Err(UrlRefusal::EmptyHost);
    }
    let name = host.strip_suffix('.').unwrap_or(host);
    if name.len() > NAME_OCTETS {
        return Err(UrlRefusal::NameTooLong(name.len()));
    }
    for label in name.split('.') {
        if label.is_empty() {
            return Err(UrlRefusal::EmptyLabel);
        }
        if label.len() > LABEL_OCTETS {
            return Err(UrlRefusal::LabelTooLong(label.len()));
        }
        if let Some(character) = label
            .chars()
            .find(|character| !(character.is_ascii_alphanumeric() || *character == '-'))
        {
            return Err(UrlRefusal::Character(character));
        }
    }
    Ok(())
}

/// What a server answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// RFC 9110 section 15's status code.
    pub status: u16,
    /// The answer's media type, lower-cased and without parameters, where it stated one.
    pub media_type: Option<String>,
    /// RFC 9110 section 10.2.2's `Location`, where the answer is a redirect this program did not
    /// follow.
    pub location: Option<String>,
    /// The body, at most [`RESPONSE_LIMIT`] bytes.
    pub body: Vec<u8>,
}

/// Sends §12.7.6.2's composed request, blocking, and returns what the server answered.
///
/// The method, the URL, the media type and the body are the composition's, sent unchanged: a GET
/// carries its data in the URL's query and no body, and a POST carries the body under
/// `Submission::media_type` — which is §12.7.5.3's `multipart/form-data` with its boundary where a
/// file-select control made it one.
///
/// **TLS is `rustls` over `ring`, with this machine's own trust store**, which is ADR 1291's
/// choice: no OpenSSL, and the certificates a person trusts are their setting rather than a list
/// this program ships.
///
/// # Errors
///
/// [`TransmitError`], where nothing was answered. An answer with any status is a [`Response`]: a
/// `404` is something the server *said*.
pub fn transmit(submission: &Submission) -> Result<Response, TransmitError> {
    let scheme = submission
        .url
        .split_once(':')
        .map(|(scheme, _)| scheme.to_ascii_lowercase());
    let secure = match scheme.as_deref() {
        Some("https") => true,
        Some("http") => false,
        _ => return Err(TransmitError::Scheme(submission.url.clone())),
    };
    check_url(&submission.url).map_err(|refusal| TransmitError::Url {
        url: submission.url.clone(),
        refusal,
    })?;
    let agent = agent(secure)?;
    let sent = match submission.method {
        Method::Get => agent.get(&submission.url).call(),
        Method::Post => agent
            .post(&submission.url)
            .header("Content-Type", &submission.media_type)
            .send(&submission.body[..]),
    };
    let mut answer = sent.map_err(|error| TransmitError::Network(error.to_string()))?;
    let status = answer.status().as_u16();
    let header = |name: &str| {
        answer
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    };
    let media_type = header("content-type").map(|value| media_type_of(&value));
    let location = header("location");
    let body = answer
        .body_mut()
        .with_config()
        .limit(RESPONSE_LIMIT)
        .read_to_vec()
        .map_err(|error| {
            TransmitError::Network(format!("the answer could not be read: {error}"))
        })?;
    Ok(Response {
        status,
        media_type,
        location,
        body,
    })
}

/// The agent one submission is sent with.
///
/// Built per submission rather than held, because a submission is a rare act off the launch path
/// and an agent held from startup would read the trust store before page one — `CLAUDE.md` section
/// 2's rule. The store is read only for `https`, which is the only scheme that needs it.
fn agent(secure: bool) -> Result<ureq::Agent, TransmitError> {
    let mut config = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .max_redirects(0)
        .http_status_as_error(false);
    if secure {
        let found = rustls_native_certs::load_native_certs();
        if found.certs.is_empty() {
            let why = found
                .errors
                .first()
                .map_or_else(|| "it holds no certificate".to_owned(), ToString::to_string);
            return Err(TransmitError::TrustStore(why));
        }
        let roots: Vec<ureq::tls::Certificate<'static>> = found
            .certs
            .iter()
            .map(|certificate| ureq::tls::Certificate::from_der(certificate.as_ref()).to_owned())
            .collect();
        config = config.tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::Rustls)
                .root_certs(ureq::tls::RootCerts::Specific(Arc::new(roots)))
                .unversioned_rustls_crypto_provider(Arc::new(
                    rustls::crypto::ring::default_provider(),
                ))
                .build(),
        );
    }
    Ok(config.build().new_agent())
}

/// RFC 9110 section 8.3.1's `type/subtype`, lower-cased, with any parameter dropped.
fn media_type_of(value: &str) -> String {
    value
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

/// What a window does with an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// §12.7.8's form data: hand it to `viewer_core::Command::Respond` in this format.
    Import {
        /// FDF or XFDF, by the answer's media type.
        format: DataFormat,
        /// The body.
        bytes: Vec<u8>,
        /// What to say about the exchange before the import says what it applied.
        note: String,
    },
    /// A PDF: open it beside the document that sent the form.
    Document {
        /// The body.
        bytes: Vec<u8>,
        /// What to say about the exchange.
        note: String,
    },
    /// Say this, and do nothing else.
    Say(String),
}

/// The media types an FDF answer may arrive under.
///
/// §12.7.8.1 states the first — "FDF shall use the MIME media type application/vnd.fdf" — and the
/// second is IANA's registration of the same format by ISO TC 171/SC 2, taken as the same bytes
/// because a server using the registry's name has named FDF (ADR 1291).
pub const FDF_TYPES: [&str; 2] = ["application/vnd.fdf", "application/fdf"];

/// The media types an XFDF answer may arrive under: ISO TC 171/SC 2's registration, and the alias
/// that registration names as deprecated.
pub const XFDF_TYPES: [&str; 2] = ["application/xfdf", "application/vnd.adobe.xfdf"];

/// What an answer comes to: an import, a document, or a sentence.
///
/// **Only a successful answer is acted on.** RFC 9110 section 15.3's 2xx is the server saying the
/// request was accepted; anything else is said, with its media type, and not imported — a form
/// filled from an error page would be a form filled from something the server did not mean.
#[must_use]
pub fn reply(url: &str, response: Response, warned: Option<&str>) -> Reply {
    use std::fmt::Write as _;

    let media = response.media_type.as_deref().unwrap_or("no media type");
    let mut note = format!(
        "submit-form: {url} answered {} ({media}, {} byte(s))",
        response.status,
        response.body.len()
    );
    if let Some(warned) = warned {
        note.push_str(" — ");
        note.push_str(warned);
    }
    if !(200..300).contains(&response.status) {
        if let Some(location) = &response.location {
            // `fmt::Write for String` never answers `Err`: the discard is that infallibility.
            let _ = write!(
                note,
                "; it points to {location}, which this reader does not follow on a document's \
                 behalf"
            );
        }
        return Reply::Say(note);
    }
    let format = match response.media_type.as_deref() {
        Some(media) if FDF_TYPES.contains(&media) => Some(DataFormat::Fdf),
        Some(media) if XFDF_TYPES.contains(&media) => Some(DataFormat::Xfdf),
        _ => None,
    };
    if let Some(format) = format {
        return Reply::Import {
            format,
            bytes: response.body,
            note,
        };
    }
    if response.media_type.as_deref() == Some("application/pdf") {
        return Reply::Document {
            bytes: response.body,
            note,
        };
    }
    if response.body.is_empty() {
        return Reply::Say(note);
    }
    // `fmt::Write for String` never answers `Err`: the discard is that infallibility.
    let _ = write!(
        note,
        "; this reader shows §12.7.8's form data and PDF documents, and an answer in {media} is \
         neither"
    );
    Reply::Say(note)
}

/// One submission's end, as it crosses back to the window.
#[derive(Debug)]
pub struct Returned {
    /// The document that sent the form, which is the one an FDF answer is imported into.
    pub document: DocumentId,
    /// Where it was sent.
    pub url: String,
    /// [`Submissions::Warn`](crate::policy::Submissions::Warn)'s sentence, where the level asked
    /// for one.
    pub warned: Option<String>,
    /// What the server answered, or why nothing was.
    pub answer: Result<Response, TransmitError>,
}

impl Returned {
    /// What the window does about it — [`reply`] for an answer, a sentence for none.
    #[must_use]
    pub fn reply(self) -> Reply {
        match self.answer {
            Ok(response) => reply(&self.url, response, self.warned.as_deref()),
            Err(error) => Reply::Say(format!(
                "submit-form: {} was not answered — {error}",
                self.url
            )),
        }
    }
}

/// The submissions a window has on the wire, and the answers that have come back.
///
/// Held by the window for its life. [`Self::send`] starts a thread per submission; [`Self::collect`]
/// is what the window's timer calls, and [`Self::interval`] is whether that timer runs at all.
#[derive(Debug)]
pub struct Submitter {
    /// Where each thread puts its [`Returned`].
    sender: Sender<Returned>,
    /// Where the window takes them from.
    receiver: Receiver<Returned>,
    /// How many threads have not answered yet.
    outstanding: usize,
}

impl Default for Submitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Submitter {
    /// A window with nothing on the wire.
    #[must_use]
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            sender,
            receiver,
            outstanding: 0,
        }
    }

    /// Sends one submission on a `submit-form` thread of its own.
    ///
    /// `wake` is called on that thread once the answer is on the channel, for a window whose loop
    /// can be woken from another thread (`winit`'s proxy); a window that polls passes nothing.
    ///
    /// # Errors
    ///
    /// The sentence to say where no thread could be started, in which case nothing was sent.
    pub fn send(
        &mut self,
        document: DocumentId,
        submission: Submission,
        warned: Option<String>,
        wake: Option<Box<dyn Fn() + Send>>,
    ) -> Result<(), String> {
        let sender = self.sender.clone();
        std::thread::Builder::new()
            .name("submit-form".to_owned())
            .spawn(move || {
                let answer = transmit(&submission);
                // A window that has closed has no receiver, and an answer nobody can read is one
                // nobody is owed: the discard is the window's absence rather than a lost error.
                let _ = sender.send(Returned {
                    document,
                    url: submission.url,
                    warned,
                    answer,
                });
                if let Some(wake) = wake {
                    wake();
                }
            })
            .map_err(|error| format!("submit-form: no thread to send it on: {error}"))?;
        self.outstanding = self.outstanding.saturating_add(1);
        Ok(())
    }

    /// Every answer that has arrived since the last look.
    pub fn collect(&mut self) -> Vec<Returned> {
        let arrived: Vec<Returned> = self.receiver.try_iter().collect();
        self.outstanding = self.outstanding.saturating_sub(arrived.len());
        arrived
    }

    /// How long to wait before looking again, or `None` where nothing is on the wire — which is
    /// what stops a window's timer, so an idle window wakes for this never.
    #[must_use]
    pub const fn interval(&self) -> Option<Duration> {
        if self.outstanding == 0 {
            None
        } else {
            Some(LOOK)
        }
    }
}

/// Where a PDF answer is kept so that a window can open it beside the document that sent the form.
///
/// **A file, because every window opens a tab from a path**: `crate::Arrivals` carries one, and a
/// document's path is what §12.7.6.4's import resolves against and what §7.6.4.1's second attempt
/// reads again. The file is created new — never over another — readable by this user alone, in
/// `$XDG_RUNTIME_DIR` where the session has one (the directory the XDG base directory
/// specification gives a user's non-persistent runtime files) and the system's temporary
/// directory otherwise. The window says where it is, so the person can keep it (ADR 1291).
///
/// # Errors
///
/// The sentence to say where it could not be written.
pub fn keep_answer(bytes: &[u8]) -> Result<std::path::PathBuf, String> {
    use std::io::Write as _;

    let directory = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .filter(|directory| directory.is_dir())
        .unwrap_or_else(std::env::temp_dir);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let path = directory.join(format!("quorra-answer-{}-{stamp}.pdf", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(&path).map_err(|error| {
        format!(
            "the answer could not be kept at {}: {error}",
            path.display()
        )
    })?;
    file.write_all(bytes).map_err(|error| {
        format!(
            "the answer could not be kept at {}: {error}",
            path.display()
        )
    })?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::{Reply, Response, reply};
    use pdf_model::action::DataFormat;

    fn answered(status: u16, media: Option<&str>, body: &[u8]) -> Response {
        Response {
            status,
            media_type: media.map(str::to_owned),
            location: None,
            body: body.to_vec(),
        }
    }

    /// §12.7.8.1's own media type, and the registry's name for the same bytes, both import.
    #[test]
    fn an_fdf_answer_is_imported_under_either_name() {
        for media in super::FDF_TYPES {
            match reply("http://h/", answered(200, Some(media), b"%FDF-1.2"), None) {
                Reply::Import { format, .. } => assert_eq!(format, DataFormat::Fdf),
                other => panic!("{media}: {other:?}"),
            }
        }
    }

    /// A PDF opens beside; an error page is said and nothing is imported from it.
    #[test]
    fn a_pdf_opens_and_an_error_is_said() {
        assert!(matches!(
            reply(
                "http://h/",
                answered(200, Some("application/pdf"), b"%PDF"),
                None
            ),
            Reply::Document { .. }
        ));
        match reply(
            "http://h/",
            answered(500, Some("application/vnd.fdf"), b"%FDF"),
            None,
        ) {
            Reply::Say(note) => assert!(note.contains("answered 500"), "{note}"),
            other => panic!("{other:?}"),
        }
        match reply(
            "http://h/",
            answered(200, Some("text/html"), b"<p>thanks</p>"),
            None,
        ) {
            Reply::Say(note) => assert!(note.contains("text/html"), "{note}"),
            other => panic!("{other:?}"),
        }
    }
}
