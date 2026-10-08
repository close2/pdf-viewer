//! `util`'s seven helpers that are pure functions of their arguments: `scand`, `crackURL`,
//! `spansToXML`, `xmlToSpans`, `streamFromString`, `stringFromStream` and `iconStreamFromIcon`
//! (ADR 1762).
//!
//! RFC 0008 section 4.2 admits each to Tier 1, and none reaches anything outside the realm: each
//! reads its arguments and answers a value. Their meanings are Adobe's *JavaScript for Acrobat API
//! Reference*, "util methods", read at `adobe/dc-acrobat-sdk-docs` commit `ab3b42a7`, cited and
//! never quoted (`doc/todo/56`); where the reference leaves a meaning open the item that decides
//! it says so, as a documented choice under principle 5. `util.readFileIntoStream`, the eighth
//! stream member, reaches the filesystem and is RFC 0008 section 4.3's (`crate::surface`).

use boa_engine::object::ObjectInitializer;
use boa_engine::object::builtins::JsArray;
use boa_engine::{Context, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction};
use pdf_model::aform::{DateTime, parse_date};
use pdf_model::span::{self, Span};

use super::bridge::{data, function, text_argument};
use super::{guard, refuse};
use crate::RefusalKind;

/// The most bytes `stringFromStream` reads from one stream, whatever the string budget says.
///
/// The stream's `read` may be a script's own function, which the step budget does not see while a
/// native calls it; each answer it gives grows the text this function holds, so the string budget
/// stops a stream that never ends, and this bound stops it where a realm runs with none. Sixteen
/// mebibytes is far past any form's attachment a script could have been handed.
const MAX_STREAM_BYTES: usize = 16 << 20;

/// How many bytes `stringFromStream` asks a stream's `read` for at a time.
const READ_CHUNK: u32 = 64 << 10;

/// The most times `stringFromStream` calls one stream's `read`.
///
/// A script's own `read` runs inside this native, where neither the step budget nor the clock is
/// looked at, so a stream answering one byte at a time would hold the realm for as many calls as
/// the byte bounds allow. A stream that answers what it is asked for reaches
/// [`MAX_STREAM_BYTES`] in 256 calls; sixteen times that is room for one that answers in smaller
/// pieces.
const MAX_READS: usize = 4096;

/// Installs the seven on `util`.
///
/// # Errors
///
/// The engine's, where a property cannot be defined.
pub(super) fn install(util: &JsObject, context: &mut Context) -> JsResult<()> {
    let methods: [(&str, NativeFunction); 7] = [
        ("scand", NativeFunction::from_fn_ptr(scan_date)),
        ("crackURL", NativeFunction::from_fn_ptr(crack_url)),
        ("spansToXML", NativeFunction::from_fn_ptr(spans_to_xml)),
        ("xmlToSpans", NativeFunction::from_fn_ptr(xml_to_spans)),
        (
            "streamFromString",
            NativeFunction::from_fn_ptr(stream_from_string),
        ),
        (
            "stringFromStream",
            NativeFunction::from_fn_ptr(string_from_stream),
        ),
        (
            "iconStreamFromIcon",
            NativeFunction::from_fn_ptr(icon_stream_from_icon),
        ),
    ];
    for (name, native) in methods {
        let callable = function(context, name, native);
        data(util, name, JsValue::from(callable), false, context)?;
    }
    Ok(())
}

/// A `TypeError` naming the member, for an argument the reference says the method throws on.
fn parameter_error(member: &str, why: &str) -> boa_engine::JsError {
    JsNativeError::typ()
        .with_message(format!("util.{member}: {why}"))
        .into()
}

// ---------------------------------------------------------------------------------------------
// scand

/// `util.scand(cFormat, cDate)`: a date's text read back through `printd`'s picture, as a `Date`
/// in local time at the request's offset, or `null` where the text does not follow the picture.
///
/// The picture is read by `pdf_model::aform::parse_date`, the reader `AFParseDateEx` and
/// `AFDate_KeystrokeEx` already read a field's text through, so a script's `scand` and a form's
/// date keystroke agree on every text. That reader applies the reference's two-digit-year horizon
/// (below 50 is the twenty-first century). **The numbered formats are a documented choice**: the
/// reference says only that `cFormat` takes `printd`'s syntax, and `printd` takes 0, 1 and 2, so
/// each is read as the inverse of what `printd` writes for it (ADR 1615) — 0 and 1 as §7.9.4's date
/// string (`D:` and fourteen digits, then `Z`, an offset, or nothing, which §7.9.4 makes Universal
/// Time), 2 as `yyyy/mm/dd HH:MM:ss` in local time. Any other number is a format that converts
/// nothing, and answers `null` as a failed conversion does.
fn scan_date(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let format = arguments.first().cloned().unwrap_or_default();
    let text = text_argument(arguments, 1, context)?;
    let numbered = format.as_number().map(|number| {
        (number.fract().abs() < f64::EPSILON).then(|| super::bridge::integral(number))
    });
    let read = match numbered {
        Some(Some(0 | 1)) => date_string(&text),
        Some(Some(2)) => parse_date(&text, "yyyy/mm/dd HH:MM:ss").map(|moment| (moment, None)),
        Some(_) => None,
        None => {
            let picture = format.to_string(context)?.to_std_string_lossy();
            parse_date(&text, &picture).map(|moment| (moment, None))
        }
    };
    let Some((moment, offset)) = read else {
        return Ok(JsValue::null());
    };
    let date = context.intrinsics().constructors().date().constructor();
    let parts = [
        f64::from(moment.year),
        // ECMA-262 counts months from zero.
        f64::from(moment.month).max(1.0) - 1.0,
        f64::from(moment.day),
        f64::from(moment.hour),
        f64::from(moment.minute),
        f64::from(moment.second),
    ]
    .map(JsValue::from);
    let Some(offset_minutes) = offset else {
        return Ok(JsValue::from(date.construct(&parts, None, context)?));
    };
    // A stated offset fixes the moment in Universal Time: `Date.UTC` of the fields, less the
    // offset east of it.
    let utc = date.get(JsString::from("UTC"), context)?;
    let Some(utc) = utc.as_callable() else {
        return Ok(JsValue::null());
    };
    let millis = utc
        .call(&JsValue::undefined(), &parts, context)?
        .to_number(context)?
        - f64::from(offset_minutes) * 60_000.0;
    Ok(JsValue::from(date.construct(
        &[JsValue::from(millis)],
        None,
        context,
    )?))
}

/// §7.9.4's date string as `printd(0)` and `printd(1)` write it, with its offset in minutes east
/// of Universal Time — zero where the string states none, which §7.9.4 reads as Universal Time.
fn date_string(text: &str) -> Option<(DateTime, Option<i32>)> {
    let body = text.trim().strip_prefix("D:")?;
    let digits = body.get(..14)?;
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let moment = parse_date(digits, "yyyymmddHHMMss")?;
    let zone = body.get(14..)?;
    let offset = match zone {
        "" | "Z" => 0,
        signed => {
            let (west, rest) = match signed.as_bytes().first()? {
                b'+' => (false, signed.get(1..)?),
                b'-' => (true, signed.get(1..)?),
                _ => return None,
            };
            let rest = rest.strip_suffix('\'').unwrap_or(rest);
            let (hours, minutes) = rest.split_once('\'')?;
            if hours.len() != 2 || minutes.len() != 2 {
                return None;
            }
            let hours: i32 = hours.parse().ok()?;
            let minutes: i32 = minutes.parse().ok()?;
            if hours > 23 || minutes > 59 {
                return None;
            }
            // Both fields are range-checked above, so neither operation can overflow.
            let minutes = hours.saturating_mul(60).saturating_add(minutes);
            if west {
                minutes.saturating_neg()
            } else {
                minutes
            }
        }
    };
    Some((moment, Some(offset)))
}

// ---------------------------------------------------------------------------------------------
// crackURL

/// A URL's parts, as `crackURL` answers them.
#[derive(Debug, Default, PartialEq, Eq)]
struct Parts {
    /// The scheme, in lower case.
    scheme: String,
    /// The user name, where the authority states one.
    user: Option<String>,
    /// The password, where the authority states one.
    password: Option<String>,
    /// The host, an IPv6 literal without its brackets.
    host: String,
    /// The port: stated, or the scheme's default; `None` for `file`, which has none.
    port: Option<u16>,
    /// The path, where there is one.
    path: Option<String>,
    /// The query, without its `?`.
    query: Option<String>,
    /// The fragment, without its `#`.
    fragment: Option<String>,
    /// Whether the host is an IPv6 literal.
    ipv6: bool,
}

/// `util.crackURL(cURL)`: a URL split into the reference's properties — `cScheme`, `cUser`,
/// `cPassword`, `cHost`, `nPort`, `cPath`, `cQuery`, `cFragment` and `nURLType`.
///
/// The split is RFC 3986's generic syntax (section 3): the scheme up to the first `:`, the
/// authority after `//` up to the path, its user information before the last `@` and split at the
/// first `:`, an IPv6 literal in brackets (section 3.2.2), the port after the host; the query after
/// the first `?` and the fragment after the first `#`. The reference's documented choices, each
/// this program's: the components are answered as written, never percent-decoded, since the
/// reference's example shows its query undecoded; an optional property is present only where the
/// URL states it; `nPort` is the scheme's default (80 for `http`, 443 for `https`) where none is
/// written, and absent for `file`, which RFC 8089 gives no port; `nURLType` is 1 for an IPv6
/// literal and 0 for anything else, a host name included, since the reference makes no effort to
/// resolve one. A missing argument, a URL that is not well formed and a scheme other than `file`,
/// `http` or `https` throw a `TypeError`, the reference's parameter error.
fn crack_url(_this: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    if arguments.first().is_none_or(JsValue::is_undefined) {
        return Err(parameter_error("crackURL", "its URL argument is missing"));
    }
    let url = text_argument(arguments, 0, context)?;
    let parts = split_url(&url).map_err(|why| parameter_error("crackURL", why))?;
    let object = ObjectInitializer::new(context).build();
    let text = |text: &str| JsValue::from(JsString::from(text));
    let mut properties: Vec<(&str, JsValue)> = vec![("cScheme", text(&parts.scheme))];
    if let Some(user) = &parts.user {
        properties.push(("cUser", text(user)));
    }
    if let Some(password) = &parts.password {
        properties.push(("cPassword", text(password)));
    }
    properties.push(("cHost", text(&parts.host)));
    if let Some(port) = parts.port {
        properties.push(("nPort", JsValue::from(port)));
    }
    if let Some(path) = &parts.path {
        properties.push(("cPath", text(path)));
    }
    if let Some(query) = &parts.query {
        properties.push(("cQuery", text(query)));
    }
    if let Some(fragment) = &parts.fragment {
        properties.push(("cFragment", text(fragment)));
    }
    properties.push(("nURLType", JsValue::from(u8::from(parts.ipv6))));
    for (name, value) in properties {
        object.create_data_property_or_throw(JsString::from(name), value, context)?;
    }
    Ok(JsValue::from(object))
}

/// RFC 3986's split of one URL, or the sentence saying why it is not one `crackURL` reads.
fn split_url(url: &str) -> Result<Parts, &'static str> {
    if url.is_empty() {
        return Err("its URL is empty");
    }
    // RFC 3986 section 2 admits no space or control character anywhere in a URI.
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(
            "its URL holds white space or a control character, which RFC 3986 admits nowhere",
        );
    }
    let (scheme, rest) = url.split_once(':').ok_or("its URL states no scheme")?;
    // Section 3.1: a letter, then letters, digits, `+`, `-` and `.`.
    let mut letters = scheme.chars();
    if !letters.next().is_some_and(|c| c.is_ascii_alphabetic())
        || !letters.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    {
        return Err("its URL's scheme is not one RFC 3986 section 3.1 admits");
    }
    let scheme = scheme.to_ascii_lowercase();
    let default_port = match scheme.as_str() {
        "http" => Some(80),
        "https" => Some(443),
        "file" => None,
        _ => return Err("its URL's scheme is not file, http or https"),
    };
    let mut parts = Parts {
        scheme,
        port: default_port,
        ..Parts::default()
    };
    let (rest, fragment) = match rest.split_once('#') {
        Some((before, fragment)) => (before, Some(fragment)),
        None => (rest, None),
    };
    let (rest, query) = match rest.split_once('?') {
        Some((before, query)) => (before, Some(query)),
        None => (rest, None),
    };
    let path = if let Some(after) = rest.strip_prefix("//") {
        let end = after.find('/').unwrap_or(after.len());
        let (authority, path) = after.split_at(end);
        read_authority(authority, &mut parts)?;
        path
    } else {
        rest
    };
    if parts.host.is_empty() && parts.scheme != "file" {
        return Err("its URL names no host, which an http or https URL must");
    }
    parts.path = (!path.is_empty()).then(|| path.to_owned());
    parts.query = query.filter(|query| !query.is_empty()).map(str::to_owned);
    parts.fragment = fragment
        .filter(|fragment| !fragment.is_empty())
        .map(str::to_owned);
    Ok(parts)
}

/// RFC 3986 section 3.2's authority: `[userinfo@]host[:port]`.
fn read_authority(authority: &str, parts: &mut Parts) -> Result<(), &'static str> {
    let host_and_port = match authority.rsplit_once('@') {
        Some((information, host)) => {
            match information.split_once(':') {
                Some((user, password)) => {
                    parts.user = Some(user.to_owned());
                    parts.password = Some(password.to_owned());
                }
                None => parts.user = Some(information.to_owned()),
            }
            host
        }
        None => authority,
    };
    let (host, port) = if let Some(literal) = host_and_port.strip_prefix('[') {
        let (address, after) = literal
            .split_once(']')
            .ok_or("its URL opens an IPv6 literal it does not close")?;
        if address.is_empty()
            || !address
                .chars()
                .all(|c| c.is_ascii_hexdigit() || matches!(c, ':' | '.'))
        {
            return Err("its URL's IPv6 literal is not an address");
        }
        parts.ipv6 = true;
        let port = match after {
            "" => None,
            stated => Some(
                stated
                    .strip_prefix(':')
                    .ok_or("its URL writes something after its IPv6 literal that is no port")?,
            ),
        };
        (address, port)
    } else {
        match host_and_port.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (host_and_port, None),
        }
    };
    if host.contains(['[', ']', '@']) {
        return Err("its URL's host is not one RFC 3986 section 3.2.2 admits");
    }
    host.clone_into(&mut parts.host);
    // Section 3.2.3: an empty port is the scheme's default.
    if let Some(port) = port.filter(|port| !port.is_empty()) {
        if !port.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err("its URL's port is not a number");
        }
        parts.port = Some(port.parse().map_err(|_| "its URL's port is past 65535")?);
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// spansToXML and xmlToSpans

/// `util.xmlToSpans(cXML)`: a rich text string as an array of `Span` objects, read by
/// `pdf_model::span` — the reader the field's own appearance is drawn through.
///
/// **Two documented choices.** A string that opens no element is not a rich text string, and this
/// tree reads such a value as the characters it is (ADR 1197), so it answers one span holding
/// them. A span whose markup states no colour answers the reference's default, black, since the
/// reference gives `textColor` no "no colour" value.
fn xml_to_spans(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let markup = text_argument(arguments, 0, context)?;
    let read = span::spans(&markup, None, span::DEFAULT_SIZE).unwrap_or_else(|| {
        vec![Span {
            text: markup.clone(),
            ..Span::default()
        }]
    });
    let mut objects = Vec::with_capacity(read.len());
    for one in &read {
        objects.push(JsValue::from(span_object(one, context)?));
    }
    Ok(JsValue::from(JsArray::from_iter(objects, context)))
}

/// One `Span` object with the reference's twelve properties.
pub(super) fn span_object(one: &Span, context: &mut Context) -> JsResult<JsObject> {
    let object = ObjectInitializer::new(context).build();
    let text = |text: &str| JsValue::from(JsString::from(text));
    let families: Vec<JsValue> = one.font_family.iter().map(|name| text(name)).collect();
    let colour = one.text_color.unwrap_or([0.0; 3]);
    let colour = JsArray::from_iter(
        [
            text("RGB"),
            JsValue::from(f64::from(colour[0])),
            JsValue::from(f64::from(colour[1])),
            JsValue::from(f64::from(colour[2])),
        ],
        context,
    );
    let properties: [(&str, JsValue); 12] = [
        ("alignment", text(&one.alignment)),
        (
            "fontFamily",
            JsValue::from(JsArray::from_iter(families, context)),
        ),
        ("fontStretch", text(&one.font_stretch)),
        ("fontStyle", text(&one.font_style)),
        ("fontWeight", JsValue::from(one.font_weight)),
        ("strikethrough", JsValue::from(one.strikethrough)),
        ("subscript", JsValue::from(one.subscript)),
        ("superscript", JsValue::from(one.superscript)),
        ("text", text(&one.text)),
        ("textColor", JsValue::from(colour)),
        ("textSize", JsValue::from(f64::from(one.text_size))),
        ("underline", JsValue::from(one.underline)),
    ];
    for (name, value) in properties {
        object.create_data_property_or_throw(JsString::from(name), value, context)?;
    }
    Ok(object)
}

/// `util.spansToXML(aSpans)`: an array of `Span` objects written as a rich text string, by
/// `pdf_model::span::markup`.
///
/// Each element is read as an object with the reference's properties, a property it lacks taking
/// the reference's default; a `textColor` is read as a colour array — `G` as §10.4.2.2's grey on
/// all three components, `CMYK` through §10.4.2.5's conversion, `T` as no colour of its own. The
/// output is held to the string budget before it is built: each span writes at most its text,
/// escaped, and a style attribute of a few hundred characters.
fn spans_to_xml(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let Some(array) = arguments.first().and_then(JsValue::as_object) else {
        return Err(parameter_error(
            "spansToXML",
            "its argument is not an array of Span objects",
        ));
    };
    let length = array
        .get(JsString::from("length"), context)?
        .to_length(context)?;
    let mut read = Vec::new();
    let mut units: u64 = 0;
    for index in 0..length {
        let element = array.get(index, context)?;
        let Some(object) = element.as_object() else {
            return Err(parameter_error(
                "spansToXML",
                "an element of its array is not a Span object",
            ));
        };
        let one = span_of_object(&object, context)?;
        // Escaping writes at most six units for one (`&quot;`), and a style attribute is bounded.
        units = units
            .saturating_add(
                u64::try_from(one.text.len())
                    .unwrap_or(u64::MAX)
                    .saturating_mul(6),
            )
            .saturating_add(512);
        guard::string_units(units, context)?;
        read.push(one);
    }
    Ok(JsValue::from(JsString::from(span::markup(&read).as_str())))
}

/// One `Span` object read back, a missing property taking the reference's default.
fn span_of_object(object: &JsObject, context: &mut Context) -> JsResult<Span> {
    let mut one = Span::default();
    let get = |name: &str, context: &mut Context| object.get(JsString::from(name), context);
    let text = |value: &JsValue, context: &mut Context| -> JsResult<Option<String>> {
        if value.is_undefined() || value.is_null() {
            return Ok(None);
        }
        Ok(Some(value.to_string(context)?.to_std_string_lossy()))
    };
    if let Some(alignment) = text(&get("alignment", context)?, context)? {
        one.alignment = alignment;
    }
    if let Some(stretch) = text(&get("fontStretch", context)?, context)? {
        one.font_stretch = stretch;
    }
    if let Some(style) = text(&get("fontStyle", context)?, context)? {
        one.font_style = style;
    }
    if let Some(characters) = text(&get("text", context)?, context)? {
        one.text = characters;
    }
    let family = get("fontFamily", context)?;
    if let Some(list) = family.as_object().filter(JsObject::is_array) {
        let count = list
            .get(JsString::from("length"), context)?
            .to_length(context)?;
        // A family list is a search path of a few names; more than sixteen is not one.
        for index in 0..count.min(16) {
            if let Some(name) = text(&list.get(index, context)?, context)? {
                one.font_family.push(name);
            }
        }
    } else if let Some(name) = text(&family, context)? {
        one.font_family.push(name);
    }
    let weight = get("fontWeight", context)?;
    if !weight.is_undefined() {
        let number = weight.to_number(context)?;
        if (100.0..=900.0).contains(&number) {
            one.font_weight = super::bridge::integral(number)
                .try_into()
                .unwrap_or(one.font_weight);
        }
    }
    let size = get("textSize", context)?;
    if !size.is_undefined() {
        let number = size.to_number(context)?;
        // The reference's range, 0 to 32 767 points.
        if (0.0..=32_767.0).contains(&number) {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a size from 0 to 32 767 points is held as f32 to well under a \
                          thousandth of a point"
            )]
            let points = number as f32;
            one.text_size = points;
        }
    }
    for (name, slot) in [
        ("strikethrough", &mut one.strikethrough),
        ("subscript", &mut one.subscript),
        ("superscript", &mut one.superscript),
        ("underline", &mut one.underline),
    ] {
        *slot = get(name, context)?.to_boolean();
    }
    let colour = get("textColor", context)?;
    match colour_components(&colour, context)? {
        Some(Stated::Transparent) => one.text_color = None,
        Some(Stated::Rgb(components)) => one.text_color = Some(components),
        None => {}
    }
    Ok(one)
}

/// What a `textColor` states.
enum Stated {
    /// `T`: no colour of its own, the `/DA`'s.
    Transparent,
    /// A colour, as sRGB components.
    Rgb([f32; 3]),
}

/// A colour array read; `None` for a value that is no colour array, which leaves the default.
fn colour_components(value: &JsValue, context: &mut Context) -> JsResult<Option<Stated>> {
    let Some(array) = value.as_object().filter(JsObject::is_array) else {
        return Ok(None);
    };
    let space = array
        .get(0, context)?
        .to_string(context)?
        .to_std_string_lossy();
    let component = |index: u32, context: &mut Context| -> JsResult<f32> {
        let number = array.get(index, context)?.to_number(context)?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a colour component from 0 to 1, clamped first"
        )]
        let narrow = number.clamp(0.0, 1.0) as f32;
        Ok(if number.is_nan() { 0.0 } else { narrow })
    };
    Ok(match space.as_str() {
        "T" => Some(Stated::Transparent),
        // §10.4.2.2: a grey level is the same level on each of the three components.
        "G" => {
            let grey = component(1, context)?;
            Some(Stated::Rgb([grey; 3]))
        }
        "RGB" => Some(Stated::Rgb([
            component(1, context)?,
            component(2, context)?,
            component(3, context)?,
        ])),
        // §10.4.2.5: each of the three plus black, complemented.
        "CMYK" => {
            let black = component(4, context)?;
            let mut out = [0.0_f32; 3];
            for (slot, index) in out.iter_mut().zip(1..) {
                *slot = 1.0 - (component(index, context)? + black).min(1.0);
            }
            Some(Stated::Rgb(out))
        }
        _ => None,
    })
}

// ---------------------------------------------------------------------------------------------
// streamFromString and stringFromStream

/// The character sets the reference names for the two stream methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharSet {
    /// `utf-8`, the default.
    Utf8,
    /// `utf-16`.
    Utf16,
}

/// The character set an optional argument names, or the error saying why it names none this
/// program reads.
///
/// The reference's options are `utf-8`, `utf-16`, `Shift-JIS`, `BigFive`, `GBK` and `UHC`. The
/// last four are tables this program does not carry — no dependency of this crate holds them,
/// and writing them out is a codec project of its own — so each is refused by name with that
/// sentence; anything else is not an option the reference lists and is a `TypeError`.
fn char_set(member: &str, value: Option<&JsValue>, context: &mut Context) -> JsResult<CharSet> {
    let Some(value) = value.filter(|value| !value.is_undefined()) else {
        return Ok(CharSet::Utf8);
    };
    let name = value.to_string(context)?.to_std_string_lossy();
    match name.to_ascii_lowercase().as_str() {
        "utf-8" | "utf8" => Ok(CharSet::Utf8),
        "utf-16" | "utf16" => Ok(CharSet::Utf16),
        "shift-jis" | "bigfive" | "gbk" | "uhc" => Err(refuse(
            format!("util.{member}"),
            RefusalKind::Unreachable(format!(
                "its character set {name} is a table this program does not carry; utf-8 and \
                 utf-16 are read"
            )),
            context,
        )),
        _ => Err(parameter_error(
            member,
            "its character set is none of utf-8, utf-16, Shift-JIS, BigFive, GBK and UHC",
        )),
    }
}

/// `util.streamFromString(cString, cCharSet)`: the string's bytes in the character set, as a
/// `ReadStream` — an object whose `read(nBytes)` answers the next bytes as hexadecimal and an
/// empty string at the end (the reference's `ReadStream` page).
///
/// **UTF-16 is big-endian behind a byte order mark**, a documented choice: the reference names no
/// byte order, and that is how §7.9.2.2 writes a text string in the same encoding. The hex is
/// held to the string budget before it is built: two digits a byte.
fn stream_from_string(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let text = text_argument(arguments, 0, context)?;
    let set = char_set("streamFromString", arguments.get(1), context)?;
    let bytes: Vec<u8> = match set {
        CharSet::Utf8 => text.into_bytes(),
        CharSet::Utf16 => [0xFE, 0xFF]
            .into_iter()
            .chain(text.encode_utf16().flat_map(u16::to_be_bytes))
            .collect(),
    };
    guard::string_units(
        u64::try_from(bytes.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(2),
        context,
    )?;
    Ok(JsValue::from(read_stream(&hex(&bytes), context)?))
}

/// A `ReadStream` over hexadecimal digits: `read(nBytes)` takes the next `2 × nBytes` of them,
/// all that remain where `nBytes` is missing or not positive.
fn read_stream(digits: &str, context: &mut Context) -> JsResult<JsObject> {
    // The digits that remain are held on an object only the `read` function holds, so a script
    // reads the stream through `read` alone, and the read is destructive as the reference says.
    let held = ObjectInitializer::new(context).build();
    held.set(
        JsString::from("rest"),
        JsValue::from(JsString::from(digits)),
        false,
        context,
    )?;
    let read = function(
        context,
        "read",
        NativeFunction::from_copy_closure_with_captures(
            |_this, arguments, held: &JsObject, context| {
                let rest = held
                    .get(JsString::from("rest"), context)?
                    .to_string(context)?
                    .to_std_string_lossy();
                let asked = arguments.first().cloned().unwrap_or_default();
                let take = if asked.is_undefined() {
                    rest.len()
                } else {
                    let bytes = asked.to_number(context)?;
                    if bytes.is_nan() || bytes <= 0.0 {
                        rest.len()
                    } else {
                        usize::try_from(super::bridge::integral(bytes))
                            .unwrap_or(usize::MAX)
                            .saturating_mul(2)
                            .min(rest.len())
                    }
                };
                // The digits are ASCII, so a byte offset is a character boundary.
                let (now, later) = rest.split_at(take);
                held.set(
                    JsString::from("rest"),
                    JsValue::from(JsString::from(later)),
                    false,
                    context,
                )?;
                Ok(JsValue::from(JsString::from(now)))
            },
            held,
        ),
    );
    let stream = ObjectInitializer::new(context).build();
    data(&stream, "read", JsValue::from(read), false, context)?;
    Ok(stream)
}

/// Bytes as lower-case hexadecimal.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0F)]));
    }
    out
}

/// `util.stringFromStream(oStream, cCharSet)`: a stream read to its end and its bytes decoded.
///
/// Any object with a callable `read` is a stream here, the reference's streams being object
/// literals with that one method: `read` is asked for 65 536 bytes at a time until it answers an
/// empty string. **Two documented choices**: bytes that are not valid in the character set decode
/// to U+FFFD rather than failing, since the reference states no failure; and UTF-16 honours a byte
/// order mark either way round and reads big-endian without one, as [`stream_from_string`] writes.
/// A stream that never ends meets the string budget, or [`MAX_STREAM_BYTES`] where the realm runs
/// with none, and one answering in pieces too small to reach either meets [`MAX_READS`]; a `read`
/// that answers something other than an even run of hexadecimal digits is a `TypeError`.
fn string_from_stream(
    _this: &JsValue,
    arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let Some(stream) = arguments.first().and_then(JsValue::as_object) else {
        return Err(parameter_error(
            "stringFromStream",
            "its argument is not a stream",
        ));
    };
    let set = char_set("stringFromStream", arguments.get(1), context)?;
    let read = stream.get(JsString::from("read"), context)?;
    let Some(read) = read.as_callable() else {
        return Err(parameter_error(
            "stringFromStream",
            "its argument has no read method, so it is not a stream",
        ));
    };
    let mut bytes: Vec<u8> = Vec::new();
    for calls in 0.. {
        if calls >= MAX_READS {
            return Err(parameter_error(
                "stringFromStream",
                "its stream's read was called 4 096 times without reaching the end",
            ));
        }
        let answer = read
            .call(
                &JsValue::from(stream.clone()),
                &[JsValue::from(READ_CHUNK)],
                context,
            )?
            .to_string(context)?
            .to_std_string_lossy();
        if answer.is_empty() {
            break;
        }
        let Some(decoded) = unhex(&answer) else {
            return Err(parameter_error(
                "stringFromStream",
                "its stream's read answered something other than hexadecimal bytes",
            ));
        };
        bytes.extend(decoded);
        guard::string_units(u64::try_from(bytes.len()).unwrap_or(u64::MAX), context)?;
        if bytes.len() > MAX_STREAM_BYTES {
            return Err(parameter_error(
                "stringFromStream",
                "its stream is longer than the sixteen mebibytes this program reads from one",
            ));
        }
    }
    let text = match set {
        CharSet::Utf8 => String::from_utf8_lossy(&bytes).into_owned(),
        CharSet::Utf16 => utf16(&bytes),
    };
    Ok(JsValue::from(JsString::from(text.as_str())))
}

/// Hexadecimal digits as bytes; `None` for an odd count or a character that is no digit.
fn unhex(digits: &str) -> Option<Vec<u8>> {
    let digits = digits.as_bytes();
    if !digits.len().is_multiple_of(2) {
        return None;
    }
    digits
        .chunks_exact(2)
        .map(|pair| {
            let high = char::from(pair[0]).to_digit(16)?;
            let low = char::from(pair[1]).to_digit(16)?;
            u8::try_from(high << 4 | low).ok()
        })
        .collect()
}

/// UTF-16 bytes decoded: the byte order mark's order, big-endian without one; a lone trailing
/// byte and an unpaired surrogate are U+FFFD.
fn utf16(bytes: &[u8]) -> String {
    let (little, body) = match bytes {
        [0xFF, 0xFE, rest @ ..] => (true, rest),
        [0xFE, 0xFF, rest @ ..] => (false, rest),
        _ => (false, bytes),
    };
    let units = body.chunks_exact(2).map(|pair| {
        let pair = [pair[0], pair[1]];
        if little {
            u16::from_le_bytes(pair)
        } else {
            u16::from_be_bytes(pair)
        }
    });
    let mut out: String = char::decode_utf16(units)
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect();
    if !body.len().is_multiple_of(2) {
        out.push(char::REPLACEMENT_CHARACTER);
    }
    out
}

// ---------------------------------------------------------------------------------------------
// iconStreamFromIcon

/// `util.iconStreamFromIcon(oIcon)`: refused with the reason, whatever it is handed.
///
/// The reference's `Icon` object is what `Doc.getIcon`, `Doc.importIcon` and a button's
/// `buttonGetIcon` answer, and its Icon Stream is what `app.addToolButton` and
/// `collab.addStateModel` take. This realm carries none of the three that make one — the
/// first two are no member here at all, and `buttonImportIcon` beside them is RFC 0008 section
/// 4.3's — and neither consumer is reachable, so no argument a script can hold is an `Icon` and
/// nothing could read the stream. Answering a stream built from an arbitrary object would be a
/// plausible value standing for nothing; the call is refused by name instead, so the outcome says
/// it (trap 5), and the refusal is `Unreachable` rather than `NotBridged` because no later bridge
/// of this member alone would change it.
fn icon_stream_from_icon(
    _this: &JsValue,
    _arguments: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    Err(refuse(
        "util.iconStreamFromIcon".to_owned(),
        RefusalKind::Unreachable(
            "an Icon object is what Doc.getIcon, Doc.importIcon and a button's buttonGetIcon \
             answer, and this realm carries none of them, so no argument is an Icon to convert"
                .to_owned(),
        ),
        context,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_url_splits_into_rfc_3986_s_parts() {
        let parts = split_url("https://ann:secret@Example.org:8443/a/b?x=1&y=2#top").ok();
        assert_eq!(
            parts,
            Some(Parts {
                scheme: "https".to_owned(),
                user: Some("ann".to_owned()),
                password: Some("secret".to_owned()),
                host: "Example.org".to_owned(),
                port: Some(8443),
                path: Some("/a/b".to_owned()),
                query: Some("x=1&y=2".to_owned()),
                fragment: Some("top".to_owned()),
                ipv6: false,
            })
        );
    }

    #[test]
    fn a_file_url_has_no_port_and_may_have_no_host() {
        let parts = split_url("file:///C/temp/a.pdf").ok();
        assert_eq!(
            parts
                .as_ref()
                .map(|parts| (parts.port, parts.host.as_str(), parts.path.as_deref())),
            Some((None, "", Some("/C/temp/a.pdf")))
        );
    }

    #[test]
    fn a_url_that_is_not_one_is_refused_with_its_reason() {
        for (url, says) in [
            ("ftp://example.org/", "not file, http or https"),
            ("http://exa mple.org/", "white space"),
            ("http:///path", "names no host"),
            ("http://example.org:99999/", "past 65535"),
            ("http://example.org:8o/", "not a number"),
            ("http://[::1/", "does not close"),
            ("example.org", "no scheme"),
        ] {
            let why = split_url(url).err();
            assert!(why.is_some_and(|why| why.contains(says)), "{url}: {why:?}");
        }
    }

    #[test]
    fn printd_s_date_strings_read_back_with_their_offsets() {
        let read = date_string("D:20000801145605+07'00");
        assert_eq!(read.map(|(_, offset)| offset), Some(Some(420)));
        assert_eq!(
            date_string("D:20000801075605Z").map(|(_, o)| o),
            Some(Some(0))
        );
        assert_eq!(
            date_string("D:20000801075605-05'30'").map(|(_, o)| o),
            Some(Some(-330))
        );
        assert_eq!(date_string("20000801075605Z"), None);
        assert_eq!(date_string("D:2000080107560Z"), None);
    }

    #[test]
    fn bytes_round_trip_through_hexadecimal_and_utf_16() {
        assert_eq!(
            unhex(&hex(&[0, 1, 0xAB, 0xFF])),
            Some(vec![0, 1, 0xAB, 0xFF])
        );
        assert_eq!(unhex("abc"), None);
        assert_eq!(unhex("zz"), None);
        let written: Vec<u8> = [0xFE, 0xFF]
            .into_iter()
            .chain("Grüße 😀".encode_utf16().flat_map(u16::to_be_bytes))
            .collect();
        assert_eq!(utf16(&written), "Grüße 😀");
        assert_eq!(utf16(&[0xFF, 0xFE, b'A', 0]), "A");
        assert_eq!(utf16(&[0, b'A', 0]), "A\u{FFFD}");
    }
}
