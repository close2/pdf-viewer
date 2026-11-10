//! Regression for §7.11.4.2: each related-file name must remain paired with its stream.
//!
//! A standalone integration test using the public
//! `pdf_model::attachment::related` API. The complete PDF has a Catalog, Pages and Page, exact stream
//! lengths, and a strict cross-reference table. The expected names and payloads come from the
//! alternating `[string stream ...]` rule and the five-member DCS example in §7.11.4.2.

#[cfg(test)]
mod related_stream_pairs {
    use pdf_model::attachment::related;
    use pdf_syntax::{Document, Object, ObjectId};

    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("strict-xref PDF fixture opens")
    }

    fn stream(bytes: &str) -> String {
        format!(
            "<< /Type /EmbeddedFile /Length {} >>\nstream\n{bytes}\nendstream",
            bytes.len()
        )
    }

    /// Swapping streams between adjacent filename/stream pairs must fail this assertion.
    #[test]
    fn every_related_filename_stays_paired_with_its_embedded_bytes() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles << /Names [(Sunset.eps) 4 0 R] >> >> >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> >>",
            "<< /Type /Filespec /F (Sunset.eps) /UF (Sunset.eps) /Desc (Related colour-separated files) /EF << /UF 5 0 R >> /RF << /UF [\
             (Sunset.eps) 5 0 R (Sunset.C) 6 0 R (Sunset.M) 7 0 R (Sunset.Y) 8 0 R (Sunset.K) 9 0 R] >> >>",
            &stream("eps-data"),
            &stream("cyan-data"),
            &stream("magenta-data"),
            &stream("yellow-data"),
            &stream("black-data"),
        ]);
        assert!(!doc.was_recovered(), "the fixture must use its strict xref");

        let specification = doc.resolve(&Object::Reference(ObjectId {
            number: 4,
            generation: 0,
        }));
        let specification = specification
            .as_dict()
            .expect("file specification dictionary");
        let pairs = related(&doc, specification);
        let observed: Vec<_> = pairs
            .iter()
            .map(|(name, stream)| {
                let bytes = doc
                    .decoded_stream_data(stream)
                    .expect("related embedded stream decodes");
                (name.as_str(), bytes.to_vec())
            })
            .collect();
        assert_eq!(
            observed,
            vec![
                ("Sunset.eps", b"eps-data".to_vec()),
                ("Sunset.C", b"cyan-data".to_vec()),
                ("Sunset.M", b"magenta-data".to_vec()),
                ("Sunset.Y", b"yellow-data".to_vec()),
                ("Sunset.K", b"black-data".to_vec()),
            ],
            "§7.11.4.2 defines alternating string/stream pairs"
        );
    }
}

// Writer entry-point assertions for the output required by §7.11.4.1 Table 44 and Table 45.

#[cfg(test)]
mod embedded_stream_writer_entries {
    use pdf_model::attachment::filing::embedded_file_stream;
    use pdf_syntax::Object;

    /// The stream builder writes its stated type, payload length and parameter values.
    #[test]
    fn embedded_file_stream_writes_type_and_params_dictionary() {
        let payload = b"payload bytes";
        let object = embedded_file_stream(payload, Some("application/octet-stream"), None);
        let Object::Stream(stream) = object else {
            panic!("embedded file stream object");
        };
        assert_eq!(stream.data.as_ref(), payload, "positive content control");
        assert_eq!(
            stream.dict.get("Length").and_then(Object::as_integer),
            Some(13)
        );
        assert_eq!(
            stream
                .dict
                .get("Subtype")
                .and_then(Object::as_name)
                .map(pdf_syntax::Name::as_bytes),
            Some(&b"application/octet-stream"[..])
        );
        assert_eq!(
            stream
                .dict
                .get("Type")
                .and_then(Object::as_name)
                .map(pdf_syntax::Name::as_bytes),
            Some(&b"EmbeddedFile"[..]),
            "Table 44: if Type is present, it shall be EmbeddedFile"
        );
        let params = stream.dict.get("Params").and_then(Object::as_dict);
        let params = params.expect("Table 44 writer supplies Params");
        assert_eq!(params.get("Size").and_then(Object::as_integer), Some(13));
        let Some(Object::String(checksum)) = params.get("CheckSum") else {
            panic!("Table 45 writer supplies CheckSum");
        };
        assert_eq!(
            checksum.as_ref(),
            &[
                150, 78, 229, 199, 213, 211, 202, 189, 79, 100, 72, 212, 88, 23, 226, 127
            ],
            "MD5 of the literal uncompressed payload"
        );
    }
}
