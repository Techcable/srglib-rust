use std::borrow::{Borrow, Cow};

use super::prelude::*;

pub mod simple;
pub mod frozen;
mod transformer;

pub use self::simple::SimpleMappings;
pub use self::frozen::FrozenMappings;

/// Chain all the specified mappings together,
/// using the renamed result of each mapping as the original for the next
#[macro_export]
macro_rules! chain {
    () => (FrozenMappings::empty());
    ($target:expr) => ($target.frozen());
    ($first:expr, $($remaining:expr),*) => {{
        let mut chained = $first.frozen();
        $(chained = chained.chain($remaining);)*
        chained
    }};
}

/// A mapping from one set of source names to another
pub trait Mappings: Default + ::std::fmt::Debug {
    /// Get the remapped class name
    fn get_remapped_class(&self, original: &ReferenceType) -> Option<&ReferenceType>;
    #[inline]
    fn remap_type(&self, original: &TypeDescriptor) -> TypeDescriptor {
        original.map_class(|original| self.get_remapped_class(original).cloned())
    }
    #[inline]
    fn remap_class(&self, original: &ReferenceType) -> ReferenceType {
        self.get_remapped_class(original).unwrap_or(original).clone()
    }
    #[inline]
    fn remap_class_name(&self, original: &str) -> ReferenceType {
        self.remap_class(&ReferenceType::from_name(original))
    }
    /// Get the remapped field data, or `None` if the field doesn't exist
    fn get_remapped_field(&self, original: &FieldData) -> Option<Cow<FieldData>>;
    /// Get the remapped field data.
    ///
    /// Even if the field name remains the same,
    /// this will automatically remaps class names in the signature as needed.
    #[inline]
    fn remap_field(&self, original: &FieldData) -> FieldData {
        self.get_remapped_field(original).map(Cow::into_owned).unwrap_or_else(|| {
            original.map_class(|t| self.get_remapped_class(t).cloned())
        })
    }
    /// Get the remapped method data, or `None` if the field doesn't exist
    fn get_remapped_method(&self, original: &MethodData) -> Option<Cow<MethodData>>;
    /// Get the remapped method data.
    ///
    /// Even if the method name remains the same,
    /// this will automatically remaps class names in the signature as needed.
    #[inline]
    fn remap_method(&self, original: &MethodData) -> MethodData {
        self.get_remapped_method(original).map(Cow::into_owned).unwrap_or_else(|| {
            original.map_class(|t| self.get_remapped_class(t).cloned())
        })
    }
    fn frozen(&self) -> FrozenMappings;
    fn inverted(&self) -> FrozenMappings {
        self.frozen().inverted()
    }
}
pub trait MutableMappings: Mappings {
    fn set_remapped_class(&mut self, original: ReferenceType, renamed: ReferenceType);
    fn set_method_name(&mut self, original: MethodData, renamed: String);
    fn set_field_name(&mut self, original: FieldData, renamed: String);
    fn retain_classes<F: FnMut(&ReferenceType, &ReferenceType) -> bool>(&mut self, func: F);
    fn retain_fields<F: FnMut(&FieldData, &str) -> bool>(&mut self, func: F);
    fn retain_methods<F: FnMut(&MethodData, &str) -> bool>(&mut self, func: F);
    fn clear_classes(&mut self);
    fn clear_fields(&mut self);
    fn clear_methods(&mut self);
}
pub trait IterableMappings<'a>: Mappings {
    type FieldValue: Borrow<FieldData> + Into<FieldData>;
    type MethodValue: Borrow<MethodData> + Into<MethodData>;
    type OriginalClasses: Iterator<Item=&'a ReferenceType>;
    type OriginalFields: Iterator<Item=&'a FieldData>;
    type OriginalMethods: Iterator<Item=&'a MethodData>;
    type Classes: Iterator<Item=(&'a ReferenceType, &'a ReferenceType)>;
    type Fields: Iterator<Item=(&'a FieldData, Self::FieldValue)>;
    type Methods: Iterator<Item=(&'a MethodData, Self::MethodValue)>;

    fn original_classes(&'a self) -> Self::OriginalClasses;
    fn original_fields(&'a self) -> Self::OriginalFields;
    fn original_methods(&'a self) -> Self::OriginalMethods;
    fn classes(&'a self) -> Self::Classes;
    fn fields(&'a self) -> Self::Fields;
    fn methods(&'a self) -> Self::Methods;

    /// Transform all of this mapping's data using the specified mappings.
    ///
    /// The returned mapping data is guaranteed to have the same originals
    /// as the data of the old mapping data
    #[inline]
    fn transform<T: Mappings>(&'a self, transformer: T) -> FrozenMappings  {
        self::transformer::transform(self, transformer)
    }
    fn transform_packages<F>(&'a self, func: F) -> FrozenMappings
        where F: Fn(&str) -> Option<String> {
        self.transform_classes(|t| {
            let (package_name, simple_name) = t.split_name();
            match func(package_name) {
                Some(updated_package) => {
                    let mut result: String = updated_package.clone();
                    if !result.is_empty() {
                        result.push('/');
                    }
                    result.push_str(simple_name);
                    Some(ReferenceType::from_internal_name(&result))
                },
                None => None
            }
        })
    }
    #[inline]
    fn transform_classes<F>(&'a self, func: F) -> FrozenMappings
        where F: Fn(&ReferenceType) -> Option<ReferenceType> {
        self::transformer::transform(
            self,
            self::transformer::TypeTransformer(func)
        )
    }
    #[inline]
    fn transform_fields<F>(&'a self, func: F) -> FrozenMappings
        where F: Fn(&FieldData) -> Option<String> {
        self::transformer::transform(
            self,
            self::transformer::FieldRenamer(func)
        )
    }
    #[inline]
    fn transform_methods<F>(&'a self, func: F) -> FrozenMappings
        where F: Fn(&MethodData) -> Option<String> {
        self::transformer::transform(
            self,
            self::transformer::MethodRenamer(func)
        )
    }
}