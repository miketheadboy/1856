//! Headless runner (§21): no engine, just the machine working.
//!
//!   cargo run --bin headless --no-default-features -- [options]
//!
//!   --seed N       world seed (default 1856)
//!   --days N       days to simulate (default 365)
//!   --truth        show true causes in the chronicle (the player never sees these)
//!   --trees N      print cascade trees for the first N harms (fires/deaths)
//!   --survey N     run seeds 1..=N and report how often wars start from nothing

use bleeding_kansas::sim::chronicle;
use bleeding_kansas::sim::{EventKind, FireCause, Suspect, World};

struct Args {
    seed: u64,
    days: u32,
    truth: bool,
    trees: usize,
    survey: Option<u64>,
}

fn parse_args() -> Args {
    let mut args = Args {
        seed: 1856,
        days: 365,
        truth: false,
        trees: 0,
        survey: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut num = || it.next().and_then(|v| v.parse::<u64>().ok());
        match flag.as_str() {
            "--seed" => args.seed = num().unwrap_or(args.seed),
            "--days" => args.days = num().map(|d| d as u32).unwrap_or(args.days),
            "--truth" => args.truth = true,
            "--trees" => args.trees = num().unwrap_or(3) as usize,
            "--survey" => args.survey = Some(num().unwrap_or(50)),
            "-h" | "--help" => {
                println!(
                    "usage: headless [--seed N] [--days N] [--truth] [--trees N] [--survey N]"
                );
                std::process::exit(0);
            }
            other => eprintln!("ignoring unknown flag {other}"),
        }
    }
    args
}

struct Stats {
    fires: [usize; 4],
    deaths: usize,
    feuds: usize,
    wars_from_nothing: usize,
    victim_beliefs: usize,
    victim_wrong: usize,
}

fn stats(w: &World) -> Stats {
    let mut s = Stats {
        fires: [0; 4],
        deaths: 0,
        feuds: 0,
        wars_from_nothing: chronicle::wars_nobody_started(w).len(),
        victim_beliefs: 0,
        victim_wrong: 0,
    };
    for ev in &w.events {
        match ev.kind {
            EventKind::Fire { cause, .. } => {
                let i = match cause {
                    FireCause::Lightning => 0,
                    FireCause::Hearth => 1,
                    FireCause::EscapedBurn => 2,
                    FireCause::Arson(_) => 3,
                };
                s.fires[i] += 1;
            }
            EventKind::Death { .. } => s.deaths += 1,
            EventKind::FeudDeclared { .. } => s.feuds += 1,
            EventKind::Belief {
                about,
                blamed,
                source: bleeding_kansas::sim::events::Source::Victim,
                ..
            } => {
                s.victim_beliefs += 1;
                let truth = match w.events[about as usize].kind {
                    EventKind::Fire { cause, .. } => cause.truth(),
                    EventKind::Death { killer, .. } => {
                        killer.map(Suspect::Person).unwrap_or(Suspect::Accident)
                    }
                    _ => continue,
                };
                if truth != blamed {
                    s.victim_wrong += 1;
                }
            }
            _ => {}
        }
    }
    s
}

fn print_stats(w: &World) {
    let s = stats(w);
    println!("\n=== {} days, seed {} ===", w.day.0, w.seed);
    println!(
        "fires: {} lightning, {} hearth, {} escaped burn, {} arson",
        s.fires[0], s.fires[1], s.fires[2], s.fires[3]
    );
    println!("deaths: {}   feuds: {}", s.deaths, s.feuds);
    if s.victim_beliefs > 0 {
        println!(
            "victims who blamed the wrong cause: {}/{} ({:.0}%)",
            s.victim_wrong,
            s.victim_beliefs,
            100.0 * s.victim_wrong as f32 / s.victim_beliefs as f32
        );
    }
    println!(
        "events: {}   relationships stored: {}   cascades truncated: {}",
        w.events.len(),
        w.relationship_count(),
        w.truncated_cascades
    );
    println!("living: {}/{}", w.living().count(), w.npcs.len());
}

fn survey(n: u64, days: u32) {
    let mut with_war = 0;
    let mut totals = (0usize, 0usize, 0usize);
    let mut law = (0usize, 0usize, 0usize);
    for seed in 1..=n {
        let mut w = World::new(seed);
        w.run_days(days);
        let s = stats(&w);
        if s.wars_from_nothing > 0 {
            with_war += 1;
        }
        totals.0 += s.deaths;
        totals.1 += s.feuds;
        totals.2 += s.fires.iter().sum::<usize>();
        let l = bleeding_kansas::sim::warrant::ledger(&w);
        law.0 += l.papers;
        law.1 += l.wrong_man;
        law.2 += l.shot_by_law + l.law_shot;
        println!(
            "seed {:>3}: fires {:>2} (arson {:>2})  deaths {:>2}  feuds {:>2}  wars-from-nothing {}  papers {:>2} (wrong man {:>2})",
            seed,
            s.fires.iter().sum::<usize>(),
            s.fires[3],
            s.deaths,
            s.feuds,
            s.wars_from_nothing,
            l.papers,
            l.wrong_man
        );
    }
    println!(
        "\n{}/{} seeds started a war nobody started. avg per seed: {:.1} fires, {:.1} deaths, {:.1} feuds",
        with_war,
        n,
        totals.2 as f32 / n as f32,
        totals.0 as f32 / n as f32,
        totals.1 as f32 / n as f32
    );
    println!(
        "the law: {:.1} papers per seed, {:.0}% on the wrong man, {:.1} shot serving them",
        law.0 as f32 / n as f32,
        100.0 * law.1 as f32 / law.0.max(1) as f32,
        law.2 as f32 / n as f32
    );
}

fn main() {
    // Piping into `head` closes stdout early; exit quietly instead of panicking.
    std::panic::set_hook(Box::new(|info| {
        if info.to_string().contains("Broken pipe") {
            std::process::exit(0);
        }
        eprintln!("{info}");
    }));
    let args = parse_args();
    if let Some(n) = args.survey {
        survey(n, args.days);
        return;
    }

    let mut w = World::new(args.seed);
    println!("BLEEDING KANSAS — headless run, seed {}", args.seed);
    for f in w.families.iter().skip(1) {
        let members: Vec<_> = w
            .npcs
            .iter()
            .filter(|n| n.family == f.id)
            .map(|n| n.name.as_str())
            .collect();
        println!(
            "  {:<8} {:<11} farm {:?}: {}",
            f.surname,
            f.faction.label(),
            f.farm,
            members.join(", ")
        );
    }
    println!();

    w.run_days(args.days);

    let mut month = String::new();
    for line in chronicle::chronicle(&w, args.truth) {
        let label = line[3..11].trim().to_string();
        if label != month {
            println!("\n— {} —", label);
            month = label;
        }
        println!("{line}");
    }

    let wars = chronicle::wars_nobody_started(&w);
    if !wars.is_empty() {
        println!("\n=== WARS NOBODY STARTED ===");
        for (feud, _) in wars.iter().take(3) {
            println!();
            for line in chronicle::lineage(&w, *feud) {
                println!("  {line}");
            }
        }
    }

    if args.trees > 0 {
        println!("\n=== CASCADE TREES ===");
        let roots: Vec<_> = w
            .events
            .iter()
            .filter(|e| e.origin().is_none())
            .filter(|e| matches!(e.kind, EventKind::Fire { .. } | EventKind::Death { .. }))
            .map(|e| e.id)
            .take(args.trees)
            .collect();
        for root in roots {
            println!("\n{}", chronicle::cascade_tree(&w, root, 60));
        }
    }

    print_stats(&w);
}
