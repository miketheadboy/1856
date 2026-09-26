//! Institutions (§9): places that do things to the people who use them.
//!
//! Period-accurate for Douglas County, 1855–56. New-settled Lawrence was a
//! New England temperance town; the drinking was in pro-slavery Franklin, and
//! the vice was downriver in Westport and Leavenworth. Each institution earns
//! its place with a mechanic:
//!
//! - gathering places amplify gossip (ext. §14: the tavern hardens a story),
//!   give alibis, and bring enemies into the same room;
//! - the church thaws grudges and feeds its hungry;
//! - the river towns drain cash, carry disease, and make secrets that the
//!   malicious can sell back to you (§14.5).

use super::events::{EventKind, Suspect};
use super::geography;
use super::market::Good;
use super::psyche::{self, LifeStage};
use super::world::{Faction, NpcId, PLAYER, World, distance};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Venue {
    /// Franklin groggery. Pro-Slavery crowd, whiskey, fights.
    JackOfHearts,
    /// Lawrence. Emigrants, baths, Free-State meetings. Burned May 1856.
    FreeStateHotel,
    /// Lawrence, Congregational. Sunday meeting and poor relief.
    PlymouthChurch,
    /// Lawrence. Shaves, baths, talk.
    Barbershop,
}

impl Venue {
    pub const ALL: [Venue; 4] = [
        Venue::JackOfHearts,
        Venue::FreeStateHotel,
        Venue::PlymouthChurch,
        Venue::Barbershop,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Venue::JackOfHearts => "the Jack of Hearts",
            Venue::FreeStateHotel => "the Free State Hotel",
            Venue::PlymouthChurch => "Plymouth church",
            Venue::Barbershop => "the barbershop",
        }
    }

    fn town(self) -> &'static str {
        match self {
            Venue::JackOfHearts => "Franklin",
            _ => "Lawrence",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

/// A shameful thing someone did, and who knows it (§14.5).
#[derive(Clone, Debug)]
pub struct Secret {
    pub about: NpcId,
    pub known_by: Vec<NpcId>,
    pub exposed: bool,
}

#[derive(Clone, Debug)]
pub struct Institutions {
    /// Which venues are standing (indexed by `Venue`).
    pub open: [bool; 4],
    /// Food the church has collected for its poor, in days.
    pub relief: f32,
    pub secrets: Vec<Secret>,
}

impl Default for Institutions {
    fn default() -> Self {
        Self {
            open: [true; 4],
            relief: 60.0,
            secrets: Vec::new(),
        }
    }
}

/// Would this person go to this place this week?
fn draws(world: &World, id: NpcId, venue: Venue) -> f32 {
    let n = world.npc(id);
    if LifeStage::of(n.age) == LifeStage::Child || n.wounded {
        return 0.0;
    }
    let t = n.temperament;
    let town = geography::to_tile(geography::place(venue.town()));
    let near = (1.0 - distance(world.farm_of(id), town) / 40.0).max(0.1);
    let lean = match venue {
        Venue::JackOfHearts => {
            let side = if n.faction == Faction::ProSlavery {
                1.0
            } else {
                0.25
            };
            side * (0.3 * t.temper + 0.4 * (1.0 - t.piety))
        }
        Venue::FreeStateHotel | Venue::Barbershop => {
            let side = if n.faction == Faction::FreeState {
                1.0
            } else {
                0.2
            };
            side * 0.3 * t.sociability
        }
        Venue::PlymouthChurch => {
            let side = if n.faction == Faction::FreeState {
                1.0
            } else {
                0.3
            };
            side * 0.8 * t.piety
        }
    };
    lean * near
}

/// Weekly: who went where, and what happened there.
pub fn weekly(world: &mut World) {
    // Saturday night in town; Sunday meeting.
    let sunday = world.day.0 % 7 == 3;
    let saturday = world.day.0 % 7 == 2;
    if !sunday && !saturday {
        return;
    }
    for venue in Venue::ALL {
        if !world.institutions.open[venue.index()] {
            continue;
        }
        // Church meets Sundays; the rest any day of the week.
        if (venue == Venue::PlymouthChurch) != sunday {
            continue;
        }
        let people: Vec<NpcId> = world
            .living()
            .filter(|n| n.id != PLAYER)
            .map(|n| n.id)
            .collect();
        let mut present = Vec::new();
        for id in people {
            let p = draws(world, id, venue);
            if world.rng.chance(p) {
                present.push(id);
            }
        }
        if present.len() < 2 {
            continue;
        }
        gather(world, venue, &present);
    }
}

fn gather(world: &mut World, venue: Venue, present: &[NpcId]) {
    let day = world.day;
    for &id in present {
        world.npc_mut(id).alibi = Some(day);
    }

    // Talk. Each patron with a fresh accusation tells someone in the room:
    // a public room hardens one version of events.
    for &teller in present {
        let fresh = world
            .npc(teller)
            .memories
            .iter()
            .filter(|m| day.0.saturating_sub(m.day.0) <= 30 && m.confidence >= 30)
            .filter(|m| matches!(m.believed, Suspect::Person(_) | Suspect::Nation(_)))
            .max_by_key(|m| m.day)
            .map(|m| (m.event, m.believed));
        let Some((about, blamed)) = fresh else {
            continue;
        };
        let listeners: Vec<NpcId> = present.iter().copied().filter(|&l| l != teller).collect();
        if let Some(&listener) = world.rng.pick(&listeners) {
            world.emit_root(
                EventKind::Gossip {
                    teller,
                    listener,
                    about,
                    blamed,
                },
                Some(about),
            );
        }
    }

    match venue {
        Venue::JackOfHearts => groggery(world, present),
        Venue::PlymouthChurch => church(world, present),
        _ => {}
    }
}

/// Whiskey, and men who hate each other in the same room.
fn groggery(world: &mut World, present: &[NpcId]) {
    for &id in present {
        let family = world.npc(id).family;
        if super::market::buy(world, family, Good::Whiskey, 1.0) > 0.0 {
            super::market::consume(world, family, Good::Whiskey);
            psyche::feel(world, id, |e| {
                e.anger += 5.0;
                e.fear -= 8.0;
            });
        }
    }
    let mut pairs = Vec::new();
    for (i, &a) in present.iter().enumerate() {
        for &b in &present[i + 1..] {
            let bad = world.opinion(a, b).min(world.opinion(b, a));
            let heat = world.npc(a).emotions.anger.max(world.npc(b).emotions.anger);
            if bad <= -40 && heat > 30.0 {
                pairs.push((a, b));
            }
        }
    }
    if let Some(&(a, b)) = world.rng.pick(&pairs) {
        for (x, y) in [(a, b), (b, a)] {
            let n = world.npc_mut(x);
            n.health = (n.health - 10).max(1);
            world.adjust_opinion(x, y, -10);
        }
        world.emit_root(
            EventKind::Brawl {
                venue: Venue::JackOfHearts,
                a,
                b,
            },
            None,
        );
    }
}

/// Sunday meeting: grudges soften, and the congregation feeds its hungry.
fn church(world: &mut World, present: &[NpcId]) {
    for &id in present {
        let family = world.npc(id).family as usize;
        let n = world.npc(id);
        let gives = n.temperament.generosity > 0.5 && world.families[family].stores.food > 150.0;
        if gives {
            world.families[family].stores.food -= 5.0;
            world.institutions.relief += 5.0;
        }
        // A sermon on forgiveness takes a little off every grudge.
        psyche::feel(world, id, |e| e.anger *= 0.8);
    }
    for &id in present {
        let family = world.npc(id).family as usize;
        if world.families[family].stores.hungry_days > 0 && world.institutions.relief >= 20.0 {
            world.institutions.relief -= 20.0;
            world.families[family].stores.food += 20.0;
        }
    }
}

/// Monthly: trips downriver. Money leaves the county; sometimes disease comes
/// back; always someone might have seen you.
pub fn monthly(world: &mut World) {
    let travelers: Vec<NpcId> = world
        .living()
        .filter(|n| n.id != PLAYER && LifeStage::of(n.age) == LifeStage::Adult)
        .filter(|n| n.temperament.temper > 0.55 && n.temperament.piety < 0.35)
        .map(|n| n.id)
        .collect();
    for id in travelers {
        let family = world.npc(id).family as usize;
        if world.families[family].stores.cash < 5 || !world.rng.chance(0.25) {
            continue;
        }
        let spent = world.rng.range(3, 12) as i32;
        world.families[family].stores.cash = (world.families[family].stores.cash - spent).max(0);
        if world.rng.chance(0.08) {
            let n = world.npc_mut(id);
            n.health = (n.health - 20).max(1);
        }
        // Somebody from the county was on the same boat.
        let witnesses: Vec<NpcId> = world
            .living()
            .filter(|w| w.id != id && w.family as usize != family)
            .map(|w| w.id)
            .collect();
        if let Some(&w) = world.rng.pick(&witnesses) {
            if let Some(s) = world
                .institutions
                .secrets
                .iter_mut()
                .find(|s| s.about == id)
            {
                if !s.known_by.contains(&w) {
                    s.known_by.push(w);
                }
            } else {
                world.institutions.secrets.push(Secret {
                    about: id,
                    known_by: vec![w],
                    exposed: false,
                });
            }
        }
    }
    blackmail(world);
}

/// The malicious sell silence. When the victim can't pay, everyone hears.
fn blackmail(world: &mut World) {
    for i in 0..world.institutions.secrets.len() {
        let s = world.institutions.secrets[i].clone();
        if s.exposed || !world.npc(s.about).alive {
            continue;
        }
        let Some(&knower) = s
            .known_by
            .iter()
            .filter(|k| world.npc(**k).alive)
            .max_by(|a, b| {
                world
                    .npc(**a)
                    .hidden
                    .malice
                    .total_cmp(&world.npc(**b).hidden.malice)
            })
        else {
            continue;
        };
        let malice = world.npc(knower).hidden.malice;
        if malice < 0.2 || !world.rng.chance(malice) {
            continue;
        }
        let victim_family = world.npc(s.about).family as usize;
        let knower_family = world.npc(knower).family as usize;
        let cash = world.families[victim_family].stores.cash;
        if cash >= 10 {
            world.families[victim_family].stores.cash -= 10;
            world.families[knower_family].stores.cash += 10;
            world.adjust_opinion(s.about, knower, -30);
            world.emit_root(
                EventKind::Blackmail {
                    victim: s.about,
                    extorter: knower,
                    paid: true,
                },
                None,
            );
        } else {
            expose(world, i, knower);
        }
    }
}

/// Tell the county. The pious take it hardest.
pub fn expose(world: &mut World, i: usize, by: NpcId) {
    let s = world.institutions.secrets[i].clone();
    let victim_family = world.npc(s.about).family as usize;
    world.institutions.secrets[i].exposed = true;
    let id = world.emit_root(
        EventKind::Blackmail {
            victim: s.about,
            extorter: by,
            paid: false,
        },
        None,
    );
    let judges: Vec<(NpcId, f32)> = world
        .living()
        .filter(|n| n.id != s.about && n.family as usize != victim_family)
        .map(|n| (n.id, n.temperament.piety))
        .collect();
    let parent = world.events[id as usize].clone();
    for (j, piety) in judges {
        let delta = -(5.0 + 20.0 * piety) as i16;
        world.emit_child(
            &parent,
            EventKind::OpinionChange {
                holder: j,
                target: s.about,
                delta,
                after: 0,
            },
        );
    }
}

/// The Sack of Lawrence burns the hotel.
pub fn close(world: &mut World, venue: Venue) {
    world.institutions.open[venue.index()] = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gatherings_give_alibis_and_spread_talk() {
        let mut w = World::new(3);
        let present: Vec<NpcId> = w
            .living()
            .filter(|n| n.id != PLAYER)
            .map(|n| n.id)
            .take(4)
            .collect();
        gather(&mut w, Venue::FreeStateHotel, &present);
        for id in &present {
            assert_eq!(w.npc(*id).alibi, Some(w.day));
        }
    }

    #[test]
    fn church_relief_feeds_the_hungry() {
        let mut w = World::new(3);
        let id = w.head_of(2).unwrap();
        let family = w.npc(id).family as usize;
        w.families[family].stores.hungry_days = 3;
        let before = w.families[family].stores.food;
        church(&mut w, &[id]);
        assert!(w.families[family].stores.food > before);
    }

    #[test]
    fn unpaid_blackmail_becomes_scandal() {
        let mut w = World::new(3);
        let (victim, knower) = (w.head_of(1).unwrap(), w.head_of(2).unwrap());
        w.npc_mut(knower).hidden.malice = 1.0;
        let vf = w.npc(victim).family as usize;
        w.families[vf].stores.cash = 0;
        w.institutions.secrets.push(Secret {
            about: victim,
            known_by: vec![knower],
            exposed: false,
        });
        blackmail(&mut w);
        assert!(w.institutions.secrets[0].exposed);
        assert!(
            w.events
                .iter()
                .any(|e| matches!(e.kind, EventKind::Blackmail { paid: false, .. }))
        );
    }

    #[test]
    fn the_hotel_can_burn() {
        let mut w = World::new(3);
        close(&mut w, Venue::FreeStateHotel);
        assert!(!w.institutions.open[Venue::FreeStateHotel.index()]);
    }
}
