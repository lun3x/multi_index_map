// An iterator for a restricted indexed field must stay inside the allowed scope,
// even when its element is public.

use multi_index_map::MultiIndexMap;

mod scope {
    pub mod elements {
        use super::super::MultiIndexMap;

        #[derive(MultiIndexMap, Debug)]
        pub struct Element {
            #[multi_index(ordered_non_unique)]
            pub(in crate::scope) id: u32,
        }
    }
}

fn main() {
    let _: Option<scope::elements::MultiIndexElementMapIdIter<'_>> = None;
}
