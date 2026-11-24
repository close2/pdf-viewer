//! A stream that carries an image: its data with the codec still to be applied.

use std::sync::Arc;

use super::{Document, ImageStream, StreamRefusal};
use crate::object::{Dictionary, Object, Stream};

impl Document {
    /// Returns a stream's data with every filter applied up to a trailing image codec.
    ///
    /// An image stream's chain may mix ordinary filters with a codec:
    /// `[/ASCIIHexDecode /JBIG2Decode]` is the arrangement ISO 32000-2 §7.4.7's own worked
    /// example uses, and `[/FlateDecode /DCTDecode]` occurs in the corpus. Only the last
    /// entry can be a codec — everything before it is a byte-to-byte transformation that
    /// has to run first, and a codec handed still-compressed bytes fails in a way that
    /// reads as a broken image rather than as a missing step.
    ///
    /// **Memoised, through the same table [`Self::decoded_stream_data`] uses**, because this call
    /// is not made once per image. `pdf_model::content::image` asks it of every `Do` — once for
    /// each of §7.4.8's, §7.3.8.2's and §7.4.6's three reports, and once more for the samples on a
    /// raster-cache miss — and a raster cache dies with the page's display list while an image
    /// drawn on forty pages is one stream. Over the 964-document pdf.js corpus 2420 of 2997 image
    /// `XObject`s run a filter here, producing 467.7 MB per pass; ADR 0585 has the A/B.
    ///
    /// # Errors
    ///
    /// Returns `None` when a filter before the codec is unsupported, for the same reason
    /// [`Self::decoded_stream_data`] does; [`Self::image_stream_reported`] says which refusal.
    #[must_use]
    pub fn image_stream(&self, stream: &Stream) -> Option<ImageStream> {
        self.image_stream_reported(stream).ok()
    }

    /// [`Self::image_stream`], with the reason a stream gave no samples.
    ///
    /// The reason is what tells the file's fault from this reader's: a filter this reader runs
    /// that met bytes its grammar does not admit ([`crate::FilterRefusal::Corrupt`]) is a statement
    /// about the stream, where a filter it does not run or a bound it sets is a statement about
    /// the reader, exactly as [`Self::decoded_stream_data_reported`] keeps them apart.
    ///
    /// # Errors
    ///
    /// [`StreamRefusal`], whose variants are the reasons.
    pub fn image_stream_reported(&self, stream: &Stream) -> Result<ImageStream, StreamRefusal> {
        if stream.decryption_failed {
            return Err(StreamRefusal::DecryptionFailed);
        }
        if self.is_external(stream) {
            return Err(StreamRefusal::External);
        }
        // A chain Table 5 does not admit is refused here as `decoded_under` refuses it (ADR 3274).
        let mut chain = self
            .stated_chain(&stream.dict)
            .map_err(StreamRefusal::Chain)?;
        let filters: Vec<Vec<u8>> = chain.iter().map(|(filter, _)| filter.clone()).collect();
        let codec_at = Self::codec_position(&filters);
        let codec = codec_at.and_then(|index| chain.get(index).cloned());

        // The chain in front of the codec, with each stage's own parameters, which is what
        // identifies this decode to the memo — the same construction `decoded_under` makes,
        // over a prefix rather than over the whole of `/Filter`.
        // not a repair: a chain with no codec stage is wholly in front of one, a position this
        // reader computed, not a value the file states.
        chain.truncate(codec_at.unwrap_or(filters.len()));
        let prefix: Vec<(Vec<u8>, Option<Dictionary>)> = chain;
        // An empty chain is not memoised, for the reason `decoded_under` returns early on one:
        // there is nothing to remember, and an entry holding a second `Arc` to the bytes it is
        // keyed by would charge the budget twice for a decode that never ran.
        let data = if prefix.is_empty() {
            Arc::clone(&stream.data)
        } else {
            self.chain_over(&stream.data, prefix, self.limits)?.data
        };

        let (codec, parms) = codec.map_or((None, None), |(codec, parms)| (Some(codec), parms));
        Ok(ImageStream { codec, parms, data })
    }

    /// Which image codec this stream's `/Filter` ends in, decoding nothing.
    ///
    /// [`Self::image_stream`] answers the same question and runs the chain to do it. A caller
    /// that only wants to know *whether* this is a `DCTDecode` — §7.4.8's frame-dimension report
    /// is one, §7.4.6's `/Rows` report another — asks this instead and declines before spending
    /// the decode, which is Table 5 read rather than run.
    ///
    /// `None` for a stream whose chain is empty or does not end in one of §7.4's image codecs, in
    /// which case the decoded bytes *are* the samples — and for a chain Table 5 does not admit,
    /// which every decode of the stream refuses (ADR 3274), so no report this asks before is owed.
    #[must_use]
    pub fn image_codec(&self, stream: &Stream) -> Option<Vec<u8>> {
        let filters = self.filter_chain(&stream.dict).ok()?;
        Self::codec_position(&filters).and_then(|index| filters.get(index).cloned())
    }

    /// Where §7.4's image codec sits in a filter chain, which can only be its last entry.
    ///
    /// Everything before it is a byte-to-byte transformation that has to run first; a codec
    /// handed still-compressed bytes fails in a way that reads as a broken image rather than as
    /// a missing step.
    fn codec_position(filters: &[Vec<u8>]) -> Option<usize> {
        filters.len().checked_sub(1).filter(|last| {
            filters
                .get(*last)
                .is_some_and(|name| crate::filter::is_image_codec(name))
        })
    }

    /// Whether a stream's data lives in an external file, ISO 32000-2 §7.3.8.1.
    ///
    /// > Alternatively, beginning with PDF 1.2, the bytes may be contained in an external
    /// > file, in which case the stream dictionary specifies the file, and any bytes between
    /// > stream and endstream shall be ignored by a PDF processor.
    ///
    /// So such a stream has **no usable data here**, and returning the embedded bytes would
    /// be drawing exactly what the clause says to ignore. The renderer has no filesystem
    /// (`CLAUDE.md` principle 3, and ADR 0014's sandbox), so it cannot fetch the file
    /// either; the honest answer is the one every unsupported stream already gets, which is
    /// a refusal its caller reports. Table 5's `/FFilter` and `/FDecodeParms` describe the
    /// external data's own filters and are unread for the same reason.
    ///
    /// Not one of the 974 corpus documents writes one, measured rather than assumed — which
    /// is why this is a rule that only reading §7.3.8 could have found.
    ///
    /// **Present means present after §7.3.9's null rule** (ADR 3282). Table 5 conditions on
    /// whether `/F` "is present", and a null value or a reference to an undefined object is
    /// not:
    ///
    /// > Specifying the null object as the value of a dictionary entry (7.3.7, "Dictionary
    /// > objects") shall be equivalent to omitting the entry entirely.
    ///
    /// with §7.3.10 making an undefined object's reference "a reference to the null object".
    /// So the entry is resolved, and only a reference is: a direct value is its own answer, and
    /// resolving a direct file specification dictionary would copy it on every decode.
    #[must_use]
    pub fn is_external(&self, stream: &Stream) -> bool {
        match stream.dict.get("F") {
            None | Some(Object::Null) => false,
            Some(reference @ Object::Reference(_)) => {
                !matches!(self.resolve(reference), Object::Null)
            }
            Some(_) => true,
        }
    }
}
