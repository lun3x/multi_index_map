// Generated types follow the public element's visibility, while keyed methods
// follow the private indexed field's visibility. Widening the iterator type
// must not expose either of these methods outside the element's module.

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
