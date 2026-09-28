//! Who someone *is*: archetypes that emerge from stat combinations, the
//! hidden numbers the player never sees, and the chance of evil.
//!
//! Archetypes are not assigned. They fall out of the stats, and some of them
//! (Zealot, Coward) come and go with emotion. Each one multiplies something.

use super::events::{Cruelty, EventKind, Suspect};
use super::psyche::LifeStage;
use super::rng::SimRng;
use super::world::{Npc, NpcId, PLAYER, World};

/// Never shown to the player (§21 asymmetric legibility).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hidden {
    /// 0.7 .. 1.3. Multiplies harvests, hunts, ambushes; divides lightning.
    pub luck: f32,
    /// 0 .. 1, heavily skewed low. Most people have almost none.
    pub malice: f32,
}

impl Hidden {
    pub fn roll(rng: &mut SimRng) -> Self {
        let m = rng.unit();
        Self {
            luck: 0.7 + rng.unit() * 0.6,
            // Eighth power: a county of mostly decent people and one or two
            // bad ones (about 1 in 12 above 0.5).
            malice: m.powi(8),
        }
    }
}

impl Default for Hidden {
    fn default() -> Self {
        Self {
            luck: 1.0,
            malice: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Archetype {
    /// Burning zeal + loyalty. Revenge ×1.5, never signs.
    Zealot,
    /// Low courage + fear. Tells nobody what they saw; signs anything.
    Coward,
    /// Very sociable. Their rumors land 60% harder.
    SilverTongue,
    /// Marksman + stealthy. Ambushes don't miss; hard to see.
    Bushwhacker,
    /// Pious + generous. Always feeds a beggar; grudges thaw double.
    Deacon,
    /// Stingy + dishonest. Steals when not even hungry.
    Skinflint,
    /// Temper + courage. Revenge comes twice as fast, and with a rifle.
    Hothead,
    /// Hardy + strong. Works like two; starves slowly.
    Survivor,
    /// Disloyal + dishonest. Sells neighbors to the storekeeper.
    Informer,
}

impl Archetype {
    pub fn label(self) -> &'static str {
        match self {
            Archetype::Zealot => "Zealot",
            Archetype::Coward => "Coward",
            Archetype::SilverTongue => "Silver Tongue",
            Archetype::Bushwhacker => "Bushwhacker",
            Archetype::Deacon => "Deacon",
            Archetype::Skinflint => "Skinflint",
            Archetype::Hothead => "Hothead",
            Archetype::Survivor => "Survivor",
            Archetype::Informer => "Informer",
        }
    }

    pub const ALL: [Archetype; 9] = [
        Archetype::Zealot,
        Archetype::Coward,
        Archetype::SilverTongue,
        Archetype::Bushwhacker,
        Archetype::Deacon,
        Archetype::Skinflint,
        Archetype::Hothead,
        Archetype::Survivor,
        Archetype::Informer,
    ];
}

pub fn is(n: &Npc, a: Archetype) -> bool {
    let t = n.temperament;
    let b = n.body;
    let e = n.emotions;
    let grown = LifeStage::of(n.age) != LifeStage::Child;
    match a {
        Archetype::Zealot => e.zeal > 50.0 && t.loyalty > 0.65,
        Archetype::Coward => t.courage < 0.25 && e.fear > 30.0,
        Archetype::SilverTongue => t.sociability > 0.8,
        Archetype::Bushwhacker => grown && b.marksmanship > 0.7 && b.stealth > 0.65,
        Archetype::Deacon => t.piety > 0.75 && t.generosity > 0.65,
        Archetype::Skinflint => t.generosity < 0.2 && t.honesty < 0.3,
        Archetype::Hothead => grown && t.temper > 0.8 && t.courage > 0.55,
        Archetype::Survivor => b.hardiness > 0.75 && b.strength > 0.6,
        Archetype::Informer => t.loyalty < 0.25 && t.honesty < 0.4,
    }
}

pub fn archetypes(n: &Npc) -> Vec<Archetype> {
    Archetype::ALL.into_iter().filter(|a| is(n, *a)).collect()
}

/// Daily: people with malice in them may do something cruel for its own
/// sake. Anger and hunger feed it. It's attributed like any other harm, so
/// the wrong people get blamed (§11).
pub fn daily_evil(world: &mut World) {
    let candidates: Vec<NpcId> = world
        .living()
        .filter(|n| n.id != PLAYER && LifeStage::of(n.age) != LifeStage::Child && !n.wounded)
        .map(|n| n.id)
        .collect();
    for actor in candidates {
        let n = world.npc(actor);
        let malice = n.hidden.malice;
        if malice < 0.15 {
            continue;
        }
        // Per day. A truly malicious person acts a few times a year.
        let mut p = malice * malice * malice * (1.0 + n.emotions.anger / 100.0) / 120.0;
        if world.families[n.family as usize].stores.desperate() {
            p *= 1.5;
        }
        if is(n, Archetype::Skinflint) {
            p *= 1.3;
        }
        if !world.rng.chance(p.min(0.05)) {
            continue;
        }
        commit(world, actor);
    }
}

fn commit(world: &mut World, actor: NpcId) {
    let family = world.npc(actor).family;
    // The victim: someone they dislike, or with enough malice, anyone.
    let victims: Vec<(NpcId, i16)> = world
        .living()
        .filter(|v| v.family != family && LifeStage::of(v.age) != LifeStage::Child)
        .map(|v| (v.id, world.opinion(actor, v.id)))
        .collect();
    let grudge = victims.iter().min_by_key(|v| v.1).copied();
    let victim = match grudge {
        Some((v, o)) if o <= -20 => v,
        _ => match world.rng.pick(&victims) {
            Some(&(v, _)) => v,
            None => return,
        },
    };
    let victim_family = world.npc(victim).family;

    let silver = is(world.npc(actor), Archetype::SilverTongue);
    let roll = world.rng.unit();
    let act = if roll < 0.45 || silver {
        // Slander: pin someone's old misfortune on the victim. Tell it to
        // someone who already has an opinion about it; a good liar can change it.
        let memories: Vec<_> = world
            .living()
            .filter(|n| n.id != actor && n.family != victim_family && n.family != family)
            .flat_map(|n| n.memories.iter().map(move |m| (n.id, m.event)))
            .collect();
        world
            .rng
            .pick(&memories)
            .copied()
            .map(|(listener, about)| Cruelty::Slander { listener, about })
    } else if roll < 0.8 {
        Some(Cruelty::KillStock)
    } else {
        Some(Cruelty::FoulWell)
    };
    let Some(act) = act else {
        return;
    };
    world.npc_mut(actor).alibi = None;
    world.emit_root(EventKind::Cruelty { actor, victim, act }, None);
}

/// How hard a rumor from this teller lands.
pub fn rumor_weight(world: &World, teller: NpcId) -> f32 {
    if is(world.npc(teller), Archetype::SilverTongue) {
        1.6
    } else {
        1.0
    }
}

/// Slander is gossip with a lie in it.
pub fn slander_target(victim: NpcId) -> Suspect {
    Suspect::Person(victim)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archetypes_fall_out_of_stats() {
        let mut w = World::new(1);
        let n = w.npc_mut(1);
        n.age = 30;
        n.body.marksmanship = 0.9;
        n.body.stealth = 0.9;
        n.temperament.sociability = 0.95;
        let n = w.npc(1);
        assert!(is(n, Archetype::Bushwhacker));
        assert!(is(n, Archetype::SilverTongue));
        assert!(!is(n, Archetype::Deacon) || n.temperament.piety > 0.75);
    }

    #[test]
    fn children_are_never_bushwhackers() {
        let mut w = World::new(1);
        let n = w.npc_mut(1);
        n.age = 9;
        n.body.marksmanship = 1.0;
        n.body.stealth = 1.0;
        assert!(!is(w.npc(1), Archetype::Bushwhacker));
    }

    #[test]
    fn malice_is_rare() {
        let mut rng = SimRng::new(4);
        let cruel = (0..10_000)
            .filter(|_| Hidden::roll(&mut rng).malice > 0.5)
            .count();
        assert!(cruel < 1_000, "{cruel} of 10000 have malice > 0.5");
    }

    #[test]
    fn silver_tongues_carry_further() {
        let mut w = World::new(1);
        w.npc_mut(1).temperament.sociability = 0.95;
        w.npc_mut(2).temperament.sociability = 0.1;
        assert!(rumor_weight(&w, 1) > rumor_weight(&w, 2));
    }
}
