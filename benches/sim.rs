//! Tick-cost benchmarks: `cargo bench --no-default-features`.
//!
//! The master plan's performance envelope (§17.4) assumes the daily tick is
//! cheap. These keep it honest as systems are added.

use bleeding_kansas::sim::attribution;
use bleeding_kansas::sim::market::Good;
use bleeding_kansas::sim::{EventKind, FireCause, World};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn year(c: &mut Criterion) {
    c.bench_function("simulate one year", |b| {
        b.iter(|| {
            let mut w = World::new(black_box(1856));
            w.run_days(365);
            w.events.len()
        })
    });
}

fn one_day(c: &mut Criterion) {
    c.bench_function("advance one day (fresh world)", |b| {
        b.iter_batched(
            || World::new(1856),
            |mut w| {
                w.advance_day();
            },
            criterion::BatchSize::LargeInput,
        )
    });
}

fn attribution_run(c: &mut Criterion) {
    let mut w = World::new(1856);
    w.run_days(120);
    let owner = w.head_of(1).unwrap();
    let fire = w.emit_root(
        EventKind::Fire {
            owner,
            cause: FireCause::Hearth,
            spread_from: None,
        },
        None,
    );
    c.bench_function("attribution candidates", |b| {
        b.iter(|| attribution::candidates(&w, black_box(owner), fire, None).len())
    });
}

fn market_trade(c: &mut Criterion) {
    let mut w = World::new(1856);
    w.families[0].stores.cash = 1_000_000;
    c.bench_function("buy and sell corn", |b| {
        b.iter(|| {
            w.player_buy(Good::Corn, 5.0);
            w.player_sell(Good::Corn, 5.0)
        })
    });
}

criterion_group!(benches, year, one_day, attribution_run, market_trade);
criterion_main!(benches);
