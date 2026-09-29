//! Hearts (ext. §13). Couples, courtship, seduction and what it costs.
//!
//! Marriage is politics by other means: a match across a feud ends it, and a
//! discovered affair can start one. Most settlers came married; the single
//! are the grown children and the widowed, and a widow did not stay one long
//! on the frontier. Affairs are secret acts: only the omniscient chronicle
//! sees them until someone is caught.

use super::calendar::Day;
use super::events::{EventKind, WorldEvent};
use super::psyche::LifeStage;
use super::world::{FamilyId, NpcId, PLAYER, World, is_woman};

/// Affection at which a courtship becomes a proposal (or an affair).
pub const SWEETHEARTS: f32 = 60.0;

#[derive(Clone, Debug, Default)]
pub struct Hearts {
    /// Married pairs, (a, b) with a < b.
    pub couples: Vec<(NpcId, NpcId)>,
    /// One-way affection: (suitor, beloved) -> 0..100. A Vec, not a map:
    /// iteration order must be deterministic (CLAUDE.md rule 4).
    pub affection: Vec<(NpcId, NpcId, f32)>,
    /// Secret affairs, and the day they began.
    pub affairs: Vec<(NpcId, NpcId, Day)>,
}

impl Hearts {
    /// The first two adults of each household are the couple; your spouse
    /// is the other adult in yours.
    pub fn founding(world: &World) -> Self {
        let mut couples = Vec::new();
        for f in &world.families {
            let adults: Vec<NpcId> = world
                .npcs
                .iter()
                .filter(|n| n.family == f.id && LifeStage::of(n.age) != LifeStage::Child)
                .map(|n| n.id)
                .take(2)
                .collect();
            if let [a, b] = adults[..] {
                couples.push((a.min(b), a.max(b)));
            }
        }
        Hearts {
            couples,
            ..Default::default()
        }
    }

    pub fn spouse_of(&self, id: NpcId) -> Option<NpcId> {
        self.couples.iter().find_map(|&(a, b)| {
            if a == id {
                Some(b)
            } else if b == id {
                Some(a)
            } else {
                None
            }
        })
    }

    pub fn affection(&self, from: NpcId, to: NpcId) -> f32 {
        self.affection
            .iter()
            .find(|&&(a, b, _)| a == from && b == to)
            .map_or(0.0, |x| x.2)
    }

    pub fn add_affection(&mut self, from: NpcId, to: NpcId, delta: f32) -> f32 {
        if let Some(e) = self.affection.iter_mut().find(|e| e.0 == from && e.1 == to) {
            e.2 = (e.2 + delta).clamp(0.0, 100.0);
            return e.2;
        }
        let v = delta.clamp(0.0, 100.0);
        self.affection.push((from, to, v));
        v
    }

    pub fn in_affair(&self, a: NpcId, b: NpcId) -> bool {
        self.affairs
            .iter()
            .any(|&(x, y, _)| (x == a && y == b) || (x == b && y == a))
    }
}

/// A living spouse, if any.
pub fn married_to(world: &World, id: NpcId) -> Option<NpcId> {
    world.hearts.spouse_of(id).filter(|&s| world.npc(s).alive)
}

/// How much a person resists a suitor: faith, honesty, and a living spouse.
pub fn fidelity(world: &World, id: NpcId) -> f32 {
    let t = &world.npc(id).temperament;
    let bound = if married_to(world, id).is_some() {
        1.0
    } else {
        0.3
    };
    (0.4 * t.piety + 0.4 * t.honesty + 0.2 * t.loyalty) * bound
}

/// Charm: talk, nerve, and luck.
pub fn charm(world: &World, id: NpcId) -> f32 {
    let n = world.npc(id);
    // A boiled shirt helps; rags don't.
    let dressed = n.outfit.bonus().charm - 0.2 * n.outfit.wear;
    0.5 * n.temperament.sociability
        + 0.3 * n.temperament.courage
        + 0.2 * n.hidden.luck.min(1.5)
        + dressed
}

/// One afternoon of courting. Returns the beloved's affection after.
/// `oratory` is the suitor's silver tongue (0..1), for the player.
pub fn court(world: &mut World, suitor: NpcId, beloved: NpcId, oratory: f32) -> f32 {
    let warmth = world.opinion(beloved, suitor) as f32 / 100.0;
    let gain = 4.0 + 10.0 * charm(world, suitor) + 6.0 * oratory + 8.0 * warmth
        - 10.0 * fidelity(world, beloved);
    if suitor == PLAYER {
        world.adjust_opinion(beloved, suitor, 2);
    }
    let noise = world.rng.unit();
    let after = world
        .hearts
        .add_affection(beloved, suitor, gain.max(0.5) * (0.5 + noise));
    if after < SWEETHEARTS {
        return after;
    }
    let taken = married_to(world, suitor).is_some() || married_to(world, beloved).is_some();
    if taken {
        if !world.hearts.in_affair(suitor, beloved) {
            world.emit_root(
                EventKind::Affair {
                    a: suitor,
                    b: beloved,
                },
                None,
            );
        }
    } else if suitor == PLAYER
        || beloved == PLAYER
        || is_woman(&world.npc(suitor).name) != is_woman(&world.npc(beloved).name)
    {
        world.emit_root(
            EventKind::Marriage {
                a: suitor,
                b: beloved,
            },
            None,
        );
    }
    after
}

/// The chance, each time they meet, that someone sees.
pub fn exposure(world: &World, a: NpcId, b: NpcId) -> f32 {
    let stealth = (world.npc(a).body.stealth + world.npc(b).body.stealth) / 2.0;
    (0.12 * (1.4 - stealth)).clamp(0.02, 0.2)
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::Affair { a, b } => {
            world.hearts.affairs.push((a, b, ev.day));
        }
        EventKind::Scandal { a, b, wronged } => {
            world
                .hearts
                .affairs
                .retain(|&(x, y, _)| !((x == a && y == b) || (x == b && y == a)));
            // The wronged spouse, and their kin, and the churchgoers.
            for (cheat, other) in [(a, b), (b, a)] {
                if let Some(spouse) = married_to(world, cheat) {
                    let hurt = if spouse == wronged { -70 } else { -40 };
                    world.emit_child(
                        ev,
                        EventKind::OpinionChange {
                            holder: spouse,
                            target: other,
                            delta: hurt,
                            after: 0,
                        },
                    );
                    world.adjust_opinion(spouse, cheat, hurt / 2);
                    world.npc_mut(spouse).emotions.anger += 40.0;
                    world.npc_mut(spouse).emotions.grief += 20.0;
                }
            }
            let pious: Vec<NpcId> = world
                .living()
                .filter(|n| n.temperament.piety > 0.6 && n.id != a && n.id != b)
                .map(|n| n.id)
                .collect();
            for p in pious {
                world.adjust_opinion(p, a, -8);
                world.adjust_opinion(p, b, -8);
            }
        }
        EventKind::Marriage { a, b } => marry(world, ev, a, b),
        EventKind::Born { child, .. } if world.npc(child).family == 0 => {
            world.life.spirits = (world.life.spirits + 15.0).min(100.0);
        }
        EventKind::Death { victim, .. } | EventKind::Perished { victim, .. } => {
            world
                .hearts
                .affairs
                .retain(|&(x, y, _)| x != victim && y != victim);
        }
        _ => {}
    }
}

/// The bride (or groom) moves in. Two families are kin now.
fn marry(world: &mut World, ev: &WorldEvent, a: NpcId, b: NpcId) {
    if married_to(world, a).is_some() || married_to(world, b).is_some() {
        return;
    }
    world.hearts.couples.push((a.min(b), a.max(b)));
    // Whoever isn't the player, or the younger, moves to the other's household.
    let (stays, moves) = if a == PLAYER || (b != PLAYER && world.npc(a).age >= world.npc(b).age) {
        (a, b)
    } else {
        (b, a)
    };
    let (home, old) = (world.npc(stays).family, world.npc(moves).family);
    world.npc_mut(moves).family = home;
    world.npc_mut(moves).faction = world.families[home as usize].faction;
    world.adjust_opinion(a, b, 30);
    world.adjust_opinion(b, a, 30);
    if home == old {
        return;
    }
    // In-laws.
    let (h1, h2) = (kin(world, home), kin(world, old));
    for &x in &h1 {
        for &y in &h2 {
            world.adjust_opinion(x, y, 20);
            world.adjust_opinion(y, x, 20);
        }
    }
    if world.feud_between(home, old) {
        world.emit_child(
            ev,
            EventKind::FeudEnded {
                a: home,
                b: old,
                how: super::reconcile::Peace::Marriage,
            },
        );
    }
}

fn kin(world: &World, family: FamilyId) -> Vec<NpcId> {
    world
        .living()
        .filter(|n| n.family == family)
        .map(|n| n.id)
        .collect()
}

/// NPC hearts: affairs go on or get found out; the widowed remarry.
pub fn monthly(world: &mut World) {
    // Ongoing affairs: every month is another chance to be seen.
    let affairs = world.hearts.affairs.clone();
    for (a, b, _) in affairs {
        if a == PLAYER || b == PLAYER {
            continue; // the player's are exposed when they meet (life::court)
        }
        if world.rng.chance(exposure(world, a, b) * 2.0) {
            let wronged = married_to(world, a).or(married_to(world, b)).unwrap_or(a);
            world.emit_root(EventKind::Scandal { a, b, wronged }, None);
        }
    }

    // Temptation: a married neighbor with a wandering eye.
    let restless: Vec<NpcId> = world
        .living()
        .filter(|n| {
            n.id != PLAYER
                && LifeStage::of(n.age) == LifeStage::Adult
                && n.temperament.sociability > 0.6
                && n.temperament.piety < 0.45
        })
        .map(|n| n.id)
        .collect();
    for r in restless {
        if married_to(world, r).is_none() || !world.rng.chance(0.3) {
            continue;
        }
        let family = world.npc(r).family;
        let others: Vec<NpcId> = world
            .living()
            .filter(|n| {
                n.id != PLAYER
                    && n.family != family
                    && LifeStage::of(n.age) == LifeStage::Adult
                    && world.opinion(n.id, r) >= 0
            })
            .map(|n| n.id)
            .collect();
        // Keep after the same one; a first glance is random.
        let fond = others
            .iter()
            .copied()
            .filter(|&o| world.hearts.affection(o, r) > 0.0)
            .max_by(|&a, &b| {
                world
                    .hearts
                    .affection(a, r)
                    .total_cmp(&world.hearts.affection(b, r))
            });
        let pick = match fond {
            Some(o) => o,
            None if !others.is_empty() => others[world.rng.range(0, others.len() as u32) as usize],
            None => continue,
        };
        court(world, r, pick, 0.0);
    }

    // The widowed and the grown: matches made at church and barn raisings.
    let single: Vec<NpcId> = world
        .living()
        .filter(|n| {
            n.id != PLAYER && (18..=60).contains(&n.age) && married_to(world, n.id).is_none()
        })
        .map(|n| n.id)
        .collect();
    births(world);
    for &s in &single {
        if !world.rng.chance(0.5) {
            continue;
        }
        let family = world.npc(s).family;
        let best = single
            .iter()
            .copied()
            .filter(|&o| o != s && world.npc(o).family != family)
            .max_by_key(|&o| world.opinion(s, o) + world.opinion(o, s));
        if let Some(o) = best {
            court(world, s, o, 0.0);
        }
    }
}

/// Names for frontier babies.
const BABIES: [&str; 16] = [
    "Kansas", "Lucy", "John", "Mary", "Charles", "Emma", "George", "Ellen", "Henry", "Nancy",
    "Freedom", "Lorinda", "Asa", "Belle", "Willis", "Susannah",
];

/// Couples have children: one every few years, some of them born into a
/// winter that kills them, and now and then the mother with them.
pub fn births(world: &mut World) {
    let couples = world.hearts.couples.clone();
    for (a, b) in couples {
        let (na, nb) = (world.npc(a), world.npc(b));
        if !na.alive || !nb.alive || na.departed || nb.departed {
            continue;
        }
        // Who carries: the woman, or the player if the player's spouse is a man.
        let mother = match (is_woman(&na.name), is_woman(&nb.name)) {
            (true, _) => a,
            (_, true) => b,
            _ if a == PLAYER || b == PLAYER => PLAYER,
            _ => continue,
        };
        if !(18..=44).contains(&world.npc(mother).age) || !world.rng.chance(0.025) {
            continue;
        }
        let family = world.npc(mother).family;
        let name = BABIES[world.rng.range(0, BABIES.len() as u32) as usize];
        let child = world.add_npc(family, name, 0);
        world.emit_root(EventKind::Born { child, mother }, None);
        if mother != PLAYER && world.rng.chance(0.015) {
            world.emit_root(
                EventKind::Perished {
                    victim: mother,
                    cause: super::events::Hardship::Fever,
                },
                None,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everyone_starts_married_to_their_household() {
        let w = World::new(1);
        let spouse = married_to(&w, PLAYER).expect("you came with a spouse");
        assert_eq!(w.npc(spouse).family, 0);
        assert!(w.hearts.couples.len() >= 8);
    }

    #[test]
    fn a_match_across_a_feud_ends_it() {
        let mut w = World::new(4);
        let (a, b) = (1, 2);
        let ha = w.head_of(a).unwrap();
        let hb = w.head_of(b).unwrap();
        w.feuds.insert((a, b));
        // Widow them both.
        let sa = married_to(&w, ha).unwrap();
        let sb = married_to(&w, hb).unwrap();
        w.npc_mut(sa).alive = false;
        w.npc_mut(sb).alive = false;
        w.set_opinion(ha, hb, -30);
        w.set_opinion(hb, ha, -30);
        w.emit_root(EventKind::Marriage { a: ha, b: hb }, None);
        w.run_cascades();
        assert_eq!(w.npc(ha).family, w.npc(hb).family);
        assert!(!w.feud_between(a, b));
    }

    #[test]
    fn a_scandal_turns_the_spouse() {
        let mut w = World::new(6);
        let a = w.head_of(1).unwrap();
        let b = w.head_of(2).unwrap();
        let wronged = married_to(&w, a).unwrap();
        let before = w.opinion(wronged, b);
        w.emit_root(EventKind::Affair { a, b }, None);
        w.run_cascades();
        assert!(w.hearts.in_affair(a, b));
        w.emit_root(EventKind::Scandal { a, b, wronged }, None);
        w.run_cascades();
        assert!(!w.hearts.in_affair(a, b));
        assert!(w.opinion(wronged, b) <= before - 60);
    }

    #[test]
    fn the_faithful_are_hard_to_turn() {
        let mut w = World::new(8);
        let target = w.head_of(3).unwrap();
        let courted = |w: &mut World, faith: f32| {
            let t = &mut w.npc_mut(target).temperament;
            t.piety = faith;
            t.honesty = faith;
            t.loyalty = faith;
            w.hearts.affection.clear();
            (0..6)
                .map(|_| court(w, PLAYER, target, 0.5))
                .last()
                .unwrap()
        };
        let loose = courted(&mut w, 0.0);
        let faithful = courted(&mut w, 1.0);
        assert!(loose > faithful + 10.0, "{loose} vs {faithful}");
    }
}
