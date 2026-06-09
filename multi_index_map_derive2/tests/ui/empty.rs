use multi_index_map_derive2::MultiIndexMap;

#[derive(MultiIndexMap)]
struct Empty {
    #[multi_index()]
    key: u64,
}

fn main() {}
