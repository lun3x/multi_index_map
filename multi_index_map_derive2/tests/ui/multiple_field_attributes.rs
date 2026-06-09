use multi_index_map_derive2::MultiIndexMap;

#[derive(MultiIndexMap)]
struct MultipleAttributes {
    #[multi_index(ById)]
    #[multi_index(ByTimestamp)]
    key: u64,
}

fn main() {}
