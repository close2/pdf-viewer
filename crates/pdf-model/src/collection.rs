//! ISO 32000-2 §12.3.5's collections: a PDF that is a folder of other files.
//!
//! > Beginning with PDF 1.7, PDF documents may specify how an interactive PDF processor's user
//! > interface presents collections of file attachments , where the attachments are related in
//! > structure or content. Such a presentation is called a portable collection.
//!
//! An email archive, a photo set, an engineering bid — the container's own pages exist to
//! explain the collection, and the documents are §7.11.4's embedded files, which
//! [`crate::attachment`] already reads. This module reads what the clause adds on top: the
//! *columns* a viewer would show, the order to sort them in, the folder tree the files hang in,
//! and the layout to present it all with.
//!
//! The one sentence that makes a collection more than a hint is in §12.3.5.1: "[i]f this
//! dictionary is present in a PDF document, the interactive PDF processor shall present the
//! document as a portable collection". A viewer without a file browser cannot obey it, which is
//! what keeps this crate's part of the clause `partial` — the data is all here.
//!
//! # Three rules a reader has to get right rather than store
//!
//! **A file belongs to a folder by the *shape of its name*.** §12.3.5.2 gives the association no
//! entry at all: an `/EmbeddedFiles` key such as `<3>report.pdf` puts that file in folder 3, and
//! files that "do not conform to these rules shall be treated
//! as associated with the root folder". [`folder_of`] is that convention, and it is the reason
//! this module reads name-tree *keys* rather than only the files behind them.
//!
//! **A layout is chosen, not read.** §12.3.6: "[w]hen multiple names are provided, an
//! interactive PDF processor should present the first one it is capable of displaying in the
//! order present in the array". [`Navigator::preferred`] takes the list of layouts a viewer can
//! draw and answers with the first match, which is a different function from "what does the file
//! say".
//!
//! **`/D` names the document to open, and states its own three fallbacks.** Missing or not a
//! valid byte string: the container itself. Naming a file the tree does not hold: "the first
//! item from the list of files". No files at all: "an empty preview window".
//! [`Collection::initial_document`] returns those three cases as three values rather than as an
//! `Option`, because they are three different instructions.
//!
//! # What the ledger said about navigators, and what §12.3.6 says
//!
//! This row read, for thirty-odd sessions, that a navigator is "a collection's own presentation,
//! supplied as SWF" and that widening `CLAUDE.md`'s exclusion list "should start here". ISO
//! 32000-2's §12.3.6 contains no media format at all: a navigator dictionary holds `/Layout`,
//! one or more of seven **named layouts** — `D`, `T`, `H`, `FilmStrip`, `FreeForm`, `Linear`,
//! `Tree` — and the clause describes each in prose. The SWF navigator was an Adobe extension
//! that this standard replaced, and the exclusion argument that rested on it was about a
//! document nobody in this project had read. It is a reader's question, and it is here.
//!
//! **One corpus document states a `/Collection`**, and this line read "[n]o corpus document" until the
//! five-hundred-and-seventieth session, because the count had only ever been taken over pdf.js:
//! `doc/corpora/format-corpus/pdfCabinetOfHorrors/digitally_signed_3D_Portfolio.pdf` states one with eight
//! schema fields and a `/Folders` tree — which is the one entry here a hand-built fixture was the only
//! witness for. ADR 0405.

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Object, ObjectId, tree};

/// Most folders read from one document.
///
/// A folder tree is a list a person clicks through; a document with more folders than this is
/// making a reader work rather than organising anything.
const MAX_FOLDERS: usize = 1 << 14;

/// Deepest folder nesting followed.
const MAX_DEPTH: usize = 64;

/// What a collection says about itself. Table 153.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Collection {
    /// `/Schema`, the fields a viewer would show as columns.
    ///
    /// Absent is not empty: Table 153 says an absent schema lets a processor "choose useful
    /// defaults that are known to exist in a file specification dictionary, such as the file
    /// name, file size, and modified date" — a permission this reader records by leaving the map
    /// empty rather than by inventing three fields nobody wrote.
    pub schema: BTreeMap<String, Field>,
    /// `/D`, the key in the `/EmbeddedFiles` tree of the document to show first.
    pub initial: Option<String>,
    /// `/View`, the initial view. Table 153's default is [`View::Details`].
    pub view: View,
    /// `/Sort`, the order items are shown in.
    pub sort: Option<Sort>,
    /// `/Navigator`, PDF 2.0's interactive layout.
    pub navigator: Option<Navigator>,
    /// `/Colors`, Table 157's five suggested `DeviceRGB` colours for a layout.
    pub colours: Colours,
    /// `/Split`, Table 158's splitter bar.
    pub split: Option<Split>,
    /// The root of `/Folders`, where the collection has a folder tree.
    pub folders: Option<Folder>,
}

/// Table 153's `/View`: how the collection is first presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    /// `D` — "details mode, with all information in the Schema dictionary presented in a
    /// multi-column format". Table 153's default.
    #[default]
    Details,
    /// `T` — "tile mode, with each file in the collection denoted by a small icon".
    Tile,
    /// `H` — "initially hidden", with the processor providing "means for the user to view the
    /// collection by some explicit action".
    ///
    /// §7.6.7's unencrypted wrapper document requires exactly this value, which is the one place
    /// a `/View` is load-bearing rather than a preference: the wrapper's own page says the
    /// payload is encrypted, and showing a file browser over it would hide that.
    Hidden,
    /// `C` — present the collection with `/Navigator`'s layout. PDF 2.0, and Table 153 makes it
    /// "valid only when Navigator is present".
    Navigator,
}

/// One column of a collection. Table 155.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// `/Subtype`, which decides both the type of the data and where it comes from.
    pub kind: FieldKind,
    /// `/N`, "[t]he textual field name that shall be presented to the user".
    pub name: String,
    /// `/O`, "[t]he relative order of the field name in the user interface".
    pub order: Option<i64>,
    /// `/V`, "[t]he initial visibility of the field". Default `true`.
    pub visible: bool,
    /// `/E`, whether a processor "should provide support for editing the field value". Default
    /// `false`.
    pub editable: bool,
}

/// Table 155's `/Subtype`, in the clause's own two groups.
///
/// The distinction is where the value lives, and it is the reason this is one enum rather than a
/// name: the first three are "types of fields in the collection item … dictionary", so their
/// data is in §7.11.6's `/CI`; the rest are "file-related fields", whose data is already in the
/// file specification or its embedded file parameters and is *not* repeated in the item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    /// `S` — a text field, "stored as a PDF text string".
    Text,
    /// `D` — a date field, "stored as a PDF date string (see 7.9.4)".
    Date,
    /// `N` — a number field, "stored as a PDF number".
    Number,
    /// `F` — the file name, from the file specification's `/UF` "if present; otherwise by the F
    /// entry".
    FileName,
    /// `Desc` — the description, from the file specification's `/Desc`.
    Description,
    /// `ModDate` — from the embedded file parameter dictionary's `/ModDate`.
    ModificationDate,
    /// `CreationDate` — from the embedded file parameter dictionary's `/CreationDate`.
    CreationDate,
    /// `Size` — from the embedded file parameter dictionary's `/Size`.
    Size,
    /// `CompressedSize` — PDF 2.0, from the embedded file stream's `/Length`.
    CompressedSize,
    /// A subtype this standard does not define.
    Other(String),
}

impl FieldKind {
    /// Whether the field's data comes from §7.11.6's collection item rather than from the file.
    ///
    /// The clause's own division: `S`, `D` and `N` "identify the types of fields in the
    /// collection item or collection subitem dictionary", and the rest "identify the types of
    /// file-related fields". A viewer filling a column asks this to know where to look.
    #[must_use]
    pub fn is_in_the_item(&self) -> bool {
        matches!(self, Self::Text | Self::Date | Self::Number)
    }
}

/// Table 156's collection sort dictionary.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sort {
    /// `/S`, the field keys to sort by, in order — one name or an array of them.
    pub fields: Vec<String>,
    /// `/A`, ascending or not, per field.
    ///
    /// Table 156 allows a single boolean or an array, and [`Sort::ascending`] applies the
    /// clause's own rule for a short array rather than storing a padded copy.
    pub ascending: Vec<bool>,
}

impl Sort {
    /// Whether the field at `index` sorts ascending.
    ///
    /// Table 156 makes `/A` "[a] boolean or an array of booleans" and gives the default as
    /// `true`; a single boolean applies to every field, and a shorter array leaves the rest at
    /// the default.
    #[must_use]
    pub fn ascending(&self, index: usize) -> bool {
        match self.ascending.as_slice() {
            [] => true,
            [only] => *only,
            many => many.get(index).copied().unwrap_or(true),
        }
    }
}

/// Table 157's five suggested colours, each `DeviceRGB`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Colours {
    /// `/Background`, "used for the background of the view".
    pub background: Option<[f32; 3]>,
    /// `/CardBackground`, "used for the background of the card".
    pub card_background: Option<[f32; 3]>,
    /// `/CardBorder`, "used for the border of the card".
    pub card_border: Option<[f32; 3]>,
    /// `/PrimaryText`, "used for the primary text in a navigator".
    pub primary_text: Option<[f32; 3]>,
    /// `/SecondaryText`, "used for other text in a navigator".
    pub secondary_text: Option<[f32; 3]>,
}

/// Table 158's splitter bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Split {
    /// `/Direction`: `H` horizontal, `V` vertical, `N` not split.
    pub direction: SplitDirection,
    /// `/Position`, "the initial position of the splitter bar, specified as a percentage of the
    /// available window area", clamped to the table's stated 0..=100.
    ///
    /// Table 158 says the entry "shall be ignored if Direction is set to N", which is a rule for
    /// whoever draws the bar rather than for whoever reads it.
    pub position: Option<f32>,
}

/// Table 158's `/Direction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitDirection {
    /// `H` — "the window is split horizontally".
    #[default]
    Horizontal,
    /// `V` — "the window is split vertically".
    Vertical,
    /// `N` — not split: "[t]he entire window region shall be dedicated to the file navigation
    /// view".
    None,
}

/// Table 160's navigator: which layout presents the collection.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Navigator {
    /// `/Layout`, one name or several, in the file's own order of preference.
    pub layouts: Vec<Layout>,
}

impl Navigator {
    /// The first layout the file names that a caller can draw.
    ///
    /// §12.3.6: "[w]hen multiple names are provided, an interactive PDF processor should present
    /// the first one it is capable of displaying in the order present in the array." So the
    /// answer depends on the *viewer* as well as the file, which is why this takes the list of
    /// layouts a caller supports rather than answering from the document alone.
    ///
    /// `None` where nothing matches — which the clause says cannot happen in a valid file, since
    /// one of the seven "shall always be present, either singly or as the final entry in the
    /// array".
    #[must_use]
    pub fn preferred(&self, supported: &[Layout]) -> Option<Layout> {
        self.layouts
            .iter()
            .find(|layout| supported.contains(layout))
            .cloned()
    }
}

/// Table 160's named layouts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layout {
    /// `D`, `T` and `H`, "[c]orresponding to the value of" the same name in `/View`.
    View(View),
    /// `FilmStrip` — "a strip of thumbnails, providing an index to the file attachments".
    FilmStrip,
    /// `FreeForm` — thumbnails "randomly in the view".
    FreeForm,
    /// `Linear` — "a large size preview of one file attachment … alongside the preview the
    /// metadata".
    Linear,
    /// `Tree` — "a tree view, showing the folder structure and the files as leaf nodes".
    Tree,
    /// A custom layout name. §12.3.6 says the mechanism "is inherently extensible and allows
    /// inclusion of custom named layouts", so an unknown name is a document using one rather
    /// than a malformed file.
    Custom(String),
}

/// One folder of §12.3.5.2's hierarchy. Table 159.
#[derive(Debug, Clone, PartialEq)]
pub struct Folder {
    /// `/ID`, "a non-negative integer value representing the unique folder identification
    /// number", and the number a file's name-tree key refers to.
    pub id: u32,
    /// `/Name`, the folder's name.
    pub name: String,
    /// `/Desc`, "[a] text description associated with this folder".
    pub description: Option<String>,
    /// `/CI`, §7.11.6's collection item — "user-defined metadata … just as it does for embedded
    /// files in a collection".
    pub item: Item,
    /// Whether `/Thumb` is present. §12.3.4's thumbnails, on a folder rather than a page.
    pub has_thumbnail: bool,
    /// The child folders, in the order `/Child` and `/Next` chain them.
    pub children: Vec<Folder>,
}

/// §7.11.6's collection item: the values behind a schema's columns. Table 46.
///
/// A map from the schema's own keys to values, because Table 46 has no fixed entries at all —
/// "[o]ther keys … [p]rovides the data corresponding to the related fields in the collection
/// dictionary", each named by the writer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Item {
    /// The values, by the schema key that names them.
    pub values: BTreeMap<String, Value>,
}

/// One value in a collection item, with Table 47's optional prefix.
#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    /// The data: a text string, a date string or a number, as the schema's field says.
    ///
    /// Kept as the object the file wrote. Which of the three it is, is not this dictionary's to
    /// say — "[t]he type of each entry shall match the type of data identified by the collection
    /// field dictionary … referenced by the same key" — so interpreting it needs the schema, and
    /// a reader that guessed from the object's own type would disagree with the schema on any
    /// file where they differ.
    pub data: Object,
    /// Table 47's `/P`, "[a] prefix string that shall be concatenated with the text string
    /// presented to the user".
    ///
    /// The clause states its own exception, which is why the prefix is kept apart from the data
    /// rather than glued to it: "[t]his entry is ignored when an interactive PDF processor sorts
    /// the items in the collection".
    pub prefix: Option<String>,
}

/// What §12.3.5.1's `/D` entry asks a viewer to open first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Initial {
    /// The container itself: "[i]f the D entry is missing or is not a valid byte string, the
    /// initial document shall be the one that contains the collection dictionary".
    Container,
    /// The named embedded file, which the `/EmbeddedFiles` tree holds.
    Embedded(String),
    /// "[T]he first item from the list of files": `/D` named something the tree does not have.
    FirstFile,
    /// "[A]n empty preview window": `/D` named nothing the tree has, and the tree is empty.
    Empty,
}

impl Collection {
    /// Reads the catalog's `/Collection`, which almost no document has.
    #[must_use]
    pub fn read(document: &Document) -> Option<Self> {
        let catalog = document.catalog().ok()?;
        let collection = document.get_key(&catalog, "Collection");
        let dict = collection.as_dict()?;
        Some(Self {
            schema: schema(document, dict),
            initial: match document.get_key(dict, "D") {
                Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
                _ => None,
            },
            view: view(
                document
                    .get_key(dict, "View")
                    .as_name()
                    .map(|name| name.as_bytes().to_vec())
                    .as_deref(),
            ),
            sort: sort(document, dict),
            navigator: navigator(document, dict),
            colours: colours(document, dict),
            split: split(document, dict),
            folders: folders(document, dict),
        })
    }

    /// Which document a viewer opens first, by §12.3.5.1's three fallbacks.
    ///
    /// See [`Initial`]. The `/EmbeddedFiles` tree decides two of the three, so this takes the
    /// document rather than answering from the collection dictionary alone.
    #[must_use]
    pub fn initial_document(&self, document: &Document) -> Initial {
        let Some(name) = self.initial.as_deref() else {
            return Initial::Container;
        };
        let keys = embedded_file_keys(document);
        if keys.iter().any(|candidate| candidate == name) {
            Initial::Embedded(name.to_owned())
        } else if keys.is_empty() {
            Initial::Empty
        } else {
            Initial::FirstFile
        }
    }

    /// Every folder of the tree, breadth first, for a caller that wants them flat.
    #[must_use]
    pub fn all_folders(&self) -> Vec<&Folder> {
        let mut out = Vec::new();
        let mut queue: Vec<&Folder> = self.folders.iter().collect();
        while let Some(folder) = queue.pop() {
            out.push(folder);
            queue.extend(folder.children.iter());
        }
        out
    }
}

/// The folder ID an `/EmbeddedFiles` key names, and the file name after it.
///
/// §12.3.5.2 states the convention as four bulleted rules, and it is the only place in the
/// standard where a name tree's *key* carries structure: the key is a text string whose first
/// character — "excluding any byte order marker" — is a less-than sign, followed by one or more
/// digits and a closing greater-than sign, and "[t]he remainder of the string is a file name".
///
/// `None` is a key that does not conform, and §12.3.5.2 says such files "shall be treated as
/// associated with the root folder" rather than rejected — so a caller reads `None` as "the root", and the file name is
/// the whole key.
#[must_use]
pub fn folder_of(key: &str) -> Option<(u32, &str)> {
    // The byte order mark is already gone: `pdf_syntax::text_string` consumed it when it decided
    // the encoding, which is what "excluding any byte order marker" asks for.
    let rest = key.strip_prefix('<')?;
    let (digits, name) = rest.split_once('>')?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some((digits.parse().ok()?, name))
}

/// Whether a string is a *file name* by §12.3.5.2's five requirements.
///
/// The clause's bullets: a PDF text string, with no "embedded NULL (U+0000) characters", a
/// length where "[t]he number of characters in the string shall be between 1 and 255 inclusive",
/// none of "the eight special characters" it then lists, and a last character that "shall not be
/// a FULL STOP (U+002E) (.)". The
/// clause leaves what to do about a bad one open — "[a]n interactive PDF processor may choose to
/// support invalid names or not" — so this answers the question and refuses nothing.
///
/// The bound is counted in **characters** rather than bytes, which is what the sentence says, so
/// a text string's scalar values are counted and not its UTF-8 length. It used to be left out
/// here on a reason that named an operation this function does not perform — that truncating a
/// name would rename a file — while the doc line above it claimed the clause's five rules and
/// the code applied four (session 600).
#[must_use]
pub fn is_file_name(name: &str) -> bool {
    (1..=255).contains(&name.chars().count())
        && !name.ends_with('.')
        && !name
            .chars()
            .any(|c| c == '\0' || matches!(c, '/' | '\\' | ':' | '*' | '"' | '<' | '>' | '|'))
}

/// Table 154's schema: a field dictionary per writer-chosen key.
fn schema(document: &Document, dict: &Dictionary) -> BTreeMap<String, Field> {
    let schema = document.get_key(dict, "Schema");
    let Some(schema) = schema.as_dict() else {
        return BTreeMap::new();
    };
    schema
        .iter()
        .filter(|(key, _)| key.as_bytes() != b"Type")
        .filter_map(|(key, value)| {
            let field = document.resolve(value);
            let field = field.as_dict()?;
            Some((
                String::from_utf8_lossy(key.as_bytes()).into_owned(),
                Field {
                    kind: field_kind(document.get_key(field, "Subtype").as_name()?.as_bytes()),
                    name: match document.get_key(field, "N") {
                        Object::String(bytes) => pdf_syntax::text_string(&bytes),
                        _ => String::new(),
                    },
                    order: document.get_key(field, "O").as_integer(),
                    visible: !matches!(document.get_key(field, "V"), Object::Boolean(false)),
                    editable: matches!(document.get_key(field, "E"), Object::Boolean(true)),
                },
            ))
        })
        .collect()
}

/// Table 155's `/Subtype`.
fn field_kind(name: &[u8]) -> FieldKind {
    match name {
        b"S" => FieldKind::Text,
        b"D" => FieldKind::Date,
        b"N" => FieldKind::Number,
        b"F" => FieldKind::FileName,
        b"Desc" => FieldKind::Description,
        b"ModDate" => FieldKind::ModificationDate,
        b"CreationDate" => FieldKind::CreationDate,
        b"Size" => FieldKind::Size,
        b"CompressedSize" => FieldKind::CompressedSize,
        other => FieldKind::Other(String::from_utf8_lossy(other).into_owned()),
    }
}

/// Table 153's `/View`, whose default is `D`.
fn view(name: Option<&[u8]>) -> View {
    match name {
        Some(b"T") => View::Tile,
        Some(b"H") => View::Hidden,
        Some(b"C") => View::Navigator,
        _ => View::Details,
    }
}

/// Table 156's sort dictionary.
fn sort(document: &Document, dict: &Dictionary) -> Option<Sort> {
    let sort = document.get_key(dict, "Sort");
    let sort = sort.as_dict()?;
    let fields = match document.get_key(sort, "S") {
        Object::Name(name) => vec![String::from_utf8_lossy(name.as_bytes()).into_owned()],
        Object::Array(items) => items
            .iter()
            .filter_map(|entry| {
                document
                    .resolve(entry)
                    .as_name()
                    .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
            })
            .collect(),
        _ => Vec::new(),
    };
    let ascending = match document.get_key(sort, "A") {
        Object::Boolean(value) => vec![value],
        Object::Array(items) => items
            .iter()
            .filter_map(|entry| match document.resolve(entry) {
                Object::Boolean(value) => Some(value),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    Some(Sort { fields, ascending })
}

/// Table 160's navigator.
fn navigator(document: &Document, dict: &Dictionary) -> Option<Navigator> {
    let navigator = document.get_key(dict, "Navigator");
    let navigator = navigator.as_dict()?;
    let layout = |name: &[u8]| match name {
        b"D" => Layout::View(View::Details),
        b"T" => Layout::View(View::Tile),
        b"H" => Layout::View(View::Hidden),
        b"FilmStrip" => Layout::FilmStrip,
        b"FreeForm" => Layout::FreeForm,
        b"Linear" => Layout::Linear,
        b"Tree" => Layout::Tree,
        other => Layout::Custom(String::from_utf8_lossy(other).into_owned()),
    };
    let layouts = match document.get_key(navigator, "Layout") {
        Object::Name(name) => vec![layout(name.as_bytes())],
        Object::Array(items) => items
            .iter()
            .filter_map(|entry| {
                document
                    .resolve(entry)
                    .as_name()
                    .map(|name| layout(name.as_bytes()))
            })
            .collect(),
        _ => Vec::new(),
    };
    Some(Navigator { layouts })
}

/// Table 157's colours.
fn colours(document: &Document, dict: &Dictionary) -> Colours {
    let colours = document.get_key(dict, "Colors");
    let Some(colours) = colours.as_dict() else {
        return Colours::default();
    };
    let rgb = |key: &str| {
        let value = document.get_key(colours, key);
        let array = value.as_array()?;
        let [r, g, b, ..] = array else {
            return None;
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a colour component in f32, as every colour in this crate is"
        )]
        Some([
            document.resolve(r).as_number()? as f32,
            document.resolve(g).as_number()? as f32,
            document.resolve(b).as_number()? as f32,
        ])
    };
    Colours {
        background: rgb("Background"),
        card_background: rgb("CardBackground"),
        card_border: rgb("CardBorder"),
        primary_text: rgb("PrimaryText"),
        secondary_text: rgb("SecondaryText"),
    }
}

/// Table 158's splitter bar.
fn split(document: &Document, dict: &Dictionary) -> Option<Split> {
    let split = document.get_key(dict, "Split");
    let split = split.as_dict()?;
    Some(Split {
        direction: match document
            .get_key(split, "Direction")
            .as_name()
            .map(|name| name.as_bytes().to_vec())
            .as_deref()
        {
            Some(b"V") => SplitDirection::Vertical,
            Some(b"N") => SplitDirection::None,
            _ => SplitDirection::Horizontal,
        },
        position: document
            .get_key(split, "Position")
            .as_number()
            .map(|value| {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a percentage, clamped to 0..=100 on the next line"
                )]
                let value = value as f32;
                value.clamp(0.0, 100.0)
            }),
    })
}

/// Table 159's folder tree, from Table 153's `/Folders` downwards.
fn folders(document: &Document, dict: &Dictionary) -> Option<Folder> {
    let root = document.get_key(dict, "Folders");
    let root = root.as_dict()?;
    let mut visited = std::collections::BTreeSet::new();
    // The root's own identity goes into the visited set before the walk starts, because Table
    // 153 makes `/Folders` "an indirect reference" and a `/Child` pointing back at it would
    // otherwise read the whole tree a second time. §12.3.5.2 says the root is "the single common
    // ancestor of all other folders", so a folder that contains it is a file contradicting
    // itself.
    if let Some(id) = dict.get("Folders").and_then(Object::as_reference) {
        visited.insert(id);
    }
    let mut budget = MAX_FOLDERS;
    Some(folder(document, root, 0, &mut visited, &mut budget))
}

/// One folder and its `/Child`–`/Next` descendants.
fn folder(
    document: &Document,
    dict: &Dictionary,
    depth: usize,
    visited: &mut std::collections::BTreeSet<ObjectId>,
    budget: &mut usize,
) -> Folder {
    let mut children = Vec::new();
    if depth < MAX_DEPTH {
        // `/Child` is "the first child folder" and `/Next` chains the rest at that level, which
        // is §12.3.3's outline shape again — so it gets the outline's defence: follow the two
        // forward links only, and refuse to visit an object twice.
        let mut next = dict.get("Child").and_then(Object::as_reference);
        while let Some(id) = next {
            if *budget == 0 || !visited.insert(id) {
                break;
            }
            *budget = budget.saturating_sub(1);
            let child = document.get(id);
            let Some(child) = child.as_dict() else {
                break;
            };
            children.push(folder(
                document,
                child,
                depth.saturating_add(1),
                visited,
                budget,
            ));
            next = child.get("Next").and_then(Object::as_reference);
        }
    }
    Folder {
        id: document
            .get_key(dict, "ID")
            .as_integer()
            .and_then(|id| u32::try_from(id).ok())
            .unwrap_or(0),
        name: match document.get_key(dict, "Name") {
            Object::String(bytes) => pdf_syntax::text_string(&bytes),
            _ => String::new(),
        },
        description: match document.get_key(dict, "Desc") {
            Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
            _ => None,
        },
        item: item(document, dict),
        has_thumbnail: document.get_key(dict, "Thumb").as_stream().is_some(),
        children,
    }
}

/// §7.11.6's collection item, from a `/CI` on a folder or on a file specification.
#[must_use]
pub fn item(document: &Document, dict: &Dictionary) -> Item {
    let item = document.get_key(dict, "CI");
    let Some(item) = item.as_dict() else {
        return Item::default();
    };
    Item {
        values: item
            .iter()
            .filter(|(key, _)| key.as_bytes() != b"Type")
            .map(|(key, value)| {
                let resolved = document.resolve(value);
                let (data, prefix) = match resolved.as_dict() {
                    // Table 47's subitem: the data and a prefix the sort ignores.
                    Some(subitem) => (
                        document.get_key(subitem, "D"),
                        match document.get_key(subitem, "P") {
                            Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
                            _ => None,
                        },
                    ),
                    None => (resolved, None),
                };
                (
                    String::from_utf8_lossy(key.as_bytes()).into_owned(),
                    Value { data, prefix },
                )
            })
            .collect(),
    }
}

/// The `/EmbeddedFiles` keys, in the tree's own order.
///
/// Kept here rather than in [`crate::attachment`] because the *keys* are this clause's business:
/// §12.3.5.2 puts a folder identifier inside them, and an attachment's own name is its file
/// specification's.
#[must_use]
pub fn embedded_file_keys(document: &Document) -> Vec<String> {
    let Ok(catalog) = document.catalog() else {
        return Vec::new();
    };
    let names = document.get_key(&catalog, "Names");
    let Some(names) = names.as_dict() else {
        return Vec::new();
    };
    let files = document.get_key(names, "EmbeddedFiles");
    let Some(files) = files.as_dict() else {
        return Vec::new();
    };
    tree::name_pairs(files, &|object| document.resolve(object))
        .into_iter()
        .map(|(key, _)| pdf_syntax::text_string(&key))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        Collection, FieldKind, Initial, Layout, SplitDirection, View, folder_of, is_file_name,
    };
    use pdf_syntax::{Document, Object};

    /// Builds a document from object bodies numbered from 1.
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
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// §12.3.5.2's EXAMPLE 1, an email in-box, read as five columns and a sort.
    ///
    /// > /Collection << /Type /Collection /Schema << /Type /CollectionSchema
    /// > /from <</Subtype /S /N (From) /O 1 /V true /E false>> … >> /D (Doc1) /View /D
    /// > /Sort <</S /date /A false>> >>
    ///
    /// The interesting entry is `/size`, whose `/Subtype /Size` makes it a *file-related* field:
    /// its value is not in the collection item at all but in the embedded file's parameter
    /// dictionary, which is the division [`FieldKind::is_in_the_item`] answers.
    #[test]
    fn the_clauses_own_email_inbox_reads_as_five_columns() {
        let doc = document(&[
            "<< /Type /Catalog /Collection 2 0 R >>",
            "<< /Type /Collection /Schema << /Type /CollectionSchema \
             /from << /Subtype /S /N (From) /O 1 /V true /E false >> \
             /to << /Subtype /S /N (To) /O 2 /V true /E false >> \
             /date << /Subtype /D /N (Date received) /O 3 /V true /E false >> \
             /subject << /Subtype /S /N (Subject) /O 4 /V true /E false >> \
             /size << /Subtype /Size /N (Size) /O 5 /V true /E false >> >> \
             /D (Doc1) /View /D /Sort << /S /date /A false >> >>",
        ]);
        let collection = Collection::read(&doc).expect("a collection");
        assert_eq!(collection.schema.len(), 5);
        assert_eq!(
            collection.view,
            View::Details,
            "/View /D and its own default"
        );

        let from = collection.schema.get("from").expect("the from column");
        assert_eq!(from.kind, FieldKind::Text);
        assert_eq!(from.name, "From");
        assert_eq!(from.order, Some(1));
        assert!(from.visible && !from.editable);

        let date = collection.schema.get("date").expect("the date column");
        assert_eq!(date.kind, FieldKind::Date);
        assert!(date.kind.is_in_the_item());

        let size = collection.schema.get("size").expect("the size column");
        assert_eq!(size.kind, FieldKind::Size);
        assert!(
            !size.kind.is_in_the_item(),
            "a /Size field's data is the embedded file's, not the item's"
        );

        let sort = collection.sort.clone().expect("a /Sort");
        assert_eq!(sort.fields, ["date"]);
        assert!(!sort.ascending(0), "/A false is newest first for a date");
        assert!(
            !sort.ascending(1),
            "one boolean applies to every field the array names"
        );

        assert_eq!(
            collection.initial_document(&doc),
            Initial::Empty,
            "/D names Doc1 and this document embeds nothing at all"
        );
    }

    /// A collection item with a subitem: §7.11.6's two dictionaries, in the EXAMPLE 2 that
    /// §12.3.5.2 gives for them.
    ///
    /// > /CI << /Type /CollectionItem /from (Tom Jones) /to (Marry Jones)
    /// > /subject << /Type /CollectionSubitem /P (Re:) /D (Let's have lunch on Friday!) >>
    /// > /date (D:2005062109470307'00) >>
    ///
    /// The prefix is kept apart from the data because Table 47 makes them behave differently:
    /// it is shown to a person and "ignored when an interactive PDF processor sorts the items".
    #[test]
    fn a_collection_subitem_keeps_its_prefix_apart_from_its_data() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [] /Count 0 >>",
            "<< /Type /Filespec /F (mail.eml) /CI << /Type /CollectionItem \
             /from (Tom Jones) /to (Marry Jones) \
             /subject << /Type /CollectionSubitem /P (Re:) /D (Let's have lunch on Friday!) >> \
             /date (D:2005062109470307'00) >> >>",
        ]);
        let specification = doc
            .get(pdf_syntax::ObjectId {
                number: 3,
                generation: 0,
            })
            .as_dict()
            .cloned()
            .expect("a file specification");
        let item = super::item(&doc, &specification);
        assert_eq!(item.values.len(), 4, "/Type is not a field: {item:?}");

        let from = item.values.get("from").expect("the from value");
        assert_eq!(from.data, Object::String("Tom Jones".as_bytes().into()));
        assert!(from.prefix.is_none());

        let subject = item.values.get("subject").expect("the subject value");
        assert_eq!(subject.prefix.as_deref(), Some("Re:"));
        assert_eq!(
            subject.data,
            Object::String("Let's have lunch on Friday!".as_bytes().into()),
            "the prefix is not part of the value the sort compares"
        );
    }

    /// §12.3.5.2's naming convention: a folder identifier inside a name tree key.
    ///
    /// Four rules, and the failure of any of them means the root folder rather than an error —
    /// files that "do not conform to these rules shall be treated as associated with the root
    /// folder".
    #[test]
    fn a_files_folder_is_written_into_its_name_tree_key() {
        assert_eq!(folder_of("<3>report.pdf"), Some((3, "report.pdf")));
        assert_eq!(folder_of("<0>a"), Some((0, "a")));
        assert_eq!(folder_of("<12345>x.txt"), Some((12345, "x.txt")));
        assert_eq!(folder_of("<>x"), None, "one or more digits, not none");
        assert_eq!(folder_of("<3x>y"), None, "digits only inside the brackets");
        assert_eq!(
            folder_of("report.pdf"),
            None,
            "no marker is the root folder"
        );
        assert_eq!(folder_of("<3 report.pdf"), None, "unclosed");
        assert_eq!(
            folder_of("<3>"),
            Some((3, "")),
            "a marker and an empty name is still a marker"
        );
    }

    /// §12.3.5.2's file name rules: eight characters, no NUL, a length between 1 and 255, and no
    /// trailing full stop.
    #[test]
    fn a_folder_name_is_a_file_name_by_the_clauses_five_rules() {
        assert!(is_file_name("Invoices 2024"));
        assert!(is_file_name("a.b.c"));
        for bad in ["a/b", "a\\b", "a:b", "a*b", "a\"b", "a<b", "a>b", "a|b"] {
            assert!(!is_file_name(bad), "{bad} holds one of the eight");
        }
        assert!(
            !is_file_name("trailing."),
            "the last character is a full stop"
        );
        assert!(!is_file_name("nul\0inside"));
        assert!(!is_file_name(""), "the bound's low end is 1");
        assert!(
            is_file_name(&"a".repeat(255)),
            "\"between 1 and 255 inclusive\" includes 255"
        );
        assert!(!is_file_name(&"a".repeat(256)), "and excludes 256");
        assert!(
            is_file_name(&"é".repeat(255)),
            "the bound counts characters, and 255 of these are 510 bytes"
        );
    }

    /// A folder tree, a navigator's layout preference, colours and a splitter bar.
    ///
    /// The layout list is the one entry whose answer depends on the *viewer*: §12.3.6 says a
    /// processor "should present the first one it is capable of displaying in the order present
    /// in the array", so a caller that draws a tree and nothing else gets `Tree` from a file
    /// asking first for something exotic.
    #[test]
    fn a_navigator_names_layouts_in_the_order_the_file_prefers_them() {
        let doc = document(&[
            "<< /Type /Catalog /Collection 2 0 R >>",
            "<< /Type /Collection /View /C /Navigator 3 0 R /Folders 4 0 R \
             /Colors << /Background [1 1 1] /PrimaryText [0 0 0] >> \
             /Split << /Direction /V /Position 250 >> >>",
            "<< /Type /Navigator /Layout [/Carousel /FreeForm /Tree] >>",
            "<< /Type /Folder /ID 0 /Name (root) /Child 5 0 R /Free [7 99] >>",
            "<< /Type /Folder /ID 1 /Name (Invoices) /Next 6 0 R /Desc (paid) >>",
            "<< /Type /Folder /ID 2 /Name (Photos) /Child 7 0 R >>",
            "<< /Type /Folder /ID 3 /Name (2024) >>",
        ]);
        let collection = Collection::read(&doc).expect("a collection");
        assert_eq!(collection.view, View::Navigator);

        let navigator = collection.navigator.clone().expect("a /Navigator");
        assert_eq!(
            navigator.layouts,
            vec![
                Layout::Custom("Carousel".to_owned()),
                Layout::FreeForm,
                Layout::Tree
            ]
        );
        assert_eq!(
            navigator.preferred(&[Layout::Tree, Layout::View(View::Details)]),
            Some(Layout::Tree),
            "the first the file names that this viewer can draw"
        );
        assert_eq!(
            navigator.preferred(&[Layout::FreeForm, Layout::Tree]),
            Some(Layout::FreeForm),
            "the file's order decides, not the viewer's"
        );
        assert_eq!(navigator.preferred(&[Layout::Linear]), None);

        let root = collection.folders.as_ref().expect("a root folder");
        assert_eq!(root.name, "root");
        assert_eq!(
            root.children.iter().map(|f| f.id).collect::<Vec<_>>(),
            vec![1, 2],
            "/Child enters the level and /Next walks it"
        );
        assert_eq!(root.children[0].description.as_deref(), Some("paid"));
        assert_eq!(root.children[1].children[0].name, "2024");
        assert_eq!(collection.all_folders().len(), 4);

        assert_eq!(collection.colours.background, Some([1.0, 1.0, 1.0]));
        assert_eq!(collection.colours.card_border, None);
        let split = collection.split.expect("a /Split");
        assert_eq!(split.direction, SplitDirection::Vertical);
        assert_eq!(
            split.position,
            Some(100.0),
            "Table 158 states 0 to 100, and 250 is a file outside it"
        );
    }

    /// A folder tree that points back into itself ends rather than looping.
    #[test]
    fn a_folder_tree_that_re_enters_itself_terminates() {
        let doc = document(&[
            "<< /Type /Catalog /Collection 2 0 R >>",
            "<< /Type /Collection /Folders 3 0 R >>",
            "<< /Type /Folder /ID 0 /Name (root) /Child 4 0 R >>",
            "<< /Type /Folder /ID 1 /Name (a) /Child 3 0 R /Next 4 0 R >>",
        ]);
        let collection = Collection::read(&doc).expect("a collection");
        assert_eq!(collection.all_folders().len(), 2, "root and a, once each");
    }
}
