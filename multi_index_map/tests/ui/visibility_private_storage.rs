use multi_index_map::MultiIndexMap;

#[derive(MultiIndexMap, Debug)]
struct Element {
    #[multi_index(hashed_unique)]
    hashed_unique: u32,
    #[multi_index(hashed_non_unique)]
    hashed_non_unique: u32,
    #[multi_index(ordered_unique)]
    ordered_unique: u32,
    #[multi_index(ordered_non_unique)]
    ordered_non_unique: u32,
    value: bool,
}

// Generated internals must be inaccessible even in the element's own module.
fn main() {
    let mut map = MultiIndexElementMap::default();
    let _ = &map._store;
    let _ = &map._hashed_unique_index;
    let _ = &map._hashed_non_unique_index;
    let _ = &map._ordered_unique_index;
    let _ = &map._ordered_non_unique_index;

    let iter = map.iter_by_hashed_unique();
    let _ = &iter._store_ref;
    let _ = &iter._iter;
    let _ = &iter._inner_iter;
    drop(iter);
    let iter = map.iter_by_hashed_non_unique();
    let _ = &iter._store_ref;
    let _ = &iter._iter;
    let _ = &iter._inner_iter;
    drop(iter);

    let iter = map.iter_by_ordered_unique();
    let _ = &iter._store_ref;
    let _ = &iter._iter;
    let _ = &iter._iter_rev;
    let _ = &iter._inner_iter;
    drop(iter);
    let iter = map.iter_by_ordered_non_unique();
    let _ = &iter._store_ref;
    let _ = &iter._iter;
    let _ = &iter._iter_rev;
    let _ = &iter._inner_iter;
    drop(iter);

    let iter = map.iter_mut();
    let _ = &iter.0;
}
