//! The Underground Railroad through Douglas County (MVP §25: a freedom
//! seeker at your door). People escaping slavery in western Missouri came
//! through Lawrence toward Nebraska and Iowa, traveling in the dark of the
//! moon, trusting houses by word passed among them. The territorial slave
//! code of 1855 made aiding them a felony; pursuers paid for information.
//!
//! Seekers act: they pick doors by what they've heard, they wait for dark
//! nights, and they move on early when they sense pursuit. Families decide
//! by what they privately believe, not what they say in town. Food is the
//! evidence: a household feeding one more mouth is noticed by a watchful
//! neighbor, and a watchful neighbor is who the pursuers ask.

use super::calendar::Day;
use super::events::{EventKind, WorldEvent};
use super::psyche::LifeStage;
use super::world::{Faction, FamilyId, NpcId, PLAYER, World, distance};

const NAMES: [&str; 16] = [
    "Jane",
    "Isaac",
    "Mary",
    "Nelson",
    "Charlotte",
    "Moses",
    "Lucinda",
    "Henry",
    "Harriet",
    "George",
    "Dinah",
    "Peter",
    "Rachel",
    "Daniel",
    "Milly",
    "Anthony",
];
const FROM: [&str; 6] = [
    "Platte County",
    "Clay County",
    "Jackson County",
    "Lafayette County",
    "Cass County",
    "Howard County",
];

/// Stations to pass through before reaching free soil to the north.
const STATIONS_TO_FREEDOM: u8 = 2;
/// A reward, in dollars, for word that leads to a capture.
const REWARD: u16 = 50;
/// A conviction under the slave code, as a fine the county could collect.
const FINE: u16 = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Between doors, on a dark night.
    Traveling,
    Hidden(FamilyId),
    Free,
    Taken,
}

#[derive(Clone, Debug)]
pub struct Seeker {
    pub name: &'static str,
    pub from: &'static str,
    pub arrived: Day,
    pub status: Status,
    /// Stations passed.
    pub passed: u8,
    /// Senses danger and moves early.
    pub wary: f32,
    /// Doors already tried.
    pub tried: Vec<FamilyId>,
    /// Neighbors who noticed extra food going into a house.
    pub noticed_by: Vec<NpcId>,
}

#[derive(Clone, Debug, Default)]
pub struct Railroad {
    pub seekers: Vec<Seeker>,
    /// A seeker waiting at the player's door for an answer.
    pub at_door: Option<u16>,
    /// Word among seekers: houses that help, and houses to pass by.
    pub safe: Vec<FamilyId>,
    pub hostile: Vec<FamilyId>,
    /// Pursuers in the county: (seeker, next day they ride through, visits left).
    pub pursuit: Vec<(u16, Day, u8)>,
}

impl Railroad {
    pub fn name(&self, id: u16) -> &'static str {
        self.seekers[id as usize].name
    }
}

fn farm_families(world: &World) -> Vec<FamilyId> {
    world
        .families
        .iter()
        .filter(|f| f.farms() && world.head_of(f.id).is_some())
        .map(|f| f.id)
        .collect()
}

pub fn daily(world: &mut World) {
    let dark = world.day.moonlight() < 0.4;
    // Arrivals, on dark nights; more as the Lane Trail opens in 1857.
    let rate = if world.day.date().0 >= 1857 {
        0.06
    } else {
        0.035
    };
    if dark && world.rng.chance(rate) {
        arrive(world);
    }
    // Seekers on the road knock at the next door; hidden ones move on when
    // the night is dark and they've rested.
    for i in 0..world.railroad.seekers.len() {
        let s = &world.railroad.seekers[i];
        if s.status == Status::Traveling && s.arrived != world.day {
            if dark {
                knock(world, i as u16);
            }
            continue;
        }
        if let Status::Hidden(f) = s.status {
            let rested = world.day.0 >= s.arrived.0 + 4;
            let hunted = world.railroad.pursuit.iter().any(|p| p.0 as usize == i);
            if (dark && rested) || (hunted && world.rng.chance(s.wary)) {
                move_on(world, i as u16, Some(f));
            } else {
                // One more at the table: food, and someone may notice.
                world.families[f as usize].stores.food -= 1.0;
                notice(world, i, f);
            }
        }
    }
    // Pursuers ride in.
    let due: Vec<(u16, u8)> = world
        .railroad
        .pursuit
        .iter()
        .filter(|p| p.1 <= world.day)
        .map(|p| (p.0, p.2))
        .collect();
    world.railroad.pursuit.retain(|p| p.1 > world.day);
    for (s, left) in due {
        let still_here = search(world, s);
        // They stay a week or so, asking again.
        if still_here && left > 1 {
            let next = Day(world.day.0 + 2 + world.rng.range(0, 2));
            world.railroad.pursuit.push((s, next, left - 1));
        }
    }
}

fn arrive(world: &mut World) {
    let id = world.railroad.seekers.len() as u16;
    let name = NAMES[world.rng.range(0, NAMES.len() as u32) as usize];
    let from = FROM[world.rng.range(0, FROM.len() as u32) as usize];
    let wary = 0.3 + 0.6 * world.rng.unit();
    world.railroad.seekers.push(Seeker {
        name,
        from,
        arrived: world.day,
        status: Status::Traveling,
        passed: 0,
        wary,
        tried: Vec::new(),
        noticed_by: Vec::new(),
    });
    // Pursuers follow a few days behind.
    let behind = Day(world.day.0 + 1 + world.rng.range(0, 3));
    world.railroad.pursuit.push((id, behind, 3));
    knock(world, id);
}

/// Choose a door: a house the word says is safe, else a house not known to
/// be hostile.
fn knock(world: &mut World, id: u16) {
    let tried = world.railroad.seekers[id as usize].tried.clone();
    let open: Vec<FamilyId> = farm_families(world)
        .into_iter()
        .filter(|f| !tried.contains(f) && !world.railroad.hostile.contains(f))
        .collect();
    let safe: Vec<FamilyId> = open
        .iter()
        .copied()
        .filter(|f| world.railroad.safe.contains(f))
        .collect();
    let pool = if !safe.is_empty() && world.rng.chance(0.75) {
        safe
    } else {
        open
    };
    if pool.is_empty() || tried.len() >= 3 {
        // No door here: on into Lawrence, where the conductors are.
        let free = world.rng.chance(0.8);
        world.railroad.seekers[id as usize].status =
            if free { Status::Free } else { Status::Taken };
        world.emit_root(
            if free {
                EventKind::Freedom { seeker: id }
            } else {
                EventKind::Captured {
                    seeker: id,
                    at: None,
                    informer: None,
                }
            },
            None,
        );
        return;
    }
    // Word says which houses came from Missouri: those doors are the last resort.
    let weights: Vec<f32> = pool
        .iter()
        .map(|&f| match world.families[f as usize].faction {
            Faction::FreeState => 4.0,
            Faction::ProSlavery => 1.0,
        })
        .collect();
    let mut roll = world.rng.unit() * weights.iter().sum::<f32>();
    let mut door = pool[pool.len() - 1];
    for (&f, &w) in pool.iter().zip(&weights) {
        if roll < w {
            door = f;
            break;
        }
        roll -= w;
    }
    world.railroad.seekers[id as usize].tried.push(door);
    world.emit_root(
        EventKind::SeekerAtDoor {
            seeker: id,
            family: door,
        },
        None,
    );
}

/// How a household answers the door: by what the head privately believes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Shelter,
    TurnAway,
    Betray,
}

pub fn answer_for(world: &mut World, family: FamilyId) -> Answer {
    let Some(head) = world.head_of(family) else {
        return Answer::TurnAway;
    };
    let n = world.npc(head);
    let heart = n.ideology.private;
    let t = n.temperament;
    let fear = n.emotions.fear / 200.0;
    if heart > 0.0 {
        let p = 0.15 + 0.6 * heart + 0.2 * t.generosity + 0.1 * t.courage - fear;
        if world.rng.chance(p.clamp(0.05, 0.95)) {
            return Answer::Shelter;
        }
        return Answer::TurnAway;
    }
    // The reward is fifty dollars.
    let greed = 0.2 + 0.4 * (1.0 - t.honesty) + 0.3 * n.hidden.malice + 0.3 * -heart;
    if world.rng.chance(greed.clamp(0.05, 0.9)) {
        Answer::Betray
    } else {
        Answer::TurnAway
    }
}

pub fn respond(world: &mut World, id: u16, family: FamilyId, answer: Answer) {
    if family == 0 {
        world.railroad.at_door = None;
    }
    match answer {
        Answer::Shelter => {
            world.railroad.seekers[id as usize].status = Status::Hidden(family);
            world.emit_root(EventKind::Sheltered { seeker: id, family }, None);
        }
        Answer::TurnAway => {
            world.emit_root(EventKind::TurnedAway { seeker: id, family }, None);
            world.railroad.hostile.push(family);
            // Another door, another night.
            world.railroad.seekers[id as usize].status = Status::Traveling;
        }
        Answer::Betray => {
            let informer = world.head_of(family).unwrap_or(PLAYER);
            world.railroad.hostile.push(family);
            world.railroad.seekers[id as usize].status = Status::Taken;
            world.families[family as usize].stores.cash += REWARD as i32;
            world.emit_root(
                EventKind::Captured {
                    seeker: id,
                    at: None,
                    informer: Some(informer),
                },
                None,
            );
        }
    }
}

/// A watchful neighbor sees more food going into a house than the house eats.
fn notice(world: &mut World, i: usize, family: FamilyId) {
    let home = world.families[family as usize].farm;
    let watchers: Vec<NpcId> = world
        .living()
        .filter(|n| n.family != family && LifeStage::of(n.age) != LifeStage::Child)
        .filter(|n| distance(world.farm_of(n.id), home) <= 6.0)
        .map(|n| n.id)
        .collect();
    for w in watchers {
        let a = world.npc(w).body.alertness;
        if world.rng.chance(0.06 * a) && !world.railroad.seekers[i].noticed_by.contains(&w) {
            world.railroad.seekers[i].noticed_by.push(w);
        }
    }
}

fn move_on(world: &mut World, id: u16, from: Option<FamilyId>) {
    if let Some(f) = from
        && !world.railroad.safe.contains(&f)
    {
        world.railroad.safe.push(f);
    }
    let s = &mut world.railroad.seekers[id as usize];
    s.passed += 1;
    s.status = Status::Traveling;
    s.arrived = world.day;
    if s.passed >= STATIONS_TO_FREEDOM {
        s.status = Status::Free;
        world.emit_root(EventKind::Freedom { seeker: id }, None);
    }
}

/// Pursuers ride through asking. Whoever noticed, and leans pro-slavery,
/// talks; failing that, they search a Free-State house or two on a hunch.
/// Returns whether the seeker is still in the county.
fn search(world: &mut World, id: u16) -> bool {
    let s = world.railroad.seekers[id as usize].clone();
    if matches!(s.status, Status::Free | Status::Taken) {
        return false; // too late, or too late for them
    }
    if s.passed == 0 || world.rng.chance(0.5) {
        world.emit_root(EventKind::Pursuers { seeker: id }, None);
    }
    let Status::Hidden(at) = s.status else {
        return true;
    };
    let informer = s.noticed_by.iter().copied().find(|&n| {
        world.npc(n).alive
            && (world.npc(n).ideology.private < 0.0 || world.npc(n).hidden.malice > 0.3)
    });
    let hunch = world.families[at as usize].faction == Faction::FreeState && world.rng.chance(0.12);
    if informer.is_none() && !hunch {
        return true; // nobody talked, and the loft held
    }
    world.railroad.seekers[id as usize].status = Status::Taken;
    if let Some(i) = informer {
        let f = world.npc(i).family as usize;
        world.families[f].stores.cash += REWARD as i32;
    }
    world.emit_root(
        EventKind::Captured {
            seeker: id,
            at: Some(at),
            informer,
        },
        None,
    );
    false
}

/// Take them north yourself: three nights on the road.
pub fn guide(world: &mut World, id: u16) -> bool {
    let s = &world.railroad.seekers[id as usize];
    if !matches!(s.status, Status::Hidden(0)) {
        return false;
    }
    let hunted = world.railroad.pursuit.iter().any(|p| p.0 == id);
    let made_it = world.rng.chance(if hunted { 0.6 } else { 0.85 });
    world.railroad.pursuit.retain(|p| p.0 != id);
    if made_it {
        world.railroad.seekers[id as usize].status = Status::Free;
        world.emit_root(EventKind::Freedom { seeker: id }, None);
    } else {
        world.railroad.seekers[id as usize].status = Status::Taken;
        world.emit_root(
            EventKind::Captured {
                seeker: id,
                at: Some(0),
                informer: None,
            },
            None,
        );
    }
    true
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::SeekerAtDoor { seeker, family } => {
            if family == 0 && !world.autopilot_player && world.player_alive() {
                world.railroad.at_door = Some(seeker);
            } else {
                let a = answer_for(world, family);
                respond(world, seeker, family, a);
            }
        }
        EventKind::Captured { at: Some(f), .. } => {
            // Harboring under the slave code, before the pro-slavery justice.
            if let Some(head) = world.head_of(f) {
                let judge_ps = world
                    .law
                    .justice
                    .is_some_and(|j| world.npc(j).faction == Faction::ProSlavery);
                let convicted = judge_ps && world.rng.chance(0.7);
                world.emit_child(
                    ev,
                    EventKind::Charged {
                        accused: if f == 0 { PLAYER } else { head },
                        convicted,
                        fine: if convicted { FINE } else { 0 },
                    },
                );
            }
            world.add_grievance(Faction::FreeState, 8);
        }
        EventKind::Captured { at: None, .. } => world.add_grievance(Faction::FreeState, 4),
        EventKind::Charged {
            accused,
            convicted: true,
            fine,
        } => {
            let f = world.npc(accused).family as usize;
            let hh = &mut world.families[f].stores;
            let paid = (fine as i32).min(hh.cash.max(0));
            hh.cash -= paid;
            hh.debt += fine as i32 - paid;
            world.add_grievance(Faction::FreeState, 6);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_seeker(w: &mut World) -> u16 {
        w.railroad.seekers.push(Seeker {
            name: "Jane",
            from: "Platte County",
            arrived: w.day,
            status: Status::Traveling,
            passed: 0,
            wary: 0.5,
            tried: Vec::new(),
            noticed_by: Vec::new(),
        });
        (w.railroad.seekers.len() - 1) as u16
    }

    #[test]
    fn the_player_answers_their_own_door() {
        let mut w = World::new(2);
        w.autopilot_player = false;
        let id = with_seeker(&mut w);
        w.emit_root(
            EventKind::SeekerAtDoor {
                seeker: id,
                family: 0,
            },
            None,
        );
        w.run_cascades();
        assert_eq!(w.railroad.at_door, Some(id));
        respond(&mut w, id, 0, Answer::Shelter);
        w.run_cascades();
        assert_eq!(w.railroad.seekers[id as usize].status, Status::Hidden(0));
    }

    #[test]
    fn a_hidden_seeker_eats() {
        let mut w = World::new(2);
        w.autopilot_player = false;
        let id = with_seeker(&mut w);
        respond(&mut w, id, 0, Answer::Shelter);
        w.families[0].stores.food = 500.0;
        // Bright moon: they wait.
        while w.day.moonlight() < 0.6 {
            w.advance_day();
        }
        w.railroad.seekers[id as usize].status = Status::Hidden(0);
        w.railroad.seekers[id as usize].arrived = w.day;
        let food = w.families[0].stores.food;
        w.families[0].stores.hungry_days = 0;
        super::daily(&mut w);
        assert!(w.families[0].stores.food < food);
    }

    #[test]
    fn a_neighbor_who_noticed_can_bring_them_down_on_you() {
        let mut w = World::new(4);
        let id = with_seeker(&mut w);
        let f = 1;
        w.railroad.seekers[id as usize].status = Status::Hidden(f);
        let watcher = w
            .living()
            .find(|n| n.family != f && n.ideology.private < 0.0)
            .unwrap()
            .id;
        w.railroad.seekers[id as usize].noticed_by.push(watcher);
        search(&mut w, id);
        w.run_cascades();
        assert_eq!(w.railroad.seekers[id as usize].status, Status::Taken);
        assert!(
            w.events
                .iter()
                .any(|e| matches!(e.kind, EventKind::Charged { .. }))
        );
    }

    #[test]
    fn hearts_decide_the_door() {
        let mut w = World::new(6);
        let f = 3;
        let head = w.head_of(f).unwrap();
        w.npc_mut(head).ideology.private = 1.0;
        w.npc_mut(head).temperament.generosity = 1.0;
        let sheltered = (0..50)
            .filter(|_| answer_for(&mut w, f) == Answer::Shelter)
            .count();
        w.npc_mut(head).ideology.private = -1.0;
        let sheltered_ps = (0..50)
            .filter(|_| answer_for(&mut w, f) == Answer::Shelter)
            .count();
        assert!(sheltered > 35 && sheltered_ps == 0);
    }

    #[test]
    fn seekers_reach_freedom_over_time() {
        let freed = (1..=8).any(|s| {
            let mut w = World::new(s);
            w.run_days(500);
            w.events
                .iter()
                .any(|e| matches!(e.kind, EventKind::Freedom { .. }))
        });
        assert!(freed);
    }
}
