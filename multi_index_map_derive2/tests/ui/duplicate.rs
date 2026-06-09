use multi_index_map_derive2::MultiIndexMap;

#[derive(MultiIndexMap)]
struct Duplicate {
    #[multi_index(ByKey, ByKey)]
    key: u64,
}

fn main() {}
