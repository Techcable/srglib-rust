use std::borrow::Cow;
use std::sync::Arc;
use std::hash::{Hash, Hasher};

use indexmap::Equivalent;
use lazy_static::lazy_static;

use crate::utils::*;

macro_rules! descriptor_hash {
    ($target:ty) => {
        impl PartialEq for $target {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.descriptor() == other.descriptor()
            }
        }
        impl Eq for $target {}
        descriptor_hash!($target, equals = false);
    };
    ($target:ty, equals = false) => {
        impl Hash for $target {
            #[inline]
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.descriptor().hash(state);
            }
        }
    }
}

pub trait JavaType<'a>: Clone + Equivalent<TypeDescriptor> {
    type Name: Into<String> + AsRef<str> + 'a;
    type InternalName: Into<String> + AsRef<str> + 'a;
    fn parse_descriptor(s: &str) -> Option<Self>;
    fn descriptor(&'a self) -> &'a str;
    fn name(&'a self) -> Self::Name;
    fn internal_name(&'a self) -> Self::InternalName;
    // Casting
    fn into_type_descriptor(self) -> TypeDescriptor;
    // Operations
    /// Apply the specified mapping to this type, based on its class name.
    ///
    /// If type is an array, it remaps the innermost element type.
    /// If the type is a class, it invokes the specified function
    /// If the type is a primitive, it returns the same element type.
    fn map_class<F: FnMut(&ReferenceType) -> Option<ReferenceType>>(&self, func: F) -> Self {
        self.maybe_map_class(func).unwrap_or_else(|| self.clone())
    }
    fn maybe_map_class<F: FnMut(&ReferenceType) -> Option<ReferenceType>>(&self, func: F) -> Option<Self>;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PrimitiveType {
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    Char,
    Boolean,
    Void
}

lazy_static! {
static ref PRIMITIVE_DESCRIPTOR_TABLE: [TypeDescriptor; 9] = [
    TypeDescriptor::Primitive(PrimitiveType::Byte),
    TypeDescriptor::Primitive(PrimitiveType::Short),
    TypeDescriptor::Primitive(PrimitiveType::Int),
    TypeDescriptor::Primitive(PrimitiveType::Long),
    TypeDescriptor::Primitive(PrimitiveType::Float),
    TypeDescriptor::Primitive(PrimitiveType::Double),
    TypeDescriptor::Primitive(PrimitiveType::Char),
    TypeDescriptor::Primitive(PrimitiveType::Boolean),
    TypeDescriptor::Primitive(PrimitiveType::Void),
];
}
impl PrimitiveType {
    fn descriptor_str(self) -> &'static str {
        match self {
            PrimitiveType::Byte => "B",
            PrimitiveType::Short => "S",
            PrimitiveType::Int => "I",
            PrimitiveType::Long => "J",
            PrimitiveType::Float => "F",
            PrimitiveType::Double => "D",
            PrimitiveType::Char => "C",
            PrimitiveType::Boolean => "Z",
            PrimitiveType::Void => "V",
        }
    }
}
impl<'a> JavaType<'a> for PrimitiveType {
    type Name = &'static str;

    type InternalName = &'static str;

    fn parse_descriptor(s: &str) -> Option<Self> {
        Self::parse_text(s).ok()
    }
    #[inline]
    fn descriptor(&'a self) -> &'static str {
        self.descriptor_str()
    }

    #[inline]
    fn name(&self) -> &'static str {
        match *self {
            PrimitiveType::Byte => "byte",
            PrimitiveType::Short => "short",
            PrimitiveType::Int => "int",
            PrimitiveType::Long => "long",
            PrimitiveType::Float => "float",
            PrimitiveType::Double => "double",
            PrimitiveType::Char => "char",
            PrimitiveType::Boolean => "boolean",
            PrimitiveType::Void => "void",
        }
    }

    #[inline]
    fn internal_name(&self) -> &'static str {
        self.name()
    }
    #[inline]
    fn into_type_descriptor(self) -> TypeDescriptor {
        PRIMITIVE_DESCRIPTOR_TABLE[self as usize].clone()
    }
    #[inline]
    fn maybe_map_class<F: FnMut(&ReferenceType) -> Option<ReferenceType>>(&self, _func: F) -> Option<Self> {
        None
    }
}
impl Equivalent<TypeDescriptor> for PrimitiveType {
    fn equivalent(&self, key: &TypeDescriptor) -> bool {
        match *key {
            TypeDescriptor::Primitive(prim) => prim == *self,
            _ => false
        }
    }
}
impl SimpleParse for PrimitiveType {
    fn parse(parser: &mut SimpleParser) -> Result<Self, SimpleParseError> {
        let primitive_type = match parser.peek()? {
            'B' => PrimitiveType::Byte,
            'S' => PrimitiveType::Short,
            'I' => PrimitiveType::Int,
            'J' => PrimitiveType::Long,
            'F' => PrimitiveType::Float,
            'D' => PrimitiveType::Double,
            'C' => PrimitiveType::Char,
            'Z' => PrimitiveType::Boolean,
            'V' => PrimitiveType::Void,
            _ => return Err(parser.error())
        };
        parser.skip(1);
        Ok(primitive_type)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeDescriptor {
    Primitive(PrimitiveType),
    Reference(ReferenceType),
    Array(ArrayType)
}
impl SimpleParse for TypeDescriptor {
    fn parse(parser: &mut SimpleParser) -> Result<Self, SimpleParseError> {
        Ok(match parser.peek()? {
            'L' => parser.parse::<ReferenceType>()?.into_type_descriptor(),
            '[' => parser.parse::<ArrayType>()?.into_type_descriptor(),
            _ => parser.parse::<PrimitiveType>()?.into_type_descriptor()
        })
    }
}
// NOTE: Must use descriptor_hash so Borrow and hashmap will work correctly
descriptor_hash!(TypeDescriptor, equals = false);
impl<'a> JavaType<'a> for TypeDescriptor {
    type Name = Cow<'a, str>;
    type InternalName = Cow<'a, str>;

    fn parse_descriptor(s: &str) -> Option<Self> {
        Self::parse_text(s).ok()
    }

    #[inline]
    fn descriptor(&'a self) -> &'a str {
        match self {
            TypeDescriptor::Primitive(prim) => prim.descriptor(),
            TypeDescriptor::Reference(obj) => obj.descriptor(),
            TypeDescriptor::Array(array) => array.descriptor(),
        }
    }

    #[inline]
    fn name(&'a self) -> Cow<'a, str> {
        match self {
            TypeDescriptor::Primitive(prim) => Cow::Borrowed(prim.name()),
            TypeDescriptor::Reference(obj) => Cow::Owned(obj.name()),
            TypeDescriptor::Array(array) => Cow::Owned(array.name()),
        }
    }

    #[inline]
    fn internal_name(&'a self) -> Cow<'a, str> {
        match self {
            TypeDescriptor::Primitive(prim) => Cow::Borrowed(prim.internal_name()),
            TypeDescriptor::Reference(obj) => Cow::Borrowed(obj.internal_name()),
            TypeDescriptor::Array(array) => Cow::Owned(array.internal_name()),
        }
    }
    #[inline]
    fn into_type_descriptor(self) -> TypeDescriptor {
        self
    }
    fn maybe_map_class<F: FnMut(&ReferenceType) -> Option<ReferenceType>>(&self, func: F) -> Option<Self> {
        Some(match self {
            TypeDescriptor::Primitive(prim) => prim.maybe_map_class(func)?.into_type_descriptor(),
            TypeDescriptor::Reference(obj) => obj.maybe_map_class(func)?.into_type_descriptor(),
            TypeDescriptor::Array(array) => array.maybe_map_class(func)?.into_type_descriptor(),
        })
    }
}
/// The type of a java array (`int[]` or `Object[]`).
///
/// Array types aren't recursive in order to avoid an allocation.
/// Instead we maintain an explicit counter for number of dimensions in order to avoid an allocation.
#[derive(Clone, Debug)]
pub struct ArrayType(Arc<ArrayTypeInner>);
#[derive(Debug)]
struct ArrayTypeInner {
    descriptor: String,
    dimensions: usize,
    element_type: ElementType
}
descriptor_hash!(ArrayType);
impl ArrayType {
    pub fn new<'a, T: JavaType<'a>>(dimensions: usize, element_type: T) -> ArrayType {
        assert!(dimensions >= 1, "Invalid dimensions: {}", dimensions);
        let element_type = match element_type.into_type_descriptor() {
            TypeDescriptor::Primitive(prim) => ElementType::Primitive(prim),
            TypeDescriptor::Reference(obj) => ElementType::Reference(obj),
            TypeDescriptor::Array(array) => panic!("Can't have array element_type: {:?}", array),
        };
        let mut descriptor = String::with_capacity(
            dimensions * 2 + element_type.descriptor().len());
        for _ in 0..dimensions {
            descriptor.push_str("[");
        }
        descriptor.push_str(element_type.descriptor());
        ArrayType(Arc::new(ArrayTypeInner { descriptor, dimensions, element_type }))
    }
}

impl Equivalent<TypeDescriptor> for ArrayType {
    fn equivalent(&self, key: &TypeDescriptor) -> bool {
        match *key {
            TypeDescriptor::Array(_) => self.descriptor() == key.descriptor(),
            _ => false
        }
    }
}
impl SimpleParse for ArrayType {
    fn parse(parser: &mut SimpleParser) -> Result<Self, SimpleParseError> {
        let start = parser.current_index();
        parser.expect('[')?;
        let dimensions = 1 + parser.take_until(|c| c != '[').len();
        let element_type = match parser.peek()? {
            '[' => unreachable!(),
            'L' => ElementType::Reference(parser.parse()?),
            _ => ElementType::Primitive(parser.parse()?)
        };
        let end = parser.current_index();
        let descriptor = &parser.original()[start..end];
        Ok(ArrayType(Arc::new(ArrayTypeInner {
            descriptor: descriptor.into(),
            dimensions, element_type
        })))
    }
}
impl<'a> JavaType<'a> for ArrayType {
    type Name = String;
    type InternalName = String;

    #[inline]
    fn parse_descriptor(s: &str) -> Option<Self> {
        Self::parse_text(s).ok()
    }

    #[inline]
    fn descriptor(&'a self) -> &'a str {
        &self.0.descriptor
    }


    fn name(&'a self) -> String {
        let mut buffer = self.0.element_type.name();
        buffer.reserve((self.0.dimensions * 2) as usize);
        for _ in 0..self.0.dimensions {
            buffer.push_str("[]");
        }
        buffer
    }

    fn internal_name(&'a self) -> String {
        let element_internal_name = self.0.element_type.internal_name();
        let mut buffer = String::with_capacity(
            element_internal_name.len() + (self.0.dimensions * 2) as usize);
        buffer.push_str(element_internal_name);
        for _ in 0..self.0.dimensions {
            buffer.push_str("[]");
        }
        buffer
    }

    #[inline]
    fn into_type_descriptor(self) -> TypeDescriptor {
        TypeDescriptor::Array(self)
    }

    fn maybe_map_class<F: FnMut(&ReferenceType) -> Option<ReferenceType>>(&self, mut func: F) -> Option<Self> {
        if let ElementType::Reference(ref reference) = self.0.element_type {
            if let Some(remapped_reference) = func(reference) {
                return Some(ArrayType::new(self.0.dimensions, remapped_reference.into_type_descriptor()))
            }
        }
        None
    }
}
/// A possible element type for an `ArrayType`,
/// which is just a `DecodedType` without an `ArrayType`.
///
/// This is just a stub that doesn't actually implement `JavaType`
#[derive(Clone, Debug)]
enum ElementType {
    Primitive(PrimitiveType),
    Reference(ReferenceType)
}
impl ElementType {
    fn name(&self) -> String {
        match self {
            ElementType::Primitive(prim) => prim.name().into(),
            ElementType::Reference(reference) => reference.name(),
        }
    }
    #[inline]
    fn descriptor(&self) -> &str {
        match self {
            ElementType::Primitive(prim) => prim.descriptor(),
            ElementType::Reference(obj) => obj.descriptor(),
        }
    }
    #[inline]
    fn internal_name(&self) -> &str {
        match self {
            ElementType::Primitive(prim) => prim.internal_name(),
            ElementType::Reference(reference) => reference.internal_name(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReferenceType {
    // NOTE: Using an Arc makes this cheep to clone
    descriptor: Arc<str>
}
impl ReferenceType {
    pub fn from_name(name: &str) -> ReferenceType {
        Self::from_internal_name(&name.replace('.', "/"))
    }
    pub fn from_internal_name(name: &str) -> ReferenceType {
        assert!(!name.contains('.'));
        let mut descriptor = String::with_capacity(name.len() + 2);
        descriptor.push('L');
        descriptor.push_str(name);
        descriptor.push(';');
        ReferenceType { descriptor: descriptor.into() }
    }
    /// Give this package name as it's 'internal name'.
    ///
    /// For example `java/lang/String` will return `java/lang`
    #[inline]
    pub fn package_name(&self) -> &str {
        self.split_name().0
    }
    /// Split this type's internal name into a tuple of its package and simple names.
    ///
    /// For example, `java/lang/String` will give `("java/lang", "String")`
    pub fn split_name(&self) -> (&str, &str) {
        let internal_name = self.internal_name();
        if let Some(package_separator) = internal_name.rfind('/') {
            (&internal_name[..package_separator], &internal_name[package_separator + 1..])
        } else {
            ("", internal_name)
        }
    }
    #[inline]
    pub fn simple_name(&self) -> &str {
        self.split_name().1
    }
}
impl SimpleParse for ReferenceType {
    fn parse(parser: &mut SimpleParser) -> Result<Self, SimpleParseError> {
        let start = parser.current_index();
        let start_remaining = parser.remaining();
        parser.expect('L')?;
        parser.take_until(|c| c == '.' || c == ';');
        parser.expect(';')?;
        let end = parser.current_index();
        let descriptor = &start_remaining[..(end - start)];
        Ok(ReferenceType { descriptor: descriptor.into() })
    }
}
impl Equivalent<TypeDescriptor> for ReferenceType {
    #[inline]
    fn equivalent(&self, key: &TypeDescriptor) -> bool {
        match *key {
            TypeDescriptor::Reference(_) => key.descriptor() == self.descriptor(),
            _ => false
        }
    }
}
descriptor_hash!(ReferenceType);
impl<'a> JavaType<'a> for ReferenceType {
    type Name = String;
    type InternalName = &'a str;

    fn parse_descriptor(s: &str) -> Option<Self> {
        Self::parse_text(s).ok()
    }

    #[inline]
    fn descriptor(&'a self) -> &'a str {
        &self.descriptor
    }

    #[inline]
    fn name(&'a self) -> String {
        self.internal_name().replace('/', ".")
    }

    #[inline]
    fn internal_name(&'a self) -> &'a str {
        &self.descriptor[1..(self.descriptor.len() - 1)]
    }

    #[inline]
    fn into_type_descriptor(self) -> TypeDescriptor {
        TypeDescriptor::Reference(self)
    }

    #[inline]
    fn maybe_map_class<F: FnMut(&ReferenceType) -> Option<ReferenceType>>(&self, mut func: F) -> Option<Self> {
        func(self)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_internal_names() {
        assert_eq!(PrimitiveType::Byte.internal_name(), "byte");
        assert_eq!(
            ReferenceType::from_name("obf4").internal_name(),
            "obf4"
        );
        assert_eq!(
            ReferenceType::from_name("org.spigotmc.XRay").internal_name(),
            "org/spigotmc/XRay"
        );
        assert_eq!(
            ArrayType::new(2, ReferenceType::from_name("org.spigotmc.XRay"))
                .internal_name(),
            "org/spigotmc/XRay[][]"
        );
    }
}
