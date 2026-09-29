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
//!   life <routine>           live a player's life on a routine (farmer, fisher,
//!                            preacher, souse, rake, orator, builder, roamer)
//!   farm                     each family's farm year: planting, hay, corn, fences
//!   county                   weddings, affairs, scandals, claims, projects across seeds
//!   arms                     every house's guns, rounds and hiding place; shipments,
//!                            seizures and searches over --days
//!   standoff                 the men who'd come to your gate: their draw, what drives
//!                            them, and how often each way of meeting them works
//!   law                      every paper the justice wrote, who it named, who really
//!                            did it, and how it was served
//!   sick                     who fell sick with what, who died of it, and what
//!                            the doctor and the chest did, over --seeds
//!   dress                    what the county wears, the looks, and whose colors
//!   hands                    a watch at night and a hired hand, against nobody, over
//!                            --seeds: riders turned back, fires, strays, timber
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
        "life" => life_routine(&o),
        "county" => county(&o),
        "farm" => farm(&o),
        "metrics" => dump(&o, "metrics.csv", debug::metrics_csv),
        "trace" => dump(&o, "trace.tsv", debug::trace_tsv),
        "time" => time(&o),
        "standoff" => standoff(&o),
        "arms" => arms_lens(&o),
        "law" => law_lens(&o),
        "hands" => hands_lens(&o),
        "sick" => sick_lens(&o),
        "dress" => dress_lens(&o),
        _ => println!(
            "{}",
            include_str!("lab.rs")
                .lines()
                .take(35)
                .map(|l| l.trim_start_matches("//!").trim_start_matches(' '))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    }
}

/// Phase D: the justice's papers, with the truth beside them.
fn law_lens(o: &Opts) {
    use bleeding_kansas::sim::warrant;
    let w = world(o);
    println!("seed {} after {} days ({})\n", o.seed, o.days, w.day);
    for p in &w.warrants.list {
        let truth = warrant::truth(&w, p.about)
            .map(|t| w.name(t).to_string())
            .unwrap_or_else(|| "nobody".into());
        println!(
            "{:<22} ${:<4} {:<8?} day {:>4}  [truth: {}{}]",
            w.name(p.accused),
            p.bounty,
            p.state,
            p.issued.0,
            truth,
            if warrant::truth(&w, p.about) == Some(p.accused) {
                ""
            } else {
                " — the wrong man"
            }
        );
    }
    println!();
    for e in &w.events {
        if matches!(
            e.kind,
            EventKind::Warrant { .. }
                | EventKind::PosseOut { .. }
                | EventKind::Arrested { .. }
                | EventKind::Tried { .. }
                | EventKind::Fled { .. }
                | EventKind::BountyPaid { .. }
        ) {
            println!("{}", chronicle::debug_line(&w, e, true));
        }
    }
    let l = warrant::ledger(&w);
    println!(
        "\n{} papers ({} on the wrong man), {} arrested, {} convicted, {} fled, {} lapsed; {} shot by the law, {} lawmen shot; ${} in bounties",
        l.papers,
        l.wrong_man,
        l.arrested,
        l.convicted,
        l.fled,
        l.lapsed,
        l.shot_by_law,
        l.law_shot,
        l.bounties_paid
    );
}

/// The sick pass: incidence and deaths by disease across seeds.
fn sick_lens(o: &Opts) {
    use bleeding_kansas::sim::sickness;
    let mut tot: Vec<(sickness::Disease, usize, usize)> = Vec::new();
    for seed in 1..=o.seeds {
        let mut w = World::new(seed);
        w.run_days(o.days);
        for (d, s, k) in sickness::tally(&w) {
            match tot.iter_mut().find(|t| t.0 == d) {
                Some(t) => {
                    t.1 += s;
                    t.2 += k;
                }
                None => tot.push((d, s, k)),
            }
        }
        if seed == 1 {
            for e in &w.events {
                if matches!(
                    e.kind,
                    EventKind::Epidemic { .. } | EventKind::DoctorCalled { .. }
                ) {
                    println!("{}", chronicle::debug_line(&w, e, true));
                }
            }
            println!();
        }
    }
    println!(
        "{:<20} {:>8} {:>8}   per seed over {} days",
        "", "sick", "dead", o.days
    );
    for (d, s, k) in tot {
        println!(
            "{:<20} {:>8.1} {:>8.2}",
            d.label(),
            s as f32 / o.seeds as f32,
            k as f32 / o.seeds as f32
        );
    }
}

/// The outfit pass: who wears what, and whose colors they show.
fn dress_lens(o: &Opts) {
    use bleeding_kansas::sim::wardrobe;
    let w = world(o);
    for n in w.living() {
        let looks: Vec<&str> = n.outfit.looks().iter().map(|l| l.name).collect();
        println!(
            "{:<22} {:<11} {:<60} {}{}",
            n.name,
            n.faction.label(),
            wardrobe::describe(&w, n.id),
            n.outfit
                .colors()
                .map(|c| format!("reads {} ", c.label()))
                .unwrap_or_default(),
            looks.join(", ")
        );
    }
}

/// Phase D: what a watch and a hired hand are worth, against nobody.
fn hands_lens(o: &Opts) {
    use bleeding_kansas::sim::hands::{self, Task};
    let run = |seed: u64, staffed: bool| {
        let mut w = World::new(seed);
        w.autopilot_player = false;
        w.families[0].stores.cash += 200;
        if staffed {
            if let Some(&k) = hands::workers(&w).first() {
                hands::assign(&mut w, k, Task::Watch);
            }
            if let Some(&c) = hands::candidates(&w).first() {
                hands::hire(&mut w, c);
                hands::assign(&mut w, c, Task::Woods);
            }
        }
        w.run_days(o.days);
        let count = |f: &dyn Fn(&EventKind) -> bool| w.events.iter().filter(|e| f(&e.kind)).count();
        (
            count(&|k| matches!(k, EventKind::TurnedBack { target: 0, .. })),
            count(&|k| matches!(k, EventKind::RidersAtGate { .. })),
            count(&|k| matches!(k, EventKind::Fire { owner: 0, .. })),
            count(&|k| matches!(k, EventKind::Strayed { owner: 0 })),
            w.families[0].stores.goods[Good::Timber.index()],
            w.hands.hired.len(),
        )
    };
    println!(
        "{:>5}  {:>22}  {:>22}",
        "seed", "nobody: gate fire stray", "watch+hand: back gate fire timber kept"
    );
    for seed in 1..=o.seeds {
        let a = run(seed, false);
        let b = run(seed, true);
        println!(
            "{:>5}  {:>10} {:>5} {:>5}  {:>9} {:>5} {:>5} {:>6.0} {:>4}",
            seed, a.1, a.2, a.3, b.0, b.1, b.2, b.4, b.5
        );
    }
}

/// Phase C: who's armed, who sent east, what the posse carried off.
fn arms_lens(o: &Opts) {
    let mut w = World::new(o.seed);
    w.run_days(o.days);
    println!("seed {} after {} days ({})\n", o.seed, o.days, w.day);
    println!(
        "{:<12} {:<11} {:>6} {:>4} {:>6} {:>6}  kept",
        "family", "side", "rifles", "guns", "balls", "carts"
    );
    for f in w.families.iter().filter(|f| !f.store) {
        let a = &f.stores.arms;
        println!(
            "{:<12} {:<11} {:>6} {:>4} {:>6} {:>6}  {}",
            f.surname,
            format!("{:?}", f.faction),
            a.rifles,
            a.guns,
            a.balls,
            a.cartridges,
            a.hide.label()
        );
    }
    println!();
    for e in &w.events {
        if matches!(
            e.kind,
            EventKind::ArmsArrived { .. }
                | EventKind::Intercepted { .. }
                | EventKind::Searched { .. }
        ) {
            println!("{}", chronicle::debug_line(&w, e, true));
        }
    }
}

/// Phase B's terms, laid bare: who's quick, what drives them, and what works.
fn standoff(o: &Opts) {
    use bleeding_kansas::sim::action::{self, DrawEnd, Shot, Turn};
    use bleeding_kansas::sim::events::Retaliation;
    let w = World::new(o.seed);
    println!(
        "seed {}: the grown men and women who might come to your gate\n",
        o.seed
    );
    println!(
        "{:<22} {:>6}  {:<14} {:>8}",
        "who", "draw", "drives", "pressure"
    );
    for n in w.living().filter(|n| n.family != 0 && n.age >= 16) {
        let m = action::moods(&w, n.id);
        let s = action::Standoff {
            actor: n.id,
            method: Retaliation::Arson,
            caused_by: None,
            day: w.day,
            yours: false,
            riders: Vec::new(),
            serving: None,
        };
        println!(
            "{:<22} {:>5.2}s  {:<14} {:>8.2}",
            n.name,
            action::their_draw(&w, n.id),
            format!("{:?}/{:?}", m[0], m[1]),
            action::pressure(&w, &s)
        );
    }
    println!(
        "\nyour nerve (half-width of the calm): {:.2}   your sway: {:.2}\n",
        action::nerve(&w),
        action::sway(&w)
    );
    println!("across {} seeds, a Pike at the gate:", o.seeds);
    let mut talk = [0u32; 4];
    let mut face = [0u32; 3];
    let mut beaten_dead = 0;
    for seed in 0..o.seeds {
        let fresh = || {
            let mut w = World::new(seed);
            w.autopilot_player = false;
            let t = w.head_of(4).unwrap();
            action::park(&mut w, t, Retaliation::Ambush, None);
            w
        };
        for (right, n) in talk.iter_mut().enumerate() {
            let mut w = fresh();
            *n += (action::talk(&mut w, right as u8, 3) == Turn::Settled) as u32;
        }
        for (i, g) in [0.4, 0.7, 0.95].iter().enumerate() {
            let mut w = fresh();
            face[i] += (action::face(&mut w, *g) == Turn::Settled) as u32;
        }
        let mut w = fresh();
        action::draw(&mut w, DrawEnd::Slow);
        beaten_dead += (!w.npc(bleeding_kansas::sim::PLAYER).alive) as u32;
        let _ = Shot::Kill;
    }
    let pct = |n: u32| 100.0 * n as f32 / o.seeds as f32;
    for (right, n) in talk.iter().enumerate() {
        println!(
            "  talk, {right} of 3 appeals right: settled {:.0}%",
            pct(*n)
        );
    }
    for (g, n) in [0.4, 0.7, 0.95].iter().zip(face) {
        println!(
            "  stare, held {:.0}%: they back off {:.0}%",
            g * 100.0,
            pct(n)
        );
    }
    println!(
        "  lose the draw to a man with a rifle: dead {:.0}%, wounded the rest",
        pct(beaten_dead)
    );
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

/// A player who does the same few things every week: what does that life
/// become, and what does the county make of it?
fn life_routine(o: &Opts) {
    use bleeding_kansas::sim::civic::Project;
    use bleeding_kansas::sim::life::{self, Activity, Skill};
    use bleeding_kansas::sim::romance;
    use bleeding_kansas::sim::world::PLAYER;
    let routine = o.rest.first().map(String::as_str).unwrap_or("farmer");
    let mut w = World::with_winter(o.seed, o.winter);
    let neighbor = w.head_of(3).unwrap();
    for d in 0..o.days {
        let a = match (routine, d % 7) {
            ("fisher", 1 | 4) | ("roamer", 1 | 3) => Activity::Fish,
            ("roamer", _) => Activity::Roam,
            ("preacher", 3) => Activity::Preach,
            ("preacher", 5) => Activity::Baptize(w.head_of(1 + (d / 7) % 8).unwrap_or(neighbor)),
            ("preacher", 1) => Activity::Visit(w.head_of(1 + (d / 7) % 8).unwrap_or(neighbor)),
            ("preacher", _) => Activity::Study,
            ("souse", _) => Activity::Drink,
            ("rake", 1 | 3 | 5) => Activity::Court(neighbor),
            ("orator", 2) => Activity::Speech { calm: true },
            ("orator", _) => Activity::Visit(w.head_of(1 + (d % 8)).unwrap_or(neighbor)),
            ("builder", 2 | 4) => Activity::Build(
                Project::ALL
                    .into_iter()
                    .find(|&p| !w.civic.built(p))
                    .unwrap_or(Project::Lyceum),
            ),
            ("builder", 5) => Activity::FileClaim,
            _ => Activity::Chores,
        };
        w.player_do(a);
        w.advance_day();
        if w.day.is_first_of_month() {
            let skills: Vec<String> = Skill::ALL
                .iter()
                .filter(|&&k| w.life.skill(k) > 0.05)
                .map(|&k| format!("{} {:.2}", k.label(), w.life.skill(k)))
                .collect();
            let spouse = romance::married_to(&w, PLAYER).map(|s| w.opinion(s, PLAYER));
            println!(
                "{}  spirits {:>3.0}  rep {:>5.1}  spouse {:>4}  food {:>5.0}  ${:<4} [{}] {}",
                w.day,
                w.life.spirits,
                bleeding_kansas::sim::psyche::reputation(&w, PLAYER),
                spouse.map_or("-".into(), |o| o.to_string()),
                w.families[0].stores.food,
                w.families[0].stores.cash,
                skills.join(", "),
                life::paths(&w).join(", ")
            );
        }
    }
}

fn county(o: &Opts) {
    let mut t = [0usize; 11];
    for seed in 1..=o.seeds {
        let mut w = World::with_winter(seed, o.winter);
        w.run_days(o.days);
        for e in &w.events {
            let i = match e.kind {
                EventKind::Marriage { .. } => 0,
                EventKind::Affair { .. } => 1,
                EventKind::Scandal { .. } => 2,
                EventKind::ClaimJumped { .. } => 3,
                EventKind::ClaimFiled { delayed: true, .. } => 4,
                EventKind::Built { .. } => 5,
                EventKind::FeudEnded { .. } => 6,
                EventKind::SeekerAtDoor { .. } => 7,
                EventKind::Freedom { .. } => 8,
                EventKind::Captured { .. } => 9,
                EventKind::Charged { .. } => 10,
                _ => continue,
            };
            t[i] += 1;
        }
    }
    let n = o.seeds as f32;
    for (label, v) in [
        "weddings",
        "affairs",
        "scandals",
        "claims jumped",
        "papers mislaid",
        "projects built",
        "feuds ended",
        "knocks at a door",
        "reached freedom",
        "taken back",
        "charged",
    ]
    .iter()
    .zip(t)
    {
        println!("{label:<16} {:>5.1} per seed", v as f32 / n);
    }
}

fn farm(o: &Opts) {
    let mut w = World::with_winter(o.seed, o.winter);
    for _ in 0..(o.days / 30).max(1) {
        w.run_days(30);
        println!(
            "{}  {}",
            w.day,
            bleeding_kansas::sim::farmwork::due(w.day).label()
        );
        for f in &w.families {
            if f.store {
                continue;
            }
            let hh = &f.stores;
            println!(
                "   {:<10} planted {:<12} acres {:>2}  standing {:>6.0}  food {:>6.0}  hay {:>4.1}  fences {:>3.0}%  cattle {}",
                f.surname,
                hh.work
                    .planted_on
                    .map_or("-".to_string(), |d| d.to_string()),
                hh.acres,
                hh.work.standing,
                hh.food,
                hh.work.hay,
                hh.work.fences * 100.0,
                hh.cattle
            );
        }
    }
}
