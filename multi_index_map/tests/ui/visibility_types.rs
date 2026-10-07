use multi_index_map::MultiIndexMap;

// Field visibility must not make generated types narrower or wider than Element.
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

            $element_vis fn hashed_unique(map: &MultiIndexElementMap) -> MultiIndexElementMapHashedUniqueIter<'_> {
                map.iter_by_hashed_unique()
            }

            $element_vis fn hashed_non_unique(map: &MultiIndexElementMap) -> MultiIndexElementMapHashedNonUniqueIter<'_> {
                map.iter_by_hashed_non_unique()
            }

            $element_vis fn ordered_unique(map: &MultiIndexElementMap) -> MultiIndexElementMapOrderedUniqueIter<'_> {
                map.iter_by_ordered_unique()
            }

            $element_vis fn ordered_non_unique(map: &MultiIndexElementMap) -> MultiIndexElementMapOrderedNonUniqueIter<'_> {
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

elements!(public, pub,);
elements!(crate_visible, pub(crate), );
elements!(self_visible, pub(self), pub);
elements!(private, , pub);

mod scope {
    use super::MultiIndexMap;

    elements!(restricted, pub(in crate::scope), );
    elements!(restricted_public_fields, pub(in crate::scope), pub);
    elements!(parent_visible, pub(super), pub);

    pub(super) fn check() {
        check_access!(restricted);
        check_access!(restricted_public_fields);
        check_access!(parent_visible);
    }
}

// Re-exporting catches a pub(crate) iterator accidentally used for a pub element.
pub use public::{
    Element, ElementIterMut, MultiIndexElementMap, MultiIndexElementMapHashedNonUniqueIter,
    MultiIndexElementMapHashedUniqueIter, MultiIndexElementMapOrderedNonUniqueIter,
    MultiIndexElementMapOrderedUniqueIter,
};

fn main() {
    check_access!(public);
    check_access!(crate_visible);
    self_visible::check_private_types();
    private::check_private_types();
    scope::check();
}
