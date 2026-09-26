//! Observability. When something inexplicable happens (and it will, that's
//! the point), these are how you tell emergence from a bug (§15.4).
//!
//! - `DailyMetrics`: one row per day, exportable as CSV for charts.
//! - `trace_tsv`: every event with its cascade links.
//! - `explain_*`: a person, a blame decision, a price, laid bare.

use std::fmt::Write;

use super::attribution;
use super::character;
use super::chronicle::{self, suspect_label};
use super::events::{EventId, EventKind};
use super::market::Good;
use super::psyche;
use super::world::{Faction, NpcId, PLAYER, World};

#[derive(Clone, Debug, Default)]
pub struct DailyMetrics {
    pub day: u32,
    pub grievance_free: i32,
    pub grievance_pro: i32,
    pub living: usize,
    pub hungry_families: usize,
    pub fear: f32,
    pub anger: f32,
    pub grief: f32,
    pub zeal: f32,
    pub corn: f32,
    pub powder: f32,
    pub hides: f32,
    pub herd: f32,
    pub feuds: usize,
    pub events: usize,
    pub truncated: u32,
}

pub const CSV_HEADER: &str = "day,date,grievance_free,grievance_pro,living,hungry_families,fear,anger,grief,zeal,corn,powder,hides,herd,feuds,events,truncated";

pub fn snapshot(world: &World) -> DailyMetrics {
    let living: Vec<_> = world.living().collect();
    let n = living.len().max(1) as f32;
    let mean =
        |f: fn(&psyche::Emotions) -> f32| living.iter().map(|p| f(&p.emotions)).sum::<f32>() / n;
    DailyMetrics {
        day: world.day.0,
        grievance_free: world.grievance[Faction::FreeState.index()],
        grievance_pro: world.grievance[Faction::ProSlavery.index()],
        living: living.len(),
        hungry_families: world
            .families
            .iter()
            .filter(|f| f.farms() && f.stores.hungry_days > 0)
            .count(),
        fear: mean(|e| e.fear),
        anger: mean(|e| e.anger),
        grief: mean(|e| e.grief),
        zeal: mean(|e| e.zeal),
        corn: world.market.price(Good::Corn),
        powder: world.market.price(Good::Powder),
        hides: world.market.price(Good::Hides),
        herd: world.bison.population,
        feuds: world.feuds.len(),
        events: world.events.len(),
        truncated: world.truncated_cascades,
    }
}

pub fn metrics_csv(world: &World) -> String {
    let mut out = String::from(CSV_HEADER);
    out.push('\n');
    for m in &world.metrics {
        let _ = writeln!(
            out,
            "{},{},{},{},{},{},{:.1},{:.1},{:.1},{:.1},{:.2},{:.2},{:.2},{:.0},{},{},{}",
            m.day,
            super::Day(m.day),
            m.grievance_free,
            m.grievance_pro,
            m.living,
            m.hungry_families,
            m.fear,
            m.anger,
            m.grief,
            m.zeal,
            m.corn,
            m.powder,
            m.hides,
            m.herd,
            m.feuds,
            m.events,
            m.truncated
        );
    }
    out
}

/// Every event, one per line: id, day, depth, parent, caused_by, what (truth shown).
pub fn trace_tsv(world: &World) -> String {
    let mut out = String::from("id\tday\tdepth\tparent\tcaused_by\tevent\n");
    for ev in &world.events {
        let _ = writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}",
            ev.id,
            ev.day.0,
            ev.cascade_depth,
            ev.parent.map_or("-".into(), |p| p.to_string()),
            ev.caused_by.map_or("-".into(), |p| p.to_string()),
            chronicle::debug_line(world, ev, true)
        );
    }
    out
}

pub fn find_npc(world: &World, query: &str) -> Option<NpcId> {
    if let Ok(id) = query.parse::<NpcId>() {
        return ((id as usize) < world.npcs.len()).then_some(id);
    }
    let q = query.to_lowercase();
    world
        .npcs
        .iter()
        .find(|n| n.name.to_lowercase().contains(&q))
        .map(|n| n.id)
}

fn pct(v: f32) -> String {
    format!("{:>3.0}", v * 100.0)
}

/// Everything about a person, including what the player never sees.
pub fn explain_npc(world: &World, id: NpcId) -> String {
    let n = world.npc(id);
    let t = n.temperament;
    let b = n.body;
    let e = n.emotions;
    let hh = &world.families[n.family as usize].stores;
    let mut s = String::new();
    let _ = writeln!(
        s,
        "#{} {} ({}), age {}, {} — {}",
        n.id,
        n.name,
        n.faction.label(),
        n.age,
        psyche::condition(world, id).label(),
        if n.alive { "alive" } else { "dead" }
    );
    let _ = writeln!(
        s,
        "health {}  wounded {}  violence {}  plotting {:?}",
        n.health,
        n.wounded,
        n.violence,
        n.plotting.map(|p| world.name(p).to_string())
    );
    let _ = writeln!(
        s,
        "BODY      str {} hardy {} aim {} stealth {} alert {} recall {}  (frailty {:.2})",
        pct(b.strength),
        pct(b.hardiness),
        pct(b.marksmanship),
        pct(b.stealth),
        pct(b.alertness),
        pct(b.recall),
        b.frailty()
    );
    let _ = writeln!(
        s,
        "TEMPER    temper {} courage {} piety {} generous {} honest {} social {} skeptic {} loyal {} literate {}",
        pct(t.temper),
        pct(t.courage),
        pct(t.piety),
        pct(t.generosity),
        pct(t.honesty),
        pct(t.sociability),
        pct(t.skepticism),
        pct(t.loyalty),
        pct(t.literacy)
    );
    let _ = writeln!(
        s,
        "EMOTIONS  fear {:.0} anger {:.0} grief {:.0} zeal {:.0}",
        e.fear, e.anger, e.grief, e.zeal
    );
    let _ = writeln!(
        s,
        "IDEOLOGY  public {:+.2} private {:+.2} (hypocrisy {:.2})",
        n.ideology.public,
        n.ideology.private,
        n.ideology.hypocrisy()
    );
    let _ = writeln!(
        s,
        "HIDDEN    luck {:.2} malice {:.2}",
        n.hidden.luck, n.hidden.malice
    );
    let arche: Vec<_> = character::archetypes(n).iter().map(|a| a.label()).collect();
    let _ = writeln!(
        s,
        "ARCHETYPE {}",
        if arche.is_empty() {
            "-".into()
        } else {
            arche.join(", ")
        }
    );
    let _ = writeln!(
        s,
        "DRIVES    revenge {:+.2} pliability {:+.2} would-steal {:.2} reputation {:+.1}",
        psyche::revenge_drive(world, id),
        psyche::pliability(world, id),
        psyche::would_steal(world, id),
        psyche::reputation(world, id)
    );
    let _ = writeln!(
        s,
        "HOUSEHOLD food {:.0}d seed {} acres {} cattle {} oxen {} cash ${} debt ${} hungry {}d prudence {:.2} proud {} impulsive {}",
        hh.food,
        hh.seed,
        hh.acres,
        hh.cattle,
        hh.oxen,
        hh.cash,
        hh.debt,
        hh.hungry_days,
        hh.prudence,
        hh.proud,
        hh.impulsive
    );

    let mut ops: Vec<(NpcId, i16)> = world
        .living()
        .filter(|o| o.id != id)
        .map(|o| (o.id, world.opinion(id, o.id)))
        .collect();
    ops.sort_by_key(|o| o.1);
    let fmt = |v: &[(NpcId, i16)]| {
        v.iter()
            .map(|(o, x)| {
                format!(
                    "{} {:+}",
                    if *o == PLAYER { "you" } else { world.name(*o) },
                    x
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let _ = writeln!(s, "HATES     {}", fmt(&ops[..ops.len().min(4)]));
    let tail = ops.len().saturating_sub(4);
    let _ = writeln!(s, "LIKES     {}", fmt(&ops[tail..]));

    let _ = writeln!(
        s,
        "MEMORIES ({}/{}):",
        n.memories.len(),
        n.memory_capacity()
    );
    for m in n.memories.iter().rev().take(8) {
        let ev = &world.events[m.event as usize];
        let truth = truth_of(world, m.event);
        let _ = writeln!(
            s,
            "  {} {} -> blames {} ({}%, {:?}, w{}){}",
            ev.day,
            chronicle::debug_line(world, ev, false),
            suspect_label(world, m.believed),
            m.confidence,
            m.source,
            m.weight,
            truth
                .map(|t| format!("  [truth: {}]", t))
                .unwrap_or_default()
        );
    }
    s
}

fn truth_of(world: &World, event: EventId) -> Option<String> {
    let actor = match world.events[event as usize].kind {
        EventKind::Fire { cause, .. } => return Some(suspect_label(world, cause.truth())),
        EventKind::Death { killer, .. } => killer,
        EventKind::Theft { thief, .. } => Some(thief),
        EventKind::Wounded { attacker, .. } => Some(attacker),
        EventKind::Cruelty { actor, .. } => Some(actor),
        _ => None,
    };
    actor.map(|a| {
        if a == PLAYER {
            "you".into()
        } else {
            world.name(a).to_string()
        }
    })
}

/// The attribution engine's reasoning for one observer and one event,
/// every candidate with every term (§11.5: blame must be legible).
pub fn explain_blame(world: &World, observer: NpcId, event: EventId) -> String {
    let cands = attribution::candidates(world, observer, event, None);
    let probs = attribution::beliefs(&cands);
    let mut rows: Vec<_> = cands.iter().zip(probs).collect();
    rows.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut s = format!(
        "{} judging: {}\n",
        world.name(observer),
        chronicle::debug_line(world, &world.events[event as usize], true)
    );
    for (c, p) in rows.iter().take(8) {
        let parts: Vec<String> = c
            .parts
            .iter()
            .filter(|(_, v)| v.abs() > 0.05)
            .map(|(k, v)| format!("{k} {v:+.0}"))
            .collect();
        let _ = writeln!(
            s,
            "  {:>5.1}%  score {:>6.1}  {:<22} {}  [{}]",
            p * 100.0,
            c.score,
            suspect_label(world, c.suspect),
            c.reason,
            parts.join(" ")
        );
    }
    s
}

pub fn explain_price(world: &World, g: Good) -> String {
    let st = world.market.goods[g.index()];
    let hist: Vec<String> = world
        .market
        .history
        .iter()
        .rev()
        .take(12)
        .rev()
        .map(|p| format!("{:.2}", p[g.index()]))
        .collect();
    format!(
        "{}: ${:.2}/{} (x{:.2} normal), stock {:.0}{}\n  last 12 weeks: {}",
        g.label(),
        st.price,
        g.unit(),
        world.market.pressure(g),
        st.stock,
        if world.market.blockade {
            ", BLOCKADE"
        } else {
            ""
        },
        hist.join(" ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_are_recorded_daily_and_export() {
        let mut w = World::new(2);
        w.run_days(30);
        assert_eq!(w.metrics.len(), 30);
        let csv = metrics_csv(&w);
        assert_eq!(csv.lines().count(), 31);
        assert!(csv.starts_with("day,"));
    }

    #[test]
    fn trace_has_a_line_per_event() {
        let mut w = World::new(2);
        w.run_days(60);
        assert_eq!(trace_tsv(&w).lines().count(), w.events.len() + 1);
    }

    #[test]
    fn explain_blame_lists_candidates_with_parts() {
        let mut w = World::new(2);
        let owner = w.head_of(1).unwrap();
        let fire = w.emit_root(
            EventKind::Fire {
                owner,
                cause: super::super::FireCause::Hearth,
                spread_from: None,
            },
            None,
        );
        let text = explain_blame(&w, owner, fire);
        assert!(text.contains("hostility") || text.contains("proximity"));
        assert!(text.lines().count() > 2);
    }

    #[test]
    fn find_npc_by_name_or_id() {
        let w = World::new(2);
        assert_eq!(find_npc(&w, "0"), Some(0));
        let name = w.npc(3).name.clone();
        assert_eq!(find_npc(&w, &name.to_lowercase()), Some(3));
    }
}
