//! Marks: what a life leaves on a person. Each is earned by one event in one
//! system and read by others, so a fever survived in March changes who sits
//! up with the dead in June, and a man who carried the county's letters is
//! believed in court.
//!
//! Some marks are public: the county knows them, and they move a person's
//! standing (`standing::parts`), which every system that weighs a word or a
//! reputation reads (gossip, blame, oaths, credit, claims). Some are private:
//! the man who killed knows it, the county only suspects, and the mark works
//! on him from the inside (rule 3: the player never sees truth).
//!
//! Some come back. A killing, a child's grave and a sold fugitive are
//! `Remembered` later (`schedule`): the anniversary of the killing, the year
//! after the burying, the month after the reward was spent.

use super::calendar::Day;
use super::events::{EventId, EventKind, WorldEvent};
use super::psyche::LifeStage;
use super::sickness::Disease;
use super::world::{NpcId, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mark {
    /// Had the cholera, smallpox or typhoid and lived. Sits up with the
    /// dead without fear and nurses a house better (`sickness`).
    CameThrough,
    /// Killed a man. Private. Weariness spares more of his next victims
    /// (`reconcile::mercy`); the anniversary comes back on him.
    HasKilled,
    /// Buried a child. A child in the target's house stays the hand more
    /// often (`reconcile::mercy`); the year after, the grief comes back.
    BuriedAChild,
    /// Raised the barn of a man he hated. The county thinks better of him
    /// (`standing`).
    RaisedEnemyBarn,
    /// Kept the night watch and turned riders back at the fence. Riders who
    /// know it think twice (`standing` repute), and the house sleeps easier.
    TurnedRidersBack,
    /// Hid a freedom seeker. Private. The next knock is easier to answer
    /// (`railroad`).
    KeptAStation,
    /// Took the fifty dollars. Private. The second time is easier
    /// (`railroad`); a lukewarm conscience comes back on him in a month.
    SoldAFugitive,
    /// Three trips to the river and back. Knows the fords and the men at
    /// the ferries: Missourians stop him half as often (`freight`), and his
    /// word carries (`standing`).
    KnowsTheRiver,
    /// Married someone of the other side. Neither side quite trusts the
    /// couple (`standing`), and "their side does this" weighs half against
    /// them (`attribution`).
    MarriedAcross,
    /// Three desperate acts in a season (the seed corn, the ox, the neighbor's
    /// door) and lived. The house salts away more against the next winter
    /// (`economy` prudence).
    StarvingTime,
    /// Read three neighbors' letters to them: knows the county's business,
    /// and is believed (`standing`).
    CountyReader,
}

impl Mark {
    pub const ALL: [Mark; 11] = [
        Mark::CameThrough,
        Mark::HasKilled,
        Mark::BuriedAChild,
        Mark::RaisedEnemyBarn,
        Mark::TurnedRidersBack,
        Mark::KeptAStation,
        Mark::SoldAFugitive,
        Mark::KnowsTheRiver,
        Mark::MarriedAcross,
        Mark::StarvingTime,
        Mark::CountyReader,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Mark::CameThrough => "came through the fever",
            Mark::HasKilled => "has killed a man",
            Mark::BuriedAChild => "buried a child",
            Mark::RaisedEnemyBarn => "raised an enemy's barn",
            Mark::TurnedRidersBack => "turned riders back at the fence",
            Mark::KeptAStation => "kept a station",
            Mark::SoldAFugitive => "took the fifty dollars",
            Mark::KnowsTheRiver => "knows the river road",
            Mark::MarriedAcross => "married across the line",
            Mark::StarvingTime => "came through the starving time",
            Mark::CountyReader => "reads the county's letters",
        }
    }

    /// Known to the county, and so part of a person's standing.
    pub fn public(self) -> bool {
        !matches!(
            self,
            Mark::HasKilled | Mark::KeptAStation | Mark::SoldAFugitive
        )
    }

    /// (word, repute) for `standing`, for the public marks that move it.
    pub fn standing(self) -> Option<(f32, f32)> {
        match self {
            Mark::RaisedEnemyBarn => Some((1.1, 1.15)),
            Mark::TurnedRidersBack => Some((1.0, 1.05)),
            Mark::KnowsTheRiver => Some((1.1, 1.0)),
            Mark::MarriedAcross => Some((1.0, 0.9)),
            Mark::CountyReader => Some((1.1, 1.0)),
            _ => None,
        }
    }

    /// Days until the mark comes back on its holder, if it does.
    fn returns(self) -> Option<u32> {
        match self {
            Mark::HasKilled | Mark::BuriedAChild => Some(365),
            Mark::SoldAFugitive => Some(30),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Held {
    pub who: NpcId,
    pub mark: Mark,
    pub day: Day,
    /// The event that earned it.
    pub by: EventId,
}

#[derive(Clone, Debug, Default)]
pub struct Marks {
    pub held: Vec<Held>,
}

pub fn has(world: &World, who: NpcId, mark: Mark) -> bool {
    world
        .marks
        .held
        .iter()
        .any(|h| h.who == who && h.mark == mark)
}

/// Any living member of the house holds it.
pub fn house_has(world: &World, family: super::world::FamilyId, mark: Mark) -> bool {
    world
        .marks
        .held
        .iter()
        .any(|h| h.mark == mark && world.npc(h.who).family == family && world.npc(h.who).alive)
}

pub fn of(world: &World, who: NpcId) -> Vec<Mark> {
    world
        .marks
        .held
        .iter()
        .filter(|h| h.who == who)
        .map(|h| h.mark)
        .collect()
}

/// How many times an event of this shape has happened to `who` (for marks
/// earned by the third time).
fn times(world: &World, f: impl Fn(&EventKind) -> bool) -> usize {
    world.events.iter().filter(|e| f(&e.kind)).count()
}

fn earn(world: &mut World, ev: &WorldEvent, who: NpcId, mark: Mark) {
    let n = world.npc(who);
    if !n.alive || LifeStage::of(n.age) == LifeStage::Child || has(world, who, mark) {
        return;
    }
    world.emit_child(ev, EventKind::MarkEarned { who, mark });
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::MarkEarned { who, mark } => {
            if has(world, who, mark) {
                return;
            }
            let by = ev.parent.or(ev.caused_by).unwrap_or(ev.id);
            world.marks.held.push(Held {
                who,
                mark,
                day: ev.day,
                by,
            });
            match mark {
                Mark::StarvingTime => {
                    // Never again: the house lays by more.
                    let f = world.npc(who).family as usize;
                    let p = &mut world.families[f].stores.prudence;
                    *p = (*p + 0.15).min(1.0);
                }
                Mark::TurnedRidersBack => {
                    let f = world.npc(who).family;
                    for k in world.npcs.iter_mut().filter(|n| n.alive && n.family == f) {
                        k.emotions.fear *= 0.8;
                    }
                }
                _ => {}
            }
            if let Some(after) = mark.returns() {
                world.schedule(
                    after,
                    EventKind::Remembered {
                        who,
                        mark,
                        years: 1,
                    },
                    ev.id,
                );
            }
        }
        EventKind::Remembered { who, mark, years } => remembered(world, ev, who, mark, years),
        EventKind::Recovered {
            who,
            disease: Disease::Cholera | Disease::Smallpox | Disease::Typhoid,
        } => earn(world, ev, who, Mark::CameThrough),
        EventKind::Death {
            victim,
            killer: Some(k),
        } if k != victim => earn(world, ev, k, Mark::HasKilled),
        EventKind::BarnRaising { rival: Some(r), .. } => earn(world, ev, r, Mark::RaisedEnemyBarn),
        EventKind::TurnedBack {
            watchman: Some(w), ..
        } => earn(world, ev, w, Mark::TurnedRidersBack),
        EventKind::Sheltered { family, .. } => {
            if let Some(h) = world.head_of(family) {
                earn(world, ev, h, Mark::KeptAStation);
            }
        }
        EventKind::Captured {
            informer: Some(i), ..
        } => earn(world, ev, i, Mark::SoldAFugitive),
        EventKind::WagonBack { teamster, .. } => {
            let trips = times(
                world,
                |k| matches!(*k, EventKind::WagonBack { teamster: t, .. } if t == teamster),
            );
            if trips >= 3 {
                earn(world, ev, teamster, Mark::KnowsTheRiver);
            }
        }
        EventKind::Desperation { family, .. } => {
            let since = ev.day.0.saturating_sub(STARVING_WINDOW);
            let acts = world
                .events
                .iter()
                .rev()
                .take_while(|e| e.day.0 >= since)
                .filter(
                    |e| matches!(e.kind, EventKind::Desperation { family: f, .. } if f == family),
                )
                .count();
            if acts >= STARVING_ACTS {
                let grown: Vec<NpcId> = world
                    .living()
                    .filter(|n| n.family == family)
                    .map(|n| n.id)
                    .collect();
                for who in grown {
                    earn(world, ev, who, Mark::StarvingTime);
                }
            }
        }
        EventKind::Letter {
            reader: Some(r), ..
        } => {
            let read = times(
                world,
                |k| matches!(*k, EventKind::Letter { reader: Some(x), .. } if x == r),
            );
            if read >= 3 {
                earn(world, ev, r, Mark::CountyReader);
            }
        }
        EventKind::Death { victim, .. } | EventKind::Perished { victim, .. }
            if LifeStage::of(world.npc(victim).age) == LifeStage::Child =>
        {
            let f = world.npc(victim).family;
            let parents: Vec<NpcId> = world
                .living()
                .filter(|n| n.family == f && LifeStage::of(n.age) == LifeStage::Adult)
                .map(|n| n.id)
                .collect();
            for p in parents {
                earn(world, ev, p, Mark::BuriedAChild);
            }
        }
        _ => {}
    }
}

/// Desperate acts in this many days make the starving time.
pub const STARVING_WINDOW: u32 = 90;
/// How many desperate acts (seed corn eaten, the ox killed, begging).
pub const STARVING_ACTS: usize = 3;

/// The mark comes back on its holder.
fn remembered(world: &mut World, ev: &WorldEvent, who: NpcId, mark: Mark, years: u8) {
    if !world.npc(who).alive {
        return;
    }
    match mark {
        // The day of it, a year on. He sees the field where it happened; and
        // if the dead man walks anywhere, it's there, and for him.
        Mark::HasKilled => {
            let n = world.npc_mut(who);
            n.emotions.fear += 15.0;
            n.emotions.grief += 10.0;
            let spirit = world
                .ghosts
                .haunts
                .iter()
                .find(|h| h.killer == who && h.restless)
                .map(|h| h.spirit);
            if let Some(s) = spirit
                && world.rng.chance(0.5)
            {
                world.emit_child(
                    ev,
                    EventKind::SpiritSeen {
                        witness: who,
                        spirit: s,
                    },
                );
            }
            if years < 3 {
                world.schedule(
                    365,
                    EventKind::Remembered {
                        who,
                        mark,
                        years: years + 1,
                    },
                    ev.id,
                );
            }
        }
        // A year on, the grief comes back, and anger has nowhere to go.
        Mark::BuriedAChild => {
            let n = world.npc_mut(who);
            n.emotions.grief += 20.0;
            n.emotions.anger *= 0.7;
        }
        // The money's spent. A man of the other side sleeps fine; a man who
        // half believed in the cause doesn't.
        Mark::SoldAFugitive => {
            let n = world.npc_mut(who);
            if n.ideology.private > 0.0 {
                n.emotions.fear += 20.0;
                n.emotions.grief += 15.0;
            }
        }
        _ => {}
    }
}

/// Whether an event of this kind can earn this mark (`audit`).
pub fn earns(kind: &EventKind, mark: Mark) -> bool {
    match mark {
        Mark::CameThrough => matches!(kind, EventKind::Recovered { .. }),
        Mark::HasKilled => matches!(
            kind,
            EventKind::Death {
                killer: Some(_),
                ..
            }
        ),
        Mark::BuriedAChild => {
            matches!(kind, EventKind::Death { .. } | EventKind::Perished { .. })
        }
        Mark::RaisedEnemyBarn => matches!(kind, EventKind::BarnRaising { rival: Some(_), .. }),
        Mark::TurnedRidersBack => matches!(
            kind,
            EventKind::TurnedBack {
                watchman: Some(_),
                ..
            }
        ),
        Mark::KeptAStation => matches!(kind, EventKind::Sheltered { .. }),
        Mark::SoldAFugitive => matches!(
            kind,
            EventKind::Captured {
                informer: Some(_),
                ..
            }
        ),
        Mark::KnowsTheRiver => matches!(kind, EventKind::WagonBack { .. }),
        Mark::MarriedAcross => matches!(kind, EventKind::Marriage { .. }),
        Mark::StarvingTime => matches!(kind, EventKind::Desperation { .. }),
        Mark::CountyReader => matches!(
            kind,
            EventKind::Letter {
                reader: Some(_),
                ..
            }
        ),
    }
}

/// Lines for `debug::explain_npc`.
pub fn describe(world: &World, who: NpcId, omniscient: bool) -> Vec<String> {
    world
        .marks
        .held
        .iter()
        .filter(|h| h.who == who && (omniscient || h.mark.public()))
        .map(|h| format!("{} (since {})", h.mark.label(), h.day))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::events::Hardship;

    #[test]
    fn every_mark_is_labeled_and_secrets_stay_out_of_standing() {
        for m in Mark::ALL {
            assert!(!m.label().is_empty());
            if !m.public() {
                assert!(m.standing().is_none(), "{m:?} is private");
            }
        }
    }

    #[test]
    fn a_killing_comes_back_a_year_on() {
        let mut w = World::new(4);
        let (k, v) = (w.head_of(1).unwrap(), w.head_of(2).unwrap());
        w.emit_root(
            EventKind::Death {
                victim: v,
                killer: Some(k),
            },
            None,
        );
        w.run_cascades();
        assert!(has(&w, k, Mark::HasKilled));
        assert!(w.is_scheduled(
            |e| matches!(*e, EventKind::Remembered { who, mark: Mark::HasKilled, .. } if who == k)
        ));
        let fear = w.npc(k).emotions.fear;
        let grief = w.npc(k).emotions.grief;
        w.run_days(366);
        assert!(
            w.events
                .iter()
                .any(|e| matches!(e.kind, EventKind::Remembered { who, .. } if who == k)),
            "the anniversary came"
        );
        let _ = (fear, grief);
    }

    #[test]
    fn marks_are_earned_once() {
        let mut w = World::new(4);
        let who = w.head_of(1).unwrap();
        for _ in 0..3 {
            w.emit_root(
                EventKind::Recovered {
                    who,
                    disease: Disease::Cholera,
                },
                None,
            );
        }
        w.run_cascades();
        assert_eq!(
            w.marks
                .held
                .iter()
                .filter(|h| h.who == who && h.mark == Mark::CameThrough)
                .count(),
            1
        );
    }

    #[test]
    fn a_child_buried_marks_both_parents() {
        let mut w = World::new(4);
        let Some(child) = w
            .living()
            .find(|n| LifeStage::of(n.age) == LifeStage::Child && n.family != 0)
            .map(|n| n.id)
        else {
            return;
        };
        let f = w.npc(child).family;
        w.emit_root(
            EventKind::Perished {
                victim: child,
                cause: Hardship::Sickness(Disease::Measles),
            },
            None,
        );
        w.run_cascades();
        let parents = w
            .living()
            .filter(|n| n.family == f && LifeStage::of(n.age) == LifeStage::Adult)
            .count();
        let marked = w
            .marks
            .held
            .iter()
            .filter(|h| h.mark == Mark::BuriedAChild)
            .count();
        assert_eq!(marked, parents);
    }
}
