use multi_index_map::MultiIndexMap;

#[derive(MultiIndexMap, PartialEq, Debug)]
#[multi_index_derive(Debug)]
struct TestElement {
    #[multi_index(ordered_non_unique)]
    field1: usize,
    #[multi_index(ordered_unique)]
    field2: usize,
}

#[test]
fn test_non_unique_reverse_iter() {
    let mut s = MultiIndexTestElementMap::default();
    for i in 0..3 {
        s.insert(TestElement {
            field1: 1,
            field2: 10 + i,
        });
    }
    for i in 3..6 {
        s.insert(TestElement {
            field1: 0,
            field2: i,
        });
    }

    let mut prev = 6;
    for (_i, elem) in s.iter_by_field1().rev().enumerate() {
        assert!(elem.field1 <= prev);
        prev = elem.field1;
    }

    let mut prev = 20;
    for (_i, elem) in s.iter_by_field2().rev().enumerate() {
        assert!(elem.field2 <= prev);
        prev = elem.field2;
    }

    s.modify_by_field2(&12, |e| e.field1 = 2);
    s.modify_by_field2(&3, |e| e.field1 = 2);
    let mut prev = 6;
    for (_i, elem) in s.iter_by_field1().rev().enumerate() {
        assert!(elem.field1 <= prev);
        prev = elem.field1;
    }

    let mut prev = 20;
    for (_i, elem) in s.iter_by_field2().rev().enumerate() {
        assert!(elem.field2 <= prev);
        prev = elem.field2;
    }

    let mut it = s.iter_by_field2();
    assert_eq!(it.next_back().unwrap().field2, 12);
    assert_eq!(it.next_back().unwrap().field2, 11);
    assert_eq!(it.next_back().unwrap().field2, 10);
    assert_eq!(it.next_back().unwrap().field2, 5);
    assert_eq!(it.next_back().unwrap().field2, 4);
    assert_eq!(it.next_back().unwrap().field2, 3);
    assert_eq!(it.next_back(), None);
}
