use multi_index_map::MultiIndexMap;

#[derive(MultiIndexMap, Clone, Debug)]
#[multi_index_derive(Clone, Debug)]
struct TestElement {
    #[multi_index(hashed_non_unique)]
    field1: usize,
    field2: usize,
    #[multi_index(hashed_unique)]
    field3: usize,
}

#[test]
fn test_non_unique_get_mut() {
    let mut map = MultiIndexTestElementMap::default();
    for i in 0..10 {
        if i % 2 == 0 {
            map.insert(TestElement {
                field1: 42,
                field2: i,
                field3: i,
            });
        } else {
            map.insert(TestElement {
                field1: 37,
                field2: i,
                field3: i,
            });
        }
    }

    let mut_refs: Vec<(&mut usize,)> = map.get_mut_by_field1(&37);
    for r in mut_refs {
        *r.0 = *r.0 * *r.0;
    }

    let refs: Vec<&TestElement> = map.get_by_field1(&37);
    for (i, r) in refs.iter().enumerate() {
        assert_eq!(r.field2, (2 * i + 1) * (2 * i + 1));
    }
}

#[test]
fn test_non_unique_modify_mut() {
    let mut map = MultiIndexTestElementMap::default();
    for i in 0..10 {
        if i % 2 == 0 {
            map.insert(TestElement {
                field1: 42,
                field2: i,
                field3: i,
            });
        } else {
            map.insert(TestElement {
                field1: 37,
                field2: i,
                field3: i,
            });
        }
    }

    let refs = map.modify_by_field1(&37, |x| {
        if x.field2 > 5 {
            x.field3 *= 2;
        } else {
            x.field1 = 0;
        }
    });

    for (i, r) in refs.iter().enumerate() {
        if 2 * i + 1 > 5 {
            assert_eq!(r.field1, 37);
            assert_eq!(r.field2, 2 * i + 1);
            assert_eq!(r.field3, 4 * i + 2);
        } else {
            assert_eq!(r.field1, 0);
            assert_eq!(r.field2, 2 * i + 1);
            assert_eq!(r.field3, 2 * i + 1);
        }
    }
}
