use std::hash::{Hash, Hasher};
use std::sync::Arc;

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
}
impl MapClass for MethodData {
    fn maybe_transform_class<T: TypeTransformer>(&self, transformer: T) -> Option<Self> {
        let remapped_class = self.declaring_type.transform_class(&transformer);
        let remapped_signature = self.signature.transform_class(&transformer);
        Some(MethodData {
            name: self.name.clone(),
            declaring_type: remapped_class,
            signature: remapped_signature
        })
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
}
impl MapClass for FieldData {
    fn maybe_transform_class<T: TypeTransformer>(&self, transformer: T) -> Option<Self> {
        if let Some(reference_type) = transformer.maybe_remap_class(&self.declaring_type) {
            Some(FieldData { name: self.name.clone(), declaring_type: reference_type })
        } else {
            None
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
pub struct MethodSignature(Arc<MethodSignatureInner>);
#[derive(Debug)]
struct MethodSignatureInner {
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
        Self::from_raw(descriptor, return_type, parameter_types)
    }
    #[inline]
    fn from_raw(descriptor: String, return_type: TypeDescriptor, parameter_types: Vec<TypeDescriptor>) -> Self {
        MethodSignature(Arc::new(MethodSignatureInner { descriptor, return_type, parameter_types }))
    }
    #[inline]
    pub fn from_descriptor(s: &str) -> MethodSignature {
        Self::parse_descriptor(s).unwrap_or_else(|| panic!("Invalid descriptor: {:?}", s))
    }
    #[inline]
    pub fn parse_descriptor(s: &str) -> Option<Self> {
        MethodSignature::parse_text(s).ok()
    }
    #[inline]
    pub fn descriptor(&self) -> &str {
        &self.0.descriptor
    }
    #[inline]
    pub fn return_type(&self) -> &TypeDescriptor {
        &self.0.return_type
    }
    #[inline]
    pub fn parameter_types(&self) -> &[TypeDescriptor] {
        &self.0.parameter_types
    }
    pub(crate) fn raw_transform_class<T: TypeTransformer>(&self, transformer: T) -> MethodSignature {
        MethodSignature::new(
            self.return_type().transform_class(&transformer),
            self.parameter_types().iter()
                .map(|t| t.transform_class(&transformer)).collect()
        )
    }
}
impl MapClass for MethodSignature {
    #[inline]
    fn maybe_transform_class<T: TypeTransformer>(&self, transformer: T) -> Option<Self> {
        Some(transformer.remap_signature(self))
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
        Ok(Self::from_raw(descriptor, return_type, parameter_types))
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
