//! Text readouts: the status screen's pages, a neighbor's card, the paper.
//! Nothing here decides anything; it reads the sim and says what a settler
//! could know (never the truth, never private hearts, luck or malice).

use bleeding_kansas::sim::character;
use bleeding_kansas::sim::chronicle;
use bleeding_kansas::sim::chronicle::suspect_label;
use bleeding_kansas::sim::events::Source;
use bleeding_kansas::sim::family;
use bleeding_kansas::sim::farmwork;
use bleeding_kansas::sim::intrigue;
use bleeding_kansas::sim::law;
use bleeding_kansas::sim::life::{self, Skill};
use bleeding_kansas::sim::market::Good;
use bleeding_kansas::sim::psyche;
use bleeding_kansas::sim::world::{Faction, NpcId, PLAYER, World};

/// "3 Mar 1856  [MARKET] Corn up..." set as a run-in head: "MARKET. Corn up..."
pub fn broadsheet(line: &str) -> String {
    let (date, rest) = line.split_at(line.len().min(11));
    match rest
        .trim_start()
        .strip_prefix('[')
        .and_then(|r| r.split_once("] "))
    {
        Some((tag, body)) => format!("{} {}. {}", date.trim(), tag, body),
        None => line.to_string(),
    }
}

/// Whichever paper spoke most recently gets the masthead.
pub fn masthead(world: &World) -> (&'static str, &'static str) {
    let recent = world
        .events
        .iter()
        .rev()
        .take(400)
        .find_map(|e| match e.kind {
            bleeding_kansas::sim::EventKind::Headline { paper, .. }
            | bleeding_kansas::sim::EventKind::Notice { paper, .. } => Some(paper),
            _ => None,
        });
    use bleeding_kansas::sim::history::Paper;
    match recent {
        Some(Paper::HeraldOfFreedom) => ("HERALD OF FREEDOM", "Lawrence, K.T."),
        Some(Paper::KansasFreeState) => ("KANSAS FREE STATE", "Lawrence, K.T."),
        Some(Paper::SquatterSovereign) => ("SQUATTER SOVEREIGN", "Atchison, K.T."),
        None => ("THE COUNTY TALK", "Douglas County, K.T."),
    }
}

/// The default font has no em dashes or arrows.
pub fn plain(text: &str) -> String {
    text.replace('—', "-").replace(['→', '⟶'], "->")
}

/// Lines are written about a man; if it's a woman at the gate, say so.
pub fn gendered(text: &str, woman: bool) -> String {
    if !woman {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    let flush = |w: &mut String, out: &mut String| {
        out.push_str(match w.as_str() {
            "he" => "she",
            "He" => "She",
            "his" | "him" => "her",
            "His" | "Him" => "Her",
            "himself" => "herself",
            other => other,
        });
        w.clear();
    };
    for c in text.chars() {
        if c.is_alphabetic() {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

pub fn bar(v: f32, max: f32) -> String {
    let n = ((v / max) * 10.0).clamp(0.0, 10.0) as usize;
    format!("{}{}", "#".repeat(n), ".".repeat(10 - n))
}

pub fn stance(v: f32) -> &'static str {
    match v {
        v if v <= -0.6 => "fire-eating Pro-Slavery",
        v if v <= -0.2 => "Pro-Slavery",
        v if v < 0.2 => "keeps quiet on the question",
        v if v < 0.6 => "Free-State",
        _ => "hard Free-State",
    }
}

pub fn seems(world: &World, id: NpcId) -> String {
    let e = world.npc(id).emotions;
    let mut out = Vec::new();
    for (v, word) in [
        (e.fear, "afraid"),
        (e.anger, "angry"),
        (e.grief, "grieving"),
        (e.zeal, "zealous"),
    ] {
        if v > 25.0 {
            out.push(word);
        }
    }
    if out.is_empty() {
        "steady".into()
    } else {
        out.join(", ")
    }
}

pub fn inspect(world: &World, id: NpcId) -> String {
    let npc = world.npc(id);
    if !npc.alive {
        return format!("{} (dead)", npc.name);
    }
    let known: Vec<&str> = character::archetypes(npc)
        .into_iter()
        .map(|a| a.label())
        .collect();
    let belief = npc
        .memories
        .iter()
        .filter(|m| m.confidence >= 20)
        .max_by_key(|m| m.day)
        .map(|m| {
            let line = chronicle::debug_line(world, &world.events[m.event as usize], false);
            let what = line
                .split_once("] ")
                .map(|(_, r)| r)
                .unwrap_or(&line)
                .to_string();
            let how = match m.source {
                Source::Witnessed => "saw it",
                Source::Victim => "their loss",
                Source::Bystander => "was nearby",
                Source::Told(_) => "heard it",
                Source::Newspaper(_) => "read it in the paper",
            };
            format!(
                "\n\"{}.\" Blames {} ({}%, {})",
                what,
                suspect_label(world, m.believed),
                m.confidence,
                how
            )
        })
        .unwrap_or_default();
    plain(&format!(
        "{}, {}  ({})\n{}. Health {}. Seems {}.\nSays: {}\nKnown as: {}\nOpinion of you: {}{}",
        npc.name,
        npc.age,
        npc.faction.label(),
        psyche::condition(world, id).label(),
        npc.health,
        seems(world, id),
        stance(npc.ideology.public),
        if known.is_empty() {
            "nothing in particular".into()
        } else {
            known.join(", ")
        },
        world.opinion(id, PLAYER),
        belief
    ))
}

pub fn household(world: &World) -> String {
    let f = &world.families[0];
    let hh = &f.stores;
    let people = world.living().filter(|n| n.family == 0).count().max(1) as f32;
    let kin: Vec<String> = world
        .npcs
        .iter()
        .filter(|n| n.family == 0)
        .map(|n| {
            format!(
                "{} {} ({})",
                if n.id == PLAYER { "You" } else { &n.name },
                n.health,
                psyche::condition(world, n.id).label()
            )
        })
        .collect();
    let mut s = format!(
        "YOUR HOUSEHOLD\n{}\n\nFood: {:.0} days ({:.0} each)\nSeed corn: {} bu   Acres: {}\nCattle: {}   Oxen: {}\nCash: ${}   Debt: ${}\nSalt {:.0}  Powder {:.0}  Timber {:.0}  Hides {:.0}\nBarn: {}",
        kin.join("\n"),
        hh.food,
        hh.food / people,
        hh.seed,
        hh.acres,
        hh.cattle,
        hh.oxen,
        hh.cash,
        hh.debt,
        hh.goods[Good::Salt.index()],
        hh.goods[Good::Powder.index()],
        hh.goods[Good::Timber.index()],
        hh.goods[Good::Hides.index()],
        if f.barn_standing {
            "standing"
        } else {
            "burned"
        },
    );
    let a = &hh.arms;
    s.push_str(&format!(
        "\nArms: {} Sharps, {} old gun{}, {} balls, {} cartridges, {:.0} lb lead, {} rails. Kept {}.{}",
        a.rifles,
        a.guns,
        if a.guns == 1 { "" } else { "s" },
        a.balls,
        a.cartridges,
        a.lead,
        a.rails,
        a.hide.label(),
        if a.on_order.is_some() {
            " A crate of books is coming."
        } else {
            ""
        }
    ));
    let jobs: Vec<String> = bleeding_kansas::sim::hands::workers(world)
        .into_iter()
        .map(|id| {
            format!(
                "{}{} {}",
                world.name(id),
                if world.hands.is_hired(id) {
                    " (hired)"
                } else {
                    ""
                },
                bleeding_kansas::sim::hands::task(world, id).label()
            )
        })
        .collect();
    if !jobs.is_empty() {
        s.push_str(&format!("\nHands: {}.", jobs.join("; ")));
    }
    if let Some(c) = bleeding_kansas::sim::hands::guarded(world) {
        s.push_str(&format!(" {} are in the barn.", c.name));
    }
    let price = bleeding_kansas::sim::warrant::price_on(world, PLAYER);
    if price > 0 {
        s.push_str(&format!("\nWANTED: ${price} on your head."));
    }
    s.push('\n');
    s.push_str(&farmwork::status(world));
    if world.pending_favor.is_some() {
        s.push_str("\n\nDUNMORE WANTS YOUR NAME ON HIS PETITION.");
    }
    s
}

/// The Jones panel: spirits, goals, skills, and what the county calls you.
pub fn your_life(world: &World) -> String {
    let l = &world.life;
    let mut s = format!("YOUR LIFE   spirits {}\n", bar(l.spirits, 100.0));
    let goals: Vec<String> = life::goals(world)
        .iter()
        .map(|(name, v)| format!("{name} {:.0}%", v * 100.0))
        .collect();
    s.push_str(&goals.join("   "));
    s.push('\n');
    let mut skills: Vec<(f32, &str)> = Skill::ALL
        .iter()
        .map(|&k| (l.skill(k), k.label()))
        .filter(|(v, _)| *v > 0.0)
        .collect();
    skills.sort_by(|a, b| b.0.total_cmp(&a.0));
    if !skills.is_empty() {
        let top: Vec<String> = skills
            .iter()
            .take(4)
            .map(|(v, k)| format!("{k} {:.0}", v * 10.0))
            .collect();
        s.push_str(&top.join(", "));
        s.push('\n');
    }
    s.push_str(&format!(
        "Lawrence lots ${:.0}   you hold {}\n",
        world.land.lot_price, world.land.lots
    ));
    let leverage: Vec<&str> = intrigue::leverage(world)
        .into_iter()
        .map(|n| world.name(n))
        .collect();
    if !leverage.is_empty() {
        s.push_str(&format!("You know things about: {}\n", leverage.join(", ")));
    }
    let paths = life::paths(world);
    if !paths.is_empty() {
        s.push_str(&format!("They call you: {}\n", paths.join(", ")));
    }
    if let Some(e) = family::away(world) {
        s.push_str(&format!("You are {}.\n", e.label()));
    } else if l.acted_on == Some(world.day) {
        s.push_str("Your day is spent.\n");
    }
    if let Some((bee, host, _)) = &world.gatherings.today {
        s.push_str(&format!(
            "Tonight: a {} at the {} place.\n",
            bee.label(),
            world.families[*host as usize].surname
        ));
    }
    if let Some(id) = world.railroad.at_door {
        let seeker = &world.railroad.seekers[id as usize];
        s.push_str(&format!(
            "A KNOCK AFTER DARK: {}, from {}, asking to be hidden.\n",
            seeker.name, seeker.from
        ));
    }
    if let Some(e) = law::election_today(world) {
        s.push_str(&format!("ELECTION DAY: {}.\n", e.name));
    }
    if let Some((i, _, _)) = &world.law.muster
        && !world.law.player_answered
    {
        s.push_str(&format!(
            "THE MUSTER IS CALLED: {}.\n",
            law::MUSTERS[*i].name
        ));
    }
    s
}

pub fn market_and_nations(world: &World) -> String {
    let m = &world.market;
    let mut s = String::from("DUNMORE'S, FRANKLIN\n");
    for g in Good::ALL {
        let arrow = match m.pressure(g) {
            p if p >= 1.5 => " ^",
            p if p <= 0.7 => " v",
            _ => "",
        };
        s.push_str(&format!(
            "{:<14} ${:>5.2}/{:<6} stock {:>4.0}{}\n",
            g.label(),
            m.price(g),
            g.unit(),
            m.stock(g),
            arrow
        ));
    }
    if m.blockade {
        s.push_str("The roads from Westport are closed\n");
    }
    s
}

pub fn nations(world: &World) -> String {
    let herd = world.bison.abundance();
    let mut s = format!(
        "BUFFALO RANGE, WEST\n{} {}\n\nNATIONS\npressure, trust F-S / P-S\n",
        bar(herd, 1.0),
        if herd > 0.6 {
            "herds plenty"
        } else if herd > 0.3 {
            "herds thinning"
        } else {
            "bones on the prairie"
        }
    );
    for n in &world.nations {
        s.push_str(&format!(
            "{:<9} {}  {:.0} / {:.0}\n",
            n.id.name(),
            bar(n.land_pressure, 100.0),
            n.trust[Faction::FreeState.index()],
            n.trust[Faction::ProSlavery.index()],
        ));
    }
    s
}
