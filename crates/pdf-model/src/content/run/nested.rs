//! Reading the content a stream holds besides a page's own `/Contents`: §7.8.2's self-contained
//! streams, and §8.9.7's inline image, which is the one construction in a stream that is not a token.

use pdf_syntax::Dictionary;

use crate::content::reader::{ContentReader, NestedContent};
use crate::content::report::{DamagedStream, Unsupported};

use crate::content::Interpreter;

impl Interpreter<'_> {
    /// Decodes one of §7.8.2's self-contained content streams, saying so where it is damaged.
    ///
    /// > Content streams shall also be used to package sequences of instructions as
    /// > self-contained graphical elements, such as forms (see 8.10, "Form XObjects"), patterns
    /// > (8.7, "Patterns"), certain fonts (9.6.4, "Type 3 fonts"), and annotation appearances
    /// > (12.5.5, "Appearance streams").
    ///
    /// Every one of those is "a sequence of instructions" by the same clause's first sentence,
    /// so §7.4.1's two halves are owed here exactly as they are for a page's `/Contents`: the
    /// prefix a damaged filter produced goes on the page, and that it is only a prefix goes in
    /// the report. `what` says which kind and which resource, because each of the five costs a
    /// different mark and a report that did not distinguish them could not be acted on.
    ///
    /// `None` where nothing decoded at all. The caller words that one itself: there the whole
    /// element is missing rather than the end of it, and the five callers already had five
    /// different sentences for it.
    ///
    /// **What comes back is a source rather than a buffer** (ADR 0427): each of the four is
    /// read more than once, so a reader is made per run, and whether the bytes are held whole
    /// or inflated through a window each time is
    /// [`pdf_syntax::Document::nested_content_source`]'s decision. Where they are held whole
    /// the damage is known now and is reported here; where they are windowed it is met during
    /// the run and [`Interpreter::run`] reports it then, in the same words.
    pub(in crate::content) fn content_stream(
        &mut self,
        stream: &pdf_syntax::Stream,
        what: &str,
    ) -> Option<NestedContent> {
        let content = NestedContent::of(self.document, stream, what.to_owned()).ok()?;
        if let Some((damage, kept)) = content.stated_damage() {
            self.note(Unsupported::DamagedContentStream {
                stream: DamagedStream {
                    detail: what.to_owned(),
                    damage,
                    kept,
                },
            });
        }
        Some(content)
    }

    /// Reads the inline image whose `BI` the reader has just consumed, ISO 32000-2 §8.9.7.
    ///
    /// An inline image is the one construction in a content stream that is not a token: `ID`
    /// is followed by data whose length the dictionary need not state, so where it ends is
    /// found by reading it. `examples/token_window_census` measured what that costs a bounded
    /// reader — of **93 930** inline images in 39 976 documents, **90 304 state or imply their
    /// length before their data is read** (336 by §8.9.7's `/L`, 89 968 by §8.9.3's sample
    /// arithmetic), and of the **3 455** that need the forward `EI` search the largest is
    /// **2.99 KiB**.
    ///
    /// So the lookahead starts at the window's own size and doubles only while the answer may
    /// have been cut by it, up to [`crate::content::reader::LOOKAHEAD`]. Past that the image
    /// is refused *by name* rather than read short: the data goes the resource route, whole,
    /// as an image always has, and a bounded reader that stopped looking has said something
    /// different from a file that states no `EI`.
    pub(super) fn inline_image(
        &mut self,
        reader: &mut ContentReader<'_>,
        resources: &Dictionary,
    ) -> crate::inline_image::Scan {
        let bound = crate::content::reader::LOOKAHEAD.min(self.document.limits().max_stream_len);
        let mut want = crate::content::reader::WINDOW;
        loop {
            let (ahead, complete) = reader.lookahead(want);
            let scanned = crate::inline_image::scan(self.document, ahead, 0, resources, complete);
            if complete || scanned.image.is_ok() {
                return scanned;
            }
            if want >= bound {
                return crate::inline_image::Scan {
                    resume: scanned.resume,
                    image: Err(crate::inline_image::InlineImageError::Unbuffered { bound }),
                };
            }
            want = want.saturating_mul(2).min(bound);
        }
    }
}
