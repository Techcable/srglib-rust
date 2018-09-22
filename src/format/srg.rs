use std::io::{self, Write};
use std::borrow::Borrow;

use indexmap::IndexMap;

use crate::prelude::*;
use super::{MappingsLineProcessor, MappingsFormat, MappingsParseError};
use crate::utils::*;

pub struct SrgMappingsFormat;
impl MappingsFormat for SrgMappingsFormat {
    type Processor = SrgLineProcessor;

    fn write<'a, T: IterableMappings<'a>, W: Write>(mappings: &'a T, mut writer: W) -> io::Result<()> {
        for (original, renamed) in mappings.classes() {
            writeln!(writer, "CL: {} {}", original.internal_name(), renamed.borrow().internal_name())?;
        }
        for (original, renamed) in mappings.fields() {
            writeln!(writer, "FD: {} {}", original.internal_name(), renamed.borrow().internal_name())?;
        }
        for (original, renamed) in mappings.methods() {
            writeln!(
                writer, "MD: {} {} {} {}",
                original.internal_name(),
                original.signature().descriptor(),
                renamed.borrow().internal_name(),
                renamed.borrow().signature().descriptor()
            )?;
        }
        Ok(())
    }

    #[inline]
    fn processor() -> SrgLineProcessor {
        SrgLineProcessor::default()
    }
}

#[derive(Default)]
pub struct SrgLineProcessor {
    result: SimpleMappings,
    packages: IndexMap<String, String>
}
impl SrgLineProcessor {
    fn parse_line(&mut self, parser: &mut SimpleParser) -> Result<(), SimpleParseError> {
        parser.skip_whitespace();
        if parser.is_finished() || parser.peek()? == '#' { return Ok(()) }
        match parser.peek_str(2)? {
            "MD" => {
                parser.expect_str("MD: ")?;
                let original_internal_name = parser.parse::<JoinedInternalName>()?;
                parser.expect(' ')?;
                let original_signature = parser.parse::<MethodSignature>()?;
                parser.expect(' ')?;
                let renamed_internal_name = parser.parse::<JoinedInternalName>()?;
                parser.expect(' ')?;
                let renamed_signature = parser.parse::<MethodSignature>()?;
                let original_data = MethodData::new(
                    original_internal_name.name,
                    original_internal_name.declaring_type,
                    original_signature
                );
                let renamed_data = MethodData::new(
                    renamed_internal_name.name,
                    renamed_internal_name.declaring_type,
                    renamed_signature
                );
                self.result.set_method_name(original_data, renamed_data.name);
            },
            "FD" => {
                parser.expect_str("FD: ")?;
                let original_internal_name = parser.parse::<JoinedInternalName>()?;
                parser.expect(' ')?;
                let renamed_internal_name = parser.parse::<JoinedInternalName>()?;
                let original_data = FieldData::new(
                    original_internal_name.name,
                    original_internal_name.declaring_type
                );
                let renamed_data = FieldData::new(
                    renamed_internal_name.name,
                    renamed_internal_name.declaring_type
                );
                self.result.set_field_name(original_data, renamed_data.name);
            },
            "CL" => {
                parser.expect_str("CL: ")?;
                let original = ReferenceType::from_internal_name(
                    parser.parse_internal_name()?);
                parser.expect(' ')?;
                let renamed = ReferenceType::from_internal_name(
                    parser.parse_internal_name()?);
                self.result.set_remapped_class(original, renamed);
            },
            "PK" => {
                parser.expect_str("PK: ")?;
                let mut original = String::from(parser.take_until(|c| c == ' '));
                if original == "./" {
                    // This is the magic indicator for no package
                    original.clear();
                }
                parser.expect(' ')?;
                let renamed = parser.take_until(|c| c == ' ').into();
                self.packages.insert(original, renamed);
            }
            _ => return Err(parser.error())
        }
        parser.skip_whitespace();
        parser.ensure_finished()?;
        Ok(())
    }
}
/// Parsing utility for parsing things like `java/lang/String/concat`
struct JoinedInternalName {
    declaring_type: ReferenceType,
    name: String
}
impl SimpleParse for JoinedInternalName {
    fn parse(parser: &mut SimpleParser) -> Result<Self, SimpleParseError> {
        let start = parser.current_index();
        let s = parser.parse_internal_name()?;
        match s.rfind('/') {
            Some(index) => {
                let declaring_type = ReferenceType::from_internal_name(&s[..index]);
                let name = String::from(&s[(index + 1)..]);
                Ok(JoinedInternalName { declaring_type, name })
            },
            None => Err(SimpleParseError { index: start, reason: Some(format!("Invalid joined name: {:?}", s)) })
        }
    }
}
impl MappingsLineProcessor for SrgLineProcessor {
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
        Ok(self.result.transform_packages(|s| self.packages.get(s).cloned()))
    }
}
