//! Kin and absence. You can go fishing, drinking or courting for a week and
//! the world does not stop for it: if the barn burns or a cousin dies while
//! you're gone, your family will want to know where you were. Being away is
//! an alibi too — nobody blames the man who was on the Smoky Hill.

use super::calendar::Day;
use super::events::{EventId, EventKind, WorldEvent};
use super::world::{NpcId, PLAYER, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Errand {
    /// Weeks on the plains, meat for the winter. Forgivable.
    Buffalo,
    /// A few days on the Wakarusa.
    Fishing,
    /// Calling on someone. Kin take a dim view.
    Courting,
    /// In Lawrence on business: land office, the store, the paper.
    Town,
    /// At the groggery. Kin take the dimmest view.
    Drinking,
}

impl Errand {
    pub fn days(self) -> u32 {
        match self {
            Errand::Buffalo => 21,
            Errand::Fishing => 3,
            Errand::Courting => 4,
            Errand::Town => 2,
            Errand::Drinking => 3,
        }
    }

    /// How much kin hold the absence against you.
    pub fn reproach(self) -> f32 {
        match self {
            Errand::Buffalo => 0.4,
            Errand::Town => 0.6,
            Errand::Fishing => 0.9,
            Errand::Courting => 1.2,
            Errand::Drinking => 1.5,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Errand::Buffalo => "out on the buffalo range",
            Errand::Fishing => "off fishing",
            Errand::Courting => "off courting",
            Errand::Town => "in town",
            Errand::Drinking => "at the groggery",
        }
    }
}

pub fn leave(world: &mut World, errand: Errand) {
    let back = Day(world.day.0 + errand.days());
    world.player_away = Some((back, errand));
    world.npc_mut(PLAYER).alibi = Some(world.day);
}

pub fn away(world: &World) -> Option<Errand> {
    world
        .player_away
        .filter(|&(back, _)| world.day < back)
        .map(|(_, e)| e)
}

pub fn daily(world: &mut World) {
    match world.player_away {
        Some((back, _)) if world.day >= back => world.player_away = None,
        Some(_) => {
            let day = world.day;
            world.npc_mut(PLAYER).alibi = Some(day);
        }
        None => {}
    }
}

/// How bad a thing it was to miss.
fn weight(ev: &WorldEvent) -> Option<(NpcId, f32)> {
    let (who, w) = match ev.kind {
        EventKind::Death { victim, .. } | EventKind::Perished { victim, .. } => (victim, 35.0),
        EventKind::Wounded { victim, .. } => (victim, 20.0),
        EventKind::Fire { owner, .. } => (owner, 20.0),
        EventKind::Theft { victim, .. } => (victim, 10.0),
        _ => return None,
    };
    Some((who, w))
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::WhereWereYou {
            kin,
            errand,
            missed,
        } => {
            let Some((_, w)) = weight(&world.events[missed as usize]) else {
                return;
            };
            let forgiving = 1.0 - 0.5 * world.npc(kin).temperament.generosity;
            let delta = -(w * errand.reproach() * forgiving) as i16;
            world.adjust_opinion(kin, PLAYER, delta);
        }
        _ => {
            let Some(errand) = away(world) else {
                return;
            };
            let Some((who, _)) = weight(ev) else {
                return;
            };
            if world.npc(who).family != 0 || !world.player_alive() {
                return;
            }
            let kin: Vec<NpcId> = world
                .living()
                .filter(|n| n.family == 0 && n.id != PLAYER)
                .map(|n| n.id)
                .collect();
            for k in kin {
                world.emit_child(
                    ev,
                    EventKind::WhereWereYou {
                        kin: k,
                        missed: ev.id,
                        errand,
                    },
                );
            }
        }
    }
}

/// For the view: the one line kin say when you come home.
pub fn missed(world: &World) -> Vec<EventId> {
    world
        .events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::WhereWereYou { missed, .. } => Some(missed),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::events::Hardship;

    fn cousin(w: &World) -> NpcId {
        w.living()
            .find(|n| n.family == 0 && n.id != PLAYER)
            .unwrap()
            .id
    }

    fn opinion_after_death(errand: Option<Errand>) -> i16 {
        let mut w = World::new(2);
        let c = cousin(&w);
        let other = w
            .living()
            .find(|n| n.family == 0 && n.id != PLAYER && n.id != c)
            .map(|n| n.id);
        if let Some(e) = errand {
            w.player_leave(e);
        }
        let victim = other.unwrap_or(c);
        w.emit_root(
            EventKind::Perished {
                victim,
                cause: Hardship::Fever,
            },
            None,
        );
        w.run_cascades();
        w.opinion(c, PLAYER)
    }

    #[test]
    fn kin_remember_where_you_were() {
        let home = opinion_after_death(None);
        let fishing = opinion_after_death(Some(Errand::Fishing));
        let drunk = opinion_after_death(Some(Errand::Drinking));
        assert!(fishing < home, "{fishing} vs {home}");
        assert!(drunk < fishing, "{drunk} vs {fishing}");
    }

    #[test]
    fn you_come_home() {
        let mut w = World::new(2);
        w.player_leave(Errand::Fishing);
        assert!(away(&w).is_some());
        w.run_days(4);
        assert!(away(&w).is_none());
    }
}
