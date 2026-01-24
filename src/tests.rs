use std::sync::Arc;

use nucleo_matcher::Config;

use crate::pattern::CaseMatching;
use crate::{Nucleo, SortStrategy};

#[test]
fn active_injector_count() {
    let mut nucleo: Nucleo<()> = Nucleo::new(Config::DEFAULT, Arc::new(|| ()), Some(1), 1);
    assert_eq!(nucleo.active_injectors(), 0);
    let injector = nucleo.injector();
    assert_eq!(nucleo.active_injectors(), 1);
    let injector2 = nucleo.injector();
    assert_eq!(nucleo.active_injectors(), 2);
    drop(injector2);
    assert_eq!(nucleo.active_injectors(), 1);
    nucleo.restart(false);
    assert_eq!(nucleo.active_injectors(), 0);
    let injector3 = nucleo.injector();
    assert_eq!(nucleo.active_injectors(), 1);
    nucleo.tick(0);
    assert_eq!(nucleo.active_injectors(), 1);
    drop(injector);
    assert_eq!(nucleo.active_injectors(), 1);
    drop(injector3);
    assert_eq!(nucleo.active_injectors(), 0);
}

fn wait_for_nucleo<T: Sync + Send + 'static>(nucleo: &mut Nucleo<T>) {
    loop {
        let status = nucleo.tick(100);
        if !status.running {
            break;
        }
    }
}

#[derive(Debug, Clone)]
struct TestItem {
    name: &'static str,
    priority: u32,
}

#[test]
fn sort_strategy_score_default() {
    let mut nucleo: Nucleo<TestItem> = Nucleo::new(Config::DEFAULT, Arc::new(|| ()), Some(1), 1);

    let injector = nucleo.injector();
    injector.push(
        TestItem {
            name: "aXXXb",
            priority: 1,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    injector.push(
        TestItem {
            name: "abc",
            priority: 2,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    injector.push(
        TestItem {
            name: "ab",
            priority: 3,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    drop(injector);

    nucleo.pattern.reparse(
        0,
        "ab",
        CaseMatching::Ignore,
        crate::pattern::Normalization::Smart,
        false,
    );

    wait_for_nucleo(&mut nucleo);

    let snapshot = nucleo.snapshot();
    assert_eq!(snapshot.matched_item_count(), 3);

    let items: Vec<_> = snapshot.matched_items(..).map(|i| i.data.name).collect();
    assert_eq!(items[0], "ab");
    assert_eq!(items[1], "abc");
    assert_eq!(items[2], "aXXXb");
}

#[test]
fn sort_strategy_none_preserves_order() {
    let mut nucleo: Nucleo<TestItem> = Nucleo::new(Config::DEFAULT, Arc::new(|| ()), Some(1), 1);
    nucleo.set_sort_strategy(SortStrategy::Index);

    let injector = nucleo.injector();
    injector.push(
        TestItem {
            name: "aXXXb",
            priority: 1,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    injector.push(
        TestItem {
            name: "abc",
            priority: 2,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    injector.push(
        TestItem {
            name: "ab",
            priority: 3,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    drop(injector);

    nucleo.pattern.reparse(
        0,
        "ab",
        CaseMatching::Ignore,
        crate::pattern::Normalization::Smart,
        false,
    );

    wait_for_nucleo(&mut nucleo);

    let snapshot = nucleo.snapshot();
    assert_eq!(snapshot.matched_item_count(), 3);

    let items: Vec<_> = snapshot.matched_items(..).map(|i| i.data.name).collect();
    assert_eq!(items[0], "aXXXb");
    assert_eq!(items[1], "abc");
    assert_eq!(items[2], "ab");
}

#[test]
fn sort_strategy_custom() {
    let mut nucleo: Nucleo<TestItem> = Nucleo::new(Config::DEFAULT, Arc::new(|| ()), Some(1), 1);

    nucleo.set_sort_strategy(SortStrategy::Custom(Box::new(|_m1, i1, _m2, i2| {
        i1.data.priority.cmp(&i2.data.priority)
    })));

    let injector = nucleo.injector();
    injector.push(
        TestItem {
            name: "ab",
            priority: 30,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    injector.push(
        TestItem {
            name: "abc",
            priority: 10,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    injector.push(
        TestItem {
            name: "aXXXb",
            priority: 20,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    drop(injector);

    nucleo.pattern.reparse(
        0,
        "ab",
        CaseMatching::Ignore,
        crate::pattern::Normalization::Smart,
        false,
    );

    wait_for_nucleo(&mut nucleo);

    let snapshot = nucleo.snapshot();
    assert_eq!(snapshot.matched_item_count(), 3);

    let items: Vec<_> = snapshot
        .matched_items(..)
        .map(|i| (i.data.name, i.data.priority))
        .collect();
    assert_eq!(items[0], ("abc", 10));
    assert_eq!(items[1], ("aXXXb", 20));
    assert_eq!(items[2], ("ab", 30));
}

#[test]
fn sort_strategy_custom_score_then_priority() {
    let mut nucleo: Nucleo<TestItem> = Nucleo::new(Config::DEFAULT, Arc::new(|| ()), Some(1), 1);

    nucleo.set_sort_strategy(SortStrategy::Custom(Box::new(|m1, i1, m2, i2| {
        m1.score
            .cmp(&m2.score)
            .reverse()
            .then_with(|| i1.data.priority.cmp(&i2.data.priority))
    })));

    let injector = nucleo.injector();
    injector.push(
        TestItem {
            name: "ab",
            priority: 20,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    injector.push(
        TestItem {
            name: "ab",
            priority: 10,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    injector.push(
        TestItem {
            name: "aXXXb",
            priority: 5,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    drop(injector);

    nucleo.pattern.reparse(
        0,
        "ab",
        CaseMatching::Ignore,
        crate::pattern::Normalization::Smart,
        false,
    );

    wait_for_nucleo(&mut nucleo);

    let snapshot = nucleo.snapshot();
    assert_eq!(snapshot.matched_item_count(), 3);

    let items: Vec<_> = snapshot
        .matched_items(..)
        .map(|i| (i.data.name, i.data.priority))
        .collect();
    assert_eq!(items[0], ("ab", 10));
    assert_eq!(items[1], ("ab", 20));
    assert_eq!(items[2], ("aXXXb", 5));
}

#[test]
fn sort_results_bool_backwards_compat() {
    let mut nucleo: Nucleo<TestItem> = Nucleo::new(Config::DEFAULT, Arc::new(|| ()), Some(1), 1);

    let injector = nucleo.injector();
    injector.push(
        TestItem {
            name: "aXXXb",
            priority: 1,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    injector.push(
        TestItem {
            name: "ab",
            priority: 2,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    drop(injector);

    nucleo.pattern.reparse(
        0,
        "ab",
        CaseMatching::Ignore,
        crate::pattern::Normalization::Smart,
        false,
    );

    wait_for_nucleo(&mut nucleo);
    let items: Vec<_> = nucleo
        .snapshot()
        .matched_items(..)
        .map(|i| i.data.name)
        .collect();
    assert_eq!(items[0], "ab");

    nucleo.sort_results(false);
    nucleo.restart(true);

    let injector = nucleo.injector();
    injector.push(
        TestItem {
            name: "aXXXb",
            priority: 1,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    injector.push(
        TestItem {
            name: "ab",
            priority: 2,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    drop(injector);

    nucleo.pattern.reparse(
        0,
        "ab",
        CaseMatching::Ignore,
        crate::pattern::Normalization::Smart,
        false,
    );

    wait_for_nucleo(&mut nucleo);
    let items: Vec<_> = nucleo
        .snapshot()
        .matched_items(..)
        .map(|i| i.data.name)
        .collect();
    assert_eq!(items[0], "aXXXb");
    assert_eq!(items[1], "ab");
}

#[test]
fn sort_strategy_switch_at_runtime() {
    let mut nucleo: Nucleo<TestItem> = Nucleo::new(Config::DEFAULT, Arc::new(|| ()), Some(1), 1);

    let injector = nucleo.injector();
    injector.push(
        TestItem {
            name: "aXXXb",
            priority: 1,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    injector.push(
        TestItem {
            name: "ab",
            priority: 2,
        },
        |item, cols| cols[0] = item.name.into(),
    );
    drop(injector);

    nucleo.pattern.reparse(
        0,
        "ab",
        CaseMatching::Ignore,
        crate::pattern::Normalization::Smart,
        false,
    );

    wait_for_nucleo(&mut nucleo);
    let items: Vec<_> = nucleo
        .snapshot()
        .matched_items(..)
        .map(|i| i.data.name)
        .collect();
    assert_eq!(items[0], "ab");

    nucleo.set_sort_strategy(SortStrategy::Custom(Box::new(|_m1, i1, _m2, i2| {
        i1.data.priority.cmp(&i2.data.priority)
    })));

    nucleo.pattern.reparse(
        0,
        "ab ",
        CaseMatching::Ignore,
        crate::pattern::Normalization::Smart,
        false,
    );
    nucleo.pattern.reparse(
        0,
        "ab",
        CaseMatching::Ignore,
        crate::pattern::Normalization::Smart,
        false,
    );

    wait_for_nucleo(&mut nucleo);
    let items: Vec<_> = nucleo
        .snapshot()
        .matched_items(..)
        .map(|i| i.data.name)
        .collect();
    assert_eq!(items[0], "aXXXb");
}
