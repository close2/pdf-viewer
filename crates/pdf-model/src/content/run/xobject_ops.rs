//! Table 50's operators that bring other content onto the page: `Do`, `sh` and the inline image's
//! `BI`, `ID` and `EI`.

use std::sync::Arc;

use super::machine::Machine;
use super::operands::name_at;

use pdf_syntax::{Dictionary, Name, Object};

use crate::content::reader::{ContentReader, Operator};
use crate::content::report::Unsupported;

use crate::content::Interpreter;

impl Interpreter<'_> {
    /// The `XObject` operator of Table 50 (ISO 32000-2 §8.10.1): `Do`.
    pub(super) fn run_xobject_op(
        &mut self,
        operands: &[Object],
        resources: &Dictionary,
        machine: &mut Machine,
    ) {
        self.draw_xobject(operands, resources, &machine.state);
    }

    /// The shading operator of Table 50 (ISO 32000-2 §8.7.4.2): `sh`.
    pub(super) fn run_shading_op(
        &mut self,
        operands: &[Object],
        resources: &Dictionary,
        machine: &mut Machine,
    ) {
        let name = name_at(operands, 0).unwrap_or_else(|| Name::new(Vec::new()));
        self.paint_shading(&name, resources, &machine.state);
    }

    /// The inline image operators of Table 50 (ISO 32000-2 §8.9.7): `BI`, and `ID` and `EI` where
    /// they are met outside one.
    pub(super) fn run_inline_image_op(
        &mut self,
        operator: Operator,
        reader: &mut ContentReader<'_>,
        resources: &Dictionary,
        machine: &mut Machine,
    ) {
        let Machine {
            state,
            compatibility,
            ..
        } = machine;
        match operator {
            // §8.9.7: an image written into the content stream rather than as an
            // `XObject`. `crate::inline_image` turns it into the stream the same image
            // would have been as an `XObject`, so from here on it is an ordinary image —
            // including §8.6.8's rule about an uncoloured figure, which `draw_image`
            // owns and which is what a Type 3 glyph drawn as an inline mask needs.
            //
            // The lexer is moved past the data on every path, error included: the bytes
            // between `ID` and `EI` are not a program, and tokenising them would emit
            // drawing commands from image samples.
            Operator::BeginImage => {
                let scanned = self.inline_image(reader, resources);
                reader.skip(scanned.resume);
                // A hidden layer suppresses the drawing and the report both: an image
                // the document turns off is not one we failed to draw (§8.11.3.1).
                if !self.is_hidden() {
                    match scanned.image {
                        Ok(stream) => {
                            // The stream is new at every `BI`, so `image::RasterCache` is
                            // told to name it by its content: a hatching states the same
                            // few samples once per tiling cell, and an entry named by this
                            // allocation's address could answer none of them (ADR 0399).
                            let stream = Arc::new(stream);
                            self.draw_image(
                                crate::image::NamedStream::inline(&stream),
                                "<inline>",
                                resources,
                                state,
                            );
                        }
                        Err(error) => self.note(Unsupported::Image {
                            name: format!("<inline>: {error}"),
                        }),
                    }
                }
            }
            // Table 90's other two are read by `BI`'s arm, as the inline image's own syntax;
            // met here, outside one, they are reported exactly as a keyword nobody lists.
            Operator::ImageData | Operator::EndImage if *compatibility == 0 => {
                self.note_unknown_operator(operator.name());
            }
            // Only this family's operators are sent here, by `run_operator`.
            _ => {}
        }
    }
}
