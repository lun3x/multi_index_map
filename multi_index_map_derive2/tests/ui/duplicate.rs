use multi_index_map_derive2::MultiIndexMap;

#[derive(MultiIndexMap)]
struct Duplicate {
    #[multi_index(hashed_unique)]
    #[multi_index(ordered_unique)]
    key: u64,
}

fn main() {}
