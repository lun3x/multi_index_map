use multi_index_map_derive2::MultiIndexMap;

#[derive(MultiIndexMap)]
struct Generic<T> {
    #[multi_index(ByKey)]
    key: T,
}

fn main() {}
