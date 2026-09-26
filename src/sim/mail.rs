//! The Lawrence post office. Mail came up from Westport when the roads were
//! open (not during the 1856 blockade). Letters from back east carried money,
//! grief, and relatives on their way out. A family that can't read takes the
//! letter to a neighbor — who then knows their business.

use super::events::{EventKind, WorldEvent};
use super::psyche::LifeStage;
use super::world::{FamilyId, NpcId, World, distance};

/// First names for kin coming out from the States.
const NEWCOMERS: [&str; 12] = [
    "Nathaniel",
    "Eliza",
    "Jacob",
    "Mercy",
    "Ebenezer",
    "Harriet",
    "Hiram",
    "Phoebe",
    "Lucius",
    "Temperance",
    "Zebulon",
    "Charity",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Letter {
    /// A remittance from home, in dollars.
    Money(u8),
    /// Someone died back in the States.
    BadNews,
    /// A brother or cousin is coming out in the spring.
    KinComing,
    /// Home asks when you're coming back. Nothing, and something.
    Homesick,
}

/// The mail is running.
fn mail_runs(world: &World) -> bool {
    !world.market.blockade
}

/// Mail day is Wednesday, when the Westport hack came in.
pub fn daily(world: &mut World) {
    if !world.day.0.is_multiple_of(7) || !mail_runs(world) {
        return;
    }
    for f in 0..world.families.len() {
        let family = f as FamilyId;
        if world.head_of(family).is_none() {
            continue;
        }
        // Writing home brings letters back.
        let wrote = if family == 0 {
            world.life.wrote_home
        } else {
            0
        };
        let p = (0.25 + 0.1 * wrote as f32) / 4.3;
        if !world.rng.chance(p.min(0.2)) {
            continue;
        }
        let roll = world.rng.unit();
        let letter = match roll {
            r if r < 0.35 + 0.05 * wrote as f32 => {
                let dollars = 3 + world.rng.range(0, 15) as u8 + 3 * wrote;
                Letter::Money(dollars)
            }
            r if r < 0.6 => Letter::BadNews,
            r if r < 0.72 => Letter::KinComing,
            _ => Letter::Homesick,
        };
        if family == 0 {
            world.life.wrote_home = 0;
        }
        let reader = reader_for(world, family);
        world.emit_root(
            EventKind::Letter {
                family,
                letter,
                reader,
            },
            None,
        );
    }
}

/// Who reads it: someone at home if they can, else the nearest lettered
/// neighbor who'll do it.
fn reader_for(world: &World, family: FamilyId) -> Option<NpcId> {
    let lettered = world
        .living()
        .filter(|n| n.family == family && LifeStage::of(n.age) != LifeStage::Child)
        .any(|n| n.temperament.literacy >= 0.4);
    if lettered {
        return None;
    }
    let home = world.families[family as usize].farm;
    world
        .living()
        .filter(|n| n.family != family && n.temperament.literacy >= 0.6)
        .filter(|n| LifeStage::of(n.age) != LifeStage::Child)
        .min_by(|a, b| {
            distance(world.farm_of(a.id), home).total_cmp(&distance(world.farm_of(b.id), home))
        })
        .map(|n| n.id)
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::Letter {
            family,
            letter,
            reader,
        } => {
            let kin: Vec<NpcId> = world
                .living()
                .filter(|n| n.family == family)
                .map(|n| n.id)
                .collect();
            match letter {
                Letter::Money(d) => world.families[family as usize].stores.cash += d as i32,
                Letter::BadNews => {
                    for &k in &kin {
                        world.npc_mut(k).emotions.grief += 25.0;
                    }
                    if family == 0 {
                        world.life.spirits = (world.life.spirits - 15.0).max(0.0);
                    }
                }
                Letter::KinComing => {
                    // They come in the spring, by river to Kansas City and overland.
                    let delay = 90 + world.rng.range(0, 60);
                    world.schedule(
                        delay,
                        EventKind::KinArrived {
                            family,
                            newcomer: None,
                        },
                        ev.id,
                    );
                }
                Letter::Homesick => {
                    for &k in &kin {
                        world.npc_mut(k).emotions.grief += 5.0;
                    }
                }
            }
            // The neighbor who read it: a kindness, and now they know your
            // business — a dishonest one knows where the money is.
            if let Some(r) = reader {
                for &k in &kin {
                    world.adjust_opinion(k, r, 3);
                }
                if matches!(letter, Letter::Money(d) if d >= 10)
                    && world.npc(r).temperament.honesty < 0.3
                    && world.rng.chance(0.3)
                {
                    let victim = world.head_of(family).unwrap_or(r);
                    super::economy::steal_from(world, r, family, victim);
                }
            }
        }
        EventKind::KinArrived {
            family,
            newcomer: None,
        } => {
            if world.head_of(family).is_none() {
                return;
            }
            let first = NEWCOMERS[world.rng.range(0, NEWCOMERS.len() as u32) as usize];
            let age = 17 + world.rng.range(0, 25) as u8;
            let id = world.add_npc(family, first, age);
            if let EventKind::KinArrived { newcomer, .. } = &mut world.events[ev.id as usize].kind {
                *newcomer = Some(id);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_from_home_arrives() {
        let mut w = World::new(3);
        let cash = w.families[2].stores.cash;
        w.emit_root(
            EventKind::Letter {
                family: 2,
                letter: Letter::Money(10),
                reader: None,
            },
            None,
        );
        w.run_cascades();
        assert_eq!(w.families[2].stores.cash, cash + 10);
    }

    #[test]
    fn kin_come_out_months_later() {
        let mut w = World::new(3);
        w.emit_root(
            EventKind::Letter {
                family: 2,
                letter: Letter::KinComing,
                reader: None,
            },
            None,
        );
        w.run_cascades();
        let arrived = |w: &World| {
            w.events.iter().any(|e| {
                matches!(e.kind, EventKind::KinArrived { family: 2, newcomer: Some(n) } if w.npc(n).family == 2)
            })
        };
        w.run_days(80);
        assert!(!arrived(&w), "not before spring");
        w.run_days(80);
        assert!(arrived(&w));
    }

    #[test]
    fn no_mail_through_a_blockade() {
        let mut w = World::new(3);
        w.market.blockade = true;
        let n = w.events.len();
        daily(&mut w);
        assert_eq!(w.events.len(), n);
    }

    #[test]
    fn the_unlettered_need_a_reader() {
        let mut w = World::new(9);
        for n in w.npcs.iter_mut().filter(|n| n.family == 4) {
            n.temperament.literacy = 0.1;
        }
        let r = reader_for(&w, 4).expect("a neighbor reads it");
        assert_ne!(w.npc(r).family, 4);
    }
}
