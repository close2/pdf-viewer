//! `include/quorra.h` against the .NET declarations in `hosts/windows/Quorra.Interop/`, read back
//! as text (RFC 0010 step N0, ADR 2054).
//!
//! The C# is hand-written for the header's own reason: it is what a .NET programmer reads. No gate
//! of this tree compiles it — the machine the gates run on has no .NET SDK, and `dotnet build` is a
//! CI job owed to a later step — so what stops it going stale is this test, which needs no
//! `dotnet`:
//!
//! - every function the header declares is declared exactly once in C#, and the C# declares no
//!   other;
//! - each has the header's parameters, by name and in order, and each parameter, result and struct
//!   field has the C# type that one rule gives its C type ([`csharp_type`]). **A count of arguments
//!   alone would pass a `uint` where the header says `size_t`**, which in a 64-bit process is a call
//!   that compiles, links, runs and reads half a word;
//! - every `QUORRA_` constant has the header's value, and the type its `u` suffix gives it;
//! - each function is a cdecl `LibraryImport` of the library this crate builds.
//!
//! `header_and_library_agree.rs` holds the header to `src/abi.rs`; the two together hold the C# to
//! the library. Neither reads the prose: the header's comments are the one place an entry point is
//! explained, and the C# repeats none of them (ADR 2054).

#![expect(
    clippy::expect_used,
    reason = "test code: a source file that cannot be read must fail loudly rather than pass by \
              doing nothing"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// A function's result and parameters, each parameter as its type and its name.
#[derive(Debug, PartialEq, Eq)]
struct Signature {
    result: String,
    parameters: Vec<(String, String)>,
}

/// A `#define QUORRA_…` or a C# `const`: its value, and whether it is unsigned.
#[derive(Debug, PartialEq, Eq)]
struct Constant {
    value: i64,
    unsigned: bool,
}

/// What the header declares, read from its text with the comments taken out.
#[derive(Debug, Default)]
struct Header {
    functions: BTreeMap<String, Signature>,
    /// Each struct with a body, as its fields in order, a field as `type name` or `type name[n]`.
    structs: BTreeMap<String, Vec<(String, String)>>,
    /// Each `typedef struct quorra_x quorra_x;`: a handle the caller holds and never looks inside.
    opaque: BTreeSet<String>,
    constants: BTreeMap<String, Constant>,
}

/// What the C# declares, read the same way.
#[derive(Debug, Default)]
struct Interop {
    /// Each declaration and how many times it is declared.
    functions: BTreeMap<String, (Signature, usize)>,
    /// The functions whose attributes are not a cdecl `LibraryImport(Library, …Utf8)`.
    unimported: Vec<String>,
    /// Each struct as its fields, a field as its C# type and name, and whether it is sequential.
    structs: BTreeMap<String, (Vec<(String, String)>, bool)>,
    constants: BTreeMap<String, Constant>,
    library: Option<String>,
}

/// `text` with every `/* … */` taken out, and every `// …` too where `line_comments` says so.
///
/// A comment names functions and constants in prose, and a check that read those would be checking
/// the documentation rather than the declarations.
fn without_comments(text: &str, line_comments: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("/*") {
        out.push_str(&rest[..at]);
        let after = &rest[at.saturating_add(2)..];
        let Some(end) = after.find("*/") else {
            rest = "";
            break;
        };
        rest = &after[end.saturating_add(2)..];
    }
    out.push_str(rest);
    if !line_comments {
        return out;
    }
    out.lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One declaration's text with each `*` a token of its own and the whitespace collapsed, so that
/// `const char *const *names` and `const char * const * names` read alike.
fn tokens(text: &str) -> Vec<String> {
    text.replace('*', " * ")
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// A C parameter or field split into its type and its name, the name being the last token.
fn type_and_name(declaration: &str) -> (String, String) {
    let mut parts = tokens(declaration);
    let name = parts.pop().unwrap_or_default();
    (parts.join(" "), name)
}

/// The text between a statement's first `(` and its last `)`, and what comes before the `(`.
fn split_call(statement: &str) -> Option<(&str, &str)> {
    let open = statement.find('(')?;
    let close = statement.rfind(')')?;
    Some((
        &statement[..open],
        statement.get(open.saturating_add(1)..close)?,
    ))
}

fn read_header() -> Header {
    let text =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("include/quorra.h"))
            .expect("this crate has a header");
    let mut header = Header::default();
    let mut code = String::new();
    for line in without_comments(&text, false).lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("#define QUORRA_") {
            let mut parts = rest.split_whitespace();
            if let (Some(name), Some(value)) = (parts.next(), parts.next()) {
                let unsigned = value.ends_with('u');
                if let Ok(value) = value.trim_end_matches('u').parse::<i64>() {
                    header
                        .constants
                        .insert(format!("QUORRA_{name}"), Constant { value, unsigned });
                }
            }
        } else if !trimmed.starts_with('#') && trimmed != "extern \"C\" {" && trimmed != "}" {
            code.push_str(line);
            code.push('\n');
        }
    }
    // The structs with a body are taken out first, because their fields end in `;` too.
    let mut rest = code.as_str();
    let mut statements = String::new();
    while let Some(at) = rest.find("typedef struct ") {
        statements.push_str(&rest[..at]);
        let after = &rest[at..];
        let end = after.find(';').expect("a typedef ends");
        let head = &after[..end];
        if let Some(open) = head.find('{') {
            let close = after.find('}').expect("a struct body closes");
            let name = after["typedef struct ".len()..open].trim().to_owned();
            let fields = after[open.saturating_add(1)..close]
                .split(';')
                .filter(|field| !field.trim().is_empty())
                .map(type_and_name)
                .collect();
            header.structs.insert(name, fields);
            let tail = &after[close..];
            rest = &tail[tail.find(';').expect("a typedef ends").saturating_add(1)..];
        } else {
            let name = tokens(head).pop().unwrap_or_default();
            header.opaque.insert(name);
            rest = &after[end.saturating_add(1)..];
        }
    }
    statements.push_str(rest);
    for statement in statements.split(';') {
        let Some((before, inside)) = split_call(statement) else {
            continue;
        };
        let mut before = tokens(before);
        let Some(name) = before.pop().filter(|name| name.starts_with("quorra_")) else {
            continue;
        };
        let parameters = if inside.trim() == "void" {
            Vec::new()
        } else {
            inside.split(',').map(type_and_name).collect()
        };
        header.functions.insert(
            name,
            Signature {
                result: before.join(" "),
                parameters,
            },
        );
    }
    header
}

/// The directory the .NET declarations live in: `hosts/` sits beside `crates/`, outside the Cargo
/// workspace, as `kio/` does.
fn interop_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../hosts/windows/Quorra.Interop")
}

/// A C# parameter as its type and its name, a name's `@` (C#'s way of using a keyword as one)
/// dropped so that it is the header's.
fn csharp_type_and_name(parameter: &str) -> (String, String) {
    let parameter = parameter.split_whitespace().collect::<Vec<_>>().join(" ");
    let (kind, name) = parameter
        .rsplit_once(' ')
        .unwrap_or(("", parameter.as_str()));
    (kind.to_owned(), name.trim_start_matches('@').to_owned())
}

fn read_interop() -> Interop {
    let mut interop = Interop::default();
    let mut sources: Vec<PathBuf> = std::fs::read_dir(interop_directory())
        .expect("hosts/windows/Quorra.Interop exists")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "cs"))
        .collect();
    sources.sort();
    assert!(
        !sources.is_empty(),
        "no C# under {}",
        interop_directory().display()
    );
    for source in sources {
        let text = std::fs::read_to_string(&source).expect("a C# file reads");
        let code: String = without_comments(&text, true)
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");
        read_structs(&code, &mut interop);
        for statement in code.split(';') {
            read_statement(statement, &mut interop);
        }
    }
    interop
}

/// One `;`-terminated C# statement: a function, a constant, or something this test does not read.
fn read_statement(statement: &str, interop: &mut Interop) {
    let mut flat = statement.split_whitespace().collect::<Vec<_>>().join(" ");
    // A class's opening line belongs to whatever follows its `{` in the same statement.
    if let Some(at) = flat.find(" class ") {
        flat = flat[at..]
            .split_once('{')
            .map_or_else(String::new, |(_, body)| body.to_owned());
    }
    if let Some(at) = flat.find("static partial ") {
        let declaration = &flat[at.saturating_add("static partial ".len())..];
        let Some((before, inside)) = split_call(declaration) else {
            return;
        };
        let Some((result, name)) = before.trim().rsplit_once(' ') else {
            return;
        };
        let attributes = &flat[..at];
        if !(attributes
            .contains("[LibraryImport(Library, StringMarshalling = StringMarshalling.Utf8)]")
            && attributes.contains("typeof(CallConvCdecl)"))
        {
            interop.unimported.push(name.to_owned());
        }
        let parameters = if inside.trim().is_empty() {
            Vec::new()
        } else {
            inside.split(',').map(csharp_type_and_name).collect()
        };
        let entry = interop.functions.entry(name.to_owned()).or_insert((
            Signature {
                result: result.to_owned(),
                parameters,
            },
            0,
        ));
        entry.1 = entry.1.saturating_add(1);
    } else if let Some(at) = flat.find("const ") {
        let words: Vec<&str> = flat[at.saturating_add("const ".len())..]
            .split_whitespace()
            .collect();
        match words.as_slice() {
            ["string", "Library", "=", value] => {
                interop.library = Some(value.trim_matches('"').to_owned());
            }
            [kind @ ("uint" | "int"), name, "=", value] => {
                if let Ok(value) = value.parse::<i64>() {
                    interop.constants.insert(
                        (*name).to_owned(),
                        Constant {
                            value,
                            unsigned: *kind == "uint",
                        },
                    );
                }
            }
            _ => {}
        }
    }
}

/// Every `struct quorra_… { … }` in one C# file, with whether a `LayoutKind.Sequential` stands
/// between it and whatever came before.
fn read_structs(code: &str, interop: &mut Interop) {
    let mut rest = code;
    let mut previous_end = 0usize;
    let mut offset = 0usize;
    while let Some(at) = rest.find(" struct quorra_") {
        let after = &rest[at.saturating_add(" struct ".len())..];
        let Some(open) = after.find('{') else {
            break;
        };
        let Some(close) = after.find('}') else {
            break;
        };
        let name = after[..open].trim().to_owned();
        let absolute = offset.saturating_add(at);
        let sequential =
            code[previous_end..absolute].contains("[StructLayout(LayoutKind.Sequential)]");
        let fields = after[open.saturating_add(1)..close]
            .split(';')
            .filter(|field| !field.trim().is_empty())
            .map(|field| {
                let field = field.split_whitespace().collect::<Vec<_>>().join(" ");
                let field = field
                    .strip_prefix("internal ")
                    .or_else(|| field.strip_prefix("public "))
                    .unwrap_or(&field)
                    .to_owned();
                csharp_type_and_name(&field)
            })
            .collect();
        interop.structs.insert(name, (fields, sequential));
        let consumed = at
            .saturating_add(" struct ".len())
            .saturating_add(close)
            .saturating_add(1);
        offset = offset.saturating_add(consumed);
        previous_end = offset;
        rest = &rest[consumed..];
    }
}

/// Where a C type is written, which decides two of the rule's lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    Argument,
    Result,
    Field,
}

/// The fixed-width integers and `float`, each the C# type of the same width and signedness.
fn scalar(base: &str) -> Option<&'static str> {
    Some(match base {
        "uint8_t" => "byte",
        "int8_t" => "sbyte",
        "uint16_t" => "ushort",
        "int16_t" => "short",
        "uint32_t" => "uint",
        "int32_t" => "int",
        "uint64_t" => "ulong",
        "int64_t" => "long",
        "size_t" => "nuint",
        "float" => "float",
        "double" => "double",
        _ => return None,
    })
}

/// **The rule**, which `hosts/windows/Quorra.Interop/Native.cs` states in prose (ADR 2054):
///
/// - a fixed-width integer, `size_t` and `float` are the C# type of the same width;
/// - a `bool` argument is `[MarshalAs(UnmanagedType.U1)] bool`, since C gives it one byte and the
///   marshaller's default is four; a `bool` in memory — a field, or what a `bool *` points at — is a
///   `byte`, which keeps a struct holding one blittable;
/// - a `const char *` argument is `string?`, marshalled as UTF-8, because every string the header
///   takes is UTF-8 and a null is the library's `QUORRA_NULL_ARGUMENT` rather than a crash;
/// - every other `char *` is `byte*`: an argument is a buffer the caller owns, and a result is a
///   static string a marshalled `string` would try to free;
/// - an opaque handle is `nint`, and each further `*` a pointer to one;
/// - a struct is the struct of the header's name, and any other `T *` is the rule's `T` and `*`.
fn csharp_type(c: &str, place: Place, header: &Header) -> String {
    let parts = tokens(c);
    let pointers = parts.iter().filter(|part| *part == "*").count();
    let leading_const = parts.first().is_some_and(|first| first == "const");
    let bases: Vec<&String> = parts
        .iter()
        .filter(|part| *part != "*" && *part != "const")
        .collect();
    let [base] = bases.as_slice() else {
        return format!("<no rule for `{c}`>");
    };
    let base = base.as_str();
    let stars = "*".repeat(pointers);
    match (base, pointers, place) {
        ("void", 0, Place::Result) => "void".to_owned(),
        ("char", 1, Place::Argument) if leading_const => "string?".to_owned(),
        ("bool", 0, Place::Argument) => "[MarshalAs(UnmanagedType.U1)] bool".to_owned(),
        ("char" | "bool", _, _) => format!("byte{stars}"),
        (handle, 1.., _) if header.opaque.contains(handle) => {
            format!("nint{}", "*".repeat(pointers.saturating_sub(1)))
        }
        (shape, _, _) if header.structs.contains_key(shape) => format!("{shape}{stars}"),
        (number, _, _) => scalar(number).map_or_else(
            || format!("<no rule for `{c}`>"),
            |number| format!("{number}{stars}"),
        ),
    }
}

/// A header field as the C# field the rule makes of it: an array `T name[n]` becomes a `fixed`
/// buffer, which is the one C# field with the array's own layout.
fn csharp_field(kind: &str, name: &str, header: &Header) -> (String, String) {
    let mapped = csharp_type(kind, Place::Field, header);
    if name.contains('[') {
        (format!("fixed {mapped}"), name.to_owned())
    } else {
        (mapped, name.to_owned())
    }
}

#[test]
fn every_function_in_the_header_is_declared_once_in_the_interop_layer_with_its_types() {
    let header = read_header();
    let interop = read_interop();
    assert!(
        !header.functions.is_empty(),
        "the header reader found no functions, which says the reader is broken"
    );
    let mut wrong = String::new();
    for name in header.functions.keys() {
        if !interop.functions.contains_key(name) {
            let _ = writeln!(wrong, "the header declares {name} and the C# does not");
        }
    }
    for (name, (signature, times)) in &interop.functions {
        if *times != 1 {
            let _ = writeln!(wrong, "{name} is declared {times} times in the C#");
        }
        let Some(declared) = header.functions.get(name) else {
            let _ = writeln!(wrong, "the C# declares {name} and the header does not");
            continue;
        };
        let expected = Signature {
            result: csharp_type(&declared.result, Place::Result, &header),
            parameters: declared
                .parameters
                .iter()
                .map(|(kind, parameter)| {
                    (
                        csharp_type(kind, Place::Argument, &header),
                        parameter.clone(),
                    )
                })
                .collect(),
        };
        if *signature != expected {
            let _ = writeln!(
                wrong,
                "{name}: the C# says {signature:?}, the header's rule {expected:?}"
            );
        }
    }
    for name in &interop.unimported {
        let _ = writeln!(
            wrong,
            "{name} is not a cdecl LibraryImport(Library, StringMarshalling.Utf8)"
        );
    }
    assert!(wrong.is_empty(), "the C# and the header disagree:\n{wrong}");
    println!(
        "{} functions declared in both, against the header as it stands",
        header.functions.len()
    );
}

#[test]
fn every_struct_in_the_header_is_the_same_fields_in_the_interop_layer() {
    let header = read_header();
    let interop = read_interop();
    let expected: BTreeMap<String, (Vec<(String, String)>, bool)> = header
        .structs
        .iter()
        .map(|(name, fields)| {
            let fields = fields
                .iter()
                .map(|(kind, field)| csharp_field(kind, field, &header))
                .collect();
            (name.clone(), (fields, true))
        })
        .collect();
    assert!(!expected.is_empty(), "the header reader found no structs");
    let mut wrong = String::new();
    for (name, (fields, _)) in &expected {
        match interop.structs.get(name) {
            None => {
                let _ = writeln!(wrong, "the header defines {name} and the C# does not");
            }
            Some((_, false)) => {
                let _ = writeln!(wrong, "{name} is not LayoutKind.Sequential in the C#");
            }
            Some((declared, true)) if declared != fields => {
                let _ = writeln!(
                    wrong,
                    "{name}: the C# fields are {declared:?}, the header's rule {fields:?}"
                );
            }
            Some(_) => {}
        }
    }
    for name in interop.structs.keys() {
        if !expected.contains_key(name) {
            let _ = writeln!(wrong, "the C# defines {name} and the header does not");
        }
    }
    assert!(
        wrong.is_empty(),
        "the C# structs and the header's have come apart:\n{wrong}"
    );
}

#[test]
fn every_constant_in_the_header_has_its_value_and_its_signedness_in_the_interop_layer() {
    let header = read_header();
    let interop = read_interop();
    assert!(
        !header.constants.is_empty(),
        "the header reader found no constants"
    );
    let mut wrong = String::new();
    for (name, constant) in &header.constants {
        match interop.constants.get(name) {
            None => {
                let _ = writeln!(wrong, "the header defines {name} and the C# does not");
            }
            Some(declared) if declared != constant => {
                let _ = writeln!(
                    wrong,
                    "{name}: the C# says {declared:?}, the header {constant:?}"
                );
            }
            Some(_) => {}
        }
    }
    for name in interop.constants.keys() {
        if !header.constants.contains_key(name) {
            let _ = writeln!(wrong, "the C# defines {name} and the header does not");
        }
    }
    assert!(
        wrong.is_empty(),
        "the C# numbers and the header's have come apart:\n{wrong}"
    );
}

#[test]
fn the_interop_layer_names_the_library_this_crate_builds() {
    // Cargo names a library after its package with `-` made `_` where `[lib]` names none, as this
    // crate's manifest does not; the runtime adds `lib` and `.so` or `.dll` around it.
    let built = env!("CARGO_PKG_NAME").replace('-', "_");
    assert_eq!(read_interop().library, Some(built));
}
