//! The two packagings of an event's `submit` that are XML of the form's own: XFA 3.3 page 859's
//! `xdp` and `xml` (ADR 2771).
//!
//! **Both are written from the document a save would write**, never from a second account of the
//! data. §K.2 binds a writer of a hybrid file to keep the `datasets` packet consistent with the
//! `/V` entries, and [`crate::view::ViewState::save`] is where this program does that; a submission
//! that composed its data some other way could send values the saved file does not hold. So the
//! caller saves, opens what it wrote, and hands that document here.
//!
//! - **`xdp`**: "\[t]he data is packaged in XDP format, as described in XDP Specification" (page
//!   859). Chapter 23 makes an XFA resource an XDP document already — §K.2 says the resource's
//!   format "is described in the XML Data Package (XDP) Specification", and its array form's
//!   first and last packets are the `xdp:xdp` element's tags — so the body is that document with
//!   the packets `xdpContent` selects (page 861), its root's start tag the producer's own, with
//!   the `uuid` and `timeStamp` page 1042 has every later serialiser keep. Two packets are not
//!   the resource's and are written here: `pdf` (page 1039), the saved file base64-encoded where
//!   `embedPDF` is `1` and empty where it is `0`, because page 858's URI comes from the XFDF
//!   `F`-key path of the system that made the file and this process is given no path; and `xfdf`
//!   (page 1042), the collaboration annotations as XFDF, which this program does not write, so it
//!   is written empty where the document holds no markup annotation and left out, and said, where
//!   it holds some.
//! - **`xml`**: "\[t]he data is packaged in XML format … The schema is determined according to the
//!   same rules used for data unloading" (page 859). Page 149's unloader produces "an XML data
//!   document which reflects the contents of the XML Data DOM", which in a `datasets` packet is the
//!   one element `xfa:data` holds; the body is that element with the namespaces its packet had
//!   declared around it, under an XML declaration.
//!
//! **What is not decided by the pages is a choice, and it is written as one.** `xdpContent` is
//! cdata, so page 597's underline names no default for it; an omitted one is read as page 861's
//! first value, `datasets pdf xfdf`, the content its own description gives first. `*` is "\[a]ll
//! data elements", read as every packet. And the `xml` body's media type
//! ([`crate::submission::Format::content_type`]) is `text/xml`, the one page 1038 gives the
//! `datasets` packet the data is unloaded from; the `xdp` body's is page 1041's registered
//! `application/vnd.adobe.xdp+xml`.

use std::fmt::Write as _;

use pdf_syntax::Document;

/// The namespace page 1039 gives the `pdf` packet.
const PDF_NAMESPACE: &str = "http://ns.adobe.com/xdp/pdf/";
/// The namespace page 1042 gives the `xfdf` packet.
const XFDF_NAMESPACE: &str = "http://ns.adobe.com/xfdf/";
/// The XML declaration every body starts with: both are written in UTF-8, the encoding page 860
/// names last, and a `textEncoding` asking for another is said where the `submit` is read.
const DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
/// RFC 2045 section 6.8: "\[t]he encoded output stream must be represented in lines of no more
/// than 76 characters each", the encoding page 1039 names for the `pdf` packet's `chunk`.
const BASE64_LINE: usize = 76;

/// Which packets an `xdp` body holds: page 861's `xdpContent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    /// `*`: "\[a]ll data elements are submitted to the host" — every packet.
    All,
    /// "Elements with tags matching any of the specified tags are submitted to the host", by
    /// local name.
    Tags(Vec<String>),
}

impl Content {
    /// Reads `xdpContent`: its tags, separated by white space; `*` for every packet; and, where it
    /// is omitted or blank, `datasets pdf xfdf` (the module's choice).
    pub(crate) fn read(stated: Option<&str>) -> Self {
        let tags: Vec<String> = stated
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        if tags.iter().any(|tag| tag == "*") {
            Self::All
        } else if tags.is_empty() {
            Self::Tags(["datasets", "pdf", "xfdf"].map(str::to_owned).to_vec())
        } else {
            Self::Tags(tags)
        }
    }

    /// Whether the packet of this local name is submitted.
    fn holds(&self, local: &str) -> bool {
        match self {
            Self::All => true,
            Self::Tags(tags) => tags.iter().any(|tag| tag == local),
        }
    }
}

/// Which of the two packagings, with what page 858 lets the `xdp` one vary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packaging {
    /// `format="xdp"`, the default page 858 underlines.
    Xdp {
        /// `xdpContent` (page 861).
        content: Content,
        /// `embedPDF` (page 858): whether the `pdf` packet holds the document.
        embed_pdf: bool,
    },
    /// `format="xml"`.
    Xml,
}

/// A `submit` whose body is one of the two packagings: where it is posted, and how it is packaged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packaged {
    /// The `submit`'s `target` (page 860).
    pub url: String,
    /// The body's packaging.
    pub packaging: Packaging,
}

impl Packaging {
    /// The body, written from `saved`, the document a save of the form writes, whose bytes are
    /// `saved_bytes`; what the packaging asked for that it does not hold is pushed onto `owed`.
    ///
    /// # Errors
    ///
    /// The sentence saying why no body can be written: the saved file states no readable XFA
    /// resource, its root is not `xdp:xdp`, or its data is not one XML document.
    pub(crate) fn body(
        &self,
        saved: &Document,
        saved_bytes: &[u8],
        owed: &mut Vec<String>,
    ) -> Result<String, String> {
        let (text, tree) = match super::resource_text(saved) {
            None => return Err("the document states no XFA resource".to_owned()),
            Some(Err(error)) => return Err(error.to_string()),
            Some(Ok(read)) => read,
        };
        let root = tree
            .get(0)
            .filter(|root| root.local == "xdp")
            .ok_or_else(|| {
                "the XFA resource's root element is not the xdp element page 1041 makes the root \
                 of an XDP document"
                    .to_owned()
            })?;
        match self {
            Self::Xml => {
                let datasets = tree
                    .child(0, "datasets")
                    .ok_or_else(|| "the XFA resource holds no datasets packet".to_owned())?;
                let data = tree
                    .child(datasets, "data")
                    .and_then(|at| tree.get(at))
                    .ok_or_else(|| "the datasets packet holds no xfa:data element".to_owned())?;
                let [only] = data.children.as_slice() else {
                    return Err(format!(
                        "the form's data holds {} elements where an XML data document has one \
                         root (page 149's unloading)",
                        data.children.len()
                    ));
                };
                let element = tree.extracted(&text, *only).ok_or_else(unreadable)?;
                Ok(format!("{DECLARATION}{element}\n"))
            }
            Self::Xdp { content, embed_pdf } => {
                let mut out = String::from(DECLARATION);
                // The start tag without its `>` or `/>`: an empty root gains the packets below.
                out.push_str(
                    text.get(root.start..root.start_end.start)
                        .ok_or_else(unreadable)?,
                );
                out.push_str(">\n");
                let mut present = Vec::new();
                for &child in &root.children {
                    let packet = tree.get(child).ok_or_else(unreadable)?;
                    present.push(packet.local.as_str());
                    // These two are the document's, written below from the file this submits.
                    if matches!(packet.local.as_str(), "pdf" | "xfdf")
                        || !content.holds(&packet.local)
                    {
                        continue;
                    }
                    out.push_str(text.get(packet.start..packet.end).ok_or_else(unreadable)?);
                    out.push('\n');
                }
                if let Content::Tags(tags) = content {
                    for tag in tags {
                        if !matches!(tag.as_str(), "pdf" | "xfdf")
                            && !present.contains(&tag.as_str())
                        {
                            owed.push(format!(
                                "the submit's xdpContent names {tag}, and the form's XFA resource \
                                 holds no packet of that name, so none is sent (XFA 3.3 page 861)"
                            ));
                        }
                    }
                }
                if content.holds("pdf") {
                    if *embed_pdf {
                        let _ = write!(
                            out,
                            "<pdf xmlns=\"{PDF_NAMESPACE}\"><document><chunk>\n{}</chunk></document></pdf>\n",
                            base64(saved_bytes)
                        );
                    } else {
                        let _ = writeln!(out, "<pdf xmlns=\"{PDF_NAMESPACE}\"/>");
                    }
                }
                if content.holds("xfdf") {
                    let markup = markup_annotations(saved);
                    if markup == 0 {
                        let _ = writeln!(
                            out,
                            "<xfdf xmlns=\"{XFDF_NAMESPACE}\" xml:space=\"preserve\"/>"
                        );
                    } else {
                        owed.push(format!(
                            "the xfdf packet (XFA 3.3 page 1042) would carry the document's \
                             {markup} markup annotation(s) as XFDF, which this reader does not \
                             write, so the packet is left out"
                        ));
                    }
                }
                let _ = writeln!(out, "</{}>", root.qualified);
                Ok(out)
            }
        }
    }
}

/// The sentence for a range the parse handed out that does not lie in its own text, which
/// [`super::xml::parse`] cannot produce; said rather than unwrapped.
fn unreadable() -> String {
    "the XFA resource's text does not hold the element ranges read from it".to_owned()
}

/// How many §12.5.6.2 markup annotations the pages of `document` hold.
fn markup_annotations(document: &Document) -> usize {
    let pages = crate::page::Pages::new(document);
    (0..pages.len())
        .filter_map(|ordinal| pages.get(ordinal))
        .map(|page| {
            let listed = document.get_key(&page.dict, "Annots");
            listed.as_array().map_or(0, |items| {
                items
                    .iter()
                    .filter(|item| {
                        document
                            .resolve(item)
                            .as_dict()
                            .is_some_and(|dict| crate::submission::is_markup(document, dict))
                    })
                    .count()
            })
        })
        .sum()
}

/// RFC 2045 section 6.8's base64, in lines of [`BASE64_LINE`] characters, each ended by a line
/// feed: XML's end-of-line handling reads RFC 2045's CRLF as one anyway (XML 1.0 section 2.11),
/// and a base64 decoder ignores the line breaks.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let sextet = |value: u32| char::from(ALPHABET[(value & 63) as usize]);
    let mut out = String::with_capacity(
        bytes
            .len()
            .div_ceil(3)
            .saturating_mul(4)
            .saturating_add(bytes.len().div_ceil(57)),
    );
    let mut line: usize = 0;
    for group in bytes.chunks(3) {
        let (first, second, third) = match *group {
            [a, b, c] => (a, Some(b), Some(c)),
            [a, b] => (a, Some(b), None),
            [a] => (a, None, None),
            // `chunks(3)` hands out one to three bytes.
            _ => continue,
        };
        let joined = u32::from(first) << 16
            | u32::from(second.unwrap_or(0)) << 8
            | u32::from(third.unwrap_or(0));
        out.push(sextet(joined >> 18));
        out.push(sextet(joined >> 12));
        out.push(if second.is_some() {
            sextet(joined >> 6)
        } else {
            '='
        });
        out.push(if third.is_some() { sextet(joined) } else { '=' });
        line = line.saturating_add(4);
        if line == BASE64_LINE {
            out.push('\n');
            line = 0;
        }
    }
    if line > 0 {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Content, base64};

    /// RFC 4648 section 10's test vectors, which RFC 2045 section 6.8's alphabet and padding
    /// produce alike, and a line broken at RFC 2045's 76 characters.
    #[test]
    fn base64_is_rfc_2045_s_in_lines_of_76() {
        for (plain, encoded) in [
            ("", ""),
            ("f", "Zg==\n"),
            ("fo", "Zm8=\n"),
            ("foo", "Zm9v\n"),
            ("foob", "Zm9vYg==\n"),
            ("fooba", "Zm9vYmE=\n"),
            ("foobar", "Zm9vYmFy\n"),
        ] {
            assert_eq!(base64(plain.as_bytes()), encoded, "{plain:?}");
        }
        let long = base64(&[0; 58]);
        let lines: Vec<&str> = long.lines().collect();
        assert_eq!(
            lines.iter().map(|line| line.len()).collect::<Vec<_>>(),
            [76, 4]
        );
    }

    /// Page 861: tags separated by white space, `*` for all; an omitted value is the first one the
    /// page lists (the module's choice).
    #[test]
    fn xdp_content_is_read_as_page_861_lists_it() {
        assert_eq!(
            Content::read(None),
            Content::Tags(vec!["datasets".into(), "pdf".into(), "xfdf".into()])
        );
        assert_eq!(
            Content::read(Some(" pdf  datasets template ")),
            Content::Tags(vec!["pdf".into(), "datasets".into(), "template".into()])
        );
        assert_eq!(Content::read(Some("*")), Content::All);
    }
}
