use std::io::{self, Write};
use std::borrow::Borrow;

use crate::prelude::*;
use super::{MappingsLineProcessor, MappingsFormat, MappingsParseError};
use crate::utils::*;

pub struct CompactSrgMappingsFormat;
impl MappingsFormat for CompactSrgMappingsFormat {
    type Processor = CompactSrgLineProcessor;

    fn write<'a, T: IterableMappings<'a>, W: Write>(mappings: &'a T, mut writer: W) -> io::Result<()> {
        for (original, renamed) in mappings.classes() {
            writeln!(writer, "{} {}", original.internal_name(), renamed.borrow().internal_name())?;
        }
        for (original, renamed) in mappings.fields() {
            writeln!(
                writer, "{} {} {}",
                original.declaring_type().internal_name(),
                original.name,
                renamed.borrow().name
            )?;
        }
        for (original, renamed) in mappings.methods() {
            writeln!(
                writer, "{} {} {} {}",
                original.declaring_type().internal_name(),
                original.name,
                original.signature().descriptor(),
                renamed.borrow().name
            )?;
        }
        Ok(())
    }

    #[inline]
    fn processor() -> CompactSrgLineProcessor {
        CompactSrgLineProcessor::default()
    }
}

#[derive(Default)]
pub struct CompactSrgLineProcessor {
    result: SimpleMappings,
}
impl CompactSrgLineProcessor {
    fn parse_line(&mut self, parser: &mut SimpleParser) -> Result<(), SimpleParseError> {
        parser.skip_whitespace();
        if parser.is_finished() || parser.peek()? == '#' { return Ok(()) }
        match parser.remaining().split_whitespace().count() {
            4 => {
                let original_declaring_type = ReferenceType::from_internal_name(
                    parser.parse_internal_name()?);
                parser.expect(' ')?;
                let original_name = parser.take_until(|c| c == ' ');
                parser.expect(' ')?;
                let original_signature = parser.parse::<MethodSignature>()?;
                parser.expect(' ')?;
                let renamed_name = parser.take_until(|c| c == ' ');
                let original_data = MethodData::new(
                    original_name.into(),
                    original_declaring_type,
                    original_signature
                );
                self.result.set_method_name(original_data, renamed_name.into());
            },
            3 => {
                let original_declaring_type = ReferenceType::from_internal_name(
                    parser.parse_internal_name()?);
                parser.expect(' ')?;
                let original_name = parser.take_until(|c| c == ' ');
                parser.expect(' ')?;
                let renamed_name = parser.take_until(|c| c == ' ');
                let original_data = FieldData::new(
                    original_name.into(),
                    original_declaring_type,
                );
                self.result.set_field_name(original_data, renamed_name.into());
            },
            2 => {
                let original = ReferenceType::from_internal_name(
                    parser.parse_internal_name()?);
                parser.expect(' ')?;
                let renamed = ReferenceType::from_internal_name(
                    parser.parse_internal_name()?);
                self.result.set_remapped_class(original, renamed);
            },
            _ => return Err(parser.error())
        }
        parser.skip_whitespace();
        parser.ensure_finished()?;
        Ok(())
    }
}
impl MappingsLineProcessor for CompactSrgLineProcessor {
    fn process_line(&mut self, s: &str) -> Result<(), MappingsParseError> {
        let mut parser = SimpleParser::new(s);
        self.parse_line(&mut parser)
            .map_err(|cause| MappingsParseError::InvalidLine {
                index: cause.index,
                line: s.into(),
                reason: cause.reason
            })
    }

    #[inline]
    fn finish(self) -> Result<FrozenMappings, MappingsParseError> {
        Ok(self.result.frozen())
    }
}
