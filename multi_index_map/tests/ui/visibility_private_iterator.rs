// Public indexed fields must not expose a private element through its iterator.
// The derive should succeed inside the module, while naming the iterator here
// must fail because the generated type has the element's private visibility.

use multi_index_map::MultiIndexMap;

mod elements {
    use super::MultiIndexMap;

    #[derive(MultiIndexMap, Debug)]
    struct Element {
        #[multi_index(hashed_unique)]
        pub id: u32,
    }
}

fn main() {
    let _: Option<elements::MultiIndexElementMapIdIter<'_>> = None;
}
