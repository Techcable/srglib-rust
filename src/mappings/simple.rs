use std::borrow::Cow;
use std::iter;

use indexmap::{map};

use crate::prelude::*;
use crate::utils::FnvIndexMap;

#[derive(Clone, Debug, Default)]
pub struct SimpleMappings {
    pub(super) classes: FnvIndexMap<ReferenceType, ReferenceType>,
    pub(super) method_names: FnvIndexMap<MethodData, String>,
    pub(super) field_names: FnvIndexMap<FieldData, String>
}
impl Mappings for SimpleMappings {
    #[inline]
    fn get_remapped_class(&self, original: &ReferenceType) -> Option<&ReferenceType> {
        self.classes.get(original)
    }

    #[inline]
    fn get_remapped_field(&self, original: &FieldData) -> Option<Cow<FieldData>> {
        self.field_names.get(original).map(|name| {
            Cow::Owned(FieldData::new(
                name.clone(),
                self.remap_class(original.declaring_type())
            ))
        })
    }

    #[inline]
    fn get_remapped_method(&self, original: &MethodData) -> Option<Cow<MethodData>> {
        self.method_names.get(original).map(|name| {
            let mut data = original
                .transform_class(&*self);
            data.name = name.clone();
            Cow::Owned(data)
        })
    }

    fn frozen(&self) -> FrozenMappings {
        FrozenMappings::new_ref(&self.classes, &self.field_names, &self.method_names)
    }
}
impl MutableMappings for SimpleMappings {
    #[inline]
    fn set_remapped_class(&mut self, original: ReferenceType, renamed: ReferenceType) {
        self.classes.insert(original, renamed);
    }

    #[inline]
    fn set_method_name(&mut self, original: MethodData, renamed: String) {
        self.method_names.insert(original, renamed);
    }

    #[inline]
    fn set_field_name(&mut self, original: FieldData, renamed: String) {
        self.field_names.insert(original, renamed);
    }

    #[inline]
    fn retain_classes<F: FnMut(&ReferenceType, &ReferenceType) -> bool>(&mut self, mut func: F) {
        self.classes.retain(|key, value| func(key, value));
    }

    #[inline]
    fn retain_fields<F: FnMut(&FieldData, &str) -> bool>(&mut self, mut func: F) {
        self.field_names.retain(|key, value| func(key, &**value));
    }

    #[inline]
    fn retain_methods<F: FnMut(&MethodData, &str) -> bool>(&mut self, mut func: F) {
        self.method_names.retain(|key, value| func(key, &**value));
    }

    #[inline]
    fn clear_classes(&mut self) {
        self.classes.clear();
    }

    #[inline]
    fn clear_fields(&mut self) {
        self.field_names.clear();
    }

    #[inline]
    fn clear_methods(&mut self) {
        self.method_names.clear();
    }
}
impl<'a> IterableMappings<'a> for SimpleMappings {
    type FieldValue = FieldData;
    type MethodValue = MethodData;
    type OriginalClasses = map::Keys<'a, ReferenceType, ReferenceType>;
    type OriginalFields = map::Keys<'a, FieldData, String>;
    type OriginalMethods = map::Keys<'a, MethodData, String>;
    type Classes = map::Iter<'a, ReferenceType, ReferenceType>;
    type Fields = Fields<'a>;
    type Methods = Methods<'a>;

    #[inline]
    fn original_classes(&'a self) -> Self::OriginalClasses {
        self.classes.keys()
    }

    #[inline]
    fn original_fields(&'a self) -> Self::OriginalFields {
        self.field_names.keys()
    }

    #[inline]
    fn original_methods(&'a self) -> Self::OriginalMethods {
        self.method_names.keys()
    }

    #[inline]
    fn classes(&'a self) -> Self::Classes {
        self.classes.iter()
    }

    #[inline]
    fn fields(&'a self) -> Self::Fields {
        Fields {
            mappings: self,
            iter: self.field_names.iter()
        }
    }

    #[inline]
    fn methods(&'a self) -> Self::Methods {
        Methods {
            mappings: self,
            iter: self.method_names.iter()
        }
    }
}
impl TypeTransformer for SimpleMappings {
    #[inline]
    fn maybe_remap_class(&self, original: &ReferenceType) -> Option<ReferenceType> {
        self.get_remapped_class(original).cloned()
    }
}
pub struct Fields<'a> {
    mappings: &'a SimpleMappings,
    iter: map::Iter<'a, FieldData, String>
}
impl<'a> Iterator for Fields<'a> {
    type Item = (&'a FieldData, FieldData);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        Iterator::next(&mut self.iter).map(|(original, renamed)| {
            let mut data = original.transform_class(self.mappings);
            data.name = renamed.clone();
            (original, data)
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        Iterator::size_hint(&self.iter)
    }
}
impl<'a> iter::ExactSizeIterator for Fields<'a> {}
impl<'a> iter::FusedIterator for Fields<'a> {}


pub struct Methods<'a> {
    mappings: &'a SimpleMappings,
    iter: map::Iter<'a, MethodData, String>
}
impl<'a> Iterator for Methods<'a> {
    type Item = (&'a MethodData, MethodData);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        Iterator::next(&mut self.iter).map(|(original, renamed)| {
            let mut data = original
                .transform_class(self.mappings);
            data.name = renamed.clone();
            (original, data)
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        Iterator::size_hint(&self.iter)
    }
}
impl<'a> iter::ExactSizeIterator for Methods<'a> {}
impl<'a> iter::FusedIterator for Methods<'a> {}