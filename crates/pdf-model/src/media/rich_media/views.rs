//! A rich media annotation's view and animation as this program's sound and video players read
//! them: ISO 32000-2 §13.7.2.2.2's default view, §13.7.2.3.5's view dictionary (Table 344),
//! §13.7.2.3.6's view params (Table 345) and §13.7.2.2.4's animation (Table 337), for `doc/todo/80`
//! section 1's last two steps (ADRs 3881 to 3883).
//!
//! **The view.** Table 335's `/View`, "Default value: The first element in the Views array of the
//! annotation specified in the `RichMediaContent` dictionary", is the view activation invokes, and
//! [`view`] answers it. Of what a view states, three parts are read:
//!
//! - **Table 315's components** — the camera, the background, the render and lighting modes, the
//!   cross sections and the nodes — describe 3D artwork. A `Sound` or `Video` instance's asset has
//!   no scene for them to set, and rendering the `3D` configuration's is excluded (`CLAUDE.md`'s
//!   clause 13 entry), so they are said as applying to nothing this program presents.
//! - **Table 344's `/Snapshot`** — "If the Snapshot entry exists, the image it describes shall be
//!   displayed in place of the current artwork" — is read and checked to be an image `XObject`.
//!   The page displays it from activation to deactivation, while a clip associated with the
//!   annotation is in the players' hands: `crate::view::ViewState::show_snapshot` holds that, and
//!   `crate::annotation::decide` sets the stored appearance aside for the image (ADR 3889).
//! - **Table 344's `/Params`** — "state data that shall be passed to the instance when the view is
//!   triggered" — is §13.7.2.3.6.3's loading, which "the run time shall" perform by a protocol the
//!   standard gives only for 3D content, an ECMAScript `StateEvent`, and that run time is ISO
//!   21757-1's, which the JavaScript exclusion keeps out. A `Sound` or `Video` instance this
//!   program plays takes no state data, so each entry is read past by name (ADR 3881).
//!
//! **The animation.** Table 337's dictionary "describes the preferred method that interactive PDF
//! processor should use to drive keyframe animations present in this artwork", and Table 313's
//! styles are stated over "keyframe settings specified in the 3D artwork" — `[r0, r1]`, "the
//! keyframe animation time line". A `Sound` or `Video` asset carries no keyframe animation, so
//! [`animation`] reads the dictionary as the table states it — its style, its play count and its
//! speed, with their defaults and the `None` rule — and the players set it aside by name: there is
//! no keyframe time line in a clip for its play count to repeat or its speed to scale (ADR 3882).

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// Table 313's animation style, as Table 337's `/Subtype` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationStyle {
    /// `None`, the default: "Keyframe animations shall not be driven directly by the interactive
    /// PDF processor."
    None,
    /// `Linear`: "Keyframe animations shall be driven linearly from beginning to end."
    Linear,
    /// `Oscillating`: "Keyframe animations shall oscillate along their time range."
    Oscillating,
}

/// Table 337's animation dictionary, read with its defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    /// Table 337's `/Subtype`: "If an animation style is encountered other than those described,
    /// an animation style of None shall be used. Default value: None".
    pub style: AnimationStyle,
    /// Table 337's `/PlayCount`: "A nonnegative integer shall represent the number of times the
    /// animation is played. A negative integer shall indicate that the animation is infinitely
    /// repeated." `None` stands for a negative count, the default -1; the value is not read for
    /// the style `None`, for which it "shall be ignored".
    pub play_count: Option<u64>,
    /// Table 337's `/Speed`, "A positive number specifying the speed that shall be used when
    /// running the animation", default 1; like the play count, ignored for the style `None`.
    pub speed: f64,
    /// One sentence for each entry read other than as stated: a style the table does not list,
    /// a play count that is not an integer and a speed that is not positive, each read as its
    /// default.
    pub notes: Vec<String>,
}

/// Table 335's `/Animation` of `activation`, read as Table 337 states it; `None` where the
/// activation states none, so that no animation is to be driven.
#[must_use]
pub fn animation(document: &Document, activation: &Dictionary) -> Option<Animation> {
    let stated = document.get_key(activation, "Animation");
    let dictionary = stated.as_dict()?;
    let mut notes = Vec::new();
    let style = match document.get_key(dictionary, "Subtype") {
        Object::Null => AnimationStyle::None,
        Object::Name(name) if name.as_bytes() == b"None" => AnimationStyle::None,
        Object::Name(name) if name.as_bytes() == b"Linear" => AnimationStyle::Linear,
        Object::Name(name) if name.as_bytes() == b"Oscillating" => AnimationStyle::Oscillating,
        _ => {
            notes.push(
                "Table 337's /Subtype is none of None, Linear and Oscillating, so the style is \
                 None, as the table requires"
                    .to_owned(),
            );
            AnimationStyle::None
        }
    };
    let mut out = Animation {
        style,
        play_count: None,
        speed: 1.0,
        notes,
    };
    // "This value shall be ignored for an animation subtype of type None", for both entries.
    if style == AnimationStyle::None {
        return Some(out);
    }
    match document.get_key(dictionary, "PlayCount") {
        Object::Null => {}
        // not a repair: a negative count is the table's own "the animation is infinitely
        //   repeated", which `None` stands for.
        Object::Integer(count) => out.play_count = u64::try_from(count).ok(),
        _ => out.notes.push(
            "Table 337's /PlayCount is not an integer, so the table's default, -1, repeats the \
             animation without end"
                .to_owned(),
        ),
    }
    match document.get_key(dictionary, "Speed") {
        Object::Null => {}
        stated => match stated
            .as_number()
            .filter(|speed| speed.is_finite() && *speed > 0.0)
        {
            Some(speed) => out.speed = speed,
            None => out.notes.push(
                "Table 337's /Speed is not a positive number, so the table's default, 1, is the \
                 speed"
                    .to_owned(),
            ),
        },
    }
    Some(out)
}

impl Animation {
    /// What a `Sound` or `Video` instance's player does with this animation, a sentence beside
    /// the clip: nothing, because the clip carries no keyframe animation to drive (ADR 3882).
    /// `None` for the style `None`, which asks nothing of the processor.
    #[must_use]
    pub fn set_aside(&self) -> Option<String> {
        let style = match self.style {
            AnimationStyle::None => return None,
            AnimationStyle::Linear => "Linear",
            AnimationStyle::Oscillating => "Oscillating",
        };
        let count = self.play_count.map_or_else(
            || "without end".to_owned(),
            |count| format!("{count} time(s)"),
        );
        Some(format!(
            "Table 337's /Subtype {style}, played {count} at speed {}, because it drives a \
             keyframe animation of 3D artwork (Table 313's time line is the artwork's keyframes) \
             and a sound or video clip carries none, so the clip plays as its own time line \
             states (ADR 3882)",
            self.speed
        ))
    }
}

/// The view activation invokes, as far as this program's players read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    /// Table 344's `/Snapshot`, where it is an image `XObject`: the image the view's activation
    /// "shall" display "in place of the current artwork".
    pub snapshot: Option<ObjectId>,
    /// How many of Table 344's `/Params` entries the view states, whose state data is not passed.
    pub params: usize,
    /// One sentence for each part of the view this program reads differently from what the file
    /// states, or does not present.
    pub notes: Vec<String>,
}

/// Table 335's `/View` of a rich media annotation's activation, or "the first element in the
/// Views array" of Table 341 where the activation names none; `None` where neither states a
/// view, for which "default values shall be used for all camera and display parameters", which
/// set nothing a sound or a video shows.
///
/// `three_d` is whether the configuration loaded is the `3D` kind, whose view's components are
/// excluded artwork's rather than nothing's.
#[must_use]
pub(super) fn view(
    document: &Document,
    activation: Option<&Dictionary>,
    content: &Dictionary,
    three_d: bool,
) -> Option<View> {
    let views: Vec<ObjectId> = document
        .get_key(content, "Views")
        .as_array()
        .map_or_else(Vec::new, |views| {
            views.iter().filter_map(Object::as_reference).collect()
        });
    let mut notes = Vec::new();
    let named = activation.and_then(|activation| activation.get("View"));
    let (reference, object) = match named {
        Some(Object::Reference(id)) => {
            if !views.contains(id) {
                notes.push(
                    "Table 335's /View is not one of Table 341's /Views, which the table requires \
                     of the file; it is the view used"
                        .to_owned(),
                );
            }
            (Some(*id), document.get(*id))
        }
        Some(Object::Null) | None => {
            let first = *views.first()?;
            (Some(first), document.get(first))
        }
        Some(direct) => {
            notes.push(
                "Table 335's /View is a direct object where the table requires an indirect \
                 reference; it is the view used"
                    .to_owned(),
            );
            (None, direct.clone())
        }
    };
    let Some(dictionary) = object.as_dict() else {
        notes.push(match reference {
            Some(_) => {
                "the view activation invokes is not a dictionary, so it is not read".to_owned()
            }
            None => "Table 335's /View is not a dictionary, so it is not read".to_owned(),
        });
        return Some(View {
            snapshot: None,
            params: 0,
            notes,
        });
    };
    if !three_d
        && dictionary
            .iter()
            .any(|(key, _)| sets_a_scene(key.as_bytes()))
    {
        notes.push(
            "the view's Table 315 entries — camera, background, render and lighting modes, cross \
             sections and nodes — describe 3D artwork, and a sound or video clip has no scene for \
             them to set"
                .to_owned(),
        );
    }
    let snapshot = snapshot(document, dictionary, &mut notes);
    let params = document
        .get_key(dictionary, "Params")
        .as_array()
        .map_or(0, <[Object]>::len);
    if params > 0 {
        notes.push(format!(
            "Table 344's /Params' {params} state data entr(y/ies) are not passed: §13.7.2.3.6.3's \
             loading is the rich media run time's, whose protocol the standard states only as a \
             3D ECMAScript StateEvent (ISO 21757-1, excluded from this program), and this \
             program's players take no state data (ADR 3881)"
        ));
    }
    Some(View {
        snapshot,
        params,
        notes,
    })
}

/// Whether `key` is one of Table 315's entries that set up a 3D scene, rather than the view's
/// name (`XN`, `IN`), its type, or Table 344's two additions.
fn sets_a_scene(key: &[u8]) -> bool {
    !matches!(key, b"Type" | b"XN" | b"IN" | b"Snapshot" | b"Params")
}

/// Table 344's `/Snapshot` of `view`, where it is an image `XObject`, with what it is otherwise
/// said in `notes` (ADRs 3881, 3889).
fn snapshot(document: &Document, view: &Dictionary, notes: &mut Vec<String>) -> Option<ObjectId> {
    let stated = view.get("Snapshot")?;
    let reference = stated.as_reference();
    let resolved = document.resolve(stated);
    let is_image = resolved.as_stream().is_some_and(|stream| {
        document
            .get_key(&stream.dict, "Subtype")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Image")
    });
    if !is_image {
        notes.push(
            "Table 344's /Snapshot is not a stream holding an image XObject, which the table \
             requires, so no snapshot is read"
                .to_owned(),
        );
        return None;
    }
    reference
}

#[cfg(test)]
mod tests {
    use super::{AnimationStyle, animation, view};
    use pdf_syntax::{Dictionary, Document, ObjectId};

    /// Builds a document from object bodies numbered from 1, the first the catalog.
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

    fn dictionary(doc: &Document, number: u32) -> Dictionary {
        doc.get(ObjectId::new(number, 0))
            .as_dict()
            .cloned()
            .expect("a dictionary")
    }

    /// Table 337's defaults — style `None`, play count -1, speed 1 — the `None` rule that ignores
    /// both numbers, a style the table does not list read as `None`, and a stated `Linear`
    /// animation set aside by name for a clip, which carries no keyframe time line (ADR 3882).
    #[test]
    fn table_337_is_read_with_its_defaults_and_set_aside_for_a_clip() {
        let read = |stated: &str| {
            let doc = document(&[
                "<< /Type /Catalog >>",
                &format!("<< /Animation {stated} >>"),
            ]);
            animation(&doc, &dictionary(&doc, 2)).expect("an animation is stated")
        };
        let default = read("<< /Type /RichMediaAnimation >>");
        assert_eq!(
            (default.style, default.play_count, default.speed),
            (AnimationStyle::None, None, 1.0)
        );
        assert_eq!(default.set_aside(), None);
        let ignored = read("<< /Subtype /None /PlayCount 3 /Speed 2 >>");
        assert_eq!((ignored.play_count, ignored.speed), (None, 1.0));
        let unknown = read("<< /Subtype /Bounce /PlayCount 3 >>");
        assert_eq!(unknown.style, AnimationStyle::None);
        assert!(unknown.notes[0].contains("style is None"));
        let linear = read("<< /Subtype /Linear /PlayCount 3 /Speed 2 >>");
        assert_eq!(
            (linear.style, linear.play_count, linear.speed),
            (AnimationStyle::Linear, Some(3), 2.0)
        );
        let said = linear.set_aside().expect("a stated style is said");
        assert!(said.starts_with("Table 337's /Subtype Linear, played 3 time(s) at speed 2"));
        let endless = read("<< /Subtype /Oscillating /PlayCount -1 /Speed 0 >>");
        assert_eq!((endless.play_count, endless.speed), (None, 1.0));
        assert!(endless.notes[0].contains("/Speed is not a positive number"));
        assert!(
            endless
                .set_aside()
                .is_some_and(|said| said.contains("played without end"))
        );
        let doc = document(&["<< /Type /Catalog >>", "<< >>"]);
        assert_eq!(animation(&doc, &dictionary(&doc, 2)), None);
    }

    /// Table 335's `/View` names the view, else Table 341's first; a snapshot that is an image
    /// `XObject` is read and nothing said of it, one that is not is said and not read; the
    /// view's Table 315 components are said for a clip and not for the `3D` kind; Table 344's
    /// `/Params` are read past by name (ADR 3881).
    #[test]
    fn the_view_is_the_named_or_the_first_and_its_snapshot_and_params_are_said() {
        let doc = document(&[
            "<< /Type /Catalog >>",
            "<< /View 4 0 R >>",
            "<< /Views [5 0 R 4 0 R] >>",
            "<< /Type /3DView /XN (named) /C2W [1 0 0 0 1 0 0 0 1 0 0 0] /Snapshot 6 0 R \
             /Params [<< /Instance 7 0 R /Data (state) >>] >>",
            "<< /Type /3DView /XN (first) /Snapshot 8 0 R >>",
            "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray \
             /BitsPerComponent 8 /Length 1 >>\nstream\n\u{0}\nendstream",
            "<< /Type /RichMediaInstance /Subtype /Video >>",
            "<< /Length 1 >>\nstream\n\u{0}\nendstream",
        ]);
        let activation = dictionary(&doc, 2);
        let content = dictionary(&doc, 3);
        let named = view(&doc, Some(&activation), &content, false).expect("a view is named");
        assert_eq!(named.snapshot, Some(ObjectId::new(6, 0)));
        assert_eq!(named.params, 1);
        assert!(
            named
                .notes
                .iter()
                .any(|note| note.contains("Table 315 entries"))
        );
        assert!(!named.notes.iter().any(|note| note.contains("Snapshot")));
        assert!(
            named
                .notes
                .iter()
                .any(|note| note.contains("/Params' 1 state data"))
        );
        let three_d = view(&doc, Some(&activation), &content, true).expect("a view is named");
        assert!(
            !three_d
                .notes
                .iter()
                .any(|note| note.contains("Table 315 entries"))
        );
        let first = view(&doc, None, &content, false).expect("the first view is the default");
        assert_eq!(first.snapshot, None);
        assert!(first.notes[0].contains("is not a stream holding an image XObject"));
        let none = document(&["<< /Type /Catalog >>", "<< >>"]);
        assert_eq!(view(&none, None, &dictionary(&none, 2), false), None);
    }
}
