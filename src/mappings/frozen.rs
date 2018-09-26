use std::sync::Arc;
use std::borrow::Cow;
use std::fmt::{self, Debug};

use indexmap::{map};
use lazy_static::*;
use difference::Changeset;

use crate::utils::FnvIndexMap;
use crate::prelude::*;

#[derive(Clone)]
pub struct FrozenMappings {
    primary: Arc<FrozenMappingsInner>,
    inverted: Arc<FrozenMappingsInner>
}
impl PartialEq for FrozenMappings {
    fn eq(&self, other: &FrozenMappings) -> bool {
        self.primary == other.primary
    }
}
impl Debug for FrozenMappings {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FrozenMappings")
            .field("classes", &self.primary.classes)
            .field("fields", &self.primary.fields)
            .field("methods", &self.primary.methods)
            .finish()
    }
}
#[derive(Debug, PartialEq,)]
struct FrozenMappingsInner {
    classes: FnvIndexMap<ReferenceType, ReferenceType>,
    methods: FnvIndexMap<MethodData, MethodData>,
    fields: FnvIndexMap<FieldData, FieldData>
}
impl FrozenMappingsInner {
    #[inline]
    fn empty() -> Self {
        FrozenMappingsInner {
            classes: FnvIndexMap::default(),
            methods: FnvIndexMap::default(),
            fields: FnvIndexMap::default(),
        }
    }
}
lazy_static! {
    static ref EMPTY_MAPPINGS: FrozenMappings = FrozenMappings {
        primary: Arc::new(FrozenMappingsInner::empty()),
        inverted: Arc::new(FrozenMappingsInner::empty())
    };
}
impl Default for FrozenMappings {
    #[inline]
    fn default() -> Self {
        FrozenMappings::empty()
    }
}
impl FrozenMappings {
    #[inline]
    pub fn empty() -> FrozenMappings {
        EMPTY_MAPPINGS.clone()
    }
    pub fn new_ref<'a, C, F, M>(classes: C, fields: F, methods: M) -> FrozenMappings
        where C: IntoIterator<Item=(&'a ReferenceType, &'a ReferenceType)>,
              F: IntoIterator<Item=(&'a FieldData, &'a String)>,
              M: IntoIterator<Item=(&'a MethodData, &'a String)> {
        FrozenMappings::new(
            classes.into_iter().map(|(original, renamed)| (original.clone(), renamed.clone())),
            fields.into_iter().map(|(original, renamed)| (original.clone(), renamed.clone())),
            methods.into_iter().map(|(original, renamed)| (original.clone(), renamed.clone())),
        )
    }
    pub fn new<C, F, M>(classes: C, fields: F, methods: M) -> FrozenMappings
        where C: IntoIterator<Item=(ReferenceType, ReferenceType)>,
              F: IntoIterator<Item=(FieldData, String)>,
              M: IntoIterator<Item=(MethodData, String)> {
        let classes: FnvIndexMap<ReferenceType, ReferenceType> = classes.into_iter().collect();
        let remap_func = |original: &ReferenceType| {
            classes.get(original).cloned()
        };
        let fields = fields.into_iter().map(|(first, name): (FieldData, String)| {
            let mut second = first.map_class(remap_func);
            second.name = name.clone();
            (first, second)
        }).collect();
        let methods = methods.into_iter().map(|(first, name): (MethodData, String)| {
            let mut second = first.map_class(remap_func);
            second.name = name.clone();
            (first, second)
        }).collect();
        Self::new_raw(classes, fields, methods)
    }
    /// Create a new FrozenMappings from the specified indexmaps,
    /// without checking that the mappings are consistent.
    fn new_raw(
        classes: FnvIndexMap<ReferenceType, ReferenceType>,
        fields: FnvIndexMap<FieldData, FieldData>,
        methods: FnvIndexMap<MethodData, MethodData>
    ) -> FrozenMappings {
        let primary = FrozenMappingsInner { classes, fields, methods };
        let inverted = FrozenMappingsInner {
            classes: primary.classes.iter()
                .map(|(original, revised)| (revised.clone(), original.clone()))
                .collect(),
            methods: primary.methods.iter()
                .map(|(original, revised)| (revised.clone(), original.clone()))
                .collect(),
            fields: primary.fields.iter()
                .map(|(original, revised)| (revised.clone(), original.clone()))
                .collect(),
        };
        FrozenMappings { primary: Arc::new(primary), inverted: Arc::new(inverted) }
    }
    /// Chain the specified mappings onto this one,
    /// using the renamed result of each mapping as the original for the next
    pub fn chain<T: for<'a> IterableMappings<'a> >(&self, mapping: T) -> FrozenMappings {
        let mut classes = FnvIndexMap::default();
        let mut fields = FnvIndexMap::default();
        let mut methods = FnvIndexMap::default();
        let inverted = self.inverted();

        // If we encounter a new name, add it to the set
        for (original, renamed) in mapping.classes() {
            if inverted.get_remapped_class(original).is_none() {
                classes.insert(original.clone(), renamed.clone());
            }
        }
        for (original, renamed) in mapping.fields() {
            if inverted.get_remapped_field(original).is_none() {
                /*
                 * We need to make sure the originals we put in the map have the
                 * oldest possible type name to remain consistent
                 * Since inverted is a map of new->old, use the old type name
                 * if we've ever seen this class before
                 */
                fields.insert(
                    original.map_class(|t| inverted.get_remapped_class(t).cloned()),
                    renamed.into()
                );
            }
        }
        for (original, renamed) in mapping.methods() {
            if inverted.get_remapped_method(original).is_none() {
                methods.insert(
                    original.map_class(|t| inverted.get_remapped_class(t).cloned()),
                    renamed.into()
                );
            }
        }
        // Now run all our current chain through the mapping to get our new result
        for (original, renamed) in self.classes() {
            let renamed = mapping.get_remapped_class(renamed)
                .unwrap_or_else(|| renamed).clone();
            classes.insert(original.clone(), renamed);
        }
        for (original, renamed) in self.fields() {
            let renamed = mapping.remap_field(renamed);
            fields.insert(original.clone(), renamed);
        }
        for (original, renamed) in self.methods() {
            let renamed = mapping.remap_method(renamed);
            methods.insert(original.clone(), renamed);
        }
        FrozenMappings::new_raw(classes, fields, methods)
    }
    #[doc(hidden)]
    pub fn srg_difference(&self, other: &FrozenMappings) -> Changeset {
        let mut lines = SrgMappingsFormat::write_line_array(self);
        lines.sort();
        let mut other_lines = SrgMappingsFormat::write_line_array(other);
        other_lines.sort();
        let text = lines.join("\n");
        let other_text = other_lines.join("\n");
        Changeset::new(&text, &other_text, "\n")
    }
    #[doc(hidden)]
    pub fn assert_equal(&self, other: &FrozenMappings) {
        if self != other {
            panic!("Expected self = other, diff {}", self.srg_difference(other))
        }
    }
    pub fn rebuild(&self) -> SimpleMappings {
        SimpleMappings {
            classes: self.classes()
                .map(|(first, second)| (first.clone(), second.clone()))
                .collect(),
            field_names: self.fields()
                .map(|(first, second)| (first.clone(), second.name.clone()))
                .collect(),
            method_names: self.methods()
                .map(|(first, second)| (first.clone(), second.name.clone()))
                .collect()
        }
    }
}
impl Mappings for FrozenMappings {
    #[inline]
    fn get_remapped_class(&self, original: &ReferenceType) -> Option<&ReferenceType> {
        self.primary.classes.get(original)
    }

    #[inline]
    fn get_remapped_field(&self, original: &FieldData) -> Option<Cow<FieldData>> {
        self.primary.fields.get(original).map(Cow::Borrowed)
    }

    #[inline]
    fn get_remapped_method(&self, original: &MethodData) -> Option<Cow<MethodData>> {
        self.primary.methods.get(original).map(Cow::Borrowed)
    }

    #[inline]
    fn frozen(&self) -> FrozenMappings {
        self.clone()
    }

    #[inline]
    fn inverted(&self) -> FrozenMappings {
        FrozenMappings {
            primary: self.inverted.clone(),
            inverted: self.primary.clone()
        }
    }
}
impl<'a> IterableMappings<'a> for FrozenMappings {
    type FieldValue = &'a FieldData;
    type MethodValue = &'a MethodData;
    type Classes = map::Iter<'a, ReferenceType, ReferenceType>;
    type Fields = map::Iter<'a, FieldData, FieldData>;
    type Methods = map::Iter<'a, MethodData, MethodData>;
    type OriginalClasses = map::Keys<'a, ReferenceType, ReferenceType>;
    type OriginalFields = map::Keys<'a, FieldData, FieldData>;
    type OriginalMethods = map::Keys<'a, MethodData, MethodData>;


    #[inline]
    fn original_classes(&'a self) -> <Self as IterableMappings<'a>>::OriginalClasses {
        self.primary.classes.keys()
    }

    #[inline]
    fn original_fields(&'a self) -> <Self as IterableMappings<'a>>::OriginalFields {
        self.primary.fields.keys()
    }

    #[inline]
    fn original_methods(&'a self) -> <Self as IterableMappings<'a>>::OriginalMethods {
        self.primary.methods.keys()
    }

    #[inline]
    fn classes(&'a self) -> Self::Classes {
        self.primary.classes.iter()
    }

    #[inline]
    fn fields(&'a self) -> Self::Fields {
        self.primary.fields.iter()
    }

    #[inline]
    fn methods(&'a self) -> Self::Methods {
        self.primary.methods.iter()
    }
}
