use multi_index_map::MultiIndexMap;

// Map types follow Element; index iterator types and methods follow each field.
macro_rules! elements {
    ($module:ident, $element_vis:vis, $($field_vis:tt)*) => {
        mod $module {
            use super::MultiIndexMap;

            #[derive(MultiIndexMap, Debug)]
            $element_vis struct Element {
                #[multi_index(hashed_unique)]
                $($field_vis)* hashed_unique: u32,
                #[multi_index(hashed_non_unique)]
                $($field_vis)* hashed_non_unique: u32,
                #[multi_index(ordered_unique)]
                $($field_vis)* ordered_unique: u32,
                #[multi_index(ordered_non_unique)]
                $($field_vis)* ordered_non_unique: u32,
                value: bool,
            }

            $element_vis fn map() -> MultiIndexElementMap {
                let mut map = MultiIndexElementMap::default();
                map.insert(Element {
                    hashed_unique: 1,
                    hashed_non_unique: 2,
                    ordered_unique: 3,
                    ordered_non_unique: 4,
                    value: false,
                });
                map
            }

            $($field_vis)* fn hashed_unique(map: &MultiIndexElementMap) -> MultiIndexElementMapHashedUniqueIter<'_> {
                map.iter_by_hashed_unique()
            }

            $($field_vis)* fn hashed_non_unique(map: &MultiIndexElementMap) -> MultiIndexElementMapHashedNonUniqueIter<'_> {
                map.iter_by_hashed_non_unique()
            }

            $($field_vis)* fn ordered_unique(map: &MultiIndexElementMap) -> MultiIndexElementMapOrderedUniqueIter<'_> {
                map.iter_by_ordered_unique()
            }

            $($field_vis)* fn ordered_non_unique(map: &MultiIndexElementMap) -> MultiIndexElementMapOrderedNonUniqueIter<'_> {
                map.iter_by_ordered_non_unique()
            }

            pub(super) fn check_private_types() {
                let mut map = map();
                assert_eq!(hashed_unique(&map).count(), 1);
                assert_eq!(hashed_non_unique(&map).count(), 1);
                assert_eq!(ordered_unique(&map).count(), 1);
                assert_eq!(ordered_non_unique(&map).count(), 1);
                let mut iter: ElementIterMut<'_> = map.iter_mut();
                assert!(iter.next().is_some());
            }
        }
    };
}

macro_rules! check_access {
    ($module:ident) => {{
        let mut map: $module::MultiIndexElementMap = $module::map();
        let iter: $module::MultiIndexElementMapHashedUniqueIter<'_> = $module::hashed_unique(&map);
        assert_eq!(iter.count(), 1);
        let iter: $module::MultiIndexElementMapHashedNonUniqueIter<'_> =
            $module::hashed_non_unique(&map);
        assert_eq!(iter.count(), 1);
        let iter: $module::MultiIndexElementMapOrderedUniqueIter<'_> =
            $module::ordered_unique(&map);
        assert_eq!(iter.count(), 1);
        let iter: $module::MultiIndexElementMapOrderedNonUniqueIter<'_> =
            $module::ordered_non_unique(&map);
        assert_eq!(iter.count(), 1);
        let mut iter: $module::ElementIterMut<'_> = map.iter_mut();
        assert!(iter.next().is_some());
    }};
}

elements!(public, pub, pub);
elements!(crate_visible, pub(crate), pub(crate));
elements!(public_private_fields, pub,);
elements!(self_visible, pub(self), pub(self));
elements!(private, , );

mod scope {
    use super::MultiIndexMap;

    elements!(restricted, pub, pub(in crate::scope));
    elements!(restricted_element, pub(in crate::scope), pub(in crate::scope));
    elements!(parent_visible, pub, pub(super));

    pub(super) fn check() {
        check_access!(restricted);
        check_access!(restricted_element);
        check_access!(parent_visible);
    }
}

// Public indexed fields must allow their iterator types to be publicly re-exported.
pub use public::{
    Element, ElementIterMut, MultiIndexElementMap, MultiIndexElementMapHashedNonUniqueIter,
    MultiIndexElementMapHashedUniqueIter, MultiIndexElementMapOrderedNonUniqueIter,
    MultiIndexElementMapOrderedUniqueIter,
};

fn main() {
    check_access!(public);
    check_access!(crate_visible);
    let mut map: public_private_fields::MultiIndexElementMap = public_private_fields::map();
    let mut iter: public_private_fields::ElementIterMut<'_> = map.iter_mut();
    assert!(iter.next().is_some());
    public_private_fields::check_private_types();
    self_visible::check_private_types();
    private::check_private_types();
    scope::check();
}
