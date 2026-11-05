//! W3C Canonical XML 1.1 (2008) section 2.4's `xml:base` fix-up: the "join-URI-References"
//! function, which folds the `xml:base` values of the elements a document subset omits into the
//! value of the element below them that it keeps.
//!
//! The Recommendation defines the function as RFC 3986 sections 5.2.1, 5.2.2 and 5.2.4 with five
//! changes, each paraphrased where the code makes it: the base needs no scheme; a base whose last
//! segment is `..` has a `/` appended before it is used; the fragment of the reference is
//! dropped; and "Remove Dot Segments" keeps leading `../` segments, collapses runs of `/` into one
//! and appends a `/` to a trailing `..` — so that two relative values combine into a relative
//! value rather than climbing past their own start. The informative table of the Recommendation's
//! appendix is this module's test of the last.

#[cfg(test)]
mod tests;

/// A URI reference split as RFC 3986 appendix B's expression splits it, its fragment dropped
/// because section 2.4 discards the reference's and the base's never reaches the result.
struct Reference<'a> {
    scheme: Option<&'a str>,
    authority: Option<&'a str>,
    path: &'a str,
    query: Option<&'a str>,
}

impl<'a> Reference<'a> {
    fn split(text: &'a str) -> Self {
        let text = text.split_once('#').map_or(text, |(before, _)| before);
        let (scheme, rest) = match text.find([':', '/', '?']).map(|at| text.split_at(at)) {
            Some((scheme, tail)) if !scheme.is_empty() && tail.starts_with(':') => {
                (Some(scheme), tail.get(1..).unwrap_or_default())
            }
            _ => (None, text),
        };
        let (authority, rest) = match rest.strip_prefix("//") {
            Some(after) => {
                let end = after.find(['/', '?']).unwrap_or(after.len());
                (Some(&after[..end]), &after[end..])
            }
            None => (None, rest),
        };
        let (path, query) = match rest.split_once('?') {
            Some((path, query)) => (path, Some(query)),
            None => (rest, None),
        };
        Self {
            scheme,
            authority,
            path,
            query,
        }
    }
}

/// Section 2.4's "join-URI-References": `reference` resolved against `base`, both `xml:base`
/// values, the outer one the base.
///
/// RFC 3986 section 5.2.2's transformation with section 2.4's changes; the result is recomposed
/// by section 5.3 without a fragment.
#[must_use]
pub fn join(base: &str, reference: &str) -> String {
    let reference = Reference::split(reference);
    // Section 2.4's change to RFC 3986 section 5.2.1: a trailing `..` segment of the base is
    // read as `../`, so that the merge below keeps it rather than replacing it.
    let base_text;
    let base = {
        let split = Reference::split(base);
        if split.path == ".." || split.path.ends_with("/..") {
            base_text = format!("{}/", split.path);
            Reference {
                path: &base_text,
                ..split
            }
        } else {
            split
        }
    };
    let (scheme, authority, path, query);
    if reference.scheme.is_some() {
        scheme = reference.scheme;
        authority = reference.authority;
        path = remove_dot_segments(reference.path);
        query = reference.query;
    } else {
        scheme = base.scheme;
        if reference.authority.is_some() {
            authority = reference.authority;
            path = remove_dot_segments(reference.path);
            query = reference.query;
        } else {
            authority = base.authority;
            if reference.path.is_empty() {
                path = base.path.to_owned();
                query = reference.query.or(base.query);
            } else {
                path = if reference.path.starts_with('/') {
                    remove_dot_segments(reference.path)
                } else {
                    remove_dot_segments(&merge(&base, reference.path))
                };
                query = reference.query;
            }
        }
    }
    let mut joined = String::new();
    if let Some(scheme) = scheme {
        joined.push_str(scheme);
        joined.push(':');
    }
    if let Some(authority) = authority {
        joined.push_str("//");
        joined.push_str(authority);
    }
    joined.push_str(&path);
    if let Some(query) = query {
        joined.push('?');
        joined.push_str(query);
    }
    joined
}

/// RFC 3986 section 5.2.3: the reference's path appended to the base's without its last segment.
fn merge(base: &Reference<'_>, path: &str) -> String {
    if base.authority.is_some() && base.path.is_empty() {
        return format!("/{path}");
    }
    match base.path.rfind('/') {
        Some(at) => format!("{}{path}", &base.path[..=at]),
        None => path.to_owned(),
    }
}

/// RFC 3986 section 5.2.4's "Remove Dot Segments" as section 2.4 modifies it.
///
/// A segment stack rather than the RFC's buffer-to-buffer loop, because the modification is
/// stated as outcomes — leading `../` kept, `/` runs collapsed, a trailing `..` given its `/` —
/// and a stack states each of them as one branch: `.` is dropped, `..` removes the segment before
/// it where there is one to remove, is dropped at the root of an absolute path, and is kept at the
/// start of a relative one.
#[must_use]
pub fn remove_dot_segments(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut segments: Vec<&str> = Vec::new();
    let mut directory = false;
    for segment in path.split('/') {
        directory = matches!(segment, "" | "." | "..");
        match segment {
            "" | "." => {}
            ".." => match segments.last() {
                Some(&last) if last != ".." => {
                    segments.pop();
                }
                _ if absolute => {}
                _ => segments.push(".."),
            },
            other => segments.push(other),
        }
    }
    let mut out = String::with_capacity(path.len().saturating_add(1));
    if absolute {
        out.push('/');
    }
    out.push_str(&segments.join("/"));
    if directory && !segments.is_empty() {
        out.push('/');
    }
    out
}
