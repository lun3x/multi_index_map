use multi_index_map::MultiIndexMap;

#[derive(MultiIndexMap, PartialEq, Debug)]
#[multi_index_derive(Debug)]
struct TestElement {
    #[multi_index(hashed_non_unique)]
    field1: i32,
    field2: f64,
    #[multi_index(hashed_unique)]
    field3: u32,
    field4: String,
    #[multi_index(hashed_non_unique)]
    field5: String,
}

#[test]
fn test_non_unique_update() {
    let mut map = MultiIndexTestElementMap::default();
    for i in 0..10 {
        if i % 2 == 0 {
            map.insert(TestElement {
                field1: 42,
                field2: i as f64,
                field3: i,
                field4: i.to_string(),
                field5: "42".to_string(),
            });
        } else {
            map.insert(TestElement {
                field1: 37,
                field2: i as f64,
                field3: i,
                field4: i.to_string(),
                field5: "37".to_string(),
            });
        }
    }

    let refs = map.update_by_field1(&37, |field2, field4| {
        *field2 = 99.0;
        *field4 = "NinetyNine".to_string()
    });
    for r in refs.iter() {
        assert_eq!(r.field2, 99.0);
        assert_eq!(r.field4, "NinetyNine");
    }

    let refs = map.get_by_field1(&42);
    for (i, r) in refs.iter().enumerate() {
        assert_eq!(r.field2, i as f64 * 2.0);
        assert_eq!(r.field4, (i * 2).to_string());
    }
}

#[test]
fn test_non_unique_update_borrow() {
    let mut map = MultiIndexTestElementMap::default();
    for i in 0..10 {
        if i % 2 == 0 {
            map.insert(TestElement {
                field1: 42,
                field2: i as f64,
                field3: i,
                field4: i.to_string(),
                field5: "42".to_string(),
            });
        } else {
            map.insert(TestElement {
                field1: 37,
                field2: i as f64,
                field3: i,
                field4: i.to_string(),
                field5: "37".to_string(),
            });
        }
    }

    let refs = map.update_by_field5("37", |field2, field4| {
        *field2 = 99.0;
        *field4 = "NinetyNine".to_string()
    });
    for r in refs.iter() {
        assert_eq!(r.field2, 99.0);
        assert_eq!(r.field4, "NinetyNine");
    }

    let refs = map.get_by_field1(&42);
    for (i, r) in refs.iter().enumerate() {
        assert_eq!(r.field2, i as f64 * 2.0);
        assert_eq!(r.field4, (i * 2).to_string());
    }
}

#[test]
fn test_unique_update() {
    let mut map = MultiIndexTestElementMap::default();
    for i in 0..10 {
        if i % 2 == 0 {
            map.insert(TestElement {
                field1: 42,
                field2: i as f64,
                field3: i,
                field4: i.to_string(),
                field5: "42".to_string(),
            });
        } else {
            map.insert(TestElement {
                field1: 37,
                field2: i as f64,
                field3: i,
                field4: i.to_string(),
                field5: "37".to_string(),
            });
        }
    }

    let elem = map.update_by_field3(&0, |field2, field4| {
        *field2 = 99.0;
        *field4 = "NinetyNine".to_string()
    });

    assert_eq!(
        elem,
        Some(&TestElement {
            field1: 42,
            field2: 99.0,
            field3: 0,
            field4: "NinetyNine".to_string(),
            field5: "42".to_string()
        })
    );

    let elem = map.get_by_field3(&1);

    assert_eq!(
        elem,
        Some(&TestElement {
            field1: 37,
            field2: 1.0,
            field3: 1,
            field4: 1.to_string(),
            field5: "37".to_string()
        })
    );
}
