#![cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use multi_index_map::MultiIndexMap;

#[derive(Hash, PartialEq, Eq, Clone, Deserialize, Serialize)]
struct TestNonPrimitiveType(u64);

#[derive(MultiIndexMap, Deserialize, Serialize)]
#[multi_index_derive(Deserialize, Serialize)]
struct TestElement {
    #[multi_index(hashed_unique)]
    field1: TestNonPrimitiveType,
}

#[test]
fn should_compile() {
    let mut map = MultiIndexTestElementMap::default();

    let elem1 = TestElement {
        field1: TestNonPrimitiveType(42),
    };
    map.insert(elem1);

    let s = serde_json::to_string(&map);
}
