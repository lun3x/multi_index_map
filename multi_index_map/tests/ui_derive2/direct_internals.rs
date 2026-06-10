use multi_index_map::{MultiIndexAccessor, MultiIndexMap2};

#[derive(MultiIndexAccessor)]
#[multi_index(hashed_unique)]
struct ById;

#[derive(MultiIndexMap2)]
struct Record {
    #[multi_index(ById)]
    id: u64,
}

fn main() {
    let mut map = MultiIndexRecordMap::new();
    let _ = &map.nodes;
    let _ = &map.__mim_index_0;
    map.link_all(multi_index_map::__private::NodeId(0));
    map.modify_id(multi_index_map::__private::NodeId(0), |_| {});
}
