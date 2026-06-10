use multi_index_map::{MultiIndexAccessor, MultiIndexMap2};
use std::fmt::Debug;
use std::hash::Hash;

#[derive(MultiIndexAccessor)]
#[multi_index(hashed_unique)]
struct ByHashed;

#[derive(MultiIndexAccessor)]
#[multi_index(ordered_unique)]
struct ByOrdered;

#[derive(MultiIndexAccessor)]
#[multi_index(hashed_non_unique)]
struct ByGroup;

#[derive(MultiIndexAccessor)]
#[multi_index(ordered_non_unique)]
struct ByRank;

#[derive(Debug, MultiIndexMap2)]
struct GenericRecord<'a, H: Eq + Hash = String, O: Eq + Ord = u64, T = (), const N: usize = 4>
where
    T: std::fmt::Debug,
{
    #[multi_index(ByHashed)]
    hashed: H,
    #[multi_index(ByOrdered)]
    ordered: O,
    #[multi_index(ByGroup)]
    group: H,
    #[multi_index(ByRank)]
    rank: O,
    payload: T,
    borrowed: &'a str,
    bytes: [u8; N],
}

#[derive(MultiIndexAccessor)]
#[multi_index(hashed_unique)]
struct ByCompound;

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct NoClone(u64);

#[derive(Debug, MultiIndexMap2)]
struct GenericCompound<A: Eq + Hash, B: Eq + Hash, T>
where
    T: Debug,
{
    #[multi_index(ByCompound)]
    first: A,
    #[multi_index(ByCompound)]
    second: B,
    payload: T,
}

#[derive(MultiIndexAccessor)]
#[multi_index(hashed_unique)]
struct ByOwnedKey;

#[derive(Debug, MultiIndexMap2)]
struct OwnedGeneric<K: Eq + Hash, T>
where
    T: Debug,
{
    #[multi_index(ByOwnedKey)]
    key: K,
    value: T,
}

#[allow(non_camel_case_types)]
#[derive(Debug, MultiIndexMap2)]
struct CollisionNames<
    '__mim_view,
    __MimKind: Eq + Hash,
    __MimQuery,
    __MimRange,
    __MimIter,
    const COLLISION_N: usize,
> where
    __MimQuery: Debug,
    __MimRange: Debug,
    __MimIter: Debug,
{
    #[multi_index(ByOwnedKey)]
    key: __MimKind,
    query: __MimQuery,
    range: __MimRange,
    iter: __MimIter,
    borrowed: &'__mim_view str,
    bytes: [u8; COLLISION_N],
}

pub trait RootBound: Debug {}

#[derive(Debug)]
pub struct RootPayload;

impl RootBound for RootPayload {}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct RootKey(u64);

impl RootBound for RootKey {}

mod generic_paths {
    use multi_index_map::{MultiIndexAccessor, MultiIndexMap2};

    pub trait LocalBound: crate::RootBound + Eq + std::hash::Hash {}

    impl LocalBound for crate::RootKey {}

    pub type LocalDefault = crate::RootKey;

    #[derive(MultiIndexAccessor)]
    #[multi_index(hashed_unique)]
    pub struct ByNestedKey;

    #[derive(Debug, MultiIndexMap2)]
    pub struct Nested<
        K: self::LocalBound = self::LocalDefault,
        T: super::RootBound = crate::RootPayload,
    >
    where
        K: crate::RootBound,
    {
        #[multi_index(self::ByNestedKey)]
        pub key: K,
        pub payload: T,
    }
}

#[test]
fn supports_lifetime_type_const_defaults_and_where_clauses() {
    let text = String::from("borrowed");
    let mut map: MultiIndexGenericRecordMap<'_> = Default::default();
    map.insert(GenericRecord {
        hashed: "id".to_owned(),
        ordered: 7,
        group: "group".to_owned(),
        rank: 9,
        payload: (),
        borrowed: &text,
        bytes: [1, 2, 3, 4],
    });

    assert_eq!(map.by::<ByHashed>().get("id").unwrap().borrowed, "borrowed");
    assert_eq!(map.by::<ByOrdered>().get(&7).unwrap().bytes, [1, 2, 3, 4]);
    assert_eq!(map.by::<ByGroup>().equal_range("group").count(), 1);
    assert_eq!(map.by::<ByRank>().range(9..=9).count(), 1);

    map.by_mut::<ByHashed>()
        .modify("id", |record| record.bytes[0] = 8)
        .unwrap();
    map.by_mut::<ByOrdered>()
        .modify(&7, |record| record.rank = 10)
        .unwrap();
    assert_eq!(map.by::<ByRank>().range(10..=10).count(), 1);
    map.validate().unwrap();
    map.clear();
    assert!(map.is_empty());
}

#[test]
fn supports_generic_compound_non_clone_keys() {
    let mut map = MultiIndexGenericCompoundMap::default();
    map.insert(GenericCompound {
        first: NoClone(1),
        second: NoClone(2),
        payload: "payload",
    });

    assert_eq!(
        map.by::<ByCompound>()
            .get((&NoClone(1), &NoClone(2)))
            .unwrap()
            .payload,
        "payload"
    );
    map.validate().unwrap();
}

#[test]
#[allow(deprecated)]
fn supports_generic_update_proxies_and_compatibility_methods() {
    let mut map = MultiIndexOwnedGenericMap::default();
    map.insert(OwnedGeneric {
        key: String::from("key"),
        value: vec![1_u8],
    });

    map.by_mut::<ByOwnedKey>()
        .update("key", |fields| fields.value.push(2));
    map.update_by_key("key", |value| value.push(3));
    assert_eq!(map.get_by_key("key").unwrap().value, vec![1, 2, 3]);

    map.modify_by_key(&String::from("key"), |record| record.value.push(4))
        .unwrap();
    assert_eq!(map.iter_by_key().next().unwrap().value, vec![1, 2, 3, 4]);
    assert_eq!(
        map.remove_by_key(&String::from("key")).unwrap().value,
        vec![1, 2, 3, 4]
    );
}

#[test]
fn avoids_collisions_with_generated_generic_names() {
    let text = String::from("borrowed");
    let mut map = MultiIndexCollisionNamesMap::default();
    map.insert(CollisionNames {
        key: String::from("key"),
        query: 1_u8,
        range: 2_u16,
        iter: 3_u32,
        borrowed: &text,
        bytes: [0_u8; 3],
    });

    let value = map.by::<ByOwnedKey>().get("key").unwrap();
    assert_eq!(value.query, 1);
    assert_eq!(value.range, 2);
    assert_eq!(value.iter, 3);
    assert_eq!(value.borrowed, "borrowed");
    assert_eq!(value.bytes.len(), 3);
}

#[test]
fn rebases_generic_bounds_defaults_and_where_clause_paths() {
    let mut map: generic_paths::MultiIndexNestedMap = Default::default();
    map.insert(generic_paths::Nested {
        key: RootKey(9),
        payload: RootPayload,
    });
    assert_eq!(
        map.by::<generic_paths::ByNestedKey>()
            .get(&RootKey(9))
            .unwrap()
            .key,
        RootKey(9)
    );
}
