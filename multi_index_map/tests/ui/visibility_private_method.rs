// The map and mutable iterator follow the public element's visibility.
// Keyed methods and per-index iterators follow the private indexed field and
// must not be exposed outside the element's module.

use multi_index_map::MultiIndexMap;

mod elements {
    use super::MultiIndexMap;

    #[derive(MultiIndexMap, Debug)]
    pub struct Element {
        #[multi_index(hashed_unique)]
        id: u32,
    }
}

fn main() {
    let map = elements::MultiIndexElementMap::default();
    map.get_by_id(&1);
    map.iter_by_id();
}
