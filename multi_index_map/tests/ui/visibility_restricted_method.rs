use multi_index_map::MultiIndexMap;

mod scope {
    pub mod elements {
        use super::super::MultiIndexMap;

        #[derive(MultiIndexMap, Debug)]
        pub struct Element {
            #[multi_index(hashed_unique)]
            pub(super) parent_key: u32,
            #[multi_index(ordered_unique)]
            pub(in crate::scope) scoped_key: u32,
        }
    }
}

// Re-exporting the public map must not widen the indexed methods' scope.
fn main() {
    let map = scope::elements::MultiIndexElementMap::default();
    map.get_by_parent_key(&1);
    map.iter_by_parent_key();
    map.get_by_scoped_key(&1);
    map.iter_by_scoped_key();
}
