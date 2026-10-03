//! Law, ballots and muster rolls. The territory's officers were appointed by
//! the "bogus" legislature of 1855, so the justice of the peace is a
//! pro-slavery man; a Free-State plaintiff can be right and lose anyway.
//! Elections were fought as much as voted: Missourians crossed the line for
//! territorial polls, Free-State men held their own elections under the
//! Topeka movement, and votes were bought at the store. Militia musters came
//! with each crisis; the neighbors noticed who answered and who didn't.

use super::calendar::Day;
use super::events::{EventKind, WorldEvent};
use super::psyche::LifeStage;
use super::world::{Faction, FamilyId, NpcId, PLAYER, World, is_woman};

/// Chance a captain keeps a dead man's name on the roll.
const PAD_ODDS: f32 = 0.5;

/// Days a musterer is gone.
pub const SERVICE_DAYS: u32 = 10;
/// Days to answer a muster call before you're counted a shirker.
pub const ANSWER_DAYS: u32 = 3;
/// Court fees at the justice's.
const FEES: i32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Poll {
    /// Topeka movement votes: Free-State men only; the other side calls it
    /// insurrection.
    FreeState,
    /// Territorial elections run by pro-slavery officers. Missourians cross.
    Territorial,
}

pub struct ElectionDay {
    pub date: (i32, u32, u32),
    pub name: &'static str,
    pub poll: Poll,
    /// Free-State men sat this one out.
    pub boycott: bool,
    /// Walker threw out the fraudulent returns (October 1857).
    pub fraud_rejected: bool,
}

pub const ELECTIONS: &[ElectionDay] = &[
    ElectionDay {
        date: (1855, 12, 15),
        name: "the Topeka Constitution",
        poll: Poll::FreeState,
        boycott: false,
        fraud_rejected: false,
    },
    ElectionDay {
        date: (1856, 1, 15),
        name: "Free-State officers under Topeka",
        poll: Poll::FreeState,
        boycott: false,
        fraud_rejected: false,
    },
    ElectionDay {
        date: (1856, 10, 6),
        name: "delegate to Congress",
        poll: Poll::Territorial,
        boycott: true,
        fraud_rejected: false,
    },
    ElectionDay {
        date: (1857, 10, 5),
        name: "the territorial legislature",
        poll: Poll::Territorial,
        boycott: false,
        fraud_rejected: true,
    },
];

pub struct MusterCall {
    pub date: (i32, u32, u32),
    pub name: &'static str,
    /// Which sides call men out.
    pub free_state: bool,
    pub pro_slavery: bool,
    /// Shots were fired.
    pub fight: bool,
}

pub const MUSTERS: &[MusterCall] = &[
    MusterCall {
        date: (1855, 11, 28),
        name: "the Wakarusa War",
        free_state: true,
        pro_slavery: true,
        fight: false,
    },
    MusterCall {
        date: (1856, 5, 14),
        name: "Sheriff Jones's posse",
        free_state: false,
        pro_slavery: true,
        fight: false,
    },
    MusterCall {
        date: (1856, 8, 10),
        name: "Lane's men at Franklin",
        free_state: true,
        pro_slavery: false,
        fight: true,
    },
    MusterCall {
        date: (1856, 9, 12),
        name: "Hickory Point",
        free_state: true,
        pro_slavery: true,
        fight: true,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dispute {
    pub plaintiff: FamilyId,
    pub defendant: NpcId,
    pub acres: u8,
    pub day: Day,
}

#[derive(Clone, Debug, Default)]
pub struct Law {
    pub justice: Option<NpcId>,
    pub disputes: Vec<Dispute>,
    /// Open muster: (index into MUSTERS, answer-by day, who joined).
    pub muster: Option<(usize, Day, Vec<NpcId>)>,
    /// The player answered the current call (joined or refused openly).
    pub player_answered: bool,
    /// Dead men kept on the open roll (`RollPadded`).
    pub padded: Vec<NpcId>,
}

impl Law {
    /// The justice of the peace: the best-lettered pro-slavery man.
    pub fn founding(world: &World) -> Self {
        let justice = world
            .living()
            .filter(|n| {
                n.faction == Faction::ProSlavery
                    && !is_woman(&n.name)
                    && LifeStage::of(n.age) != LifeStage::Child
            })
            .map(|n| n.id)
            .max_by(|&a, &b| {
                world
                    .npc(a)
                    .temperament
                    .literacy
                    .total_cmp(&world.npc(b).temperament.literacy)
            });
        Law {
            justice,
            ..Default::default()
        }
    }
}

/// Able to vote and to muster: a grown man, well enough to ride.
pub fn eligible(world: &World, id: NpcId) -> bool {
    let n = world.npc(id);
    n.alive
        && !n.wounded
        && LifeStage::of(n.age) == LifeStage::Adult
        && (id == PLAYER || !is_woman(&n.name))
        && !super::warrant::held(world, id)
}

/// The chance a plaintiff wins before this justice.
pub fn odds(world: &World, plaintiff: NpcId, defendant: NpcId, letters: f32) -> f32 {
    let pf = world.npc(plaintiff).faction;
    let df = world.npc(defendant).faction;
    let filed = world
        .civic
        .filed
        .get(world.npc(plaintiff).family as usize)
        .copied()
        .unwrap_or(false);
    let bias = match (pf, df) {
        (Faction::FreeState, Faction::ProSlavery) => -0.35,
        (Faction::ProSlavery, Faction::FreeState) => 0.2,
        _ => 0.0,
    };
    // Whose word the court takes (`standing`).
    let names =
        0.3 * (super::standing::word(world, plaintiff) - super::standing::word(world, defendant));
    (0.45 + if filed { 0.25 } else { 0.0 } + 0.15 * letters + bias + names).clamp(0.05, 0.95)
}

/// Today is an election day.
pub fn election_today(world: &World) -> Option<&'static ElectionDay> {
    let today = world.day.date();
    ELECTIONS.iter().find(|e| e.date == today)
}

/// Take a neighbor to law over a jumped claim. Returns whether the case was
/// heard (fees paid, a dispute on the books).
pub fn sue(world: &mut World, plaintiff: NpcId, defendant: NpcId, letters: f32) -> bool {
    let family = world.npc(plaintiff).family;
    let Some(i) = world
        .law
        .disputes
        .iter()
        .position(|d| d.plaintiff == family && d.defendant == defendant)
    else {
        return false;
    };
    let Some(judge) = world.law.justice.filter(|&j| world.npc(j).alive) else {
        return false;
    };
    let hh = &mut world.families[family as usize].stores;
    if hh.cash < FEES {
        return false;
    }
    hh.cash -= FEES;
    let d = world.law.disputes.remove(i);
    let won = world.rng.chance(odds(world, plaintiff, defendant, letters));
    world.emit_root(
        EventKind::Lawsuit {
            plaintiff,
            defendant,
            judge,
            acres: d.acres,
            won,
        },
        None,
    );
    true
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::Election { .. } | EventKind::VoteSold { .. } => on_election(world, ev),
        // The dead can't ride: they come off the list of men the day they
        // die. Whether they come off the *roll* is up to the captain.
        EventKind::Death { victim, .. } | EventKind::Perished { victim, .. } => {
            let Some((i, _, men)) = &mut world.law.muster else {
                return;
            };
            let i = *i as u8;
            if !men.contains(&victim) {
                return;
            }
            men.retain(|&m| m != victim);
            if world.rng.chance(PAD_ODDS) {
                world.emit_child(
                    ev,
                    EventKind::RollPadded {
                        name: victim,
                        index: i,
                    },
                );
            }
        }
        // Names kept while the call is open; at the close the captain
        // counts his own.
        EventKind::RollPadded { name, .. } if world.law.muster.is_some() => {
            if !world.law.padded.contains(&name) {
                world.law.padded.push(name);
            }
        }
        EventKind::ClaimJumped {
            family,
            lost,
            neighbor: Some(jumper),
        } if lost > 0 => world.law.disputes.push(Dispute {
            plaintiff: family,
            defendant: jumper,
            acres: lost,
            day: ev.day,
        }),
        EventKind::Lawsuit {
            plaintiff,
            defendant,
            judge,
            acres,
            won,
        } => {
            let family = world.npc(plaintiff).family;
            if won {
                world.families[family as usize].stores.acres += acres as u32;
                world.adjust_opinion(defendant, plaintiff, -10);
                world.adjust_opinion(plaintiff, judge, 5);
            } else {
                world.npc_mut(plaintiff).emotions.anger += 20.0;
                world.adjust_opinion(plaintiff, judge, -15);
                // "The bogus courts."
                if world.npc(plaintiff).faction == Faction::FreeState
                    && world.npc(judge).faction == Faction::ProSlavery
                {
                    world.add_grievance(Faction::FreeState, 3);
                }
            }
        }
        _ => {}
    }
}

pub fn daily(world: &mut World) {
    // Polls close at the end of election day; the count comes the next morning.
    let yesterday = Day(world.day.0.saturating_sub(1)).date();
    if world.day.0 > 0
        && let Some(i) = ELECTIONS.iter().position(|e| e.date == yesterday)
    {
        election(world, i);
    }
    let (y, m, d) = world.day.date();
    if let Some(i) = MUSTERS.iter().position(|c| c.date == (y, m, d)) {
        call_muster(world, i);
    }
    if let Some((i, by, _)) = world.law.muster.clone()
        && world.day >= by
    {
        close_muster(world, i);
    }
}

pub fn monthly(world: &mut World) {
    let today = world.day.0;
    world.law.disputes.retain(|d| today < d.day.0 + 180);
    let open = world.law.disputes.clone();
    for d in open {
        if d.plaintiff == 0 || today < d.day.0 + 14 {
            continue; // the player sues for themselves
        }
        let Some(head) = world.head_of(d.plaintiff) else {
            continue;
        };
        let t = world.npc(head).temperament;
        let prudence = world.families[d.plaintiff as usize].stores.prudence;
        let p = 0.2 + 0.3 * t.literacy + 0.3 * prudence - 0.3 * t.temper;
        if world.rng.chance(p.clamp(0.05, 0.8)) {
            sue(world, head, d.defendant, t.literacy * 0.5);
        }
    }
}

/// How one man votes, or doesn't.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ballot {
    Stayed,
    Voted(Faction),
    /// Sold to the storekeeper's side for a few dollars.
    Sold,
}

pub fn ballot(world: &mut World, id: NpcId, e: &ElectionDay) -> Ballot {
    let n = world.npc(id);
    let t = n.temperament;
    let faction = n.faction;
    let fear = n.emotions.fear / 100.0;
    let zeal = n.emotions.zeal / 100.0;
    let votes_here = match (e.poll, faction) {
        (Poll::FreeState, Faction::ProSlavery) => false,
        (Poll::Territorial, Faction::FreeState) if e.boycott => t.loyalty < 0.25,
        _ => true,
    };
    if !votes_here {
        return Ballot::Stayed;
    }
    // The store buys votes from men in debt to it.
    let hh = &world.families[n.family as usize].stores;
    if e.poll == Poll::Territorial
        && faction == Faction::FreeState
        && hh.debt >= 10
        && hh.cash < 3
        && world
            .rng
            .chance(0.3 * (1.0 - t.loyalty) * (1.0 - t.honesty))
    {
        return Ballot::Sold;
    }
    let p = 0.35 + 0.35 * t.loyalty + 0.3 * zeal - 0.4 * fear;
    if world.rng.chance(p.clamp(0.05, 0.95)) {
        Ballot::Voted(faction)
    } else {
        Ballot::Stayed
    }
}

fn election(world: &mut World, i: usize) {
    let e = &ELECTIONS[i];
    let men: Vec<NpcId> = world
        .living()
        .map(|n| n.id)
        .filter(|&id| id != PLAYER && eligible(world, id))
        .collect();
    let (mut fs, mut ps) = (0u16, 0u16);
    let mut sold = Vec::new();
    for m in men {
        match ballot(world, m, e) {
            Ballot::Voted(Faction::FreeState) => fs += 1,
            Ballot::Voted(Faction::ProSlavery) => ps += 1,
            Ballot::Sold => {
                ps += 1;
                sold.push(m);
            }
            Ballot::Stayed => {}
        }
    }
    // The player's vote was cast (or sold) through `life`, earlier today.
    match world.life.ballot {
        Some((d, b)) if d.0 + 1 == world.day.0 => match b {
            Ballot::Voted(Faction::FreeState) => fs += 1,
            Ballot::Voted(Faction::ProSlavery) | Ballot::Sold => ps += 1,
            Ballot::Stayed => {}
        },
        _ => {}
    }
    // Missourians over the line for a territorial poll, a few hundred at
    // this precinct in a bad year.
    let missourians = if e.poll == Poll::Territorial {
        20 + world.rng.range(0, 60) as u16
    } else {
        0
    };
    // Free-State polls got pro-slavery rowdies; territorial polls got the
    // Missourians. Free-State voters paid in fear either way.
    let intimidated: Vec<NpcId> = world
        .living()
        .filter(|n| n.faction == Faction::FreeState && eligible(world, n.id))
        .map(|n| n.id)
        .collect();
    for id in intimidated {
        if world.rng.chance(0.2) {
            world.npc_mut(id).emotions.fear += 15.0;
        }
    }
    world.emit_root(
        EventKind::Election {
            index: i as u8,
            free_state: fs,
            pro_slavery: ps + missourians,
            missourians,
        },
        None,
    );
    for s in sold {
        if world.rng.chance(0.3) {
            world.emit_root(EventKind::VoteSold { seller: s }, None);
        }
    }
}

fn on_election(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::Election {
            index, missourians, ..
        } => {
            let e = &ELECTIONS[index as usize];
            match e.poll {
                Poll::FreeState => world.add_grievance(Faction::ProSlavery, 6),
                Poll::Territorial if e.fraud_rejected => {
                    world.add_grievance(Faction::ProSlavery, 10);
                    world.add_grievance(Faction::FreeState, -10);
                }
                Poll::Territorial => {
                    world.add_grievance(Faction::FreeState, 4 + (missourians / 10) as i32);
                }
            }
        }
        EventKind::VoteSold { seller } => {
            // Found out: a turncoat to his own.
            let faction = world.npc(seller).faction;
            let own: Vec<NpcId> = world
                .living()
                .filter(|n| n.faction == faction && n.id != seller)
                .map(|n| n.id)
                .collect();
            for o in own {
                world.adjust_opinion(o, seller, -15);
            }
        }
        _ => {}
    }
}

fn call_muster(world: &mut World, i: usize) {
    let by = Day(world.day.0 + ANSWER_DAYS);
    world.law.muster = Some((i, by, Vec::new()));
    world.law.player_answered = false;
    let c = &MUSTERS[i];
    let called: Vec<NpcId> = world
        .living()
        .map(|n| n.id)
        .filter(|&id| id != PLAYER && eligible(world, id))
        .filter(|&id| match world.npc(id).faction {
            Faction::FreeState => c.free_state,
            Faction::ProSlavery => c.pro_slavery,
        })
        .collect();
    for id in called {
        let n = world.npc(id);
        let t = n.temperament;
        let p = 0.15 + 0.35 * t.loyalty + 0.25 * t.courage + n.emotions.zeal / 250.0
            - n.emotions.fear / 300.0;
        if world.rng.chance(p.clamp(0.05, 0.9)) {
            join(world, id);
        }
    }
}

/// Enlist for this call.
pub fn join(world: &mut World, id: NpcId) {
    let Some((_, _, joined)) = world.law.muster.as_mut() else {
        return;
    };
    if joined.contains(&id) {
        return;
    }
    joined.push(id);
    let n = world.npc_mut(id);
    n.violence = n.violence.saturating_add(1);
    n.emotions.zeal += 10.0;
}

fn close_muster(world: &mut World, i: usize) {
    let Some((_, _, joined)) = world.law.muster.take() else {
        return;
    };
    let c = &MUSTERS[i];
    // The captain fills out the roll with the year's dead of his side: a
    // name is a man's pay and rations, and a company that looks stronger.
    let side_called = |f: Faction| match f {
        Faction::FreeState => c.free_state,
        Faction::ProSlavery => c.pro_slavery,
    };
    let since = world.day.0.saturating_sub(365);
    let lately_dead: Vec<(NpcId, super::events::EventId)> = world
        .events
        .iter()
        .rev()
        .take_while(|e| e.day.0 >= since)
        .filter_map(|e| match e.kind {
            EventKind::Death { victim, .. } | EventKind::Perished { victim, .. } => {
                Some((victim, e.id))
            }
            _ => None,
        })
        .filter(|&(v, _)| {
            v != PLAYER
                && side_called(world.npc(v).faction)
                && !is_woman(&world.npc(v).name)
                && world.npc(v).age >= 18
                && !world.law.padded.contains(&v)
        })
        .collect();
    let mut padded = std::mem::take(&mut world.law.padded).len() as u8;
    for (v, died) in lately_dead {
        if world.rng.chance(0.3) {
            // The roll stands on the grave.
            world.emit_root(
                EventKind::RollPadded {
                    name: v,
                    index: i as u8,
                },
                Some(died),
            );
            padded += 1;
        }
    }
    let called = |world: &World, id: NpcId| match world.npc(id).faction {
        Faction::FreeState => c.free_state,
        Faction::ProSlavery => c.pro_slavery,
    };
    let dodgers: Vec<NpcId> = world
        .living()
        .map(|n| n.id)
        .filter(|&id| eligible(world, id) && called(world, id) && !joined.contains(&id))
        .collect();
    // Those who went think less of those who stayed home.
    for &d in &dodgers {
        let faction = world.npc(d).faction;
        let comrades: Vec<NpcId> = joined
            .iter()
            .copied()
            .filter(|&j| world.npc(j).faction == faction)
            .collect();
        for j in comrades {
            world.adjust_opinion(j, d, -8);
        }
    }
    // The other side marks who rode against them.
    for &j in &joined {
        let faction = world.npc(j).faction;
        let others: Vec<NpcId> = world
            .living()
            .filter(|n| n.faction != faction)
            .map(|n| n.id)
            .collect();
        for o in others {
            world.adjust_opinion(o, j, -4);
        }
    }
    // A fight: men from this county shot at each other.
    let mut shot: Vec<(NpcId, NpcId)> = Vec::new();
    if c.fight {
        for &j in &joined {
            let enemies: Vec<NpcId> = joined
                .iter()
                .copied()
                .filter(|&e| world.npc(e).faction != world.npc(j).faction)
                .collect();
            if !enemies.is_empty() && world.rng.chance(0.08) {
                let by = enemies[world.rng.range(0, enemies.len() as u32) as usize];
                shot.push((j, by));
            }
        }
    }
    let muster = world.emit_root(
        EventKind::Muster {
            index: i as u8,
            joined: joined.len() as u8,
            dodged: dodgers.len() as u8,
            wounded: shot.len() as u8,
            you: joined.contains(&PLAYER),
            padded,
        },
        None,
    );
    // Men shot at the muster: the wound stands on the muster (`law` ->
    // the county's violence, its blame, its oaths).
    let parent = world.events[muster as usize].clone();
    for (victim, attacker) in shot {
        world.emit_child(&parent, EventKind::Wounded { victim, attacker });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_free_state_plaintiff_faces_long_odds() {
        let w = World::new(2);
        let fs = w
            .families
            .iter()
            .find(|f| f.faction == Faction::FreeState && f.id != 0)
            .and_then(|f| w.head_of(f.id))
            .unwrap();
        let ps = w
            .families
            .iter()
            .find(|f| f.faction == Faction::ProSlavery && f.farms())
            .and_then(|f| w.head_of(f.id))
            .unwrap();
        assert!(odds(&w, fs, ps, 0.0) < odds(&w, ps, fs, 0.0) - 0.4);
        assert!(odds(&w, fs, ps, 1.0) > odds(&w, fs, ps, 0.0));
    }

    #[test]
    fn a_jumped_claim_becomes_a_case() {
        let mut w = World::new(4);
        let jumper = w.head_of(2).unwrap();
        let victim = w.head_of(1).unwrap();
        w.emit_root(
            EventKind::ClaimJumped {
                family: 1,
                lost: 2,
                neighbor: Some(jumper),
            },
            None,
        );
        w.run_cascades();
        w.families[1].stores.cash = 10;
        assert!(sue(&mut w, victim, jumper, 0.5));
        w.run_cascades();
        assert!(w.law.disputes.is_empty());
        assert!(
            w.events
                .iter()
                .any(|e| matches!(e.kind, EventKind::Lawsuit { .. }))
        );
    }

    #[test]
    fn the_territorial_election_brings_missourians() {
        let mut w = World::new(6);
        w.run_days(342); // the count comes the morning after 6 Oct 1856
        let ev = w
            .events
            .iter()
            .find_map(|e| match e.kind {
                EventKind::Election {
                    index: 2,
                    missourians,
                    ..
                } => Some(missourians),
                _ => None,
            })
            .expect("the October 1856 election happened");
        assert!(ev >= 20);
    }

    #[test]
    fn shirkers_are_noticed() {
        let mut w = World::new(7);
        w.run_days(27); // the Wakarusa muster, 28 Nov 1855
        w.run_days(ANSWER_DAYS + 1);
        let m = w
            .events
            .iter()
            .find_map(|e| match e.kind {
                EventKind::Muster {
                    index: 0,
                    joined,
                    dodged,
                    ..
                } => Some((joined, dodged)),
                _ => None,
            })
            .expect("muster closed");
        assert!(m.0 + m.1 > 0);
    }

    #[test]
    fn women_neither_vote_nor_muster() {
        let w = World::new(1);
        for n in w.living() {
            if n.id != PLAYER && is_woman(&n.name) {
                assert!(!eligible(&w, n.id));
            }
        }
    }
}
