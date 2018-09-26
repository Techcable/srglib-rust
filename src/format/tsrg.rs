use std::io::{self, Write};

use itertools::Itertools;

use crate::utils::{SimpleParser, SimpleParseError, FnvIndexMap};
use crate::prelude::*;
use super::{MappingsFormat, MappingsLineProcessor};


pub struct TabSrgMappingsFormat;
impl MappingsFormat for TabSrgMappingsFormat {
    type Processor = TabSrgLineProcessor;

    fn write<'a, T: IterableMappings<'a>, W: Write>(mappings: &'a T, mut writer: W) -> io::Result<()> {
        let data = ClassData::from_mappings(mappings);
        for (declaring_type, data) in data.iter() {
            let renamed_type = data.renamed_type.as_ref()
                .unwrap_or(declaring_type);
            writeln!(writer, "{} {}", declaring_type.internal_name(), renamed_type.internal_name())?;
            for (original, renamed) in &data.fields {
                writeln!(writer, "\t{} {}", original.name, renamed.name)?;
            }
            for (original, renamed) in &data.methods {
                writeln!(
                    writer, "\t{} {} {}",
                    original.name, original.signature().descriptor(),
                    renamed.name
                )?;
            }
        }
        Ok(())
    }

    fn processor() -> TabSrgLineProcessor {
        TabSrgLineProcessor::default()
    }
}

#[derive(Default)]
pub struct TabSrgLineProcessor {
    result: SimpleMappings,
    current_class: Option<ReferenceType>
}
impl TabSrgLineProcessor {
    fn parse_line(&mut self, parser: &mut SimpleParser) -> Result<(), SimpleParseError> {
        if parser.is_finished() || parser.remaining().trim_left().starts_with('#') { return Ok(()) }
        if parser.peek()? != '\t' {
            // We have a new class entry
            let original = ReferenceType::from_internal_name(
                parser.parse_internal_name()?);
            parser.expect(' ')?;
            let renamed = ReferenceType::from_internal_name(
                parser.parse_internal_name()?);
            self.result.set_remapped_class(original.clone(), renamed);
            self.current_class = Some(original);
            return Ok(())
        }
        parser.expect('\t')?;
        let current_class = self.current_class.clone()
            .ok_or_else(|| SimpleParseError {
                index: parser.current_index(),
                reason: Some("Missing current class".into()),
            })?;
        // Otherwise it's a member entry, implied to be part of the current class
        match parser.remaining().split_whitespace().count() {
            3 => {
                let original_name = parser.take_until(|c| c == ' ');
                parser.expect(' ')?;
                let original_signature = parser.parse::<MethodSignature>()?;
                parser.expect(' ')?;
                let renamed_name = parser.take_until(|c| c == ' ');
                let original_data = MethodData::new(
                    original_name.into(),
                    current_class,
                    original_signature
                );
                self.result.set_method_name(original_data, renamed_name.into());
            },
            2 => {
                let original_name = parser.take_until(|c| c == ' ');
                parser.expect(' ')?;
                let renamed_name = parser.take_until(|c| c == ' ');
                let original_data = FieldData::new(
                    original_name.into(),
                    current_class,
                );
                self.result.set_field_name(original_data, renamed_name.into());
            },
            _ => return Err(parser.error())
        }
        parser.skip_whitespace();
        parser.ensure_finished()?;
        Ok(())
    }
}
impl MappingsLineProcessor for TabSrgLineProcessor {
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

/*
 * TODO: This needs to be part of some sort of public API
 * Personally, I think it needs to become part of
 * the new internal representation of `FrozenMappings`
 */
#[derive(Default)]
struct ClassData {
    renamed_type: Option<ReferenceType>,
    fields: Vec<(FieldData, FieldData)>,
    methods: Vec<(MethodData, MethodData)>
}
impl ClassData {
    fn from_mappings<'a, T: IterableMappings<'a>>(mappings: &'a T) -> FnvIndexMap<ReferenceType, ClassData> {
        let mut classes: FnvIndexMap<ReferenceType, ClassData> = FnvIndexMap::with_capacity_and_hasher(
            mappings.original_classes().size_hint().1.unwrap_or(0), Default::default());
        for (declaring_type, renamed_type) in mappings.classes() {
            let data = classes.entry(declaring_type.clone())
                .or_insert_with(Default::default);
            data.renamed_type = Some(renamed_type.clone());
        }
        for (declaring_type, group) in &mappings.fields()
            .group_by(|(original, _)| original.declaring_type()) {
            let data = classes.entry(declaring_type.clone())
                .or_insert_with(Default::default);
            data.fields.extend(group.map(|(original, renamed)| (original.clone(), renamed.into())));
        }
        for (declaring_type, group) in &mappings.methods()
            .group_by(|(original, _)| original.declaring_type()) {
            let data = classes.entry(declaring_type.clone())
                .or_insert_with(Default::default);
            data.methods.extend(group.map(|(original, renamed)| (original.clone(), renamed.into())));
        }
        classes
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const TEST_TEXT: &'static str = r#"a net/minecraft/util/text/TextFormatting
	a BLACK
	b DARK_BLUE
	v RESET
	w field_96331_x
	C field_175747_C
	D field_211167_D
	E $VALUES
	a (C)La; func_211165_a
	a (I)La; func_175744_a
	d ()Z func_96302_c
	d (Ljava/lang/String;)Ljava/lang/String; func_175745_c
	e ()Ljava/lang/Integer; func_211163_e
	f ()Z func_211166_f
	g ()Ljava/lang/String; func_96297_d
	values ()[La; values
	valueOf (Ljava/lang/String;)La; valueOf
b net/minecraft/crash/CrashReport
	a field_147150_a
	b field_71513_a
	h field_85060_g
	a ()Ljava/lang/String; func_71501_a
	a (Ljava/io/File;)Z func_147149_a
	a (Ljava/lang/String;)Lc; func_85058_a
	a (Ljava/lang/String;I)Lc; func_85057_a
	l ()Ljava/lang/String; func_210201_l
	o ()Ljava/lang/String; func_210206_o
"#;

    #[test]
    fn parse() {
        TabSrgMappingsFormat::parse_text(TEST_TEXT).unwrap().assert_equal(&expected_mappings())
    }
    #[test]
    fn serialize() {
        let serialized = TabSrgMappingsFormat::write_string(&expected_mappings());
        if serialized != TEST_TEXT {
            let changelog = ::difference::Changeset::new(TEST_TEXT, &serialized, " ");
            panic!("serialized != TEST_TEXT:\n{}", changelog);
        }
    }

    fn expected_mappings() -> FrozenMappings {
        let mut builder = SimpleMappings::default();
        {
            let a = ReferenceType::from_internal_name("a");
            builder.set_remapped_class(
                ReferenceType::from_internal_name("a"),
                ReferenceType::from_internal_name("net/minecraft/util/text/TextFormatting")
            );
            builder.set_field_name(
                FieldData::new("a".into(), a.clone()),
                "BLACK".into()
            );
            builder.set_field_name(
                FieldData::new("b".into(), a.clone()),
                "DARK_BLUE".into()
            );
            builder.set_field_name(
                FieldData::new("v".into(), a.clone()),
                "RESET".into()
            );
            builder.set_field_name(
                FieldData::new("w".into(), a.clone()),
                "field_96331_x".into()
            );
            builder.set_field_name(
                FieldData::new("C".into(), a.clone()),
                "field_175747_C".into()
            );
            builder.set_field_name(
                FieldData::new("D".into(), a.clone()),
                "field_211167_D".into()
            );
            builder.set_field_name(
                FieldData::new("E".into(), a.clone()),
                "$VALUES".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "a".into(),
                    a.clone(),
                    MethodSignature::from_descriptor("(C)La;")
                ),
                "func_211165_a".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "a".into(),
                    a.clone(),
                    MethodSignature::from_descriptor("(I)La;")
                ),
                "func_175744_a".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "d".into(),
                    a.clone(),
                    MethodSignature::from_descriptor("()Z")
                ),
                "func_96302_c".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "d".into(),
                    a.clone(),
                    MethodSignature::from_descriptor("(Ljava/lang/String;)Ljava/lang/String;")
                ),
                "func_175745_c".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "e".into(),
                    a.clone(),
                    MethodSignature::from_descriptor("()Ljava/lang/Integer;")
                ),
                "func_211163_e".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "f".into(),
                    a.clone(),
                    MethodSignature::from_descriptor("()Z")
                ),
                "func_211166_f".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "g".into(),
                    a.clone(),
                    MethodSignature::from_descriptor("()Ljava/lang/String;")
                ),
                "func_96297_d".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "values".into(),
                    a.clone(),
                    MethodSignature::from_descriptor("()[La;")
                ),
                "values".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "valueOf".into(),
                    a.clone(),
                    MethodSignature::from_descriptor("(Ljava/lang/String;)La;")
                ),
                "valueOf".into()
            );
        }
        {
            let b = ReferenceType::from_internal_name("b");
            builder.set_remapped_class(
                b.clone(),
                ReferenceType::from_internal_name("net/minecraft/crash/CrashReport")
            );
            builder.set_field_name(
                FieldData::new("a".into(), b.clone()),
                "field_147150_a".into()
            );
            builder.set_field_name(
                FieldData::new("b".into(), b.clone()),
                "field_71513_a".into()
            );
            builder.set_field_name(
                FieldData::new("h".into(), b.clone()),
                "field_85060_g".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "a".into(),
                    b.clone(),
                    MethodSignature::from_descriptor("()Ljava/lang/String;"),
                ),
                "func_71501_a".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "a".into(),
                    b.clone(),
                    MethodSignature::from_descriptor("(Ljava/io/File;)Z"),
                ),
                "func_147149_a".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "a".into(),
                    b.clone(),
                    MethodSignature::from_descriptor("(Ljava/lang/String;)Lc;"),
                ),
                "func_85058_a".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "a".into(),
                    b.clone(),
                    MethodSignature::from_descriptor("(Ljava/lang/String;I)Lc;"),
                ),
                "func_85057_a".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "l".into(),
                    b.clone(),
                    MethodSignature::from_descriptor("()Ljava/lang/String;"),
                ),
                "func_210201_l".into()
            );
            builder.set_method_name(
                MethodData::new(
                    "o".into(),
                    b.clone(),
                    MethodSignature::from_descriptor("()Ljava/lang/String;"),
                ),
                "func_210206_o".into()
            );
        }
        builder.frozen()
    }
}