use multi_index_map_derive2::MultiIndexMap;

#[derive(MultiIndexMap)]
struct ExtraArguments {
    #[multi_index(hashed_unique, ordered_unique)]
    key: u64,
}

fn main() {}
