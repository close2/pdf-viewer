//! §7.4: a stream's filter chain run to the end, and the bounded cache of what it produced.

use std::sync::Arc;

use super::{DecodedStreamCache, Document, StreamRefusal, read, write};
use crate::filter::Decoded;
use crate::object::{Dictionary, Object, Stream};
use crate::parser::Limits;

impl Document {
    /// Returns a stream's decoded data.
    ///
    /// # Errors
    ///
    /// Returns `None` when a filter in the chain is not supported, rather than returning
    /// the encoded bytes. Handing back compressed data as if it were decoded would produce
    /// garbage that looks like a rendering bug. Also for a stream whose data lives in an
    /// external file — see [`Self::is_external`]. [`Self::decoded_stream_data_reported`] says
    /// which of [`StreamRefusal`]'s answers it was.
    #[must_use]
    pub fn decoded_stream_data(&self, stream: &Stream) -> Option<Arc<[u8]>> {
        self.decoded_stream_data_reported(stream)
            .ok()
            .map(|decoded| decoded.data)
    }

    /// The same, naming what refused.
    ///
    /// The distinction a caller needs is [`crate::filter::FilterRefusal::TooLarge`]: a stream this reader
    /// *could* have decoded and declined to, which is a statement about the file rather than
    /// about this program's filter table. Reporting it as an unsupported filter would be the
    /// silent-fallback failure one layer up from the one ADR 0306 removed.
    ///
    /// The second distinction is [`Decoded::damage`], which is not a refusal at all: bytes
    /// came out, and they stop short of what the file says the stream is. ADR 0343.
    ///
    /// # Errors
    ///
    /// [`StreamRefusal`], whose variants are the reasons.
    pub fn decoded_stream_data_reported(&self, stream: &Stream) -> Result<Decoded, StreamRefusal> {
        self.decoded_under(stream, self.limits)
    }

    /// The same, under bounds that need not be the document's.
    ///
    /// The one caller that passes anything else is [`Self::nested_content_source`], which asks
    /// for a decode no larger than the memo would keep — see there for why that is the whole of
    /// the routing rule. The bytes are the same bytes: a decode that succeeds under a smaller
    /// bound is byte for byte the decode that succeeds under a larger one, so what it puts in
    /// the memo is what the memo would have held anyway.
    pub(super) fn decoded_under(
        &self,
        stream: &Stream,
        limits: Limits,
    ) -> Result<Decoded, StreamRefusal> {
        self.decoded_chain(stream, self.chain_with_parms(&stream.dict), limits)
    }

    /// The same again, over a chain the caller has already read out of the dictionary.
    ///
    /// [`Self::nested_content_source`] has one in hand — [`Self::pumping`] read the same
    /// `/Filter` and the same `/DecodeParms` to decide the route — and reading them a second
    /// time here would resolve every indirect reference in both twice for one decode.
    pub(super) fn decoded_chain(
        &self,
        stream: &Stream,
        chain: Vec<(Vec<u8>, Option<Dictionary>)>,
        limits: Limits,
    ) -> Result<Decoded, StreamRefusal> {
        if stream.decryption_failed {
            return Err(StreamRefusal::DecryptionFailed);
        }
        if Self::is_external(stream) {
            return Err(StreamRefusal::External);
        }
        if chain.is_empty() || self.states_no_data(stream) {
            return Ok(Decoded {
                data: Arc::clone(&stream.data),
                damage: None,
            });
        }

        self.chain_over(&stream.data, chain, limits)
    }

    /// §7.4's chain over one stream dictionary, each filter beside its own `/DecodeParms`.
    ///
    /// **Half of what identifies a decode**, the other half being the bytes — see
    /// [`DecodedEntry::chain`] — so it is built in one place and the memo's key and the work
    /// the memo remembers come from the same call. The parameters are read here rather than
    /// inside [`Self::chain_over`]'s loop for that reason: an entry has to be keyed on all of
    /// what produced it, and `decode_parms` was already called once per filter either way.
    fn chain_with_parms(&self, dict: &Dictionary) -> Vec<(Vec<u8>, Option<Dictionary>)> {
        self.filter_chain(dict)
            .into_iter()
            .enumerate()
            .map(|(index, filter)| (filter, self.decode_parms(dict, index)))
            .collect()
    }

    /// Runs one filter chain over one buffer, through [`DecodedStreams`].
    ///
    /// **The one place a chain is run, so that the two callers cannot memoise it differently.**
    /// [`Self::decoded_under`] asks for a stream's whole `/Filter`; [`Self::image_stream`] asks
    /// for the part of it in front of an image codec, which is the same kind of work over the
    /// same bytes and was the one filter chain in this crate with no memo at all (ADR 0585).
    ///
    /// The chain is passed by value because the memo keeps it: it is half of what identifies a
    /// decode, and [`DecodedEntry::chain`] compares it on every hit.
    ///
    /// **One entry per allocation, and a second chain over the same buffer replaces the first.**
    /// That is [`DecodedStreams::held`]'s key rather than a decision made here, and no shipped
    /// path asks both of one stream: an image `XObject`'s bytes reach this by the image route
    /// and a font program's, an `ICCBased` profile's and a content stream's by the other.
    /// `pdf_model::thumbnail::significant` builds the one second `Stream` over another's `data`
    /// and leaves `/Filter` untouched, so its chain is equal and it *shares* the entry.
    pub(super) fn chain_over(
        &self,
        encoded: &Arc<[u8]>,
        chain: Vec<(Vec<u8>, Option<Dictionary>)>,
        limits: Limits,
    ) -> Result<Decoded, StreamRefusal> {
        if let Some(held) = write(&self.decoded).get(encoded, &chain, limits.max_stream_len) {
            return held;
        }

        let mut data: Arc<[u8]> = Arc::clone(encoded);
        // The *first* damage in the chain is the one kept, because it is the one that caused
        // the rest: a stage fed a truncated prefix has no way to end well either, and naming
        // the last stage's complaint would describe the symptom rather than the file.
        let mut damage = None;
        let mut refused = None;
        for (filter, parms) in &chain {
            match crate::filter::decode_with_parms_reported(filter, &data, parms.as_ref(), limits) {
                Ok(stage) => {
                    data = stage.data;
                    damage = damage.or(stage.damage);
                }
                Err(why) => {
                    refused = Some(StreamRefusal::Filter {
                        name: String::from_utf8_lossy(filter).into_owned(),
                        why,
                    });
                    break;
                }
            }
        }
        // The refusal is memoised for the same reason the bytes are, and it is the more
        // expensive of the two to reach: `FilterRefusal::TooLarge` is answered only after
        // inflating to the bound, so a stream refused on page one is a gibibyte per page
        // without this. ADR 0437.
        if let Some(why) = refused {
            write(&self.decoded).refuse(encoded, chain, &why, limits.max_stream_len);
            return Err(why);
        }
        let decoded = Decoded { data, damage };
        write(&self.decoded).put(encoded, chain, &decoded);
        Ok(decoded)
    }

    /// What this document's decoded-stream cache is holding, and how it has been used.
    ///
    /// An instrument rather than something a reader draws with; `viewer-core/examples/find_cost`
    /// prints it after a sweep. See [`super::cache::DecodedStreams`] for what the numbers mean.
    #[must_use]
    pub fn decoded_streams(&self) -> DecodedStreamCache {
        read(&self.decoded).report()
    }

    /// Whether the file *states* that this stream holds nothing, ISO 32000-2 §7.3.8.1.
    ///
    /// > A stream shall consist of a dictionary followed by zero or more bytes bracketed between
    /// > the keywords stream (followed by newline) and endstream :
    ///
    /// So an empty stream is conforming, and Table 5's `/Filter` names the filters "that shall be
    /// applied in processing the stream data found between the keywords stream and endstream ".
    /// With no data found there, there is nothing for a filter to process and the decoded result
    /// is the empty sequence — which is *not* the same as a filter refusing its input, and the
    /// difference is a report a page either does or does not deserve. `FlateDecode` refuses zero
    /// bytes, because RFC 1950 gives a zlib stream a six-byte floor, so without this a page whose
    /// producer wrote `<< /Filter /FlateDecode /Length 0 >>` was reported as missing drawing that
    /// an empty part cannot be missing.
    ///
    /// **Both halves of the condition are load-bearing, and the second is the whole argument.**
    /// A stream *truncated* to nothing also arrives here holding no bytes: [`crate::Parser`]
    /// recovers a wrong `/Length` by searching for `endstream`, so a file cut off mid-stream
    /// yields an empty slice as readily as an empty stream does. §7.3.8.2 tells the two apart —
    ///
    /// > Every stream dictionary shall have a Length entry that indicates how many bytes of the
    /// > PDF file are used for the stream's data.
    ///
    /// — because a truncation leaves `/Length` stating a number the bytes do not support, and
    /// "[a]ll of these constraints shall be consistent" is then false. Only a stated zero that the
    /// bytes agree with is silence the producer asked for. Two documents of the 5944 `SafeDocs`
    /// members on disk do this and two more are truncations; ADR 0266 names all four.
    ///
    /// Deliberately not applied on [`Self::image_stream`]'s path: §7.3.8.2 also says "streams are
    /// used to represent many objects from whose attributes a length can be inferred", and an
    /// image's `/Width`, `/Height` and `/BitsPerComponent` infer one, so for an image a stated
    /// zero contradicts the dictionary rather than agreeing with it.
    pub(crate) fn states_no_data(&self, stream: &Stream) -> bool {
        stream.data.is_empty()
            && self
                .get_key(&stream.dict, "Length")
                .as_integer()
                .is_some_and(|length| length == 0)
    }

    /// Returns the `/DecodeParms` entry for the filter at `index`.
    ///
    /// The key may hold a single dictionary or an array with one entry per filter, and
    /// either may be indirect.
    pub(crate) fn decode_parms(&self, dict: &Dictionary, index: usize) -> Option<Dictionary> {
        match self.get_key(dict, "DecodeParms") {
            Object::Dictionary(parms) => Some(parms),
            Object::Array(items) => items
                .get(index)
                .map(|item| self.resolve(item))
                .and_then(|item| item.as_dict().cloned()),
            _ => None,
        }
    }

    /// Returns the filter names for a stream, in application order.
    pub(crate) fn filter_chain(&self, dict: &Dictionary) -> Vec<Vec<u8>> {
        let filter = self.get_key(dict, "Filter");
        match filter {
            Object::Name(name) => vec![name.as_bytes().to_vec()],
            Object::Array(items) => items
                .iter()
                .map(|item| self.resolve(item))
                .filter_map(|item| item.as_name().map(|name| name.as_bytes().to_vec()))
                .collect(),
            _ => Vec::new(),
        }
    }
}
