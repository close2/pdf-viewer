//! A rich media annotation's settings and configuration, as far as this program's sound and video
//! players present them: ISO 32000-2 §13.7.2.2's activation and deactivation conditions (Tables
//! 334 to 336) and §13.7.2.3.3's configuration (Tables 341 to 343), read for `doc/todo/80` section
//! 1's first step (ADRs 3854, 3855).
//!
//! **What is read.** Table 335's `/Condition` — `XA`, `PO`, `PV` — says when the annotation is
//! activated, and Table 336's — `XD`, `PC`, `PI` — when it is deactivated; [`settings`] answers both.
//! Table 335's `/Configuration` names the configuration activation loads, "Default value: The
//! first element within the Configurations array"; [`activated`] reads it, the instance whose
//! `/Subtype` is the configuration's, and that instance's `/Asset`, and answers the clip this
//! program's own player is handed where the configuration is `Sound` or `Video` and the asset is a
//! file of a type that player takes.
//!
//! **What is refused by name.** The `3D` configuration — rendering U3D and PRC artwork is excluded
//! (`CLAUDE.md`'s clause 13 entry) — and every configuration the players do not take is
//! [`activated`]'s `Err`, a sentence a person is told. Table 335's `/Scripts`, which "shall be
//! executed … by the rich media run time ECMAScript interpreter", is that run time's, ISO
//! 21757-1's, which the JavaScript exclusion keeps out; a configuration that plays says so in
//! [`Clip::unhonoured`]. Table 335's `/Animation`, `/View` and `/Presentation` are later steps of
//! `doc/todo/80` section 1 and are said the same way where an activation states them.
//!
//! **The asset's type is Table 44's `/Subtype`.** §13.7 names no format for either configuration
//! (RFC 0015 section 10.1), and Table 343's "the subtype shall match the asset file type" is the
//! producer's requirement; the matching the players need is the embedded file's own stated type,
//! the reading a movie's file has. An asset that states none is not played, and that is said.

use pdf_syntax::{Dictionary, Document, Object};

use super::container;
use super::deprecated::{content_type, file};
use super::{Background, Clip, Data, Players, Playing, Processor};

/// Table 335's `/Condition`: when the annotation is activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    /// `XA`, the default: "the annotation is explicitly activated by a user action or script".
    Explicit,
    /// `PO`: "the annotation is activated as soon as the page that contains the annotation receives
    /// focus as the current page".
    PageOpen,
    /// `PV`: "the annotation is activated as soon as any part of the page that contains the
    /// annotation becomes visible".
    PageVisible,
}

/// Table 336's `/Condition`: when the annotation is deactivated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deactivation {
    /// `XD`, the default: "the annotation is explicitly deactivated by a user action or script".
    Explicit,
    /// `PC`: "the annotation is deactivated as soon as the page that contains the annotation loses
    /// focus as the current page".
    PageClose,
    /// `PI`: "the annotation is deactivated as soon as the entire page that contains the
    /// annotation is no longer visible".
    PageInvisible,
}

/// Table 334's settings as far as they decide when a rich media annotation plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Table 335's `/Condition`.
    pub activation: Activation,
    /// Table 336's `/Condition`.
    pub deactivation: Deactivation,
    /// One sentence for each condition that names no value the tables define, which is read as
    /// the default.
    pub notes: Vec<String>,
}

/// Table 333's `/RichMediaSettings` of a `RichMedia` annotation, and its two conditions; `None` for
/// an annotation of any other subtype.
///
/// "If no `RichMediaSettings` dictionary is present, the first configuration is loaded" — on the
/// default condition, `XA`, and deactivated on `XD`. A condition that is not one of the three
/// names its table lists is said, and the default stands in its place.
#[must_use]
pub fn settings(document: &Document, annotation: &Dictionary) -> Option<Settings> {
    if !is_rich_media(document, annotation) {
        return None;
    }
    let mut out = Settings {
        activation: Activation::Explicit,
        deactivation: Deactivation::Explicit,
        notes: Vec::new(),
    };
    let settings = document.get_key(annotation, "RichMediaSettings");
    let Some(settings) = settings.as_dict() else {
        return Some(out);
    };
    let condition = |key: &str| {
        let dictionary = document.get_key(settings, key);
        let condition = dictionary
            .as_dict()
            .map_or(Object::Null, |dict| document.get_key(dict, "Condition"));
        match condition {
            Object::Null => None,
            Object::Name(name) => Some(Ok(name.as_bytes().to_vec())),
            _ => Some(Err(())),
        }
    };
    match condition("Activation") {
        None => {}
        Some(Ok(name)) if name == b"XA" => {}
        Some(Ok(name)) if name == b"PO" => out.activation = Activation::PageOpen,
        Some(Ok(name)) if name == b"PV" => out.activation = Activation::PageVisible,
        Some(_) => out.notes.push(
            "Table 335's /Condition is none of XA, PO and PV, so the annotation is activated by \
             the reader, the table's default XA"
                .to_owned(),
        ),
    }
    match condition("Deactivation") {
        None => {}
        Some(Ok(name)) if name == b"XD" => {}
        Some(Ok(name)) if name == b"PC" => out.deactivation = Deactivation::PageClose,
        Some(Ok(name)) if name == b"PI" => out.deactivation = Deactivation::PageInvisible,
        Some(_) => out.notes.push(
            "Table 336's /Condition is none of XD, PC and PI, so the annotation is deactivated by \
             the reader, the table's default XD"
                .to_owned(),
        ),
    }
    Some(out)
}

/// What activating a `RichMedia` annotation loads, for this program's players: the clip of the
/// configuration's instance, judged for `processor`'s players and to be played in the
/// annotation's rectangle.
///
/// The configuration is Table 335's `/Configuration`, or "the first element within the
/// Configurations array" of Table 341. Its kind is Table 342's `/Subtype`, or where it states none,
/// the first instance's — the table's "the run time shall determines the scene type by referring
/// to the type of asset file specified by the first element in the Instances array", read through
/// Table 343's "the subtype shall match the asset file type". The instance played is the first of
/// that kind. A `Sound` configuration plays where its asset is a RIFF WAVE file or an MP4 file of
/// sound alone and the host plays sound; a `Video` configuration where its asset is an MP4 file of
/// `video/` type and the host shows video (ADR 3855).
///
/// # Errors
///
/// The sentence a person is told, ending where the configuration stops: an annotation of another
/// subtype or with no content, no configuration, a `3D` configuration (excluded), a kind Table 342
/// does not define, no instance of the kind, an asset that is not an embedded file, and an asset
/// whose type the players do not take.
pub fn activated(
    document: &Document,
    annotation: &Dictionary,
    processor: Processor<'_>,
) -> Result<Clip, String> {
    let (activation, configuration) = configuration(document, annotation)?;
    let (video, instance) = instance(document, &configuration)?;
    let kind_said = if video { "Video" } else { "Sound" };
    let Some(data) = file(document, &document.get_key(&instance, "Asset"), "asset") else {
        return Err(format!(
            "its {kind_said} instance's Table 343 /Asset is not a file specification this reader \
             can read (§7.11)"
        ));
    };
    if !matches!(data, Data::Embedded(_)) {
        return Err(format!(
            "its {kind_said} instance's asset is not an embedded file, which §13.7.2.3.2's assets \
             name tree holds"
        ));
    }
    let stated = content_type(&data);
    let takes = |kind: &str| {
        if video {
            processor.players == Players::AlsoVideoHere && container::is_mp4_video(kind)
        } else {
            processor.players != Players::HandOff && container::is_played_here(kind)
        }
    };
    match stated.as_deref() {
        Some(kind) if takes(kind) => {}
        Some(other) => {
            return Err(format!(
                "its {kind_said} instance's asset is of type {other}, which this program's {} \
                 does not play",
                if video {
                    "video player, where this window has one,"
                } else {
                    "sound player, where this window has one,"
                }
            ));
        }
        None => {
            return Err(format!(
                "its {kind_said} instance's asset states no Table 44 /Subtype, so its type is not \
                 known and this program's players do not take it"
            ));
        }
    }
    let mut unhonoured = Vec::new();
    if let Some(activation) = &activation {
        if !matches!(document.get_key(activation, "Scripts"), Object::Null) {
            unhonoured.push(
                "Table 335's /Scripts, because a rich media run time's ECMAScript (ISO 21757-1) \
                 is excluded from this program"
                    .to_owned(),
            );
        }
        for key in ["Animation", "View", "Presentation"] {
            if !matches!(document.get_key(activation, key), Object::Null) {
                unhonoured.push(format!("Table 335's /{key}"));
            }
        }
    }
    Ok(Clip {
        content_type: stated,
        data,
        temporary_file: None,
        unhonoured,
        playing: Playing::default(),
        played_here: true,
        // Table 338's default style, `Embedded`, shows the pictures in the annotation's rectangle;
        // a sound shows nothing there, so the annotation keeps its appearance.
        background: video.then(Background::default),
        window: None,
    })
}

/// Table 335's `/RichMediaActivation`, where the settings state one, and the configuration it
/// loads: its `/Configuration`, or "the first element within the Configurations array".
fn configuration(
    document: &Document,
    annotation: &Dictionary,
) -> Result<(Option<Dictionary>, Dictionary), String> {
    if !is_rich_media(document, annotation) {
        return Err("the annotation is not a rich media annotation".to_owned());
    }
    let content = document.get_key(annotation, "RichMediaContent");
    let Some(content) = content.as_dict() else {
        return Err(
            "Table 333 makes /RichMediaContent required and this annotation states none".to_owned(),
        );
    };
    let activation = document
        .get_key(annotation, "RichMediaSettings")
        .as_dict()
        .map(|settings| document.get_key(settings, "Activation"))
        .and_then(|activation| activation.as_dict().cloned());
    let named = activation
        .as_ref()
        .map(|activation| document.get_key(activation, "Configuration"))
        .and_then(|configuration| configuration.as_dict().cloned());
    let configuration = named.or_else(|| {
        document
            .get_key(content, "Configurations")
            .as_array()
            .and_then(|configurations| configurations.first())
            .and_then(|first| document.resolve(first).as_dict().cloned())
    });
    match configuration {
        Some(configuration) => Ok((activation, configuration)),
        None => Err(
            "neither Table 335's /Configuration nor Table 341's /Configurations names a \
             configuration, so there is nothing to load"
                .to_owned(),
        ),
    }
}

/// The configuration's kind — `true` for `Video`, `false` for `Sound` — and its first instance of
/// that kind; the `3D` kind and every other refused by name.
fn instance(document: &Document, configuration: &Dictionary) -> Result<(bool, Dictionary), String> {
    let instances: Vec<Dictionary> = document
        .get_key(configuration, "Instances")
        .as_array()
        .map_or_else(Vec::new, |instances| {
            instances
                .iter()
                .filter_map(|instance| document.resolve(instance).as_dict().cloned())
                .collect()
        });
    let subtype = |dict: &Dictionary| {
        document
            .get_key(dict, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec())
    };
    let Some(kind) = subtype(configuration).or_else(|| instances.first().and_then(subtype)) else {
        return Err(
            "its configuration states no Table 342 /Subtype and has no instance whose Table 343 \
             /Subtype would decide one"
                .to_owned(),
        );
    };
    let video = match kind.as_slice() {
        b"Sound" => false,
        b"Video" => true,
        b"3D" => {
            return Err(
                "its configuration is 3D artwork, and rendering U3D and PRC artwork is excluded \
                 from this program (ISO 32000-2 clause 13)"
                    .to_owned(),
            );
        }
        other => {
            return Err(format!(
                "its configuration's kind /{} is none of the 3D, Sound and Video Table 342 \
                 defines",
                String::from_utf8_lossy(other)
            ));
        }
    };
    let kind_said = if video { "Video" } else { "Sound" };
    instances
        .into_iter()
        .find(|instance| subtype(instance).as_deref() == Some(kind.as_slice()))
        .map(|instance| (video, instance))
        .ok_or_else(|| {
            format!(
                "its {kind_said} configuration has no instance whose Table 343 /Subtype is \
                 {kind_said}"
            )
        })
}

/// Whether `annotation` is Table 333's `RichMedia` annotation.
fn is_rich_media(document: &Document, annotation: &Dictionary) -> bool {
    document
        .get_key(annotation, "Subtype")
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"RichMedia")
}

#[cfg(test)]
mod tests {
    use super::{Activation, Deactivation, activated, settings};
    use crate::media::{Players, Processor};
    use pdf_syntax::{Document, ObjectId};

    /// Builds a document from object bodies numbered from 1.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-2.0\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref = out.len();
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
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    fn annotation(doc: &Document) -> pdf_syntax::Dictionary {
        doc.get(ObjectId::new(2, 0))
            .as_dict()
            .cloned()
            .expect("the annotation is a dictionary")
    }

    fn players(players: Players) -> Processor<'static> {
        Processor {
            players,
            ..Processor::default()
        }
    }

    /// A rich media annotation with `settings` as its `/RichMediaSettings` and one configuration
    /// of `kind` whose one instance's asset is an embedded file of `media_type`.
    fn rich_media(settings: &str, kind: &str, media_type: &str) -> Document {
        document(&[
            "<< /Type /Catalog >>",
            &format!(
                "<< /Type /Annot /Subtype /RichMedia /Rect [0 0 10 10] /RichMediaSettings \
                 {settings} /RichMediaContent << /Assets << /Names [(clip) 5 0 R] >> \
                 /Configurations [3 0 R] >> >>"
            ),
            &format!("<< /Type /RichMediaConfiguration /Subtype /{kind} /Instances [4 0 R] >>"),
            &format!("<< /Type /RichMediaInstance /Subtype /{kind} /Asset 5 0 R >>"),
            "<< /Type /Filespec /F (clip) /UF (clip) /EF << /F 6 0 R >> >>",
            &format!(
                "<< /Type /EmbeddedFile /Subtype /{media_type} /Length 4 >>\nstream\nmoov\n\
                 endstream"
            ),
        ])
    }

    /// Table 335's three conditions and Table 336's three, each read as its name; absent, the
    /// defaults `XA` and `XD`; a name neither table lists is said and read as the default.
    #[test]
    fn each_condition_is_read_and_the_defaults_stand_where_none_is_stated() {
        let read = |settings_text: &str| {
            let doc = rich_media(settings_text, "Video", "video#2Fmp4");
            settings(&doc, &annotation(&doc)).expect("a rich media annotation")
        };
        let none = read("<< >>");
        assert_eq!(
            (none.activation, none.deactivation),
            (Activation::Explicit, Deactivation::Explicit)
        );
        for (condition, activation) in [
            ("XA", Activation::Explicit),
            ("PO", Activation::PageOpen),
            ("PV", Activation::PageVisible),
        ] {
            let read = read(&format!("<< /Activation << /Condition /{condition} >> >>"));
            assert_eq!(read.activation, activation, "{condition}");
            assert!(read.notes.is_empty(), "{:?}", read.notes);
        }
        for (condition, deactivation) in [
            ("XD", Deactivation::Explicit),
            ("PC", Deactivation::PageClose),
            ("PI", Deactivation::PageInvisible),
        ] {
            let read = read(&format!(
                "<< /Deactivation << /Condition /{condition} >> >>"
            ));
            assert_eq!(read.deactivation, deactivation, "{condition}");
        }
        let odd = read("<< /Activation << /Condition /PX >> /Deactivation << /Condition 3 >> >>");
        assert_eq!(
            (odd.activation, odd.deactivation),
            (Activation::Explicit, Deactivation::Explicit)
        );
        assert_eq!(odd.notes.len(), 2, "{:?}", odd.notes);
    }

    /// A `Video` configuration whose asset is an MP4 file of `video/` type is played here, in the
    /// annotation's rectangle, for a host that shows video, and refused by name for one that plays
    /// only sound; a `Sound` configuration's WAVE asset is played by either. `/Scripts` is said.
    #[test]
    fn a_sound_or_video_configuration_the_players_take_is_played_here() {
        let doc = rich_media(
            "<< /Activation << /Scripts [5 0 R] >> >>",
            "Video",
            "video#2Fmp4",
        );
        let clip = activated(&doc, &annotation(&doc), players(Players::AlsoVideoHere))
            .expect("a video the host shows");
        assert!(clip.played_here);
        assert!(clip.background.is_some(), "Table 338's Embedded rectangle");
        assert_eq!(clip.content_type.as_deref(), Some("video/mp4"));
        assert!(
            clip.unhonoured[0].starts_with("Table 335's /Scripts"),
            "{:?}",
            clip.unhonoured
        );
        let refused = activated(&doc, &annotation(&doc), players(Players::AlsoHere))
            .expect_err("no video player");
        assert!(refused.contains("video/mp4"), "{refused}");
        let doc = rich_media("<< >>", "Sound", "audio#2Fwav");
        let clip = activated(&doc, &annotation(&doc), players(Players::AlsoHere))
            .expect("a sound the host plays");
        assert!(clip.played_here && clip.background.is_none());
        assert!(clip.unhonoured.is_empty(), "{:?}", clip.unhonoured);
        assert!(activated(&doc, &annotation(&doc), players(Players::HandOff)).is_err());
    }

    /// The `3D` configuration is refused by name, a kind Table 342 does not define is said, and a
    /// `/Configuration` the activation names is loaded in place of the first.
    #[test]
    fn three_d_and_unknown_configurations_are_refused_and_the_named_one_is_loaded() {
        let doc = rich_media("<< >>", "3D", "model#2Fu3d");
        let refused = activated(&doc, &annotation(&doc), players(Players::AlsoVideoHere))
            .expect_err("3D is excluded");
        assert!(refused.contains("U3D and PRC"), "{refused}");
        let doc = rich_media("<< >>", "Flash", "application#2Fx-shockwave-flash");
        let refused = activated(&doc, &annotation(&doc), players(Players::AlsoVideoHere))
            .expect_err("no such kind");
        assert!(refused.contains("/Flash"), "{refused}");
        // The first configuration is 3D; the activation names the second, a video.
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /Type /Annot /Subtype /RichMedia /Rect [0 0 10 10] /RichMediaSettings \
             << /Activation << /Configuration 7 0 R >> >> /RichMediaContent \
             << /Configurations [3 0 R 7 0 R] >> >>",
            "<< /Subtype /3D /Instances [4 0 R] >>",
            "<< /Subtype /3D /Asset 5 0 R >>",
            "<< /Type /Filespec /F (clip) /UF (clip) /EF << /F 6 0 R >> >>",
            "<< /Type /EmbeddedFile /Subtype /video#2Fmp4 /Length 4 >>\nstream\nmoov\nendstream",
            "<< /Instances [8 0 R] >>",
            "<< /Subtype /Video /Asset 5 0 R >>",
        ]);
        let clip = activated(&doc, &annotation(&doc), players(Players::AlsoVideoHere))
            .expect("the named configuration, whose kind its first instance decides");
        assert!(clip.played_here);
    }
}
