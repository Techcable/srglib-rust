use std::hash::Hash;
use std::hash::Hasher;

use crate::utils::*;
use super::prelude::*;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct MethodData {
    pub name: String,
    declaring_type: ReferenceType,
    signature: MethodSignature
}
impl MethodData {
    #[inline]
    pub fn new(name: String, declaring_type: ReferenceType, signature: MethodSignature) -> MethodData {
        MethodData { name, declaring_type, signature}
    }
    /// The declaring type of this field
    #[inline]
    pub fn declaring_type(&self) -> &ReferenceType {
        &self.declaring_type
    }
    pub fn internal_name(&self) -> String {
        let mut buffer: String = self.declaring_type.internal_name().into();
        buffer.push('/');
        buffer.push_str(&self.name);
        buffer
    }
    #[inline]
    pub fn signature(&self) -> &MethodSignature {
        &self.signature
    }

    pub fn map_class<F>(&self, mut func: F) -> MethodData
        where F: FnMut(&ReferenceType) -> Option<ReferenceType> {
        let remapped_class = self.declaring_type.map_class(|t| func(t));
        let remapped_signature = self.signature.map_class(|t| func(t));
        MethodData {
            name: self.name.clone(),
            declaring_type: remapped_class,
            signature: remapped_signature
        }
    }
}

impl<'a> From<&'a MethodData> for MethodData {
    #[inline]
    fn from(data: &'a MethodData) -> Self {
        data.clone()
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct FieldData {
    pub name: String,
    declaring_type: ReferenceType
}
impl FieldData {
    #[inline]
    pub fn new(name: String, declaring_type: ReferenceType) -> FieldData {
        FieldData { name, declaring_type }
    }
    /// The declaring type of this field
    #[inline]
    pub fn declaring_type(&self) -> &ReferenceType {
        &self.declaring_type
    }
    /// The name of this field
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn internal_name(&self) -> String {
        let mut buffer: String = self.declaring_type.internal_name().into();
        buffer.push('/');
        buffer.push_str(&self.name);
        buffer
    }
    pub fn map_class<F>(&self, func: F) -> FieldData
        where F: FnMut(&ReferenceType) -> Option<ReferenceType> {
        let remapped_class = self.declaring_type.map_class(func);
        FieldData {
            name: self.name.clone(),
            declaring_type: remapped_class
        }
    }
}
impl<'a> From<&'a FieldData> for FieldData {
    #[inline]
    fn from(data: &'a FieldData) -> Self {
        data.clone()
    }
}

#[derive(Clone, Debug)]
pub struct MethodSignature {
    descriptor: String,
    return_type: TypeDescriptor,
    parameter_types: Vec<TypeDescriptor>
}
impl MethodSignature {
    pub fn new(return_type: TypeDescriptor, parameter_types: Vec<TypeDescriptor>) -> Self {
        let mut descriptor = String::with_capacity(64 * (parameter_types.len() + 1));
        descriptor.push('(');
        for parameter_type in &parameter_types {
            descriptor.push_str(parameter_type.descriptor())
        }
        descriptor.push(')');
        descriptor.push_str(return_type.descriptor());
        MethodSignature { descriptor, return_type, parameter_types }
    }
    #[inline]
    pub fn descriptor(&self) -> &str {
        &self.descriptor
    }
    #[inline]
    pub fn return_type(&self) -> &TypeDescriptor {
        &self.return_type
    }
    #[inline]
    pub fn parameter_types(&self) -> &[TypeDescriptor] {
        &self.parameter_types
    }
    pub fn map_class<F>(&self, mut func: F) -> MethodSignature
        where F: FnMut(&ReferenceType) -> Option<ReferenceType> {
        MethodSignature::new(
            self.return_type.map_class(|c| func(c)),
            self.parameter_types.iter()
                .map(|t| t.map_class(|c| func(c))).collect()
        )
    }
}
impl SimpleParse for MethodSignature {
    fn parse(parser: &mut SimpleParser) -> Result<MethodSignature, SimpleParseError> {
        let index = parser.current_index();
        parser.expect('(')?;
        let mut parameter_types = Vec::new();
        while parser.peek()? != ')' {
            parameter_types.push(parser.parse::<TypeDescriptor>()?);
        }
        parser.expect(')')?;
        let return_type = parser.parse()?;
        let descriptor = String::from(&parser.original()[index..parser.current_index()]);
        Ok(MethodSignature { descriptor, return_type, parameter_types })
    }
}
impl Hash for MethodSignature {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.descriptor().hash(state);
    }
}
impl PartialEq for MethodSignature {
    #[inline]
    fn eq(&self, other: &MethodSignature) -> bool {
        self.descriptor() == other.descriptor()
    }
}
impl Eq for MethodSignature {}