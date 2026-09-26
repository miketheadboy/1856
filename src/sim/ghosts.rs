//! The Ghost Layer (systems framework §5; Kinship Algorithm): what violence
//! leaves behind after the smoke clears.
//!
//! - **Oaths.** Lose kin to a killer you're sure of, and you swear. A child's
//!   oath sleeps until they come of age (16); then they ride. Oaths pass down
//!   when the holder dies, and can fall on the killer's children when the
//!   killer dies by someone else's hand.
//! - **The restless dead.** Every killing leaves a haunt at the victim's
//!   claim. It stays restless until the family believes the true killer, or
//!   the killer is dead.
//! - **Sightings.** The credulous and the pious, near a restless place, see
//!   the dead at night. The sim holds the truth; the ghost is the truth
//!   leaking back through folk belief: a sighting frightens, and nudges the
//!   witness toward the real killer. Nobody has to believe in ghosts for
//!   their neighbors to act on them.

use super::events::{EventId, EventKind, Suspect, WorldEvent};
use super::psyche::{self, LifeStage};
use super::world::{NpcId, World, distance};

/// Age at which a sleeping oath wakes.
pub const COMING_OF_AGE: u8 = 16;

#[derive(Clone, Debug)]
pub struct Oath {
    pub holder: NpcId,
    pub target: NpcId,
    /// The killing that started it.
    pub origin: EventId,
    pub woke: bool,
    pub done: bool,
}

#[derive(Clone, Debug)]
pub struct Haunt {
    pub spirit: NpcId,
    pub killer: NpcId,
    pub site: (i32, i32),
    pub death: EventId,
    pub restless: bool,
    pub sightings: u32,
}

/// Kin who believe the dead had it coming don't swear. They carry it.
#[derive(Clone, Debug)]
pub struct Shame {
    pub holder: NpcId,
    /// The kinsman who deserved it (in the holder's eyes).
    pub over: NpcId,
    /// The family the dead man wronged.
    pub wronged: u32,
    pub woke: bool,
}

/// Which way shame turns a life when it wakes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShameTurn {
    /// Piety and generosity; drawn to the family their kin wronged.
    Atonement,
    /// Temper and drink; the loafer's road.
    Ruin,
}

#[derive(Clone, Debug, Default)]
pub struct Ghosts {
    pub oaths: Vec<Oath>,
    pub haunts: Vec<Haunt>,
    pub shames: Vec<Shame>,
}

impl Ghosts {
    pub fn oath_against(&self, holder: NpcId, target: NpcId) -> bool {
        self.oaths
            .iter()
            .any(|o| o.holder == holder && o.target == target && !o.done)
    }
}

/// Event hook: killings leave haunts; certain kin swear; deaths settle oaths.
pub fn on_event(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::Death {
            victim,
            killer: Some(killer),
        } => {
            let site = world.farm_of(victim);
            world.ghosts.haunts.push(Haunt {
                spirit: victim,
                killer,
                site,
                death: ev.id,
                restless: true,
                sightings: 0,
            });
            settle(world, ev, victim, Some(killer));
        }
        EventKind::Death {
            victim,
            killer: None,
        }
        | EventKind::Perished { victim, .. } => {
            settle(world, ev, victim, None);
        }
        EventKind::Belief {
            holder,
            about,
            blamed: Suspect::Person(target),
            confidence,
            ..
        } if confidence >= 50 => swear(world, ev, holder, about, target),
        _ => {}
    }
}

fn swear(world: &mut World, ev: &WorldEvent, holder: NpcId, about: EventId, target: NpcId) {
    let EventKind::Death { victim, .. } = world.events[about as usize].kind else {
        return;
    };
    let h = world.npc(holder);
    if h.family != world.npc(victim).family
        || holder == target
        || !world.npc(target).alive
        || world.ghosts.oath_against(holder, target)
    {
        return;
    }
    let child = LifeStage::of(h.age) == LifeStage::Child;

    // Did he deserve it? Not by the truth: by what the kin believes the dead
    // man did to the killer or the killer's family.
    let killer_family = world.npc(target).family;
    let deserved = h.memories.iter().any(|m| {
        m.believed == Suspect::Person(victim)
            && m.confidence >= 40
            && super::attribution::victim_of(world, m.event)
                .is_some_and(|v| world.npc(v).family == killer_family)
    });
    if deserved {
        if world
            .ghosts
            .shames
            .iter()
            .any(|s| s.holder == holder && s.over == victim)
        {
            return;
        }
        world.ghosts.shames.push(Shame {
            holder,
            over: victim,
            wronged: killer_family,
            woke: !child,
        });
        // "He had it coming." The grudge against the killer softens.
        world.adjust_opinion(holder, target, 40);
        world.emit_child(
            ev,
            EventKind::ShameCarried {
                holder,
                over: victim,
            },
        );
        return;
    }

    world.ghosts.oaths.push(Oath {
        holder,
        target,
        origin: about,
        woke: !child,
        done: false,
    });
    world.emit_child(
        ev,
        EventKind::OathSworn {
            holder,
            target,
            over: victim,
        },
    );
}

/// A death closes oaths against the dead, and passes on oaths the dead held.
fn settle(world: &mut World, ev: &WorldEvent, dead: NpcId, killer: Option<NpcId>) {
    let mut inherited = Vec::new();
    let mut transferred = Vec::new();
    for o in world.ghosts.oaths.iter_mut().filter(|o| !o.done) {
        if o.target == dead {
            o.done = true;
            // Killed by someone else: the debt may fall on the dead man's children.
            if killer != Some(o.holder) {
                transferred.push(o.clone());
            }
        } else if o.holder == dead {
            o.done = true;
            inherited.push(o.clone());
        }
    }
    for o in inherited {
        let family = world.npc(dead).family;
        let heir = world
            .living()
            .filter(|n| n.family == family && n.id != dead)
            .min_by_key(|n| n.age)
            .map(|n| (n.id, LifeStage::of(n.age) == LifeStage::Child));
        if let Some((heir, child)) = heir
            && world.npc(o.target).alive
        {
            world.ghosts.oaths.push(Oath {
                holder: heir,
                target: o.target,
                origin: o.origin,
                woke: !child,
                done: false,
            });
            world.emit_child(
                ev,
                EventKind::OathInherited {
                    heir,
                    target: o.target,
                },
            );
        }
    }
    for o in transferred {
        let family = world.npc(dead).family;
        let child = world
            .living()
            .filter(|n| n.family == family && LifeStage::of(n.age) == LifeStage::Child)
            .map(|n| n.id)
            .next();
        if let Some(child) = child
            && world.rng.chance(0.3)
            && world.npc(o.holder).alive
        {
            // Sins of the fathers: the holder turns on the killer's child.
            world.ghosts.oaths.push(Oath {
                holder: o.holder,
                target: child,
                origin: o.origin,
                woke: true,
                done: false,
            });
        }
    }
}

/// Every November: the county ages a year, and sleeping oaths wake.
pub fn yearly(world: &mut World) {
    for n in world.npcs.iter_mut().filter(|n| n.alive) {
        n.age = n.age.saturating_add(1);
    }
    wake_shame(world);
    for i in 0..world.ghosts.oaths.len() {
        let o = world.ghosts.oaths[i].clone();
        if o.done || o.woke || world.npc(o.holder).age < COMING_OF_AGE {
            continue;
        }
        if !world.npc(o.holder).alive || !world.npc(o.target).alive {
            world.ghosts.oaths[i].done = true;
            continue;
        }
        world.ghosts.oaths[i].woke = true;
        // They grew up with it. It shows.
        let n = world.npc_mut(o.holder);
        n.temperament.courage = (n.temperament.courage + 0.3).min(1.0);
        n.temperament.temper = (n.temperament.temper + 0.2).min(1.0);
        n.body.marksmanship = (n.body.marksmanship + 0.3).min(1.0);
        n.plotting = Some(o.target);
        let id = world.emit_root(
            EventKind::OathWakes {
                holder: o.holder,
                target: o.target,
            },
            Some(o.origin),
        );
        let delay = world.rng.range(5, 40);
        world.schedule(
            delay,
            EventKind::Retaliation {
                actor: o.holder,
                target: o.target,
                method: super::events::Retaliation::Ambush,
            },
            id,
        );
    }
}

/// Shame that comes of age turns a life one way or the other, by temperament.
fn wake_shame(world: &mut World) {
    for i in 0..world.ghosts.shames.len() {
        let s = world.ghosts.shames[i].clone();
        let n = world.npc(s.holder);
        if s.woke || !n.alive || n.age < COMING_OF_AGE {
            continue;
        }
        let devout = n.temperament.piety + n.temperament.generosity > 1.0;
        world.ghosts.shames[i].woke = true;
        let turn = if devout {
            ShameTurn::Atonement
        } else {
            ShameTurn::Ruin
        };
        let wronged: Vec<NpcId> = world
            .living()
            .filter(|w| w.family == s.wronged)
            .map(|w| w.id)
            .collect();
        let n = world.npc_mut(s.holder);
        match turn {
            ShameTurn::Atonement => {
                n.temperament.piety = (n.temperament.piety + 0.3).min(1.0);
                n.temperament.generosity = (n.temperament.generosity + 0.3).min(1.0);
                n.temperament.temper = (n.temperament.temper - 0.2).max(0.0);
                for w in wronged {
                    world.adjust_opinion(s.holder, w, 30);
                    world.adjust_opinion(w, s.holder, 15);
                }
            }
            ShameTurn::Ruin => {
                n.temperament.temper = (n.temperament.temper + 0.3).min(1.0);
                n.temperament.piety = (n.temperament.piety - 0.3).max(0.0);
                n.emotions.grief = (n.emotions.grief + 30.0).min(100.0);
            }
        }
        world.emit_root(
            EventKind::ShameWakes {
                holder: s.holder,
                over: s.over,
                turn,
            },
            None,
        );
    }
}

/// Carrying shame damps the appetite for revenge.
pub fn ashamed(world: &World, id: NpcId) -> bool {
    world.ghosts.shames.iter().any(|s| s.holder == id)
}

/// Is this haunt still restless? It rests when the family knows the truth, or
/// the killer is dead.
fn at_peace(world: &World, h: &Haunt) -> bool {
    if !world.npc(h.killer).alive {
        return true;
    }
    let family = world.npc(h.spirit).family;
    world.living().filter(|n| n.family == family).any(|n| {
        n.memory_of(h.death)
            .is_some_and(|m| m.believed == Suspect::Person(h.killer) && m.confidence >= 60)
    })
}

/// Nightly: the restless dead are seen; the dread settles on the neighbors.
pub fn daily(world: &mut World) {
    let light = world.night_light(world.day);
    for i in 0..world.ghosts.haunts.len() {
        let h = world.ghosts.haunts[i].clone();
        if !h.restless {
            continue;
        }
        if at_peace(world, &h) {
            world.ghosts.haunts[i].restless = false;
            world.emit_root(EventKind::SpiritRests { spirit: h.spirit }, Some(h.death));
            continue;
        }
        // Dread: families living near a restless place don't sleep easy.
        let near: Vec<NpcId> = world
            .living()
            .filter(|n| distance(world.farm_of(n.id), h.site) <= 4.0)
            .map(|n| n.id)
            .collect();
        for &n in &near {
            psyche::feel(world, n, |e| e.fear += 0.4);
        }
        // Moonlit nights, you see things. The credulous and the grieving most.
        let watchers: Vec<(NpcId, f32)> = world
            .living()
            .filter(|n| distance(world.farm_of(n.id), h.site) <= 10.0 && n.age >= 8)
            .map(|n| {
                let t = n.temperament;
                let belief = (1.0 - t.skepticism) * (0.4 + 0.6 * t.piety);
                let grief = n.emotions.grief / 100.0;
                (n.id, belief * (0.3 + 0.7 * light) * (0.5 + grief))
            })
            .collect();
        for (w, p) in watchers {
            if world.rng.chance(p * 0.01) {
                sighting(world, i, w);
                break;
            }
        }
    }
}

fn sighting(world: &mut World, haunt: usize, witness: NpcId) {
    let h = world.ghosts.haunts[haunt].clone();
    world.ghosts.haunts[haunt].sightings += 1;
    psyche::feel(world, witness, |e| e.fear += 20.0);
    let id = world.emit_root(
        EventKind::SpiritSeen {
            witness,
            spirit: h.spirit,
        },
        Some(h.death),
    );
    // The dead have one thing to say. The witness takes it as a rumor from
    // the grave: it can overturn what they believed.
    let parent = world.events[id as usize].clone();
    world.emit_child(
        &parent,
        EventKind::Gossip {
            teller: h.spirit,
            listener: witness,
            about: h.death,
            blamed: Suspect::Person(h.killer),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::super::events::Source;
    use super::*;

    /// You kill a man in front of his child. When the child turns sixteen, the
    /// oath wakes and they come for you.
    #[test]
    fn the_child_comes_back() {
        use super::super::PLAYER;
        let mut w = World::new(8);
        let child = w
            .living()
            .find(|n| n.age < 16 && n.family != 0)
            .map(|n| n.id)
            .unwrap();
        w.npc_mut(child).age = 15;
        w.ghosts.oaths.push(Oath {
            holder: child,
            target: PLAYER,
            origin: 0,
            woke: false,
            done: false,
        });
        // One November later.
        yearly(&mut w);
        assert_eq!(w.npc(child).age, 16);
        assert!(w.ghosts.oaths[0].woke);
        assert_eq!(w.npc(child).plotting, Some(PLAYER));
        assert!(w
            .events
            .iter()
            .any(|e| matches!(e.kind, EventKind::OathWakes { holder, target } if holder == child && target == PLAYER)));
    }

    #[test]
    fn a_child_swears_but_waits() {
        let mut w = World::new(8);
        let (parent, child) = (1..w.families.len() as u32)
            .find_map(|f| {
                let kin: Vec<_> = w.living().filter(|n| n.family == f).collect();
                Some((
                    kin.iter().find(|n| n.age >= 16)?.id,
                    kin.iter().find(|n| n.age < 16)?.id,
                ))
            })
            .unwrap();
        let death = w.emit_root(
            EventKind::Death {
                victim: parent,
                killer: Some(super::super::PLAYER),
            },
            None,
        );
        let belief = w.emit_root(
            EventKind::Belief {
                holder: child,
                about: death,
                blamed: Suspect::Person(super::super::PLAYER),
                confidence: 90,
                source: Source::Witnessed,
                reason: "saw it",
            },
            None,
        );
        let ev = w.events[belief as usize].clone();
        on_event(&mut w, &ev);
        let o = w.ghosts.oaths.iter().find(|o| o.holder == child).unwrap();
        assert!(!o.woke, "a child's oath sleeps");
    }

    #[test]
    fn a_killing_leaves_a_restless_haunt_until_the_truth_is_known() {
        let mut w = World::new(8);
        let victim = w.head_of(2).unwrap();
        w.player_kill(victim);
        assert!(w.ghosts.haunts.iter().any(|h| h.spirit == victim));
        // Force the family to know the truth: the haunt rests.
        let family = w.npc(victim).family;
        let death = w.ghosts.haunts[0].death;
        let kin: Vec<_> = w
            .living()
            .filter(|n| n.family == family)
            .map(|n| n.id)
            .collect();
        let today = w.day;
        for k in kin {
            w.npc_mut(k).memories.retain(|m| m.event != death);
            w.npc_mut(k).remember(super::super::world::MemoryRef {
                event: death,
                believed: Suspect::Person(super::super::PLAYER),
                confidence: 90,
                source: Source::Witnessed,
                day: today,
                weight: 255,
            });
        }
        daily(&mut w);
        assert!(!w.ghosts.haunts[0].restless);
    }

    /// If the child believes Pa deserved it, there's no oath: only shame.
    #[test]
    fn no_vengeance_if_he_had_it_coming() {
        use super::super::world::MemoryRef;
        let mut w = World::new(8);
        let (parent, child) = (1..w.families.len() as u32)
            .find_map(|f| {
                let kin: Vec<_> = w.living().filter(|n| n.family == f).collect();
                Some((
                    kin.iter().find(|n| n.age >= 16)?.id,
                    kin.iter().find(|n| n.age < 16)?.id,
                ))
            })
            .unwrap();
        let killer = w
            .living()
            .find(|n| n.family != w.npc(parent).family && n.id != 0 && n.age >= 16)
            .unwrap()
            .id;
        // The child believes Pa burned the killer's barn.
        let fire = w.emit_root(
            EventKind::Fire {
                owner: killer,
                cause: super::super::FireCause::Hearth,
                spread_from: None,
            },
            None,
        );
        let today = w.day;
        w.npc_mut(child).remember(MemoryRef {
            event: fire,
            believed: Suspect::Person(parent),
            confidence: 80,
            source: Source::Told(killer),
            day: today,
            weight: 100,
        });
        let death = w.emit_root(
            EventKind::Death {
                victim: parent,
                killer: Some(killer),
            },
            None,
        );
        let belief = w.emit_root(
            EventKind::Belief {
                holder: child,
                about: death,
                blamed: Suspect::Person(killer),
                confidence: 95,
                source: Source::Witnessed,
                reason: "saw it",
            },
            None,
        );
        let ev = w.events[belief as usize].clone();
        on_event(&mut w, &ev);
        assert!(
            !w.ghosts.oath_against(child, killer),
            "no oath if he had it coming"
        );
        assert!(ashamed(&w, child));
    }

    #[test]
    fn coming_of_age_is_sixteen() {
        assert_eq!(LifeStage::of(COMING_OF_AGE), LifeStage::Adult);
        assert_eq!(LifeStage::of(COMING_OF_AGE - 1), LifeStage::Child);
    }
}
