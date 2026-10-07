#![deny(unused_imports)]

use multi_index_map::MultiIndexMap;

type RootKey = u32;
const ROOT_OFFSET: usize = 1;
trait RootBounds: Clone + Eq + Ord + std::hash::Hash {}
impl RootBounds for u32 {}

mod elements {
    use super::MultiIndexMap;

    type Key = u32;
    type Value = String;
    type LocalHasher = std::collections::hash_map::RandomState;
    const ARRAY_LENGTH: usize = 2;
    pub(super) trait Bounds: Clone + Eq + Ord + std::hash::Hash {}
    impl Bounds for u32 {}

    #[derive(MultiIndexMap, Debug, Clone)]
    #[multi_index_derive(Debug, Clone)]
    #[multi_index_hash(self::LocalHasher)]
    pub(super) struct Element<T: self::Bounds + super::RootBounds + std::fmt::Debug> {
        #[multi_index(hashed_unique)]
        pub(in self) hashed_unique: self::Key,
        #[multi_index(hashed_non_unique)]
        hashed_non_unique: (self::Key, super::RootKey),
        #[multi_index(ordered_unique)]
        pub(in super) ordered_unique: super::RootKey,
        #[multi_index(ordered_non_unique)]
        ordered_non_unique: [self::Key; self::ARRAY_LENGTH],
        #[multi_index(ordered_unique)]
        nested_const: [u8; {
            mod sizes {
                const N: usize = 1;
                mod inner {
                    pub(super) const LEN: usize = super::N + super::super::ARRAY_LENGTH
                        - super::super::super::ROOT_OFFSET;
                }
                pub(super) const LEN: usize = self::inner::LEN;
            }
            sizes::LEN
        }],
        value: self::Value,
        generic_value: T,
        root_value: super::RootKey,
    }

    pub(super) fn check() {
        let mut map = MultiIndexElementMap::default();
        map.insert(Element {
            hashed_unique: 1,
            hashed_non_unique: (2, 3),
            ordered_unique: 4,
            ordered_non_unique: [5, 6],
            nested_const: [7, 8],
            value: String::from("value"),
            generic_value: 7u32,
            root_value: 8,
        });
        assert_eq!(map.get_by_hashed_unique(&1).unwrap().ordered_unique, 4);
        assert_eq!(map.iter_by_hashed_non_unique().count(), 1);
        assert_eq!(map.get_by_ordered_unique(&4).unwrap().hashed_unique, 1);
        assert_eq!(map.iter_by_ordered_non_unique().count(), 1);
        assert_eq!(map.get_by_nested_const(&[7, 8]).unwrap().hashed_unique, 1);
        for (value, generic_value, root_value) in map.iter_mut() {
            value.push('!');
            *generic_value += 1;
            *root_value += 1;
        }
        let cloned = map.clone();
        let element = cloned.get_by_hashed_unique(&1).unwrap();
        assert_eq!(element.value, "value!");
        assert_eq!(element.generic_value, 8);
        assert_eq!(element.root_value, 9);
        let _ = format!("{cloned:?}");
    }
}

fn main() {
    elements::check();
}
