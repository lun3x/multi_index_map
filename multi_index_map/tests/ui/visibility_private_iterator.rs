// A public element can have private indexed fields and private index iterators.
// The derive should succeed inside the module, while naming the iterator here
// must fail because the generated type has the indexed field's visibility.

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
    let _: Option<elements::MultiIndexElementMapIdIter<'_>> = None;
}
