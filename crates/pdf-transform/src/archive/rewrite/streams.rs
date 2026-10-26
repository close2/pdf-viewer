//! A stream's rewrite: its dictionary, its filter chain and its data.
use std::collections::BTreeMap;

use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;

use super::super::COMPRESSION_LEVEL;
use super::super::prepare::metadata_stream;
use super::{
    Rewrite, Rewriter, Rewritten, chain_of, count, decode_parms, filter_names,
    filter_names_spelled_out, holds_reference, parms_at, with_flate,
};

impl Rewriter<'_> {
    /// A stream object: the `XObject` rules, and the filter chain.
    #[expect(
        clippy::too_many_lines,
        reason = "one branch per rewrite that replaces a whole stream, each with the two or                   three sentences saying why its bytes and its dictionary change together, and                   the branches are tried in an order a reader has to be able to see. Splitting                   them across functions would hide that order behind calls"
    )]
    pub(super) fn rewrite_stream(
        &self,
        id: ObjectId,
        stream: &Stream,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> Rewritten {
        // ISO 32000-1's 8.8.2 defines a PostScript XObject as an XObject stream whose
        // `/Subtype` is `PS`, and says that its fragments have no effect when the document is
        // viewed on screen or printed to a non-PostScript device; ISO 32000-2 defines no such
        // object at all. So a form with no content draws on every page exactly what it drew —
        // nothing — and, written at the same object, keeps every resource entry naming it
        // defined, which a `Do` the producer wrote needs (`doc/adr/2115`).
        if self.wants(Rewrite::PostScriptXObject) && self.subtype_is(&stream.dict, b"PS") {
            count(applied, Rewrite::PostScriptXObject);
            return Rewritten::Changed(drawing_nothing());
        }
        // The destination profile this reader could not draw through, written as the profile a
        // file stating no intent is given, at the object every entry names (`doc/adr/2007`).
        if self.wants(Rewrite::DestinationProfileReplaced)
            && let Some(replaced) = self
                .replaced_destination
                .filter(|replaced| replaced.at == id)
        {
            count(applied, Rewrite::DestinationProfileReplaced);
            return Rewritten::Changed(replaced.written.clone());
        }
        // The same object's header restated as what its bytes are, where it was not replaced
        // outright above (`doc/adr/2021`).
        if self.wants(Rewrite::DestinationProfileRestated)
            && let Some(restated) = self
                .restated_destination
                .filter(|restated| restated.at == id)
        {
            count(applied, Rewrite::DestinationProfileRestated);
            return Rewritten::Changed(restated.written.clone());
        }
        // The attachment a tool derived: a whole new stream rather than a dictionary edit, because
        // its bytes, its `/Length`, its `/Subtype` and its `/Params` all change together. The
        // construction and the four edits are `super::super::remedies::derived_stream`.
        if self.wants(Rewrite::DerivedEmbeddedFile)
            && let Some(derived) = self.remedies.derived.get(&id)
        {
            count(applied, Rewrite::DerivedEmbeddedFile);
            return Rewritten::Changed(derived.clone());
        }
        // The operator said the dictionary's writing mode stands, so the program's own entry is
        // restated: new bytes and a new `/Filter` together, so a whole stream (`doc/adr/1286`).
        if self.wants(Rewrite::WriteModeAgreed)
            && let Some(restated) = self.remedies.restated_programs.get(&id)
        {
            count(applied, Rewrite::WriteModeAgreed);
            return Rewritten::Changed(restated.clone());
        }
        // The profile given the text tags it lacked: its bytes, its `/Filter` and its `/Length`
        // change together, so a whole stream (`doc/adr/1990`).
        if self.wants(Rewrite::ProfileTextSupplied)
            && let Some(supplied) = self.remedies.supplied_profiles.get(&id)
        {
            count(applied, Rewrite::ProfileTextSupplied);
            return Rewritten::Changed(supplied.clone());
        }
        // The stream whose data was outside the file: its bytes, its `/Filter`, its
        // `/DecodeParms` and its `/Length` all change together, so a whole stream crosses rather
        // than a dictionary edit. `super::super::external`'s construction is Table 5 read straight.
        if self.wants(Rewrite::ExternalDataEmbedded)
            && let Some(embedded) = self.external_data.and_then(|external| external.at.get(&id))
        {
            count(applied, Rewrite::ExternalDataEmbedded);
            return Rewritten::Changed(embedded.clone());
        }
        // A property list's `/Lang` respelled inside the stream: new bytes and a new `/Filter`
        // together, so a whole stream. Asked before the hexadecimal repair because a stream both
        // reach was respelled from the repaired bytes, so this one carries the two
        // (`doc/adr/2067`).
        if self.wants(Rewrite::MarkedLanguagesRestated)
            && let Some(restated) = self.languages
            && let Some(respelled) = restated.streams.get(&id)
        {
            if restated.carries_hexadecimal.contains(&id) {
                count(applied, Rewrite::HexadecimalDigitCompleted);
            }
            return Rewritten::Changed(respelled.clone());
        }
        // The same shape, for the same reason: the repaired bytes, their `/Filter` and their
        // `/Length` change together, so what crosses is a stream rather than a dictionary edit.
        if let Some(repaired) = self.hexadecimal_repair(id) {
            count(applied, Rewrite::HexadecimalDigitCompleted);
            return Rewritten::Changed(repaired);
        }
        // **Five writers over one packet, and the last of them is what carries the others'
        // work.** `super::super::prepare` folds the replacement into the bytes the header cut starts
        // from, those into the property removal's, those into the container respelling's, those
        // into the amendment cut's and the container's, and those into the bytes the
        // identification schema is restated into — so whichever of the branches below writes
        // this stream is writing every edit, and a rewrite that happened has to be counted
        // wherever it is carried, not only where it is the sole reason the stream changed. The
        // three `doc/adr/1245` and `doc/adr/1246` added are counted in
        // [`count_what_has_no_place`] instead, from their own preparations, because the branch
        // that writes the catalog's stream returns before any of them is reached.
        let header_cut = self.wants(Rewrite::PacketHeaderAttributes)
            && self
                .packet_headers
                .is_some_and(|headers| headers.packet(id).is_some());
        // The producer's own packet, with the identification schema's properties cut out of it
        // and this target's put in — every other byte of it the producer's. The removal of
        // section 6.6.2.3.1's properties has already been folded into these bytes, because the
        // catalog's packet is one of the packets it edits.
        if self.wants_metadata()
            && let Some(metadata) = self.metadata
            && metadata.written.is_none()
            && metadata.at == id
        {
            if header_cut {
                count(applied, Rewrite::PacketHeaderAttributes);
            }
            return Rewritten::Changed(metadata_stream(&stream.dict, &metadata.packet));
        }
        // Every *other* metadata stream one of the packet writers edited: an object's own packet
        // is not the document's, and no part restricts any of these requirements to the catalog's.
        // **Asked in reverse writing order**, because each writer's bytes carry every edit before
        // it — so the last of them is the one branch that writes the whole of what was done.
        // The field descriptions are the last, and therefore the first asked (`doc/adr/2050`).
        if self.wants(Rewrite::ExtensionFieldDescribed)
            && let Some(packet) = self
                .fields_described
                .and_then(|fields| fields.packets.get(&id))
        {
            if header_cut {
                count(applied, Rewrite::PacketHeaderAttributes);
            }
            return Rewritten::Changed(metadata_stream(&stream.dict, packet));
        }
        // The container, which ran before the field descriptions and after the amendment cut.
        if self.wants(Rewrite::ExtensionSchemaDescribed)
            && let Some(packet) = self
                .described
                .and_then(|described| described.packets.get(&id))
        {
            if header_cut {
                count(applied, Rewrite::PacketHeaderAttributes);
            }
            return Rewritten::Changed(metadata_stream(&stream.dict, packet));
        }
        // The amendment cut, which ran before the container and after the respelling.
        if self.wants(Rewrite::AmendmentIdentifierRemoved)
            && let Some(packet) = self.amended.and_then(|amended| amended.packets.get(&id))
        {
            if header_cut {
                count(applied, Rewrite::PacketHeaderAttributes);
            }
            return Rewritten::Changed(metadata_stream(&stream.dict, packet));
        }
        if (self.wants(Rewrite::ExtensionSchemaPrefixes)
            || self.wants(Rewrite::ExtensionContainerDiscarded))
            && let Some(packet) = self
                .respelled
                .and_then(|respelled| respelled.packets.get(&id))
        {
            if header_cut {
                count(applied, Rewrite::PacketHeaderAttributes);
            }
            return Rewritten::Changed(metadata_stream(&stream.dict, packet));
        }
        if self.wants(Rewrite::PropertyOutsideItsSchema)
            && let Some(packet) = self.cleaned.and_then(|cleaned| cleaned.packets.get(&id))
        {
            if header_cut {
                count(applied, Rewrite::PacketHeaderAttributes);
            }
            return Rewritten::Changed(metadata_stream(&stream.dict, packet));
        }
        // A stream whose packet this tree cannot read carries the one this conversion composed.
        // **No header cut is counted here**, unlike the branches above: the replacement skips
        // such a stream in every one of the other writers, so nothing of the producer's header
        // survived to be cut.
        if self.wants(Rewrite::FreshMetadataPacket)
            && let Some(packet) = self.fresh.and_then(|fresh| fresh.packet(id))
        {
            return Rewritten::Changed(metadata_stream(&stream.dict, packet));
        }
        // And a packet whose *only* edit is the header's, which is the common case: neither
        // attribute has anything to do with the RDF, so a file whose metadata is otherwise
        // conforming fails this requirement on its own.
        if header_cut
            && let Some(packet) = self.packet_headers.and_then(|headers| headers.packet(id))
        {
            count(applied, Rewrite::PacketHeaderAttributes);
            return Rewritten::Changed(metadata_stream(&stream.dict, packet));
        }
        // The font program itself, restated: a whole new stream rather than a dictionary edit,
        // because its bytes, its `/Length` and its `/Length1` all change together.
        // One stream whichever of the two requirements asked for it, and counted under each
        // that did: a font whose dictionary disagrees with its program in both directions is
        // restated twice into one set of bytes, and writing it twice would drop the first.
        if let Some(metrics) = self.metrics
            && let Some(restated) = metrics.at.get(&id)
        {
            let horizontal =
                self.wants(Rewrite::RestateFontMetrics) && metrics.horizontal.contains(&id);
            let vertical =
                self.wants(Rewrite::RestateVerticalFontMetrics) && metrics.vertical.contains(&id);
            if horizontal {
                count(applied, Rewrite::RestateFontMetrics);
            }
            if vertical {
                count(applied, Rewrite::RestateVerticalFontMetrics);
            }
            if horizontal || vertical {
                return Rewritten::Changed(restated.clone());
            }
        }
        // A font collection's program, now the one face its descriptor names written out as a
        // program of its own: a whole stream, because its bytes and both lengths change (ADR
        // 2343). It is filed under the row that face's format takes, and a TrueType program's row
        // is under the key it was already filed by with no `/Subtype` to state.
        if self.wants(Rewrite::FontProgramRefiled)
            && let Some(face) = self.font_files.and_then(|refiled| refiled.faces.get(&id))
        {
            count(applied, Rewrite::FontProgramRefiled);
            return Rewritten::Changed(face.clone());
        }
        // The JPEG 2000 data itself, with the colour specification boxes the part directs a
        // processor to ignore taken out: the bytes and the `/Length` change together, so it is a
        // whole stream for the same reason the font program above is.
        // ADR 1400: the whole image, written again under `FlateDecode` in the colour space its
        // dictionary states. Before the colour box reduction, which edits the data this replaces.
        if self.wants(Rewrite::Jpeg2000TranscodedToFlate)
            && let Some(transcoded) = self
                .transcodes
                .and_then(|transcodes| transcodes.at.get(&id))
        {
            count(applied, Rewrite::Jpeg2000TranscodedToFlate);
            return Rewritten::Changed(transcoded.clone());
        }
        if self.wants(Rewrite::Jpeg2000ColourSpecifications)
            && let Some(reduced) = self
                .specifications
                .and_then(|specifications| specifications.at.get(&id))
        {
            count(applied, Rewrite::Jpeg2000ColourSpecifications);
            return Rewritten::Changed(reduced.clone());
        }
        let mut changed = false;
        let mut dict = match self.rewrite_dictionary(id, &stream.dict, applied) {
            Some(rewritten) => {
                changed = true;
                rewritten
            }
            None => stream.dict.clone(),
        };
        changed |= self.rewrite_xobject(&mut dict, applied);
        // The name of the format the program's own bytes are in, and for a program moving to
        // `/FontFile3` the three lengths §9.9 says a compact program "shall not" state: the data
        // crosses as the producer wrote it (ADR 2307).
        if self.wants(Rewrite::FontProgramRefiled)
            && let Some(row) = self.font_files.and_then(|refiled| refiled.at.get(&id))
        {
            if let Some(subtype) = row.subtype {
                dict.insert(
                    Name::new(&b"Subtype"[..]),
                    Object::Name(Name::new(subtype.as_bytes())),
                );
            }
            if self
                .font_files
                .is_some_and(|refiled| refiled.moved_programs.contains(&id))
            {
                for length in ["Length1", "Length2", "Length3"] {
                    dict.remove(length);
                }
            }
            changed = true;
            count(applied, Rewrite::FontProgramRefiled);
        }
        // Before the re-encoding, which carries whatever image codec follows the stages it
        // decodes under the name this has just written out.
        if self.wants(Rewrite::FilterNamesSpelledOut)
            && let Some(spelled) = filter_names_spelled_out(&dict)
        {
            dict = spelled;
            changed = true;
            count(applied, Rewrite::FilterNamesSpelledOut);
        }
        if self.wants(Rewrite::FlateInsteadOfLzw)
            && let Some(reencoded) = self.reencode(stream, &dict)
        {
            count(applied, Rewrite::FlateInsteadOfLzw);
            return Rewritten::Changed(reencoded);
        }
        if changed {
            return Rewritten::Changed(Object::Stream(std::sync::Arc::new(Stream {
                dict,
                data: std::sync::Arc::clone(&stream.data),
                decryption_failed: stream.decryption_failed,
            })));
        }
        Rewritten::Carried
    }

    /// The image and form `XObject` keys ISO 19005 forbids.
    fn rewrite_xobject(
        &self,
        dict: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if self.subtype_is(dict, b"Image") {
            if self.wants(Rewrite::ImageAlternatesAndOpi) {
                for key in ["Alternates", "OPI"] {
                    if dict.remove(key).is_some() {
                        count(applied, Rewrite::ImageAlternatesAndOpi);
                        changed = true;
                    }
                }
            }
            // §8.9.5.1's Table 87 makes `/Interpolate` "[a] flag indicating whether image
            // interpolation shall be performed by a PDF processor", default false. Stating it
            // false says what ISO 19005 requires in the entry the file already has.
            if self.wants(Rewrite::InterpolationOff)
                && dict.get("Interpolate").is_some()
                && self.document.get_key(dict, "Interpolate") != Object::Boolean(false)
            {
                dict.insert(Name::new(&b"Interpolate"[..]), Object::Boolean(false));
                count(applied, Rewrite::InterpolationOff);
                changed = true;
            }
        }
        if self.subtype_is(dict, b"Form") {
            if self.wants(Rewrite::FormOpi) && dict.remove("OPI").is_some() {
                count(applied, Rewrite::FormOpi);
                changed = true;
            }
            if self.wants(Rewrite::FormPostScript) {
                if dict.remove("PS").is_some() {
                    count(applied, Rewrite::FormPostScript);
                    changed = true;
                }
                let is_postscript = self.subtype_is_named(dict, "Subtype2", b"PS");
                if is_postscript && dict.remove("Subtype2").is_some() {
                    count(applied, Rewrite::FormPostScript);
                    changed = true;
                }
            }
        }
        changed
    }

    /// One stream decoded through the filters this tree reads and re-encoded as one
    /// `FlateDecode`, where its chain names `LZWDecode`.
    ///
    /// `None` where the stream does not name that filter, or where the re-encoding cannot be
    /// done without changing what the file refers to — a chain whose `/Filter` is not a name or
    /// an array of names, a `/DecodeParms` entry that names another object, a stage this tree
    /// does not decode. Every one of those leaves the requirement failed, which the output's
    /// own verdict then reports rather than this function guessing.
    fn reencode(&self, stream: &Stream, dict: &Dictionary) -> Option<Object> {
        let chain = chain_of(self.document, &stream.dict);
        if !chain
            .iter()
            .any(|filter| filter.as_slice() == b"LZWDecode" || filter.as_slice() == b"LZW")
        {
            return None;
        }
        if Document::is_external(stream) || stream.decryption_failed {
            return None;
        }
        let names = filter_names(dict)?;
        if names.len() != chain.len() {
            return None;
        }
        let mut data: std::sync::Arc<[u8]> = std::sync::Arc::clone(&stream.data);
        let mut stop = chain.len();
        for (index, filter) in chain.iter().enumerate() {
            if pdf_syntax::filter::is_image_codec(filter) {
                stop = index;
                break;
            }
            let stage = pdf_syntax::filter::decode_with_parms_reported(
                filter,
                &data,
                parms_at(self.document, &stream.dict, index).as_ref(),
                self.document.limits(),
            )
            .ok()?;
            if stage.damage.is_some() {
                return None;
            }
            data = stage.data;
        }
        // An image codec after the LZW stage would leave its bytes inside the new outer filter,
        // which is right — but the LZW stage has to be one of the ones that was decoded, or
        // nothing has been fixed.
        if !chain
            .get(..stop)?
            .iter()
            .any(|filter| filter.as_slice() == b"LZWDecode" || filter.as_slice() == b"LZW")
        {
            return None;
        }
        let parms = decode_parms(dict, names.len())?;
        if parms
            .get(..stop)
            .unwrap_or_default()
            .iter()
            .any(|value| holds_reference(value, 0))
        {
            return None;
        }
        let encoded = flate_encode(&data, COMPRESSION_LEVEL)?;
        Some(Object::Stream(std::sync::Arc::new(Stream {
            dict: with_flate(dict, &names, &parms, stop, encoded.len()),
            data: encoded.into(),
            decryption_failed: false,
        })))
    }

    /// The content stream `super::super::hexadecimal` repaired for this object, where it repaired one.
    fn hexadecimal_repair(&self, id: ObjectId) -> Option<Object> {
        if !self.wants(Rewrite::HexadecimalDigitCompleted) {
            return None;
        }
        self.hexadecimal
            .and_then(|completed| completed.at.get(&id))
            .cloned()
    }
}

/// The form `XObject` a PostScript `XObject` is written as: §8.10's form with an empty content
/// stream and an empty `/BBox`, which paints nothing wherever a `Do` invokes it (`doc/adr/2115`).
fn drawing_nothing() -> Object {
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"XObject"[..])),
    );
    dict.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(&b"Form"[..])),
    );
    dict.insert(
        Name::new(&b"BBox"[..]),
        Object::Array(vec![Object::Integer(0); 4]),
    );
    dict.insert(Name::new(&b"Length"[..]), Object::Integer(0));
    Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: Vec::new().into(),
        decryption_failed: false,
    }))
}
