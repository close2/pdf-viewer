//! Resolving a URI reference against a base, for ISO 32000-2 §12.6.4.8's URI actions.
//!
//! The clause states two things about a URI and defers the rest. It says what the entry *is*
//! — Table 210's `/URI` is "[t]he uniform resource identifier to resolve, encoded in UTF-8",
//! described "in Internet RFC 3986" — and it says that Table 211's `/Base` is what a partial
//! URI is interpreted relative to. Neither sentence states how a partial reference and a base
//! combine, because RFC 3986 section 5 already does.
//!
//! So this module holds no PDF at all: it is RFC 3986 section 5.2's reference
//! transformation and section 5.3's recomposition, over strings. [`crate::action`] reads the
//! two entries and calls it.
//!
//! # Why write it out rather than take the obvious shortcut
//!
//! The shortcut — concatenate the base and the reference — is right for exactly the shape
//! `issue14802.pdf` writes and wrong for four of RFC 3986 section 5.4's twenty-four examples. A
//! reference beginning `/` replaces the base's whole path, one beginning `//` replaces its
//! authority, one that is empty means the base itself, and `..` segments have to be removed
//! after merging rather than before. The RFC's section 5.4 examples are normative and are this
//! module's tests, which is the same reason §12.4.2's own worked example is `page_label.rs`'s.
//!
//! # What this module does not do
//!
//! It does not normalise a URI (RFC 3986 section 6), percent-decode it, or check that it is
//! syntactically valid. A URI a document states is handed onward as the document wrote it,
//! because a viewer's browser is the thing that parses it and a reader that "corrects" a URI
//! has changed where a link goes.

/// A URI reference split into RFC 3986 section 3's five components.
///
/// `None` distinguishes an absent component from an empty one throughout, because the two
/// differ: `http://a` has an authority and an empty path, `mailto:a@b` has no authority at
/// all, and section 5.2.2 branches on which.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Parts<'a> {
    scheme: Option<&'a str>,
    authority: Option<&'a str>,
    path: &'a str,
    query: Option<&'a str>,
    fragment: Option<&'a str>,
}

/// Whether this reference states a scheme, and so is a URI rather than a relative reference.
///
/// RFC 3986 section 4.1 makes that the whole distinction: a URI reference is either a URI or a
/// relative reference, and it is a URI exactly when it begins with a scheme. §12.6.4.8's
/// sentence about partial URIs is a sentence about the second case.
#[must_use]
pub fn is_absolute(reference: &str) -> bool {
    split(reference).scheme.is_some()
}

/// RFC 3986 section 5.2: `reference` resolved against `base`.
///
/// The base is used only for what the reference does not state, which is what makes this an
/// algorithm rather than a concatenation. A reference with its own scheme ignores the base
/// entirely, and so does the empty-base case: with nothing to resolve against, the reference
/// is its own answer.
#[must_use]
pub fn resolve(base: &str, reference: &str) -> String {
    let base = split(base);
    let reference = split(reference);
    if base.scheme.is_none() {
        // Section 5.2.1 requires the base to be an absolute URI. A `/Base` that is not one
        // states nothing this algorithm can use, and inventing a scheme for it would be
        // inventing where the link goes.
        return recompose(&reference);
    }

    // section 5.2.2's transformation, in the order the RFC's pseudocode states its branches.
    let target = if reference.scheme.is_some() {
        Parts {
            path: &remove_dot_segments(reference.path),
            ..reference
        }
        .owned()
    } else if reference.authority.is_some() {
        Parts {
            scheme: base.scheme,
            path: &remove_dot_segments(reference.path),
            ..reference
        }
        .owned()
    } else if reference.path.is_empty() {
        Owned {
            scheme: base.scheme.map(str::to_owned),
            authority: base.authority.map(str::to_owned),
            path: base.path.to_owned(),
            query: reference.query.or(base.query).map(str::to_owned),
            fragment: reference.fragment.map(str::to_owned),
        }
    } else {
        let path = if reference.path.starts_with('/') {
            remove_dot_segments(reference.path)
        } else {
            remove_dot_segments(&merge(&base, reference.path))
        };
        Owned {
            scheme: base.scheme.map(str::to_owned),
            authority: base.authority.map(str::to_owned),
            path,
            query: reference.query.map(str::to_owned),
            fragment: reference.fragment.map(str::to_owned),
        }
    };
    target.recompose()
}

/// The same five components, owning their text, which section 5.2.2's branches need.
struct Owned {
    scheme: Option<String>,
    authority: Option<String>,
    path: String,
    query: Option<String>,
    fragment: Option<String>,
}

impl Parts<'_> {
    /// This reference with its parts copied, so that a computed path can replace one of them.
    fn owned(&self) -> Owned {
        Owned {
            scheme: self.scheme.map(str::to_owned),
            authority: self.authority.map(str::to_owned),
            path: self.path.to_owned(),
            query: self.query.map(str::to_owned),
            fragment: self.fragment.map(str::to_owned),
        }
    }
}

impl Owned {
    /// RFC 3986 section 5.3's recomposition.
    fn recompose(&self) -> String {
        recompose(&Parts {
            scheme: self.scheme.as_deref(),
            authority: self.authority.as_deref(),
            path: &self.path,
            query: self.query.as_deref(),
            fragment: self.fragment.as_deref(),
        })
    }
}

/// RFC 3986 section 5.3: the five components written back out as one string.
fn recompose(parts: &Parts<'_>) -> String {
    let mut out = String::new();
    if let Some(scheme) = parts.scheme {
        out.push_str(scheme);
        out.push(':');
    }
    if let Some(authority) = parts.authority {
        out.push_str("//");
        out.push_str(authority);
    }
    out.push_str(parts.path);
    if let Some(query) = parts.query {
        out.push('?');
        out.push_str(query);
    }
    if let Some(fragment) = parts.fragment {
        out.push('#');
        out.push_str(fragment);
    }
    out
}

/// RFC 3986 section 3's grammar, read as far as splitting a reference into its five components.
///
/// The scheme is the one component that has to be recognised rather than located: a colon
/// appears in plenty of paths, so `foo.bar.com:8080` is a scheme only if everything before
/// the colon is a letter followed by letters, digits, `+`, `-` and `.`. That test is what
/// keeps `pr19449.pdf`'s `foo.bar.com` a relative reference — it has no colon at all — and it
/// is also what stops a path with a colon in its first segment from being read as a scheme.
fn split(reference: &str) -> Parts<'_> {
    let (rest, fragment) = match reference.split_once('#') {
        Some((before, after)) => (before, Some(after)),
        None => (reference, None),
    };
    let (rest, query) = match rest.split_once('?') {
        Some((before, after)) => (before, Some(after)),
        None => (rest, None),
    };

    let mut scheme = None;
    let mut rest = rest;
    if let Some(colon) = rest.find(':')
        && let Some(candidate) = rest.get(..colon)
        && is_scheme(candidate)
    {
        scheme = Some(candidate);
        rest = rest.get(colon.saturating_add(1)..).unwrap_or_default();
    }

    let mut authority = None;
    if let Some(after) = rest.strip_prefix("//") {
        let end = after.find('/').unwrap_or(after.len());
        authority = after.get(..end);
        rest = after.get(end..).unwrap_or_default();
    }

    Parts {
        scheme,
        authority,
        path: rest,
        query,
        fragment,
    }
}

/// RFC 3986 section 3.1's scheme production: `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`.
fn is_scheme(candidate: &str) -> bool {
    let mut characters = candidate.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && characters.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// RFC 3986 section 5.2.3: the base's path with the reference's relative path merged onto it.
///
/// The clause's own two cases: a base with an authority and an empty path contributes just
/// the leading slash, and every other base contributes everything up to and including its
/// last slash — so `http://a/b/c/d;p?q` and `g` give `/b/c/g` rather than `/b/c/d;pg`.
fn merge(base: &Parts<'_>, reference: &str) -> String {
    if base.authority.is_some() && base.path.is_empty() {
        return format!("/{reference}");
    }
    match base.path.rfind('/') {
        Some(slash) => {
            let keep = base.path.get(..=slash).unwrap_or_default();
            format!("{keep}{reference}")
        }
        None => reference.to_owned(),
    }
}

/// RFC 3986 section 5.2.4's `remove_dot_segments`, as the five prefix cases the RFC states.
///
/// Written out case by case rather than as a walk over segments, because the segment
/// formulation gets two of them wrong: an input of `..` alone contributes nothing at all
/// while `/a/..` ends in a slash, and a `..` with nothing left to pop is dropped rather than
/// climbing above the root. The RFC's cases decide both without a special case, and its own
/// section 5.2.4 examples are the test.
fn remove_dot_segments(path: &str) -> String {
    let mut input = path.to_owned();
    let mut output = String::with_capacity(path.len());
    while !input.is_empty() {
        // A: a leading `../` or `./` refers to nothing this path can climb from.
        if input.starts_with("../") {
            input = input.split_off(3);
        } else if input.starts_with("./") {
            input = input.split_off(2);
        // B: `/./` and a trailing `/.` are the current segment, which is the one before them.
        } else if input.starts_with("/./") {
            input.replace_range(..3, "/");
        } else if input == "/." {
            input.replace_range(..2, "/");
        // C: `/../` and a trailing `/..` remove the segment already in the output.
        } else if input.starts_with("/../") {
            input.replace_range(..4, "/");
            pop_segment(&mut output);
        } else if input == "/.." {
            input.replace_range(..3, "/");
            pop_segment(&mut output);
        // D: the whole of what is left is a dot segment, so nothing is left.
        } else if input == "." || input == ".." {
            input.clear();
        } else {
            // E: move one segment, with its leading slash, from the input to the output.
            let after_slash = usize::from(input.starts_with('/'));
            let end = input
                .get(after_slash..)
                .and_then(|rest| rest.find('/'))
                .map_or(input.len(), |at| at.saturating_add(after_slash));
            let rest = input.split_off(end);
            output.push_str(&input);
            input = rest;
        }
    }
    output
}

/// Removes the last segment of `output` and the slash before it: RFC 3986 section 5.2.4's
/// case C.
fn pop_segment(output: &mut String) {
    match output.rfind('/') {
        Some(slash) => output.truncate(slash),
        None => output.clear(),
    }
}

#[cfg(test)]
mod tests {
    use super::{is_absolute, remove_dot_segments, resolve};

    /// RFC 3986 section 5.4.1's normal examples, against the RFC's own base.
    ///
    /// Normative test vectors for the algorithm §12.6.4.8 defers to, which is worth more than
    /// any case this project could invent: each line is a shape of reference the RFC chose to
    /// pin, and four of them are wrong under the concatenation a reader reaches for first.
    #[test]
    fn rfc_3986_normal_examples() {
        let base = "http://a/b/c/d;p?q";
        for (reference, expected) in [
            ("g:h", "g:h"),
            ("g", "http://a/b/c/g"),
            ("./g", "http://a/b/c/g"),
            ("g/", "http://a/b/c/g/"),
            ("/g", "http://a/g"),
            ("//g", "http://g"),
            ("?y", "http://a/b/c/d;p?y"),
            ("g?y", "http://a/b/c/g?y"),
            ("#s", "http://a/b/c/d;p?q#s"),
            ("g#s", "http://a/b/c/g#s"),
            ("g?y#s", "http://a/b/c/g?y#s"),
            (";x", "http://a/b/c/;x"),
            ("g;x", "http://a/b/c/g;x"),
            ("g;x?y#s", "http://a/b/c/g;x?y#s"),
            ("", "http://a/b/c/d;p?q"),
            (".", "http://a/b/c/"),
            ("./", "http://a/b/c/"),
            ("..", "http://a/b/"),
            ("../", "http://a/b/"),
            ("../g", "http://a/b/g"),
            ("../..", "http://a/"),
            ("../../", "http://a/"),
            ("../../g", "http://a/g"),
        ] {
            assert_eq!(resolve(base, reference), expected, "{reference:?}");
        }
    }

    /// RFC 3986 section 5.4.2's abnormal examples, which are where a hand-written merge fails.
    ///
    /// The RFC's own comment on the first group is the reason they are here: a `..` that
    /// would climb above the root is removed, "since parsers must be consistent in how they
    /// handle such references" whether or not the reference makes sense.
    #[test]
    fn rfc_3986_abnormal_examples() {
        let base = "http://a/b/c/d;p?q";
        for (reference, expected) in [
            ("../../../g", "http://a/g"),
            ("../../../../g", "http://a/g"),
            ("/./g", "http://a/g"),
            ("/../g", "http://a/g"),
            ("g.", "http://a/b/c/g."),
            (".g", "http://a/b/c/.g"),
            ("g..", "http://a/b/c/g.."),
            ("..g", "http://a/b/c/..g"),
            ("./../g", "http://a/b/g"),
            ("./g/.", "http://a/b/c/g/"),
            ("g/./h", "http://a/b/c/g/h"),
            ("g/../h", "http://a/b/c/h"),
            ("g;x=1/./y", "http://a/b/c/g;x=1/y"),
            ("g;x=1/../y", "http://a/b/c/y"),
            ("g?y/./x", "http://a/b/c/g?y/./x"),
            ("g?y/../x", "http://a/b/c/g?y/../x"),
            ("g#s/./x", "http://a/b/c/g#s/./x"),
            ("g#s/../x", "http://a/b/c/g#s/../x"),
        ] {
            assert_eq!(resolve(base, reference), expected, "{reference:?}");
        }
    }

    /// RFC 3986 section 5.2.4's own examples of the segment removal, checked on their own.
    #[test]
    fn dot_segments_are_removed_as_the_rfc_states() {
        assert_eq!(remove_dot_segments("/a/b/c/./../../g"), "/a/g");
        assert_eq!(remove_dot_segments("mid/content=5/../6"), "mid/6");
        assert_eq!(remove_dot_segments("/../"), "/");
    }

    /// `issue14802.pdf`'s own pair: a `/Base` and a reference that needs it.
    ///
    /// The one corpus document that states Table 211's `/Base`, and the one whose `/URI` is a
    /// relative reference to a file beside it.
    #[test]
    fn the_corpus_documents_base_resolves_its_relative_reference() {
        assert_eq!(
            resolve("http://example.com/", "./relative_link.txt"),
            "http://example.com/relative_link.txt"
        );
    }

    /// A base that is not an absolute URI resolves nothing, and says so by answering the
    /// reference unchanged rather than gluing two relative strings together.
    #[test]
    fn a_base_without_a_scheme_is_not_a_base() {
        assert_eq!(resolve("example.com/", "page.html"), "page.html");
    }

    /// RFC 3986 section 4.1's distinction, which is the one §12.6.4.8 calls a partial URI.
    #[test]
    fn a_scheme_is_what_makes_a_reference_absolute() {
        assert!(is_absolute("https://example.com/a"));
        assert!(is_absolute("mailto:someone@example.com"));
        assert!(!is_absolute("foo.bar.com"));
        assert!(!is_absolute("//example.com/a"));
        assert!(
            !is_absolute("1http://x"),
            "a scheme starts with a letter (RFC 3986 section 3.1)"
        );
    }
}
