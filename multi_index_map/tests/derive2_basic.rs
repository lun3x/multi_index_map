use multi_index_map::{
    IndexView, IndexViewMut, MultiIndexMap2, MultiIndexSelector, NonUniqueView, NonUniqueViewMut,
    OrderedView, UniqueView, UniqueViewMut,
};
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(MultiIndexSelector)]
#[multi_index(hashed_unique)]
struct ById;

#[derive(MultiIndexSelector)]
#[multi_index(ordered_unique)]
struct ByTimestamp;

#[derive(MultiIndexSelector)]
#[multi_index(hashed_non_unique)]
struct ByTrader;

#[derive(MultiIndexSelector)]
#[multi_index(ordered_non_unique)]
struct ByPrice;

#[derive(MultiIndexSelector)]
#[multi_index(ordered_non_unique)]
struct ByTraderTimestamp;

#[derive(Debug, Eq, MultiIndexMap2, PartialEq)]
struct Order {
    #[multi_index(by(ById))]
    id: u64,
    #[multi_index(by(ByTrader), by(ByTraderTimestamp))]
    trader: String,
    #[multi_index(by(ByTimestamp), by(ByTraderTimestamp))]
    timestamp: u64,
    #[multi_index(by(ByPrice))]
    price: u64,
    note: String,
    filled: bool,
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct NonCloneKey(u64);

#[derive(MultiIndexSelector)]
#[multi_index(hashed_unique)]
struct ByNonCloneKey;

#[allow(dead_code)]
#[derive(Debug, Eq, MultiIndexMap2, PartialEq)]
struct NonCloneRecord {
    #[multi_index(by(ByNonCloneKey))]
    key: NonCloneKey,
    #[allow(dead_code)]
    payload: String,
}

#[derive(MultiIndexSelector)]
#[multi_index(ordered_unique)]
struct ByNoExtrasKey;

#[derive(Debug, Eq, MultiIndexMap2, PartialEq)]
struct NoExtras {
    #[multi_index(by(ByNoExtrasKey))]
    key: u64,
}

#[derive(MultiIndexSelector)]
#[multi_index(hashed_non_unique)]
struct ByGroup;

#[allow(dead_code)]
#[derive(Debug, Eq, MultiIndexMap2, PartialEq)]
struct OtherRecord {
    #[multi_index(by(ByGroup))]
    group: u8,
    #[allow(dead_code)]
    value: u64,
}

#[derive(MultiIndexSelector)]
#[multi_index(ordered_non_unique)]
struct ByName;

#[derive(Debug, Eq, MultiIndexMap2, PartialEq)]
struct OrderedName {
    #[multi_index(by(ByName))]
    name: String,
    value: u64,
}

#[derive(MultiIndexSelector)]
#[multi_index(hashed_unique)]
struct ByHashedUniquePair;

#[derive(MultiIndexSelector)]
#[multi_index(hashed_non_unique)]
struct ByHashedPair;

#[derive(MultiIndexSelector)]
#[multi_index(ordered_unique)]
struct ByOrderedUniquePair;

#[allow(dead_code)]
#[derive(Debug, Eq, MultiIndexMap2, PartialEq)]
struct CompoundKinds {
    #[multi_index(by(ByHashedUniquePair))]
    hu_name: String,
    #[multi_index(by(ByHashedUniquePair))]
    hu_number: NonCloneKey,
    #[multi_index(by(ByHashedPair))]
    h_name: String,
    #[multi_index(by(ByHashedPair))]
    h_number: u64,
    #[multi_index(by(ByOrderedUniquePair))]
    ou_name: String,
    #[multi_index(by(ByOrderedUniquePair))]
    ou_number: u64,
    #[allow(dead_code)]
    payload: u64,
}

#[derive(Debug, Eq, MultiIndexMap2, PartialEq)]
struct ReusedSelector {
    #[multi_index(by(ById))]
    id: u64,
}

mod exposed {
    use multi_index_map::{MultiIndexMap2, MultiIndexSelector};

    #[derive(MultiIndexSelector)]
    #[multi_index(hashed_unique)]
    pub struct ByPublicId;

    #[allow(dead_code)]
    #[derive(Debug, Eq, MultiIndexMap2, PartialEq)]
    pub struct PublicRecord {
        #[multi_index(by(ByPublicId))]
        pub id: u64,
        pub note: String,
        #[allow(dead_code)]
        hidden: String,
    }

    pub fn record(id: u64) -> PublicRecord {
        PublicRecord {
            id,
            note: String::new(),
            hidden: "hidden".to_owned(),
        }
    }

    pub fn hidden(record: &PublicRecord) -> &str {
        &record.hidden
    }
}

mod rebased_paths {
    use multi_index_map::{MultiIndexMap2, MultiIndexSelector};

    pub(super) type LocalKey = u64;

    #[derive(MultiIndexSelector)]
    #[multi_index(hashed_unique)]
    pub(super) struct ByLocal;

    #[derive(Debug, Eq, MultiIndexMap2, PartialEq)]
    pub(super) struct Record {
        #[multi_index(by(self::ByLocal))]
        pub(super) local: self::LocalKey,
        #[multi_index(by(super::ById))]
        pub(crate) outer: u64,
        pub(super) payload: super::NonCloneKey,
    }

    pub(super) fn record(local: u64, outer: u64) -> Record {
        Record {
            local,
            outer,
            payload: super::NonCloneKey(0),
        }
    }
}

fn order(id: u64, timestamp: u64, trader: &str, price: u64) -> Order {
    Order {
        id,
        timestamp,
        trader: trader.to_owned(),
        price,
        note: String::new(),
        filled: false,
    }
}

#[test]
fn generates_the_four_index_categories() {
    let mut orders = MultiIndexOrderMap::new();
    orders.try_insert(order(1, 10, "Ada", 100)).unwrap();
    orders.try_insert(order(2, 20, "Ada", 90)).unwrap();
    orders.try_insert(order(3, 30, "Grace", 100)).unwrap();

    assert_eq!(orders.by::<ById>().get(&2).unwrap().timestamp, 20);
    assert!(orders.by::<ById>().contains_key(&1));
    assert_eq!(orders.by::<ByTrader>().equal_range("Ada").count(), 2);
    assert_eq!(
        orders
            .by::<ByTimestamp>()
            .range(10..=20)
            .map(|order| order.id)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(
        orders
            .by::<ByPrice>()
            .iter()
            .rev()
            .map(|order| order.price)
            .collect::<Vec<_>>(),
        vec![100, 100, 90]
    );

    orders
        .by_mut::<ById>()
        .modify(&1, |order| order.price = 80)
        .unwrap();
    orders
        .by_mut::<ByTrader>()
        .update_all("Ada", |fields| *fields.filled = true);
    assert_eq!(orders.by::<ByPrice>().equal_range(&80).count(), 1);
    assert!(orders.by::<ById>().get(&1).unwrap().filled);
    orders.validate().unwrap();

    fn assert_unique<V: UniqueView<Value = Order, Key = u64>>(view: &V) {
        assert!(view.get(&1).is_some());
    }
    fn assert_non_unique<V: NonUniqueView<Value = Order, Key = String>>(view: &V) {
        assert_eq!(view.equal_range(&"Ada".to_owned()).count(), 2);
    }
    fn assert_ordered<V: OrderedView<Value = Order, Key = u64>>(view: &V) {
        assert!(view.range(..).next().is_some());
    }
    fn assert_index<V: IndexView<Value = Order>>(view: &V) {
        assert_eq!(view.len(), 3);
    }

    assert_unique(&orders.by::<ById>());
    assert_non_unique(&orders.by::<ByTrader>());
    assert_ordered(&orders.by::<ByTimestamp>());
    assert_index(&orders.by::<ByPrice>());
}

#[test]
fn mutations_are_coordinated() {
    let mut orders = MultiIndexOrderMap::new();
    orders.insert(order(1, 10, "Ada", 100));
    orders.insert(order(2, 20, "Ada", 100));

    let conflict = orders
        .by_mut::<ById>()
        .replace(&1, order(2, 30, "Grace", 90))
        .unwrap_err();
    assert_eq!(conflict.index, "ById");
    assert_eq!(orders.by::<ById>().get(&1).unwrap().timestamp, 10);

    let conflict = orders
        .by_mut::<ById>()
        .modify(&1, |order| order.timestamp = 20)
        .unwrap_err();
    assert_eq!(conflict.index, "ByTimestamp");
    assert!(!orders.by::<ById>().contains_key(&1));

    assert_eq!(orders.by_mut::<ByPrice>().remove_all(&100).len(), 1);
    assert!(orders.is_empty());
    orders.validate().unwrap();
}

#[test]
fn removal_through_each_generated_index_unlinks_every_index() {
    let mut orders = MultiIndexOrderMap::new();
    orders.insert(order(1, 10, "A", 10));
    orders.insert(order(2, 20, "B", 20));
    orders.insert(order(3, 30, "C", 30));
    orders.insert(order(4, 40, "D", 40));

    assert_eq!(orders.by_mut::<ById>().remove(&1).unwrap().timestamp, 10);
    assert_eq!(orders.by_mut::<ByTimestamp>().remove(&20).unwrap().id, 2);
    assert_eq!(orders.by_mut::<ByTrader>().remove_all("C")[0].id, 3);
    assert_eq!(orders.by_mut::<ByPrice>().remove_all(&40)[0].id, 4);
    assert!(orders.is_empty());
    orders.validate().unwrap();
}

#[test]
fn capability_mutation_traits_work() {
    let mut orders = MultiIndexOrderMap::new();
    orders.insert(order(1, 10, "Ada", 100));
    {
        let mut view = orders.by_mut::<ById>();
        UniqueViewMut::update(&mut view, &1, |fields| fields.note.push_str("done"));
    }
    {
        let mut view = orders.by_mut::<ByTrader>();
        assert_eq!(
            NonUniqueViewMut::modify_all(&mut view, &"Ada".to_owned(), |_| {}).modified,
            1
        );
    }
    {
        let mut view = orders.by_mut::<ByTimestamp>();
        assert_eq!(
            IndexViewMut::update_each(&mut view, |fields| fields.note.push('!')),
            1
        );
    }
}

#[test]
fn update_each_visits_every_element_through_the_selected_index() {
    let mut orders = MultiIndexOrderMap::new();
    orders.insert(order(1, 30, "Grace", 100));
    orders.insert(order(2, 10, "Ada", 90));
    orders.insert(order(3, 20, "Ada", 100));

    let mut sequence = 0;
    assert_eq!(
        orders.by_mut::<ByTimestamp>().update_each(|fields| {
            *fields.note = sequence.to_string();
            sequence += 1;
        }),
        3
    );
    assert_eq!(
        orders
            .by::<ByTimestamp>()
            .iter()
            .map(|order| order.note.as_str())
            .collect::<Vec<_>>(),
        vec!["0", "1", "2"]
    );

    for update_count in [
        orders
            .by_mut::<ById>()
            .update_each(|fields| *fields.filled = true),
        orders
            .by_mut::<ByTrader>()
            .update_each(|fields| fields.note.push('h')),
        orders
            .by_mut::<ByPrice>()
            .update_each(|fields| fields.note.push('o')),
        orders
            .by_mut::<ByTraderTimestamp>()
            .update_each(|fields| fields.note.push('c')),
    ] {
        assert_eq!(update_count, 3);
    }
    assert!(orders.by::<ById>().iter().all(|order| order.filled));
    let mut notes = orders
        .by::<ById>()
        .iter()
        .map(|order| order.note.as_str())
        .collect::<Vec<_>>();
    notes.sort_unstable();
    assert_eq!(notes, vec!["0hoc", "1hoc", "2hoc"]);
    orders.validate().unwrap();
}

#[test]
fn update_each_handles_empty_zero_field_and_panicking_updates() {
    let mut empty = MultiIndexOrderMap::new();
    assert_eq!(empty.by_mut::<ById>().update_each(|_| unreachable!()), 0);

    let mut no_extras = MultiIndexNoExtrasMap::new();
    no_extras.insert(NoExtras { key: 1 });
    assert_eq!(no_extras.by_mut::<ByNoExtrasKey>().update_each(|_| {}), 1);
    no_extras.validate().unwrap();

    let mut orders = MultiIndexOrderMap::new();
    orders.insert(order(1, 10, "A", 1));
    orders.insert(order(2, 20, "B", 2));
    orders.insert(order(3, 30, "C", 3));
    let mut calls = 0;
    let panic = catch_unwind(AssertUnwindSafe(|| {
        orders.by_mut::<ByTimestamp>().update_each(|fields| {
            calls += 1;
            fields.note.push_str("visited");
            if calls == 2 {
                panic!("stop");
            }
        });
    }));
    assert!(panic.is_err());
    assert_eq!(calls, 2);
    assert_eq!(
        orders
            .by::<ByTimestamp>()
            .iter()
            .map(|order| order.note.as_str())
            .collect::<Vec<_>>(),
        vec!["visited", "visited", ""]
    );
    orders.validate().unwrap();
}

#[test]
fn panic_cleanup_batch_snapshots_and_slot_reuse_hold() {
    let mut orders = MultiIndexOrderMap::new();
    orders.insert(order(1, 10, "Ada", 100));
    orders.insert(order(2, 20, "Ada", 100));
    orders.insert(order(3, 30, "Grace", 90));

    let result = orders
        .by_mut::<ByTrader>()
        .modify_all("Ada", |order| order.trader = "Grace".to_owned());
    assert_eq!(result.modified, 2);
    assert!(result.removed.is_empty());
    assert_eq!(orders.by::<ByTrader>().equal_range("Ada").count(), 0);
    assert_eq!(orders.by::<ByTrader>().equal_range("Grace").count(), 3);

    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = orders.by_mut::<ById>().modify(&2, |order| {
            order.price = 999;
            panic!("modifier failed");
        });
    }));
    assert!(panic.is_err());
    assert!(!orders.by::<ById>().contains_key(&2));
    orders.validate().unwrap();

    assert!(orders.by_mut::<ById>().remove(&1).is_some());
    orders.insert(order(4, 40, "Ada", 80));
    orders.clear();
    assert!(orders.is_empty());
    orders.validate().unwrap();
}

#[test]
fn supports_non_clone_keys_zero_unindexed_fields_and_multiple_derives() {
    let mut records = MultiIndexNonCloneRecordMap::new();
    records.insert(NonCloneRecord {
        key: NonCloneKey(7),
        payload: "value".to_owned(),
    });
    assert_eq!(
        records
            .by::<ByNonCloneKey>()
            .get(&NonCloneKey(7))
            .unwrap()
            .payload,
        "value"
    );

    let mut no_extras = MultiIndexNoExtrasMap::new();
    no_extras.insert(NoExtras { key: 1 });
    assert!(no_extras
        .by_mut::<ByNoExtrasKey>()
        .update(&1, |_| {})
        .is_some());

    let mut others = MultiIndexOtherRecordMap::new();
    others.insert(OtherRecord { group: 1, value: 2 });
    assert_eq!(others.by::<ByGroup>().equal_range(&1).count(), 1);

    let mut names = MultiIndexOrderedNameMap::new();
    names.insert(OrderedName {
        name: "Ada".to_owned(),
        value: 1,
    });
    assert_eq!(
        names
            .by::<ByName>()
            .equal_range("Ada")
            .next()
            .unwrap()
            .value,
        1
    );
    names
        .by_mut::<ByName>()
        .update_all("Ada", |fields| *fields.value += 1);
    assert_eq!(names.by_mut::<ByName>().remove_all("Ada").len(), 1);
}

#[test]
fn supports_all_compound_categories_and_reused_selectors() {
    let mut map = MultiIndexCompoundKindsMap::new();
    map.insert(CompoundKinds {
        hu_name: "Ada".to_owned(),
        hu_number: NonCloneKey(1),
        h_name: "desk".to_owned(),
        h_number: 7,
        ou_name: "first".to_owned(),
        ou_number: 10,
        payload: 1,
    });
    map.insert(CompoundKinds {
        hu_name: "Grace".to_owned(),
        hu_number: NonCloneKey(2),
        h_name: "desk".to_owned(),
        h_number: 7,
        ou_name: "second".to_owned(),
        ou_number: 20,
        payload: 2,
    });

    assert_eq!(
        map.by::<ByHashedUniquePair>()
            .get(("Ada", &NonCloneKey(1)))
            .unwrap()
            .payload,
        1
    );
    assert_eq!(
        map.by::<ByHashedPair>().equal_range(("desk", &7)).count(),
        2
    );
    assert_eq!(
        map.by::<ByOrderedUniquePair>()
            .range(("first", &0)..=("second", &u64::MAX))
            .count(),
        2
    );
    fn compound_unique<V>(view: &V)
    where
        V: UniqueView<Value = CompoundKinds, Key = (String, NonCloneKey)>,
    {
        assert!(view.get(&("Ada".to_owned(), NonCloneKey(1))).is_some());
    }
    fn compound_non_unique<V>(view: &V)
    where
        V: NonUniqueView<Value = CompoundKinds, Key = (String, u64)>,
    {
        assert_eq!(view.equal_range(&("desk".to_owned(), 7)).count(), 2);
    }
    fn compound_ordered<V>(view: &V)
    where
        V: OrderedView<Value = CompoundKinds, Key = (String, u64)>,
    {
        assert_eq!(
            view.range(("first".to_owned(), 0)..=("second".to_owned(), u64::MAX))
                .count(),
            2
        );
    }
    compound_unique(&map.by::<ByHashedUniquePair>());
    compound_non_unique(&map.by::<ByHashedPair>());
    compound_ordered(&map.by::<ByOrderedUniquePair>());
    let conflict = map
        .try_insert(CompoundKinds {
            hu_name: "Ada".to_owned(),
            hu_number: NonCloneKey(1),
            h_name: "other".to_owned(),
            h_number: 9,
            ou_name: "third".to_owned(),
            ou_number: 30,
            payload: 3,
        })
        .unwrap_err();
    assert_eq!(conflict.index, "ByHashedUniquePair");
    map.validate().unwrap();

    let mut reused = MultiIndexReusedSelectorMap::new();
    reused.insert(ReusedSelector { id: 9 });
    assert_eq!(reused.by::<ById>().get(&9).unwrap().id, 9);
}

#[test]
fn generated_visibility_follows_the_source_struct_and_indexed_field() {
    let mut records = exposed::MultiIndexPublicRecordMap::new();
    records.insert(exposed::record(7));
    records
        .by_mut::<exposed::ByPublicId>()
        .update(&7, |fields| fields.note.push_str("visible"));
    let record = records.by::<exposed::ByPublicId>().get(&7).unwrap();
    assert_eq!(record.id, 7);
    assert_eq!(record.note, "visible");
    assert_eq!(exposed::hidden(record), "hidden");
    #[allow(deprecated)]
    {
        assert_eq!(records.get_by_id(&7).unwrap().id, 7);
    }
}

#[test]
fn generated_module_rebases_paths_and_restricted_visibility() {
    let mut records = rebased_paths::MultiIndexRecordMap::new();
    records.insert(rebased_paths::record(7, 9));
    assert_eq!(
        records
            .by::<rebased_paths::ByLocal>()
            .get(&7)
            .unwrap()
            .outer,
        9
    );
    assert_eq!(records.by::<ById>().get(&9).unwrap().local, 7);

    records
        .by_mut::<rebased_paths::ByLocal>()
        .update(&7, |fields| fields.payload.0 = 11);
    assert_eq!(
        records
            .by::<rebased_paths::ByLocal>()
            .get(&7)
            .unwrap()
            .payload
            .0,
        11
    );

    #[allow(deprecated)]
    {
        assert_eq!(records.get_by_local(&7).unwrap().outer, 9);
    }
}

#[test]
fn compound_index_supports_borrowed_lookup_ranges_and_relocation() {
    let mut orders = MultiIndexOrderMap::new();
    orders.insert(order(1, 10, "Ada", 100));
    orders.insert(order(2, 20, "Ada", 100));
    orders.insert(order(3, 30, "Grace", 90));

    assert_eq!(
        orders
            .by::<ByTraderTimestamp>()
            .equal_range(("Ada", &20))
            .next()
            .unwrap()
            .id,
        2
    );
    assert_eq!(
        orders
            .by::<ByTraderTimestamp>()
            .range(("Ada", &0)..=("Ada", &u64::MAX))
            .map(|order| order.id)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(
        orders
            .by::<ByTraderTimestamp>()
            .iter()
            .rev()
            .map(|order| order.id)
            .collect::<Vec<_>>(),
        vec![3, 2, 1]
    );

    orders
        .by_mut::<ById>()
        .modify(&1, |order| {
            order.trader = "Grace".to_owned();
            order.timestamp = 25;
        })
        .unwrap();
    assert_eq!(
        orders
            .by::<ByTraderTimestamp>()
            .equal_range(("Grace", &25))
            .count(),
        1
    );
    assert_eq!(
        orders
            .by_mut::<ByTraderTimestamp>()
            .remove_all(("Grace", &25))
            .len(),
        1
    );
    orders.validate().unwrap();
}

#[test]
fn modifying_through_a_view_removes_on_conflict() {
    let mut orders = MultiIndexOrderMap::new();
    orders.insert(order(1, 10, "Ada", 100));
    orders.insert(order(2, 20, "Grace", 90));

    let conflict = orders
        .by_mut::<ById>()
        .modify(&1, |order| order.timestamp = 20)
        .unwrap_err();
    assert_eq!(conflict.index, "ByTimestamp");
    assert!(!orders.by::<ById>().contains_key(&1));
    assert!(orders.by::<ById>().contains_key(&2));
    orders.validate().unwrap();
}

#[test]
#[allow(deprecated)]
fn compatibility_wrappers_remain_for_unambiguous_single_field_indexes() {
    let mut orders = MultiIndexOrderMap::new();
    orders.insert(order(1, 10, "Ada", 100));
    orders.insert(order(2, 20, "Ada", 100));

    assert_eq!(orders.get_by_id(&1).unwrap().timestamp, 10);
    assert_eq!(orders.get_by_trader("Ada").len(), 2);
    assert_eq!(orders.iter_by_timestamp().count(), 2);
    for (note, filled) in orders.get_mut_by_trader(&"Ada".to_owned()) {
        note.push_str("legacy");
        *filled = true;
    }
    assert_eq!(
        orders.update_by_price(&100, |note, _| note.push('!')).len(),
        2
    );
    assert_eq!(
        orders
            .modify_by_price(&100, |order| order.price += order.id)
            .len(),
        2
    );
    assert_eq!(orders.remove_by_trader(&"Ada".to_owned()).len(), 2);
    orders.validate().unwrap();
}

#[test]
fn deterministic_operations_match_a_simple_model() {
    let mut orders = MultiIndexOrderMap::new();
    let mut model = BTreeMap::<u64, (u64, String, u64, String, bool)>::new();
    let mut state = 0x4d59_5df4_d0f3_3173_u64;

    for step in 0..500 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let id = (state >> 16) % 32;
        let operation = state % 4;

        match operation {
            0 => {
                let timestamp = 100 + ((state >> 24) % 32);
                let trader = format!("T{}", (state >> 32) % 5);
                let price = 10 + ((state >> 40) % 7);
                let expected_conflict = if model.contains_key(&id) {
                    Some("ById")
                } else if model.values().any(|entry| entry.0 == timestamp) {
                    Some("ByTimestamp")
                } else {
                    None
                };
                let result = orders.try_insert(order(id, timestamp, &trader, price));
                match expected_conflict {
                    Some(index) => assert_eq!(result.unwrap_err().index, index),
                    None => {
                        result.unwrap();
                        model.insert(id, (timestamp, trader, price, String::new(), false));
                    }
                }
            }
            1 => {
                let actual = orders.by_mut::<ById>().remove(&id).map(|order| order.id);
                let expected = model.remove(&id).map(|_| id);
                assert_eq!(actual, expected);
            }
            2 => {
                let timestamp = 100 + ((state >> 24) % 32);
                let conflict = model
                    .iter()
                    .any(|(other_id, entry)| *other_id != id && entry.0 == timestamp);
                let mut view = orders.by_mut::<ById>();
                let result = view.modify(&id, |order| {
                    order.timestamp = timestamp;
                    order.price = 10 + ((state >> 40) % 7);
                });
                if let Some(entry) = model.get_mut(&id) {
                    if conflict {
                        assert_eq!(result.unwrap_err().index, "ByTimestamp");
                        model.remove(&id);
                    } else {
                        result.unwrap();
                        entry.0 = timestamp;
                        entry.2 = 10 + ((state >> 40) % 7);
                    }
                } else {
                    assert!(result.unwrap().is_none());
                }
            }
            _ => {
                let note = format!("step-{step}");
                let mut view = orders.by_mut::<ById>();
                let result = view.update(&id, |fields| {
                    *fields.note = note.clone();
                    *fields.filled = !*fields.filled;
                });
                if let Some(entry) = model.get_mut(&id) {
                    assert!(result.is_some());
                    entry.3 = note;
                    entry.4 = !entry.4;
                } else {
                    assert!(result.is_none());
                }
            }
        }

        orders.validate().unwrap();
        assert_eq!(orders.len(), model.len());

        let mut actual_ids = orders
            .by::<ById>()
            .iter()
            .map(|order| order.id)
            .collect::<Vec<_>>();
        actual_ids.sort_unstable();
        assert_eq!(actual_ids, model.keys().copied().collect::<Vec<_>>());

        let actual_timestamps = orders
            .by::<ByTimestamp>()
            .iter()
            .map(|order| order.timestamp)
            .collect::<Vec<_>>();
        let mut expected_timestamps = model.values().map(|entry| entry.0).collect::<Vec<_>>();
        expected_timestamps.sort_unstable();
        assert_eq!(actual_timestamps, expected_timestamps);

        let actual_prices = orders
            .by::<ByPrice>()
            .iter()
            .map(|order| order.price)
            .collect::<Vec<_>>();
        let mut expected_prices = model.values().map(|entry| entry.2).collect::<Vec<_>>();
        expected_prices.sort_unstable();
        assert_eq!(actual_prices, expected_prices);

        for trader in ["T0", "T1", "T2", "T3", "T4"] {
            let actual = orders.by::<ByTrader>().equal_range(trader).count();
            let expected = model.values().filter(|entry| entry.1 == trader).count();
            assert_eq!(actual, expected);
        }
    }
}
