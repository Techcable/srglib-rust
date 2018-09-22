use std::borrow::Borrow;

use crate::prelude::*;

/// Transform all of the mapping's original data using the specified transformer.
///
/// This is different from chain since it completely ignores a mapping's renamed data.
/// The returned mapping data is guaranteed to have the same originals data
/// as the original mapping
pub fn transform<'a, M: IterableMappings<'a>, T: MappingsTransformer>(mappings: &'a M, transformer: T) -> FrozenMappings {
    FrozenMappings::new(
        mappings.classes()
            .map(|(original, renamed)| (original.clone(), transformer.transform_class(renamed.borrow()).unwrap_or_else(|| renamed.clone()))),
        mappings.fields()
            .map(|(original, renamed)| (original.clone(), transformer.rename_field(renamed.borrow()).unwrap_or_else(|| renamed.borrow().name.clone()))),
        mappings.methods()
            .map(|(original, renamed)| (original.clone(), transformer.rename_method(renamed.borrow()).unwrap_or_else(|| renamed.borrow().name.clone()))),
    )
}

// TODO: Should this all become a special-case for `Mapping`?
pub trait MappingsTransformer {
    fn transform_class(&self, original: &ReferenceType) -> Option<ReferenceType>;
    #[inline]
    fn remap_type(&self, original: &TypeDescriptor) -> TypeDescriptor {
        original.map_class(|t| self.transform_class(t))
    }
    fn remap_field(&self, original: &FieldData) -> FieldData {
        self.rename_field(original).map_or_else(
            || original.map_class(|t| self.transform_class(t)),
            |renamed| {
                let mut data = original
                    .map_class(|t| self.transform_class(t));
                data.name = renamed;
                data
            }
        )
    }
    fn remap_method(&self, original: &MethodData) -> MethodData {
        self.rename_method(original).map_or_else(
            || original.map_class(|t| self.transform_class(t)),
            |renamed| {
            let mut data = original
                .map_class(|t| self.transform_class(t));
            data.name = renamed;
            data
        })
    }
    fn rename_field(&self, original: &FieldData) -> Option<String>;
    fn rename_method(&self, original: &MethodData) -> Option<String>;
}
impl<T: Mappings> MappingsTransformer for T {
    #[inline]
    fn transform_class(&self, original: &ReferenceType) -> Option<ReferenceType> {
        self.get_remapped_class(original).cloned()
    }

    #[inline]
    fn remap_type(&self, original: &TypeDescriptor) -> TypeDescriptor {
        Mappings::remap_type(self, original)
    }

    #[inline]
    fn remap_field(&self, original: &FieldData) -> FieldData {
        Mappings::remap_field(self, original)
    }

    #[inline]
    fn remap_method(&self, original: &MethodData) -> MethodData {
        Mappings::remap_method(self, original)
    }

    #[inline]
    fn rename_field(&self, original: &FieldData) -> Option<String> {
        self.get_remapped_field(original).map(|t| t.name.clone())
    }

    #[inline]
    fn rename_method(&self, original: &MethodData) -> Option<String> {
        self.get_remapped_method(original).map(|t| t.name.clone())
    }
}
pub struct TypeTransformer<F: Fn(&ReferenceType) -> Option<ReferenceType>>(pub F);
impl<F: Fn(&ReferenceType) -> Option<ReferenceType>> MappingsTransformer for TypeTransformer<F> {
    #[inline]
    fn transform_class(&self, original: &ReferenceType) -> Option<ReferenceType> {
        self.0(original)
    }

    #[inline]
    fn rename_field(&self, _original: &FieldData) -> Option<String> {
        None
    }

    #[inline]
    fn rename_method(&self, _original: &MethodData) -> Option<String> {
        None
    }
}
pub struct FieldRenamer<F: Fn(&FieldData) -> Option<String>>(pub F);
impl<F: Fn(&FieldData) -> Option<String>> MappingsTransformer for FieldRenamer<F> {
    #[inline]
    fn transform_class(&self, _original: &ReferenceType) -> Option<ReferenceType> {
        None
    }

    #[inline]
    fn rename_field(&self, original: &FieldData) -> Option<String> {
        self.0(original)
    }

    #[inline]
    fn rename_method(&self, _original: &MethodData) -> Option<String> {
        None
    }
}

pub struct MethodRenamer<F: Fn(&MethodData) -> Option<String>>(pub F);
impl<F: Fn(&MethodData) -> Option<String>> MappingsTransformer for MethodRenamer<F> {
    #[inline]
    fn transform_class(&self, _original: &ReferenceType) -> Option<ReferenceType> {
        None
    }

    #[inline]
    fn rename_field(&self, _original: &FieldData) -> Option<String> {
        None
    }

    #[inline]
    fn rename_method(&self, original: &MethodData) -> Option<String> {
        self.0(original)
    }
}
