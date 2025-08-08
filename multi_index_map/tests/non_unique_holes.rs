use multi_index_map::MultiIndexMap;

// Non-unique index traversal must work with holes in the slab (after removals).
// One compact struct and three focused tests cover modify_by_, update_by_, get_mut_by_.
#[derive(MultiIndexMap, Debug, Clone)]
#[multi_index_derive(Clone, Debug)]
struct Entry {
    // Non-unique index we query by
    #[multi_index(hashed_non_unique)]
    group: u32,
    // Unindexed payload used by update_by_/get_mut_by_
    value: i32,
    // Unique index to create deterministic holes
    #[multi_index(ordered_unique)]
    id: usize,
}

fn make_with_holes() -> MultiIndexEntryMap {
    let mut map = MultiIndexEntryMap::default();
    for id in 0..6usize {
        map.insert(Entry { group: 1, value: id as i32, id });
    }
    // Create holes at indices 0 and 1
    assert!(map.remove_by_id(&0).is_some());
    assert!(map.remove_by_id(&1).is_some());
    map
}

#[test]
fn non_unique_modify_after_holes() {
    let mut map = make_with_holes();

    // Move entries from group 1 to 2; ensure we traverse past holes
    let refs = map.modify_by_group(&1, |e| e.group = 2);

    assert_eq!(refs.len(), 4);
    for (i, e) in refs.iter().enumerate() {
        assert_eq!(e.group, 2);
        assert_eq!(e.id, i + 2);
    }

    assert!(map.get_by_group(&1).is_empty());
    let by_two = map.get_by_group(&2);
    assert_eq!(by_two.len(), 4);
    for (i, e) in by_two.iter().enumerate() {
        assert_eq!(e.group, 2);
        assert_eq!(e.id, i + 2);
    }
}

#[test]
fn non_unique_update_after_holes() {
    let mut map = make_with_holes();

    // Update unindexed field through the non-unique key
    let refs = map.update_by_group(&1, |value| *value += 10);

    assert_eq!(refs.len(), 4);
    for (i, e) in refs.iter().enumerate() {
        assert_eq!(e.group, 1);
        assert_eq!(e.id, i + 2);
        assert_eq!(e.value, (i as i32 + 2) + 10);
    }

    let by_one = map.get_by_group(&1);
    assert_eq!(by_one.len(), 4);
    for (i, e) in by_one.iter().enumerate() {
        assert_eq!(e.group, 1);
        assert_eq!(e.id, i + 2);
        assert_eq!(e.value, (i as i32 + 2) + 10);
    }

    // Absent key returns empty
    assert!(map.update_by_group(&999, |_| {}).is_empty());
}

#[test]
fn non_unique_get_mut_after_holes_aliasing_safe() {
    let mut map = make_with_holes();

    // Obtain multiple &mut safely via a single iter_mut()-driven traversal
    let mut_refs: Vec<(&mut i32,)> = map.get_mut_by_group(&1);
    assert_eq!(mut_refs.len(), 4);
    for (v,) in mut_refs.into_iter() {
        *v += 100;
    }

    let refs = map.get_by_group(&1);
    assert_eq!(refs.len(), 4);
    for (i, e) in refs.iter().enumerate() {
        assert_eq!(e.id, i + 2);
        assert_eq!(e.value, (i as i32 + 2) + 100);
    }

    // Absent key returns empty
    assert!(map.get_mut_by_group(&999).is_empty());
}
