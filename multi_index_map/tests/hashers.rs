use multi_index_map::MultiIndexMap;
use std::any::type_name;
use std::collections::hash_map::DefaultHasher;
use std::hash::{BuildHasherDefault, Hash, Hasher};

#[derive(Clone, PartialEq, Eq)]
struct Key {
    value: u32,
    expected_hasher: &'static str,
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        assert_eq!(type_name::<H>(), self.expected_hasher);
        self.value.hash(state);
    }
}

#[derive(MultiIndexMap)]
struct DefaultElement {
    #[multi_index(hashed_unique)]
    id: Key,
    #[multi_index(hashed_non_unique)]
    group: Key,
}

#[test]
fn default_indexes_use_standard_library_hasher() {
    for mut map in [
        MultiIndexDefaultElementMap::default(),
        MultiIndexDefaultElementMap::with_capacity(2),
    ] {
        let key = |value| Key {
            value,
            expected_hasher: type_name::<DefaultHasher>(),
        };
        for id in [1, 2] {
            map.insert(DefaultElement {
                id: key(id),
                group: key(10),
            });
        }

        assert_eq!(map.get_by_id(&key(1)).unwrap().id.value, 1);
        assert_eq!(map.get_by_group(&key(10)).len(), 2);
        assert_eq!(map.remove_by_group(&key(10)).len(), 2);
        assert!(map.is_empty());
    }
}

#[derive(Default)]
struct CollisionHasher;

impl Hasher for CollisionHasher {
    fn finish(&self) -> u64 {
        0
    }

    fn write(&mut self, _bytes: &[u8]) {}
}

type CustomBuildHasher = BuildHasherDefault<CollisionHasher>;

#[derive(MultiIndexMap)]
#[multi_index_hash(CustomBuildHasher)]
struct CustomElement {
    #[multi_index(hashed_unique)]
    id: Key,
    #[multi_index(hashed_non_unique)]
    group: Key,
}

#[test]
fn custom_hasher_is_used_for_unique_and_non_unique_indexes() {
    for mut map in [
        MultiIndexCustomElementMap::default(),
        MultiIndexCustomElementMap::with_capacity(3),
    ] {
        let key = |value| Key {
            value,
            expected_hasher: type_name::<CollisionHasher>(),
        };
        for (id, group) in [(1, 10), (2, 10), (3, 20)] {
            map.insert(CustomElement {
                id: key(id),
                group: key(group),
            });
        }

        assert_eq!(map.get_by_id(&key(2)).unwrap().group.value, 10);
        assert_eq!(map.get_by_group(&key(10)).len(), 2);
        assert_eq!(map.get_by_group(&key(20)).len(), 1);
        assert!(map
            .try_insert(CustomElement {
                id: key(1),
                group: key(30),
            })
            .is_err());
        assert!(map.get_by_group(&key(30)).is_empty());
        assert_eq!(map.remove_by_group(&key(10)).len(), 2);
        assert_eq!(map.get_by_id(&key(3)).unwrap().group.value, 20);
        assert_eq!(map.len(), 1);
    }
}
