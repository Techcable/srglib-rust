use std::ptr;
use std::sync::Arc;
use std::borrow::Cow;
use std::fmt::{self, Debug};

use indexmap::{map};
use lazy_static::*;
use difference::Changeset;
use owning_ref::ArcRef;
use lazycell::AtomicLazyCell;

use crate::utils::{FnvIndexMap};
use crate::prelude::*;


#[derive(Clone)]
pub struct FrozenMappings(ArcRef<FrozenMappingsBox, FrozenMappingsInner>);
struct FrozenMappingsBox {
    primary: FrozenMappingsInner,
    inverted: AtomicLazyCell<FrozenMappingsInner>
}
impl FrozenMappingsBox {
    fn inverted(&self) -> &FrozenMappingsInner {
        match self.inverted.borrow() {
            Some(inverted) => inverted,
            None => {
                // We don't care if we're the ones who fill it or if someone else already has
                drop(self.inverted.fill(self.primary.inverted()));
                self.inverted.borrow().unwrap()
            }
        }
    }
}
impl PartialEq for FrozenMappings {
    fn eq(&self, other: &FrozenMappings) -> bool {
        self.0 == other.0
    }
}
impl Debug for FrozenMappings {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FrozenMappings")
            .field("classes", &self.0.classes)
            .field("fields", &self.0.fields)
            .field("methods", &self.0.methods)
            .finish()
    }
}
#[derive(Debug, PartialEq)]
struct FrozenMappingsInner {
    classes: FnvIndexMap<ReferenceType, ReferenceType>,
    methods: FnvIndexMap<MethodData, MethodData>,
    fields: FnvIndexMap<FieldData, FieldData>
}
impl FrozenMappingsInner {
    fn inverted(&self) -> Self {
        FrozenMappingsInner {
            classes: self.classes.iter()
                .map(|(original, revised)| (revised.clone(), original.clone()))
                .collect(),
            methods: self.methods.iter()
                .map(|(original, revised)| (revised.clone(), original.clone()))
                .collect(),
            fields: self.fields.iter()
                .map(|(original, revised)| (revised.clone(), original.clone()))
                .collect(),
        }
    }
}
lazy_static! {
    static ref EMPTY_MAPPINGS: FrozenMappings = FrozenMappings::new_raw(
        Default::default(), Default::default(), Default::default()
    );
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
        let fields = fields.into_iter().map(|(first, name): (FieldData, String)| {
            let mut second = first.transform_class(&classes);
            second.name = name.clone();
            (first, second)
        }).collect();
        let methods = methods.into_iter().map(|(first, name): (MethodData, String)| {
            let mut second = first.transform_class(&classes);
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
        let boxed = Arc::new(FrozenMappingsBox {
            primary, inverted: AtomicLazyCell::NONE
        });
        FrozenMappings(ArcRef::new(boxed).map(|boxed| &boxed.primary))
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
                    original.transform_class(&inverted),
                    renamed.into()
                );
            }
        }
        for (original, renamed) in mapping.methods() {
            if inverted.get_remapped_method(original).is_none() {
                methods.insert(
                    original.transform_class(&inverted),
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
        self.0.classes.get(original)
    }

    #[inline]
    fn get_remapped_field(&self, original: &FieldData) -> Option<Cow<FieldData>> {
        self.0.fields.get(original).map(Cow::Borrowed)
    }

    #[inline]
    fn get_remapped_method(&self, original: &MethodData) -> Option<Cow<MethodData>> {
        self.0.methods.get(original).map(Cow::Borrowed)
    }

    #[inline]
    fn frozen(&self) -> FrozenMappings {
        self.clone()
    }

    fn inverted(&self) -> FrozenMappings {
        let owner = self.0.as_owner();
        let value = self.0.as_ref();
        let new_ref = ArcRef::new(owner.clone());
        FrozenMappings(if ptr::eq(&owner.primary, value) {
            new_ref.map(|owner| owner.inverted())
        } else if owner.inverted.borrow().map_or(
            false,
            |inverted| ptr::eq(inverted, value)
        ) {
            new_ref.map(|owner| &owner.primary)
        } else {
            // The only references we can create are for inverted and primary
            unreachable!()
        })
    }
}
impl TypeTransformer for FrozenMappings {
    fn maybe_remap_class(&self, original: &ReferenceType) -> Option<ReferenceType> {
        self.get_remapped_class(original).cloned()
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
        self.0.classes.keys()
    }

    #[inline]
    fn original_fields(&'a self) -> <Self as IterableMappings<'a>>::OriginalFields {
        self.0.fields.keys()
    }

    #[inline]
    fn original_methods(&'a self) -> <Self as IterableMappings<'a>>::OriginalMethods {
        self.0.methods.keys()
    }

    #[inline]
    fn classes(&'a self) -> Self::Classes {
        self.0.classes.iter()
    }

    #[inline]
    fn fields(&'a self) -> Self::Fields {
        self.0.fields.iter()
    }

    #[inline]
    fn methods(&'a self) -> Self::Methods {
        self.0.methods.iter()
    }
}
