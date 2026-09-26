//! The lab: run one feature at a time, with knobs, and look inside.
//!
//!   cargo run --bin lab --no-default-features -- <command> [options]
//!
//! Commands
//!   map                      the county as text, with claims marked
//!   npc <name|id>            everything about a person, hidden stats included
//!   events [kind]            list event ids (fire, death, theft, wound, cruelty, ...)
//!   blame <event-id> [who]   the attribution engine's reasoning, term by term
//!   tree <event-id>          the cascade tree under an event
//!   market                   weekly price table for every good
//!   corner <good> <units> <day>  buy up a good on a day, then watch the county
//!   winter                   Phase 2 gate: mild vs harsh winters across seeds
//!   gate                     Phase 1 + Phase 2 gates
//!   evil                     cruelty and slander across seeds, and who got blamed
//!   news                     every headline, both sides
//!   bison                    the herd and robe prices over time
//!   metrics <file.csv>       daily metrics for charts
//!   trace <file.tsv>         every event with its cascade links
//!   time                     how long a simulated year takes
//!
//! Options: --seed N (1856)  --days N (365)  --seeds N (30)  --winter X

use std::time::Instant;

use bleeding_kansas::sim::chronicle;
use bleeding_kansas::sim::debug;
use bleeding_kansas::sim::events::{Cruelty, Suspect};
use bleeding_kansas::sim::geography::{self, PLACES};
use bleeding_kansas::sim::market::Good;
use bleeding_kansas::sim::{EventKind, Faction, World};

struct Opts {
    seed: u64,
    days: u32,
    seeds: u64,
    winter: Option<f32>,
    rest: Vec<String>,
}

fn opts() -> (String, Opts) {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "help".into());
    let mut o = Opts {
        seed: 1856,
        days: 365,
        seeds: 30,
        winter: None,
        rest: Vec::new(),
    };
    while let Some(a) = args.next() {
        let mut val = || args.next().unwrap_or_default();
        match a.as_str() {
            "--seed" => o.seed = val().parse().unwrap_or(o.seed),
            "--days" => o.days = val().parse().unwrap_or(o.days),
            "--seeds" => o.seeds = val().parse().unwrap_or(o.seeds),
            "--winter" => o.winter = val().parse().ok(),
            _ => o.rest.push(a),
        }
    }
    (cmd, o)
}

fn world(o: &Opts) -> World {
    let mut w = World::with_winter(o.seed, o.winter);
    w.run_days(o.days);
    w
}

fn main() {
    // Piping into `head` closes stdout early; exit quietly instead of panicking.
    std::panic::set_hook(Box::new(|info| {
        if info.to_string().contains("Broken pipe") {
            std::process::exit(0);
        }
        eprintln!("{info}");
    }));
    let (cmd, o) = opts();
    match cmd.as_str() {
        "map" => map(&o),
        "npc" => npc(&o),
        "events" => events(&o),
        "blame" => blame(&o),
        "tree" => tree(&o),
        "market" => market(&o),
        "corner" => corner(&o),
        "winter" => winter(&o),
        "gate" => {
            phase1(&o);
            winter(&o);
        }
        "evil" => evil(&o),
        "news" => news(&o),
        "bison" => bison(&o),
        "metrics" => dump(&o, "metrics.csv", debug::metrics_csv),
        "trace" => dump(&o, "trace.tsv", debug::trace_tsv),
        "time" => time(&o),
        _ => println!(
            "{}",
            include_str!("lab.rs")
                .lines()
                .take(24)
                .map(|l| l.trim_start_matches("//!").trim_start_matches(' '))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    }
}

fn map(o: &Opts) {
    let w = World::new(o.seed);
    let mut marks: Vec<((i32, i32), char)> = w
        .families
        .iter()
        .map(|f| {
            let c = if f.store {
                '$'
            } else if f.id == 0 {
                '@'
            } else if f.faction == Faction::FreeState {
                'F'
            } else {
                'S'
            };
            (f.farm, c)
        })
        .collect();
    for (i, p) in PLACES.iter().enumerate() {
        marks.push((geography::to_tile(p.at), (b'a' + i as u8) as char));
    }
    print!("{}", w.map.render(&marks));
    println!("\n@ you   F Free-State claim   S Pro-Slavery claim   $ Dunmore's store");
    println!(". prairie  ♣ timber  ~ river  = road  # town  , treaty land");
    for (i, p) in PLACES.iter().enumerate() {
        println!(
            "{} {}{}",
            (b'a' + i as u8) as char,
            p.name,
            if p.approximate { " (approx.)" } else { "" }
        );
    }
    println!(
        "\nhalf a mile per character; {} x {} miles",
        geography::WIDTH / 2,
        geography::HEIGHT / 2
    );
}

fn npc(o: &Opts) {
    let w = world(o);
    let q = o.rest.first().map(String::as_str).unwrap_or("0");
    match debug::find_npc(&w, q) {
        Some(id) => print!("{}", debug::explain_npc(&w, id)),
        None => {
            println!("no one matches {q:?}. People:");
            for n in &w.npcs {
                println!("  #{} {}", n.id, n.name);
            }
        }
    }
}

fn kind_name(k: &EventKind) -> &'static str {
    match k {
        EventKind::Fire { .. } => "fire",
        EventKind::Death { .. } => "death",
        EventKind::Theft { .. } => "theft",
        EventKind::Wounded { .. } => "wound",
        EventKind::Cruelty { .. } => "cruelty",
        EventKind::Perished { .. } => "perished",
        EventKind::FeudDeclared { .. } => "feud",
        EventKind::Headline { .. } => "headline",
        EventKind::Favor { .. } => "favor",
        _ => "other",
    }
}

fn events(o: &Opts) {
    let w = world(o);
    let filter = o.rest.first().cloned();
    for ev in &w.events {
        let k = kind_name(&ev.kind);
        if k == "other" && filter.is_none() {
            continue;
        }
        if filter.as_deref().is_some_and(|f| f != k) {
            continue;
        }
        println!(
            "{:>6}  {}  {}",
            ev.id,
            ev.day,
            chronicle::debug_line(&w, ev, true)
        );
    }
}

fn blame(o: &Opts) {
    let w = world(o);
    let Some(id) = o.rest.first().and_then(|s| s.parse().ok()) else {
        println!("usage: lab blame <event-id> [who]   (find ids with `lab events fire`)");
        return;
    };
    let observers: Vec<_> = match o.rest.get(1) {
        Some(q) => debug::find_npc(&w, q).into_iter().collect(),
        None => {
            // Everyone who formed a belief about it.
            let mut v: Vec<_> = w
                .living()
                .filter(|n| n.memory_of(id).is_some())
                .map(|n| n.id)
                .collect();
            v.truncate(6);
            v
        }
    };
    for ob in observers {
        println!("{}", debug::explain_blame(&w, ob, id));
    }
}

fn tree(o: &Opts) {
    let w = world(o);
    match o.rest.first().and_then(|s| s.parse().ok()) {
        Some(id) => println!("{}", chronicle::cascade_tree(&w, id, 200)),
        None => println!("usage: lab tree <event-id>"),
    }
}

fn price_table(w: &World) {
    print!("{:<12}", "week");
    for g in Good::ALL {
        print!("{:>10}", &g.label()[..g.label().len().min(9)]);
    }
    println!();
    for (i, row) in w.market.history.iter().enumerate() {
        print!(
            "{:<12}",
            bleeding_kansas::sim::Day(i as u32 * 7).to_string().trim()
        );
        for p in row {
            print!("{:>10.2}", p);
        }
        println!();
    }
}

fn market(o: &Opts) {
    let w = world(o);
    price_table(&w);
    for g in Good::ALL {
        println!("{}", debug::explain_price(&w, g));
    }
}

fn corner(o: &Opts) {
    let good = match o.rest.first().map(String::as_str) {
        Some("powder") => Good::Powder,
        Some("salt") => Good::Salt,
        Some("timber") => Good::Timber,
        Some("seed") => Good::SeedCorn,
        _ => Good::Corn,
    };
    let units: f32 = o.rest.get(1).and_then(|s| s.parse().ok()).unwrap_or(500.0);
    let day: u32 = o.rest.get(2).and_then(|s| s.parse().ok()).unwrap_or(90);
    let mut w = World::with_winter(o.seed, o.winter);
    w.autopilot_player = false;
    w.run_days(day);
    w.families[0].stores.cash = 100_000;
    let rep0 = bleeding_kansas::sim::psyche::reputation(&w, 0);
    let before = debug::explain_price(&w, good);
    let got = w.player_buy(good, units);
    println!(
        "{}: bought {got:.0} of {}\nbefore: {before}",
        w.day,
        good.label()
    );
    for _ in 0..12 {
        w.run_days(7);
        println!(
            "{}  {}  reputation {:+.1}  thefts from you {}",
            w.day,
            debug::explain_price(&w, good).lines().next().unwrap_or(""),
            bleeding_kansas::sim::psyche::reputation(&w, 0),
            w.events
                .iter()
                .filter(|e| matches!(e.kind, EventKind::Theft { victim, .. } if w.npc(victim).family == 0))
                .count()
        );
    }
    println!(
        "reputation {:+.1} -> {:+.1}",
        rep0,
        bleeding_kansas::sim::psyche::reputation(&w, 0)
    );
}

#[derive(Default)]
struct WinterStats {
    thefts: usize,
    perished: usize,
    seed_eaten: usize,
    borrowed: usize,
    favors: usize,
    person_blame: usize,
    violent: usize,
    grievance: i32,
}

fn winter_run(seed: u64, severity: f32) -> WinterStats {
    let mut w = World::with_winter(seed, Some(severity));
    // Through the winter and into August 1856: six months on.
    w.run_days(290);
    let mut s = WinterStats {
        grievance: w.grievance[0] + w.grievance[1],
        ..Default::default()
    };
    for ev in &w.events {
        match ev.kind {
            EventKind::Theft { .. } => s.thefts += 1,
            EventKind::Perished { .. } => s.perished += 1,
            EventKind::Desperation {
                act: bleeding_kansas::sim::events::Desperate::EatSeed { .. },
                ..
            } => s.seed_eaten += 1,
            EventKind::Desperation {
                act: bleeding_kansas::sim::events::Desperate::Borrow { .. },
                ..
            } => s.borrowed += 1,
            EventKind::Favor { .. } => s.favors += 1,
            EventKind::Belief {
                blamed: Suspect::Person(_),
                confidence,
                ..
            } if confidence >= 20 => s.person_blame += 1,
            EventKind::Death { .. } | EventKind::Wounded { .. } | EventKind::Fire { .. } => {
                s.violent += 1
            }
            _ => {}
        }
    }
    s
}

/// Phase 2 gate (§24): does a bad winter change the political map six months later?
fn winter(o: &Opts) {
    let mut mild = WinterStats::default();
    let mut harsh = WinterStats::default();
    for seed in 1..=o.seeds {
        let m = winter_run(seed, 0.5);
        let h = winter_run(seed, 1.6);
        for (acc, s) in [(&mut mild, m), (&mut harsh, h)] {
            acc.thefts += s.thefts;
            acc.perished += s.perished;
            acc.seed_eaten += s.seed_eaten;
            acc.borrowed += s.borrowed;
            acc.favors += s.favors;
            acc.person_blame += s.person_blame;
            acc.violent += s.violent;
            acc.grievance += s.grievance;
        }
    }
    let n = o.seeds as f32;
    println!(
        "PHASE 2 GATE: mild (0.5) vs harsh (1.6) winters, {} seeds, to Aug 1856 (per seed)",
        o.seeds
    );
    for (label, f) in [
        (
            "thefts",
            (|s: &WinterStats| s.thefts as f32) as fn(&WinterStats) -> f32,
        ),
        ("perished", |s| s.perished as f32),
        ("ate seed corn", |s| s.seed_eaten as f32),
        ("bought on credit", |s| s.borrowed as f32),
        ("storekeeper favors", |s| s.favors as f32),
        ("accusations", |s| s.person_blame as f32),
        ("fires/wounds/deaths", |s| s.violent as f32),
        ("grievance in Aug", |s| s.grievance as f32),
    ] {
        let (a, b) = (f(&mild) / n, f(&harsh) / n);
        println!(
            "  {:<22} {:>7.1} {:>7.1}   {}",
            label,
            a,
            b,
            if b > a * 1.1 { "harsher" } else { "" }
        );
    }
}

fn phase1(o: &Opts) {
    let mut with = 0;
    for seed in 1..=o.seeds {
        let mut w = World::new(seed);
        w.run_days(o.days);
        if !chronicle::wars_nobody_started(&w).is_empty() {
            with += 1;
        }
    }
    println!(
        "PHASE 1 GATE: {with}/{} seeds produced a war nobody started in {} days\n",
        o.seeds, o.days
    );
}

fn evil(o: &Opts) {
    let (mut acts, mut blamed_right, mut blamed_wrong, mut blamed_nation) = (0, 0, 0, 0);
    for seed in 1..=o.seeds {
        let mut w = World::new(seed);
        w.run_days(o.days);
        for ev in &w.events {
            if let EventKind::Cruelty { actor, act, .. } = ev.kind {
                if matches!(act, Cruelty::Slander { .. }) {
                    continue;
                }
                acts += 1;
                for b in &w.events {
                    if let EventKind::Belief {
                        about,
                        blamed,
                        confidence,
                        ..
                    } = b.kind
                        && about == ev.id
                        && confidence >= 20
                    {
                        match blamed {
                            Suspect::Person(p) if p == actor => blamed_right += 1,
                            Suspect::Nation(_) => blamed_nation += 1,
                            Suspect::Person(_) => blamed_wrong += 1,
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    let n = o.seeds as f32;
    println!(
        "cruelty: {:.1} acts/seed. beliefs: {} right, {} wrong person, {} blamed a nation",
        acts as f32 / n,
        blamed_right,
        blamed_wrong,
        blamed_nation
    );
}

fn news(o: &Opts) {
    let w = world(o);
    for ev in &w.events {
        if matches!(
            ev.kind,
            EventKind::Headline { .. } | EventKind::History { .. }
        ) {
            println!("{}  {}", ev.day, chronicle::debug_line(&w, ev, true));
        }
    }
}

fn bison(o: &Opts) {
    let mut w = World::with_winter(o.seed, o.winter);
    for _ in 0..(o.days / 30).max(1) {
        w.run_days(30);
        println!(
            "{}  herd {:>6.0}  robes ${:.2}  trail safety {:.2}  Kaw food {:.2}",
            w.day,
            w.bison.population,
            w.market.price(Good::Hides),
            w.market.trail_safety,
            w.nations[2].food_security
        );
    }
}

fn dump(o: &Opts, default: &str, f: fn(&World) -> String) {
    let w = world(o);
    let path = o.rest.first().cloned().unwrap_or_else(|| default.into());
    std::fs::write(&path, f(&w)).expect("write");
    println!("wrote {path}");
}

fn time(o: &Opts) {
    let start = Instant::now();
    let w = world(o);
    let t = start.elapsed();
    println!(
        "{} days in {:.1} ms ({:.3} ms/day), {} events",
        o.days,
        t.as_secs_f64() * 1000.0,
        t.as_secs_f64() * 1000.0 / o.days as f64,
        w.events.len()
    );
}
