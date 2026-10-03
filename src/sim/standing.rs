//! Standing: whose word counts, who gets suspected, who gets credit, whose
//! claim holds. Nobody in the county says it out loud, and every system
//! consults it.
//!
//! Standing is a product, not a sum (Crenshaw): a widow is a woman *and*
//! alone at law; a poor unlettered newcomer is each of those, multiplied.
//! Where the positions meet, the escape routes close. Nobody vouches for the
//! newcomer, the unlettered can't read the paper sworn against them, and the
//! poor can't pay the costs, so they work them off in jail.
//!
//! Period grounds, in brief:
//! - **Women** could not vote or sit on juries in the Territory. Coverture
//!   put a wife's property under her husband. Her testimony was admissible
//!   but weighed lightly. The same prejudice made a woman an unlikely
//!   suspect for arson or a shooting.
//! - **Widows** could claim as heads of family under the 1841 Preemption
//!   Act, and claim jumpers went for them first.
//! - **The unlettered** made their mark. They depended on a neighbor to read
//!   the mail and the notice (`mail`), and they filed claims less (`civic`).
//! - **The poor** and those seen begging were the first suspects for any
//!   theft (§10). The store gave credit on a man's name, and a poor man's
//!   name was worth less.
//! - **Newcomers** had no neighbor to vouch for them. The county was
//!   filling with strangers in the 1856 emigration.
//! - **A record** followed a man: once convicted, he was a jailbird.
//! - **The other side on your road**: a Free-State family among Missourians
//!   (or the reverse) had no one close to back its word.
//!
//! Read by: `systems::gossip_system` (credibility of the teller: Fricker's
//! testimonial injustice), `attribution::candidates` ("no account" as a
//! blame term), `warrant` (whose oath counts, how few oaths it takes against
//! whom, conviction, costs worked off in jail), `economy` (the store's
//! credit line), `civic` (who claim jumpers pick), `law::odds` (who wins
//! the suit) and `sickness::tend` (who the neighbor women sit up with).
//!
//! The player never sees the number (rule 3). `lab standing` shows it.

use super::economy::CREDIT_LIMIT;
use super::psyche::LifeStage;
use super::world::{Faction, NpcId, World, distance, is_woman};

/// Days a new face stays new.
pub const NEWCOMER_DAYS: u32 = 120;

/// One position a person holds, and what it does to them: `word` is how
/// far their testimony carries (gossip, oaths, court, the store's ledger);
/// `repute` is how much benefit of the doubt they get as a suspect. The two
/// part ways: a woman's word was discounted, yet she was seldom suspected
/// of firing a barn; a beggar is believed less *and* suspected more.
#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub name: &'static str,
    pub word: f32,
    pub repute: f32,
}

const fn pos(name: &'static str, word: f32, repute: f32) -> Position {
    Position { name, word, repute }
}

/// Every position a person holds today, for the lab and the two products.
pub fn parts(world: &World, id: NpcId) -> Vec<Position> {
    let n = world.npc(id);
    let mut out = Vec::new();
    let stage = LifeStage::of(n.age);
    if stage == LifeStage::Child {
        out.push(pos("a child", 0.35, 1.0));
    }
    let woman = is_woman(&n.name);
    if woman && stage != LifeStage::Child {
        out.push(pos("a woman", 0.7, 1.0));
        let widowed = super::romance::spouses(world, id).any(|s| !world.npc(s).alive);
        if widowed && super::romance::married_to(world, id).is_none() {
            out.push(pos("a widow", 0.85, 1.0));
        }
    }
    if stage != LifeStage::Child && n.temperament.literacy < 0.3 {
        out.push(pos("makes a mark for a name", 0.85, 0.95));
    }
    if let Some(f) = world.families.get(n.family as usize) {
        let s = &f.stores;
        if s.visibly_hungry(world.day) {
            out.push(pos("seen begging", 0.75, 0.6));
        } else if s.debt >= CREDIT_LIMIT / 2 {
            out.push(pos("deep in debt", 0.85, 0.85));
        }
    }
    if stage != LifeStage::Child && n.arrived.is_some_and(|d| world.day.0 < d.0 + NEWCOMER_DAYS) {
        out.push(pos("new to the county", 0.8, 0.75));
    }
    if world.warrants.record.contains(&id) {
        out.push(pos("a jailbird", 0.6, 0.55));
    }
    if super::freight::short_weight(world, id) {
        out.push(pos("gave short weight", 0.8, 0.85));
    }
    if world.hands.hired.iter().any(|h| h.npc == id) {
        out.push(pos("a hired man", 0.85, 0.85));
    }
    if outnumbered(world, id) {
        out.push(pos("the other side all round", 0.85, 0.8));
    }
    // A man of property with his letters and no debt: the county listens,
    // and looks elsewhere.
    if let Some(f) = world.families.get(n.family as usize)
        && stage == LifeStage::Adult
        && !woman
        && f.stores.debt == 0
        && f.stores.acres >= 20
        && n.temperament.literacy >= 0.6
    {
        out.push(pos("a man of property", 1.15, 1.15));
    }
    out
}

/// How far a person's word carries: 0.05 (a hungry, unlettered widow new to
/// the county) .. 1.2. Gossip, oaths, the court, the store's ledger.
pub fn word(world: &World, id: NpcId) -> f32 {
    parts(world, id)
        .iter()
        .map(|p| p.word)
        .product::<f32>()
        .clamp(0.05, 1.2)
}

/// How much benefit of the doubt a person gets as a suspect: 0.1 .. 1.2.
/// Blame, how few oaths it takes for a paper, and conviction.
pub fn repute(world: &World, id: NpcId) -> f32 {
    parts(world, id)
        .iter()
        .map(|p| p.repute)
        .product::<f32>()
        .clamp(0.1, 1.2)
}

/// The three nearest farming families are the other side's.
fn outnumbered(world: &World, id: NpcId) -> bool {
    let n = world.npc(id);
    let Some(home) = world.families.get(n.family as usize) else {
        return false;
    };
    let mut near: Vec<(f32, Faction)> = world
        .families
        .iter()
        .filter(|f| f.id != home.id && f.farms() && world.head_of(f.id).is_some())
        .map(|f| (distance(f.farm, home.farm), f.faction))
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    near.len() >= 3 && near.iter().take(3).all(|x| x.1 != n.faction)
}

/// A person's standing in words, for the lab (never for the view).
pub fn describe(world: &World, id: NpcId) -> String {
    let words: Vec<String> = parts(world, id)
        .iter()
        .map(|p| format!("{} (word ×{}, repute ×{})", p.name, p.word, p.repute))
        .collect();
    format!(
        "word {:.2}  repute {:.2}{}",
        word(world, id),
        repute(world, id),
        if words.is_empty() {
            String::new()
        } else {
            format!("  {}", words.join(", "))
        }
    )
}

/// What the county did to each position, per person-day of holding it:
/// `[exposure days, wrongly blamed, papers, convicted, claims jumped]`.
/// Row 0 is grown people holding none of them. Runs `seeds` counties for
/// `days` and walks each day's events (`lab standing`, the disparity test).
pub fn disparity(seeds: u64, days: u32) -> Vec<(&'static str, [f64; 5])> {
    use super::events::{EventKind, Suspect};
    let mut rows: Vec<(&'static str, [f64; 5])> = vec![("none of these", [0.0; 5])];
    let bump = |rows: &mut Vec<(&'static str, [f64; 5])>, name: &'static str, c: usize| match rows
        .iter_mut()
        .find(|r| r.0 == name)
    {
        Some(r) => r.1[c] += 1.0,
        None => {
            let mut v = [0.0; 5];
            v[c] = 1.0;
            rows.push((name, v));
        }
    };
    for seed in 1..=seeds {
        let mut w = World::new(seed);
        let mut seen = w.events.len();
        for _ in 0..days {
            w.advance_day();
            let grown = |w: &World, id: NpcId| {
                id != super::PLAYER && LifeStage::of(w.npc(id).age) != LifeStage::Child
            };
            let held: Vec<Vec<&'static str>> = (0..w.npcs.len())
                .map(|i| {
                    let mut v: Vec<_> = parts(&w, i as NpcId)
                        .into_iter()
                        .map(|p| p.name)
                        .filter(|p| *p != "a man of property")
                        .collect();
                    if v.is_empty() {
                        v.push("none of these");
                    }
                    v
                })
                .collect();
            for n in w.living() {
                if grown(&w, n.id) {
                    for &name in &held[n.id as usize] {
                        bump(&mut rows, name, 0);
                    }
                }
            }
            for e in &w.events[seen..] {
                let (who, col) = match e.kind {
                    EventKind::Belief {
                        about,
                        blamed: Suspect::Person(x),
                        ..
                    } if super::warrant::truth(&w, about) != Some(x)
                        && !matches!(w.events[about as usize].kind,
                            EventKind::Theft { thief, .. } if thief == x) =>
                    {
                        (x, 1)
                    }
                    EventKind::Warrant { accused, .. } => (accused, 2),
                    EventKind::Tried {
                        accused,
                        convicted: true,
                        ..
                    } => (accused, 3),
                    EventKind::ClaimJumped { family, lost, .. } if lost > 0 => {
                        match w.head_of(family) {
                            Some(h) => (h, 4),
                            None => continue,
                        }
                    }
                    _ => continue,
                };
                if grown(&w, who) {
                    for &name in &held[who as usize] {
                        bump(&mut rows, name, col);
                    }
                }
            }
            seen = w.events.len();
        }
    }
    rows
}

/// Per 100 person-years.
pub fn rate(row: &[f64; 5], col: usize) -> f64 {
    36500.0 * row[col] / row[0].max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::PLAYER;

    fn a_woman(w: &World) -> NpcId {
        w.living()
            .find(|n| {
                n.id != PLAYER && is_woman(&n.name) && LifeStage::of(n.age) == LifeStage::Adult
            })
            .unwrap()
            .id
    }

    #[test]
    fn disadvantage_multiplies_not_adds() {
        let mut w = World::new(5);
        let id = a_woman(&w);
        w.npc_mut(id).temperament.literacy = 0.9;
        let f = w.npc(id).family as usize;
        w.families[f].stores.debt = 0;
        w.families[f].stores.hungry_days = 0;
        w.families[f].stores.begged_on = None;
        let alone = word(&w, id);
        w.npc_mut(id).temperament.literacy = 0.1;
        w.families[f].stores.hungry_days = 3;
        w.npc_mut(id).arrived = Some(w.day);
        let all = word(&w, id);
        // 0.7 → 0.7 × 0.85 × 0.75 × 0.8: each weight falls on what's left.
        assert!(
            (all - alone * 0.85 * 0.75 * 0.8).abs() < 0.01,
            "{alone} {all}"
        );
        assert!(all < 0.4);
    }

    #[test]
    fn newcomers_stop_being_new() {
        let mut w = World::new(5);
        let id = w.add_npc(2, "Levi", 30);
        assert!(parts(&w, id).iter().any(|p| p.name == "new to the county"));
        w.day = crate::sim::Day(w.day.0 + NEWCOMER_DAYS);
        assert!(!parts(&w, id).iter().any(|p| p.name == "new to the county"));
    }

    #[test]
    fn a_record_follows_a_man() {
        let mut w = World::new(5);
        let id = w.head_of(3).unwrap();
        let before = (word(&w, id), repute(&w, id));
        w.warrants.record.push(id);
        assert!(word(&w, id) < before.0 * 0.7);
        assert!(repute(&w, id) < before.1 * 0.6);
    }

    #[test]
    fn a_womans_word_is_discounted_but_she_is_not_suspected_for_it() {
        let w = World::new(5);
        let id = a_woman(&w);
        let p = parts(&w, id);
        let woman = p.iter().find(|p| p.name == "a woman").unwrap();
        assert!(woman.word < 1.0);
        assert_eq!(woman.repute, 1.0);
    }

    /// The county leans on the ones nobody vouches for. Across seeds, on
    /// the steady signal (thousands of beliefs a run): the poor are blamed
    /// for what they didn't do more than grown people with none of these
    /// positions; women less (the same prejudice that discounts their word).
    /// Papers and jumped claims are rarer; `lab standing` over 30 seeds
    /// shows them.
    #[test]
    fn the_county_leans_on_those_nobody_vouches_for() {
        let rows = disparity(10, 500);
        let get = |name: &str| rows.iter().find(|r| r.0 == name).map(|r| r.1);
        let none = get("none of these").unwrap();
        let woman = get("a woman").unwrap();
        assert!(
            rate(&woman, 1) < 0.6 * rate(&none, 1),
            "women blamed {} vs {}",
            rate(&woman, 1),
            rate(&none, 1)
        );
        let mut poor = [0.0; 5];
        for name in ["seen begging", "deep in debt"] {
            if let Some(r) = get(name) {
                for (p, x) in poor.iter_mut().zip(r) {
                    *p += x;
                }
            }
        }
        assert!(poor[0] > 0.0, "nobody poor in ten counties");
        assert!(
            rate(&poor, 1) > 1.2 * rate(&none, 1),
            "the poor blamed {} vs {}",
            rate(&poor, 1),
            rate(&none, 1)
        );
    }
}
