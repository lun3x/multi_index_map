#![allow(deprecated)]

use multi_index_map::{MultiIndexMap2, MultiIndexSelector};
use std::hash::Hash;

#[derive(Debug, Eq, MultiIndexMap2, PartialEq)]
struct LegacyOrder {
    #[multi_index(hashed_unique)]
    id: u64,
    #[multi_index(ordered_unique)]
    timestamp: u64,
    #[multi_index(hashed_non_unique)]
    trader: String,
    #[multi_index(ordered_non_unique)]
    price: u64,
    note: String,
    filled: bool,
}

fn order(id: u64, timestamp: u64, trader: &str, price: u64) -> LegacyOrder {
    LegacyOrder {
        id,
        timestamp,
        trader: trader.to_owned(),
        price,
        note: String::new(),
        filled: false,
    }
}

#[derive(MultiIndexSelector)]
#[multi_index(hashed_unique)]
struct ByHybridId;

#[derive(Debug, Eq, MultiIndexMap2, PartialEq)]
struct HybridRecord {
    #[multi_index(by(ByHybridId))]
    id: u64,
    #[multi_index(hashed_non_unique)]
    group: String,
    value: u64,
}

#[allow(non_camel_case_types)]
#[derive(MultiIndexSelector)]
#[multi_index(hashed_non_unique)]
struct hashed_non_unique;

#[derive(Debug, Eq, MultiIndexMap2, PartialEq)]
struct CategoryNamedSelector {
    #[multi_index(by(hashed_non_unique))]
    key: String,
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct NonCloneKey(u64);

#[derive(Debug, MultiIndexMap2)]
struct GenericLegacy<K: Eq + Hash, T> {
    #[multi_index(hashed_unique)]
    key: K,
    value: T,
}

#[test]
#[allow(deprecated)]
fn legacy_only_indexes_support_all_compatibility_operations() {
    let mut orders = MultiIndexLegacyOrderMap::new();
    orders.insert(order(1, 10, "Ada", 100));
    orders.insert(order(2, 20, "Ada", 90));
    orders.insert(order(3, 30, "Grace", 100));

    assert_eq!(orders.get_by_id(&2).unwrap().timestamp, 20);
    assert_eq!(orders.get_by_trader("Ada").len(), 2);
    assert_eq!(
        orders
            .iter_by_timestamp()
            .map(|order| order.id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        orders
            .iter_by_price()
            .rev()
            .map(|order| order.price)
            .collect::<Vec<_>>(),
        vec![100, 100, 90]
    );

    for (note, filled) in orders.get_mut_by_trader(&String::from("Ada")) {
        note.push_str("legacy");
        *filled = true;
    }
    for (note, filled) in orders.iter_mut().rev() {
        note.push_str(" slab");
        *filled = true;
    }
    assert_eq!(
        orders
            .update_by_trader("Ada", |note, _| note.push('!'))
            .len(),
        2
    );
    assert_eq!(
        orders
            .modify_by_price(&100, |order| order.price += order.id)
            .len(),
        2
    );
    assert_eq!(orders.get_by_price(&101).len(), 1);
    assert_eq!(orders.get_by_price(&103).len(), 1);

    assert_eq!(orders.remove_by_timestamp(&20).unwrap().id, 2);
    assert_eq!(orders.remove_by_trader(&String::from("Grace")).len(), 1);
    assert_eq!(orders.remove_by_id(&1).unwrap().id, 1);
    assert!(orders.is_empty());
    orders.validate().unwrap();
}

#[test]
#[allow(deprecated)]
fn legacy_conflicts_report_field_names_and_cleanup_modifications() {
    let mut orders = MultiIndexLegacyOrderMap::new();
    orders.insert(order(1, 10, "Ada", 100));
    let conflict = orders.try_insert(order(1, 20, "Grace", 90)).unwrap_err();
    assert_eq!(conflict.index, "id");
    let conflict = orders.try_insert(order(9, 10, "Grace", 90)).unwrap_err();
    assert_eq!(conflict.index, "timestamp");

    orders.insert(order(2, 20, "Grace", 90));
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        orders.modify_by_id(&1, |order| order.timestamp = 20);
    }));
    assert!(panic.is_err());
    assert!(orders.get_by_id(&1).is_none());
    assert!(orders.get_by_id(&2).is_some());
    orders.validate().unwrap();
    orders.clear();
    assert!(orders.is_empty());
}

#[test]
#[allow(deprecated)]
fn removal_through_every_legacy_category_unlinks_all_indexes() {
    let mut by_id = MultiIndexLegacyOrderMap::new();
    by_id.insert(order(1, 10, "Ada", 100));
    assert_eq!(by_id.remove_by_id(&1).unwrap().id, 1);
    by_id.validate().unwrap();

    let mut by_timestamp = MultiIndexLegacyOrderMap::new();
    by_timestamp.insert(order(1, 10, "Ada", 100));
    assert_eq!(by_timestamp.remove_by_timestamp(&10).unwrap().id, 1);
    by_timestamp.validate().unwrap();

    let mut by_trader = MultiIndexLegacyOrderMap::new();
    by_trader.insert(order(1, 10, "Ada", 100));
    assert_eq!(by_trader.remove_by_trader(&String::from("Ada")).len(), 1);
    by_trader.validate().unwrap();

    let mut by_price = MultiIndexLegacyOrderMap::new();
    by_price.insert(order(1, 10, "Ada", 100));
    assert_eq!(by_price.remove_by_price(&100).len(), 1);
    by_price.validate().unwrap();
}

#[test]
#[allow(deprecated)]
fn hybrid_maps_expose_selectors_and_legacy_wrappers() {
    let mut map = MultiIndexHybridRecordMap::new();
    map.insert(HybridRecord {
        id: 1,
        group: "A".to_owned(),
        value: 10,
    });
    map.insert(HybridRecord {
        id: 2,
        group: "A".to_owned(),
        value: 20,
    });

    assert_eq!(map.by::<ByHybridId>().get(&1).unwrap().value, 10);
    assert_eq!(map.get_by_group("A").len(), 2);
    map.by_mut::<ByHybridId>()
        .modify(&1, |record| record.group = "B".to_owned())
        .unwrap();
    assert_eq!(map.get_by_group("A").len(), 1);
    assert_eq!(map.get_by_group("B").len(), 1);
    assert_eq!(
        map.iter_mut().map(|(value,)| *value).collect::<Vec<_>>(),
        vec![10, 20]
    );
    map.validate().unwrap();
}

#[test]
fn category_name_inside_by_selects_a_user_type() {
    let mut map = MultiIndexCategoryNamedSelectorMap::new();
    map.insert(CategoryNamedSelector {
        key: "key".to_owned(),
    });
    assert_eq!(map.by::<hashed_non_unique>().equal_range("key").count(), 1);
}

#[test]
#[allow(deprecated)]
fn generic_non_clone_legacy_keys_work() {
    let mut map = MultiIndexGenericLegacyMap::new();
    map.insert(GenericLegacy {
        key: NonCloneKey(7),
        value: vec![1_u8],
    });
    map.update_by_key(&NonCloneKey(7), |value| value.push(2));
    assert_eq!(map.get_by_key(&NonCloneKey(7)).unwrap().value, vec![1, 2]);
    assert_eq!(
        map.remove_by_key(&NonCloneKey(7)).unwrap().value,
        vec![1, 2]
    );
}
