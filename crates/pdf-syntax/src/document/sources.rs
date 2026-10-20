//! How a caller reads a stream's decoded bytes: all at once or through a resumable pump, and
//! where an inline image's encoded data ends.

use std::sync::Arc;

use super::{Decoding, Document, Pumpable, StreamRefusal, StreamSource, write};
use crate::filter::{EncodedExtent, FilterRefusal};
use crate::object::{Dictionary, Object, Stream};
use crate::parser::Limits;

impl Document {
    /// How a stream's decoded bytes are to be obtained: whole, or a window at a time.
    ///
    /// [`Self::decoded_stream_data_reported`] answers "what does this stream decode to", which
    /// is a question with an allocation the size of the answer in it. This one answers "how
    /// shall I read it", and for the chains a window can pump — [`Self::pumping`] says which and
    /// why — the answer is a [`crate::filter::Pump`] that never holds more than the reader's
    /// window. ADR 0365, and `doc/todo/14` for the road.
    ///
    /// Every other chain comes back whole, decoded by exactly the route it always took, cache
    /// and bound included. That is a route decision and never a silence: the bytes a caller
    /// gets are the same bytes either way, and a stream this reader cannot decode is refused
    /// here as loudly as it is there.
    ///
    /// # Errors
    ///
    /// [`StreamRefusal`], whose variants are the reasons, exactly as
    /// [`Self::decoded_stream_data_reported`] gives them.
    pub fn stream_source(&self, stream: &Stream) -> Result<StreamSource, StreamRefusal> {
        if stream.decryption_failed {
            return Err(StreamRefusal::DecryptionFailed);
        }
        if Self::is_external(stream) {
            return Err(StreamRefusal::External);
        }
        let Some(Pumpable { pumping, chain }) = self.pumping(stream) else {
            return self
                .decoded_stream_data_reported(stream)
                .map(StreamSource::Whole);
        };
        Ok(StreamSource::Pumped {
            pump: crate::filter::Pump::new(pumping, Arc::clone(&stream.data)),
            decoding: Decoding {
                encoded: Arc::clone(&stream.data),
                chain,
            },
        })
    }

    /// How one of §7.8.2's *other* content streams is to be read — a form `XObject`, a tiling
    /// pattern's cell, a Type 3 glyph description, an annotation's appearance.
    ///
    /// > Each page of a document shall be represented by one or more content streams. Content
    /// > streams shall also be used to package sequences of instructions as self-contained
    /// > graphical elements, such as forms (see 8.10, "Form XObjects"), patterns (8.7,
    /// > "Patterns"), certain fonts (9.6.4, "Type 3 fonts"), and annotation appearances (12.5.5,
    /// > "Appearance streams").
    ///
    /// These differ from a page's `/Contents` in one respect that decides the route: they are
    /// read **more than once**. §11.6.6's paired runs interpret the same form twice and
    /// sometimes three times, and a Type 3 glyph description runs once per character drawn with
    /// it. A window that re-inflated the stream for each of those would trade an allocation for
    /// unbounded work, which is not the trade `doc/todo/14` is about. A tiling pattern's cell
    /// used to be the sharpest case of that — once per site, with `MAX_TILES` then allowing four
    /// thousand — and is now read *once* for the whole tiling, its sites being copies of its
    /// marks (ADR 0430).
    ///
    /// **So the memo decides, and it is not a new number.** [`super::cache::DecodedStreams::put`] already
    /// declines any decode it cannot hold beside its encoded bytes, and that condition is
    /// exactly the one wanted here:
    ///
    /// - A stream the memo would keep is decoded **whole**, as it always was, and every read
    ///   after the first is a cache hit. Nothing about that population changes.
    /// - A stream the memo would decline is **pumped through a window**. Re-reading it costs a
    ///   re-inflation, which is what it cost before as well — a decode the memo declines is
    ///   re-run on every read today — and the gibibyte it used to command is not spent at all.
    ///
    /// Every decompression bomb is in the second half by construction, because a bomb is a
    /// stream whose decode dwarfs anything a cache would keep. ADR 0427; ADR 0365 is the same
    /// road for a page's `/Contents`.
    ///
    /// # Errors
    ///
    /// [`StreamRefusal`], exactly as [`Self::decoded_stream_data_reported`] gives it.
    pub fn nested_content_source(&self, stream: &Stream) -> Result<StreamSource, StreamRefusal> {
        let Some(Pumpable { pumping, chain }) = self.pumping(stream) else {
            return self
                .decoded_stream_data_reported(stream)
                .map(StreamSource::Whole);
        };
        // The memo answers two questions here and is asked once, because both are about the same
        // entry and taking the lock twice for them would be two acquisitions where the exclusive
        // one is already the expensive part (see [`DecodedStreams`]).
        //
        // The first: a window over these bytes has already reached this document's own bound and
        // found not one instruction inside it, so a second window would spend the same inflation
        // for the same nothing. The decode is deterministic — the same bytes under the same chain
        // and the same bound — which is what makes remembering it an answer rather than a guess,
        // and is why the fact has to include the *emptiness*: a window that had delivered marks
        // would owe those marks to every later read as well. See [`Self::window_found_nothing`].
        let allowance = {
            let mut memo = write(&self.decoded);
            if let Some(limit) =
                memo.nothing_under(&stream.data, &chain, self.limits.max_stream_len)
            {
                return Ok(StreamSource::Refused { limit });
            }
            // And the second, which is the routing rule itself: how much of a decode this memo
            // would keep beside these encoded bytes.
            memo.allowance(&stream.data)
        };
        let within = Limits {
            max_stream_len: allowance.min(self.limits.max_stream_len),
            ..self.limits
        };
        match self.decoded_chain(stream, chain.clone(), within) {
            Ok(decoded) => Ok(StreamSource::Whole(decoded)),
            // The one refusal that is not the file's: this decode outgrew what the memo would
            // have kept, so there is nothing to be gained by finishing it into a buffer nobody
            // will read twice. The window takes it from here, under the document's own bound.
            Err(StreamRefusal::Filter {
                why: FilterRefusal::TooLarge { .. },
                ..
            }) => Ok(StreamSource::Pumped {
                pump: crate::filter::Pump::new(pumping, Arc::clone(&stream.data)),
                decoding: Decoding {
                    encoded: Arc::clone(&stream.data),
                    chain,
                },
            }),
            Err(other) => Err(other),
        }
    }

    /// Remembers that a window read this decode to `under` bytes and found nothing to draw.
    ///
    /// **The one fact a window has that the buffered route does not**, and the whole of
    /// `doc/todo/41`'s remainder. [`Self::nested_content_source`] hands a stream the memo
    /// declines to a window (see there), and a window makes no [`FilterRefusal::TooLarge`] of
    /// its own — it has no allocation to bound — so a bomb whose *encoded* bytes fit the budget
    /// was re-inflated to [`Limits::max_stream_len`] on every one of §7.8.2's re-reads while the
    /// answer sat one hop away. On twenty pages drawing one hex-armoured form `XObject` that is
    /// 14.32-17.99 s against 186.16-287.23 microseconds (ADR 0646; ADR 0587 measured the same
    /// pair the other way round, before the chain gained a pump).
    ///
    /// **What travels is "too large *and empty*", never "too large" alone**, and the second half
    /// is what makes it sound. A window delivers everything up to the bound and then says it
    /// stopped, so a stream that drew marks on its first read owes those marks to its second;
    /// only a run that produced no operation at all can be replaced by a refusal without the
    /// page depending on the memo — which is the rule [`super::cache::Outcome::Decoded`]'s `damage` field
    /// already states for the other half of this cache. `pdf_model::content`'s interpreter is
    /// what can see both halves, and it is the caller.
    ///
    /// A decode whose encoded bytes do not fit [`super::DECODED_BUDGET`] is still not remembered, since
    /// [`super::cache::DecodedStreams::keep`] declines what it cannot hold beside its key — ADR 0586 argued
    /// that refusal and it is unchanged.
    pub fn window_found_nothing(&self, decoding: &Decoding, under: usize) {
        // The chain's last stage is the one whose output outgrew the bound, which is the same
        // stage `chain_over` would have named had it run the chain to the end in a buffer.
        let name = decoding
            .chain
            .last()
            .map(|(filter, _)| String::from_utf8_lossy(filter).into_owned())
            .unwrap_or_default();
        write(&self.decoded).refuse(
            &decoding.encoded,
            decoding.chain.clone(),
            &StreamRefusal::Filter {
                name,
                why: FilterRefusal::TooLarge { limit: under },
            },
            under,
        );
    }

    /// Which chain of §7.4's filters a [`crate::filter::Pump`] would run over this stream, if
    /// any.
    ///
    /// **The one place the route is decided**, so that a page's `/Contents` and §7.8.2's other
    /// four cannot answer it differently, and so that a filter gaining a pump is one edit.
    ///
    /// **All five of §7.4's byte-to-byte filters pump, and the chain pumps when every one of its
    /// stages does** — which since ADR 0587 includes §7.4.1's own two cascades, EXAMPLE 2's
    /// `/Filter [/ASCII85Decode /LZWDecode]` and EXAMPLE 3's `[/ASCII85Decode /FlateDecode]`
    /// over a page's marking instructions. Before that a chain of any length came back whole, so
    /// an author defeated the whole of road D by wrapping a bomb in a second filter, at 25 000×
    /// the cost of the same bomb unwrapped (ADR 0586).
    ///
    /// Each stage's evidence is its clause's own arithmetic, and the point of writing them down
    /// is that a stage's output length has to be a *function of its input* for a window over it
    /// to mean anything:
    ///
    /// - `ASCIIHexDecode` "shall produce one byte of binary data for each pair of ASCII
    ///   hexadecimal digits" (§7.4.2) — **1:2**, and the state between two bytes is one nibble.
    /// - `ASCII85Decode` "shall produce 5 ASCII characters for every 4 bytes of binary data"
    ///   (§7.4.3), read backwards, or four bytes for one `z`: **4:1 at the very most**, and the
    ///   state is one group of at most five digits.
    /// - `RunLengthDecode` copies "1 to 128 bytes" per length byte (§7.4.5) — **64:1** at best,
    ///   and the state is one run.
    /// - `LZWDecode` is §7.4.4.2's table, its bit accumulator and its cursor (ADR 0429), and
    ///   §7.4.4.1 NOTE 2 gives it "a compression approaching 1365:1 for long files".
    /// - `FlateDecode` is `flate2::Decompress` held across turns (ADR 0343), measured at 1032:1
    ///   on `doc/todo/10` §2's Bomb B.
    ///
    /// The last two are the ones that can name a bomb and the first three cannot; **the reason
    /// the first three are pumped anyway is that a chain is only as windowed as its worst
    /// stage**, and every arrangement that hides a bomb puts one of them in front of one of the
    /// others.
    ///
    /// **A predictor refuses the whole chain a window.** A predictor is part of decoding (see
    /// [`crate::filter::decode_with_parms`]) and §7.4.4.4 reverses each row against its
    /// predecessor, which is not a transformation a window can apply to a few thousand bytes at
    /// a time; it occurs on cross-reference streams and images rather than on content streams.
    ///
    /// **What bounds a pumped chain is not here, and that is the road's whole argument.** A
    /// window has no allocation to bound, so it makes no `TooLarge`: the aggregate bound is the
    /// reader's, §7.7.3.3's, applied over the whole content by `pdf_model::content::reader`, and
    /// what ends a bomb is that bound — Table 31's `max_stream_len` over the concatenation, since
    /// every token takes at least one byte — or the person's stop, never a count of the operators
    /// it decodes to (ADR 1809). The memory a chain costs is `LINK` bytes per stage regardless of
    /// what it names.
    ///
    /// **The chain comes back beside the route, and both are the same two reads.** Deciding
    /// whether a stream pumps means reading `/Filter` and every `/DecodeParms`, which is also
    /// exactly what identifies the decode to [`DecodedStreams`] — so a caller that needs both
    /// gets both here rather than resolving those indirect references a second time.
    pub(super) fn pumping(&self, stream: &Stream) -> Option<Pumpable> {
        if self.states_no_data(stream) {
            return None;
        }
        let filters = self.filter_chain(&stream.dict);
        let mut stages = Vec::with_capacity(filters.len());
        let mut chain = Vec::with_capacity(filters.len());
        for (index, filter) in filters.into_iter().enumerate() {
            // The stage's own parameters, read exactly as `decoded_stream_data_reported` reads
            // them, so that the two routes cannot disagree about what the stream says.
            let parms = self.decode_parms(&stream.dict, index);
            let predicted = parms
                .as_ref()
                .and_then(|parms| parms.get("Predictor").and_then(Object::as_integer))
                .is_some_and(|predictor| predictor > 1);
            if predicted {
                return None;
            }
            stages.push(crate::filter::stage(&filter, parms.as_ref())?);
            chain.push((filter, parms));
        }
        Some(Pumpable {
            pumping: crate::filter::Pumping::of(stages)?,
            chain,
        })
    }

    /// Where the encoded data `dict` describes ends inside `data`, on its first filter's own
    /// end-of-data marker.
    ///
    /// ISO 32000-2 §7.3.8.2 states three constraints on a stream's extent and requires them to
    /// agree — the `/Length`, the filter's marker, and whatever the object's own attributes
    /// imply:
    ///
    /// > In addition, most filters are defined so that the data shall be self-limiting; that
    /// > is, they use an encoding scheme in which an explicit end-of-data (EOD) marker
    /// > delimits the extent of the data.
    ///
    /// **Only §8.9.7's inline image needs this**, and that is why the answer lives here rather
    /// than in the filters' callers: Table 5 makes `/Length` required of every stream *object*,
    /// so a file always states the extent of anything written with the `stream` keyword. An
    /// inline image has no such keyword and, before PDF 2.0, no length either.
    ///
    /// Two details decide what is asked and what is not:
    ///
    /// - **It is the *first* filter of the chain, because a chain is applied in order.** Table
    ///   5: "Multiple filters shall be specified in the order in which they are to be applied",
    ///   so the bytes in the file are the first stage's input and the first stage's marker is
    ///   what delimits them. `[/A85 /LZW]` is asked of `ASCII85Decode`, not of `LZWDecode`.
    /// - **A predictor is not consulted, and does not need to be.** §7.4.4.4's predictor runs
    ///   over a stage's *output*, so it moves no byte of the input and cannot move where the
    ///   input ends. That is the one respect in which this question is easier than
    ///   [`Self::stream_source`]'s.
    ///
    /// [`EncodedExtent::Unknown`] covers a filter whose end this crate does not locate, which
    /// is `JBIG2Decode`, `JPXDecode` and `Crypt` (ADR 0467)
    /// — and §8.9.7 forbids all three of those to an inline image. What each of the three answers
    /// means is that type's own documentation, and how each filter is asked is
    /// [`crate::Delimiting`]'s.
    #[must_use]
    pub fn filtered_extent(&self, dict: &Dictionary, data: &[u8]) -> EncodedExtent {
        let chain = self.filter_chain(dict);
        let Some(first) = chain.first() else {
            return EncodedExtent::Unknown;
        };
        let parms = self.decode_parms(dict, 0);
        let Some(delimiting) = delimiting(first, parms.as_ref()) else {
            return EncodedExtent::Unknown;
        };
        crate::filter::encoded_extent(delimiting, data, self.limits.max_stream_len)
    }
}
/// How the named filter says where its own encoded data ends, with `parms` its own stage's
/// parameter dictionary.
///
/// Free of the document on purpose: nothing here resolves a reference, because the entries it
/// reads are the ones [`Document::pumping`] reads the same way, and two routes over one
/// dictionary that resolved differently could build different decoders over the same bytes.
///
/// Table 92's abbreviations are admitted beside the full names because
/// [`Document::filtered_extent`]'s one caller is §8.9.7's inline image, which is the one place
/// they are legal — and `pdf_model::inline_image` expands them before it asks, so admitting
/// both is belt and braces rather than a second reading of Table 92.
fn delimiting(filter: &[u8], parms: Option<&Dictionary>) -> Option<crate::Delimiting> {
    Some(match filter {
        // Table 8's `/EarlyChange` is read by `filter::stage`, which is what `Document::pumping`
        // asks as well, so the two cannot build different decoders over the same bytes.
        b"FlateDecode" | b"Fl" | b"LZWDecode" | b"LZW" => {
            crate::Delimiting::Decoded(crate::filter::stage(filter, parms)?)
        }
        b"ASCIIHexDecode" | b"AHx" => crate::Delimiting::Marker(b">"),
        b"ASCII85Decode" | b"A85" => crate::Delimiting::Marker(b"~>"),
        b"RunLengthDecode" | b"RL" => crate::Delimiting::RunLength,
        b"DCTDecode" | b"DCT" => crate::Delimiting::Jpeg,
        b"CCITTFaxDecode" | b"CCF" => {
            // §7.4.6 Table 11. `/EndOfBlock` false is the one arm with no answer in the
            // data: the filter then "shall stop when it has decoded the number of lines
            // indicated by Rows or when its data has been exhausted", and both of those are
            // facts about the *decode* rather than about the encoded bytes.
            let stopped_by_rows = parms
                .and_then(|parms| parms.get("EndOfBlock"))
                .is_some_and(|flag| matches!(flag, Object::Boolean(false)));
            if stopped_by_rows {
                return None;
            }
            // Table 11's `/K`: negative is Group 4, which ends on T.6's EOFB — two
            // end-of-line codes; zero and positive are Group 3, which ends on T.4's RTC —
            // six. Table 11 says only "appropriate for the K parameter", and this is what
            // T.4 and T.6 make that.
            let k = parms
                .and_then(|parms| parms.get("K"))
                .and_then(Object::as_integer)
                .unwrap_or(0);
            crate::Delimiting::EndOfBlock {
                end_of_lines: if k < 0 { 2 } else { 6 },
            }
        }
        _ => return None,
    })
}
