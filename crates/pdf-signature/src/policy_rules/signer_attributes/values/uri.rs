//! ITU-T X.520 clause 8.9.1's `uriMatch`: two URIs equal once each is normalised as RFC 3986
//! section 6.2.2 says (ADR 3161). X.520 is cited and paraphrased (ADR 1085).
//!
//! The clause lists four normalisations. Case (RFC 3986 section 6.2.2.1): a percent-encoding's
//! hexadecimal digits in upper case — and, by the section the clause conforms to, the scheme and
//! the host in lower case, both being case-insensitive. Percent-encoding (6.2.2.2): an octet that
//! is an unreserved character decoded. Path segments (6.2.2.3): section 5.2.4's
//! `remove_dot_segments`. Scheme-based (6.2.3): a component that is empty, or the scheme's
//! default, removed — an empty port, user information, query or fragment with its delimiter, the
//! default port of a scheme whose specification states one, and for `http`, `https`, `ws` and
//! `wss`, whose specifications make an empty path `/`, that path written `/`.
//!
//! A value is split by RFC 3986 Appendix B's components. One that is no URI by RFC 3986's syntax
//! where this reads it — a code point outside ASCII (a URI is ASCII; an IRI is RFC 3987's), no
//! scheme, a percent sign without two hexadecimal digits, a port that is not digits — is
//! undefined (X.501 clause 8.9.2.1 (a)(3)).

use super::prepare::UTF8_STRING;
use super::structured::answer;
use crate::der::Value;
use crate::trust::names::Match;

/// Default ports, each from its scheme's specification: RFC 9110 sections 4.2.1 and 4.2.2,
/// RFC 6455 section 3, RFC 1738 section 3.2 and RFC 4516 section 2.
const DEFAULT_PORTS: &[(&str, &str)] = &[
    ("http", "80"),
    ("https", "443"),
    ("ws", "80"),
    ("wss", "443"),
    ("ftp", "21"),
    ("ldap", "389"),
];

/// Schemes whose specification makes an empty path `/`: RFC 9110 section 4.2.3 and RFC 6455
/// section 3.
const ROOT_PATH: &[&str] = &["http", "https", "ws", "wss"];

/// RFC 3986 section 2.3's unreserved characters.
fn unreserved(octet: u8) -> bool {
    octet.is_ascii_alphanumeric() || matches!(octet, b'-' | b'.' | b'_' | b'~')
}

/// Sections 6.2.2.1 and 6.2.2.2 over one component: each triplet's digits in upper case, and a
/// triplet that encodes an unreserved character decoded. `lower` lowercases everything outside
/// the triplets that remain, as the host is.
fn percent(component: &str, lower: bool) -> Option<String> {
    let mut out = String::with_capacity(component.len());
    let mut rest = component.as_bytes();
    while let Some((&octet, tail)) = rest.split_first() {
        if octet == b'%' {
            let [high, low, tail @ ..] = tail else {
                return None;
            };
            let digits = [*high, *low];
            if !digits.iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            let text = std::str::from_utf8(&digits).ok()?;
            let decoded = u8::from_str_radix(text, 16).ok()?;
            if unreserved(decoded) {
                let decoded = char::from(decoded);
                out.push(if lower {
                    decoded.to_ascii_lowercase()
                } else {
                    decoded
                });
            } else {
                out.push('%');
                out.push_str(&text.to_ascii_uppercase());
            }
            rest = tail;
        } else {
            let plain = char::from(octet);
            out.push(if lower {
                plain.to_ascii_lowercase()
            } else {
                plain
            });
            rest = tail;
        }
    }
    Some(out)
}

/// Section 5.2.4's `remove_dot_segments`.
pub(super) fn remove_dot_segments(path: &str) -> String {
    fn drop_last(output: &mut String) {
        let at = output.rfind('/').unwrap_or(0);
        output.truncate(at);
    }
    let mut input = path;
    let mut output = String::with_capacity(path.len());
    while !input.is_empty() {
        if let Some(rest) = input
            .strip_prefix("../")
            .or_else(|| input.strip_prefix("./"))
        {
            input = rest;
        } else if input.starts_with("/./") {
            input = &input[2..];
        } else if input == "/." {
            input = "/";
        } else if input.starts_with("/../") {
            input = &input[3..];
            drop_last(&mut output);
        } else if input == "/.." {
            input = "/";
            drop_last(&mut output);
        } else if input == "." || input == ".." {
            input = "";
        } else {
            // The segment runs to the next '/' after its own leading one.
            let start = usize::from(input.starts_with('/'));
            let end = input
                .char_indices()
                .skip(start)
                .find(|&(_, c)| c == '/')
                .map_or(input.len(), |(at, _)| at);
            output.push_str(&input[..end]);
            input = &input[end..];
        }
    }
    output
}

/// Section 3.1's scheme: a letter, then letters, digits, `+`, `-` and `.`.
fn scheme(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
}

/// Section 3.2's authority, normalised: user information, host and port.
fn authority(text: &str, scheme: &str) -> Option<String> {
    let (user, host_port) = match text.rsplit_once('@') {
        Some((user, host_port)) => (Some(user), host_port),
        None => (None, text),
    };
    // An IP literal is bracketed and may hold colons; any other host's port follows its last.
    let (host, port) = if host_port.starts_with('[') {
        let close = host_port.find(']')?;
        let (host, after) = (host_port.get(..=close)?, host_port.get(close..)?);
        let after = after.strip_prefix(']')?;
        let port = match after.strip_prefix(':') {
            Some(port) => Some(port),
            None if after.is_empty() => None,
            None => return None,
        };
        (host, port)
    } else {
        match host_port.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (host_port, None),
        }
    };
    if !port.is_none_or(|port| port.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let mut out = String::new();
    if let Some(user) = user.filter(|user| !user.is_empty()) {
        out.push_str(&percent(user, false)?);
        out.push('@');
    }
    out.push_str(&percent(host, true)?);
    let default = DEFAULT_PORTS
        .iter()
        .find(|(name, _)| *name == scheme)
        .map(|&(_, port)| port);
    if let Some(port) = port.filter(|port| !port.is_empty() && Some(*port) != default) {
        out.push(':');
        out.push_str(port);
    }
    Some(out)
}

/// `text` normalised by section 6.2.2, or `None` where it is no URI.
pub(super) fn normalized(text: &str) -> Option<String> {
    if !text.is_ascii() {
        return None;
    }
    // Appendix B: scheme, then the fragment after the first `#`, the query after the first `?`
    // before it, and an authority after `//` up to the path's first `/`.
    let (name, rest) = text.split_once(':')?;
    if !scheme(name) {
        return None;
    }
    let name = name.to_ascii_lowercase();
    let (rest, fragment) = match rest.split_once('#') {
        Some((rest, fragment)) => (rest, Some(fragment)),
        None => (rest, None),
    };
    let (rest, query) = match rest.split_once('?') {
        Some((rest, query)) => (rest, Some(query)),
        None => (rest, None),
    };
    let (authority_text, path) = match rest.strip_prefix("//") {
        Some(after) => {
            let end = after.find('/').unwrap_or(after.len());
            (Some(&after[..end]), &after[end..])
        }
        None => (None, rest),
    };
    let mut out = name.clone();
    out.push(':');
    if let Some(authority_text) = authority_text {
        out.push_str("//");
        out.push_str(&authority(authority_text, &name)?);
    }
    let path = remove_dot_segments(&percent(path, false)?);
    if path.is_empty() && authority_text.is_some() && ROOT_PATH.contains(&name.as_str()) {
        out.push('/');
    } else {
        out.push_str(&path);
    }
    for (delimiter, component) in [('?', query), ('#', fragment)] {
        if let Some(component) = component.filter(|component| !component.is_empty()) {
            out.push(delimiter);
            out.push_str(&percent(component, false)?);
        }
    }
    Some(out)
}

/// Clause 8.9.1's `uriMatch`.
pub(super) fn uri(carried: &Value<'_>, stated: &Value<'_>) -> Match {
    let normal = |value: &Value<'_>| {
        (value.identifier == UTF8_STRING)
            .then(|| std::str::from_utf8(value.contents).ok())
            .flatten()
            .and_then(normalized)
    };
    match (normal(carried), normal(stated)) {
        (Some(carried), Some(stated)) => answer(carried == stated),
        _ => Match::Undecided,
    }
}
