//! Canonical XML 1.1 section 2.4's join-URI-References, against the Recommendation's own
//! examples: the four in section 2.4's text and the informative table of its appendix.

use super::{join, remove_dot_segments};

/// The appendix's table of the modified "Remove Dot Segments", input and output, transcribed
/// row by row (an empty output is the table's empty cell).
const APPENDIX: [(&str, &str); 64] = [
    (
        "no/.././/pseudo-netpath/seg/file.ext",
        "pseudo-netpath/seg/file.ext",
    ),
    (
        "no/..//.///pseudo-netpath/seg/file.ext",
        "pseudo-netpath/seg/file.ext",
    ),
    (
        "yes/no//..//.///pseudo-netpath/seg/file.ext",
        "yes/pseudo-netpath/seg/file.ext",
    ),
    ("no/../yes", "yes"),
    ("no/../yes/", "yes/"),
    ("no/../yes/no/..", "yes/"),
    ("../../no/../..", "../../../"),
    ("no/../..", "../"),
    ("no/..", ""),
    ("no/../", ""),
    ("/a/b/c/./../../g", "/a/g"),
    ("mid/content=5/../6", "mid/6"),
    ("../../..", "../../../"),
    ("no/../../", "../"),
    ("..yes/..no/..no/..no/../../../..yes", "..yes/..yes"),
    ("..yes/..no/..no/..no/../../../..yes/", "..yes/..yes/"),
    ("../..", "../../"),
    ("../../../", "../../../"),
    (".", ""),
    ("./", ""),
    ("./.", ""),
    ("//no/..", "/"),
    ("../../no/..", "../../"),
    ("../../no/../", "../../"),
    ("yes/no/../", "yes/"),
    ("yes/no/no/../..", "yes/"),
    ("yes/no/no/no/../../..", "yes/"),
    ("yes/no/../yes/no/no/../..", "yes/yes/"),
    ("yes/no/no/no/../../../yes", "yes/yes"),
    ("yes/no/no/no/../../../yes/", "yes/yes/"),
    ("/no/../", "/"),
    ("/yes/no/../", "/yes/"),
    ("/yes/no/no/../..", "/yes/"),
    ("/yes/no/no/no/../../..", "/yes/"),
    ("../../..no/..", "../../"),
    ("../../..no/../", "../../"),
    ("..yes/..no/../", "..yes/"),
    ("..yes/..no/..no/../..", "..yes/"),
    ("..yes/...no/..no/..no/../../..", "..yes/"),
    ("..yes/..no/../..yes/..no/..no/../..", "..yes/..yes/"),
    ("/..no/../", "/"),
    ("/..yes/..no/../", "/..yes/"),
    ("/..yes/..no/..no/../..", "/..yes/"),
    ("/..yes/..no/..no/..no/../../..", "/..yes/"),
    ("/", "/"),
    ("/.", "/"),
    ("/./", "/"),
    ("/./.", "/"),
    ("/././", "/"),
    ("/..", "/"),
    ("/../..", "/"),
    ("/../../..", "/"),
    ("/../../..", "/"),
    ("//..", "/"),
    ("//..//..", "/"),
    ("//..//..//..", "/"),
    ("/./..", "/"),
    ("/./.././..", "/"),
    ("/./.././.././..", "/"),
    (".", ""),
    ("./", ""),
    ("./.", ""),
    ("..", "../"),
    ("../", "../"),
];

#[test]
fn the_appendix_table_of_remove_dot_segments_holds_row_by_row() {
    for (input, output) in APPENDIX {
        assert_eq!(remove_dot_segments(input), output, "{input}");
    }
}

#[test]
fn section_2_4_s_examples_of_joining_hold() {
    // "abc/" and "../" result in "", "../" and "../" in "../../", ".." and ".." in "../../".
    assert_eq!(join("abc/", "../"), "");
    assert_eq!(join("../", "../"), "../../");
    assert_eq!(join("..", ".."), "../../");
    // The document `a`, `b`, `c`, `d` with `b` and `c` omitted: `d`'s value is the reduction of
    // "..", ".." and "x", innermost first, which the section gives as "../../x".
    assert_eq!(join("..", &join("..", "x")), "../../x");
}

#[test]
fn a_reference_with_its_own_scheme_or_authority_or_root_replaces_the_base_s_path() {
    // RFC 3986 section 5.2.2's first three branches, which section 2.4 leaves as they are, and
    // the fragment section 2.4 drops.
    assert_eq!(join("http://a/b/c", "https://d/e/../f"), "https://d/f");
    assert_eq!(join("http://a/b/c", "//d/e"), "http://d/e");
    assert_eq!(join("http://a/b/c", "/e/./f"), "http://a/e/f");
    assert_eq!(join("http://a/b/c?q", ""), "http://a/b/c?q");
    assert_eq!(join("http://a/b/c", "d#part"), "http://a/b/d");
    assert_eq!(join("http://a", "d"), "http://a/d");
}
