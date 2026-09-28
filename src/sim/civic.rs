//! Community and bureaucracy. Settlers built schoolhouses and bridges with
//! their own hands, and fought over claims at a land office run by the other
//! side. Both knit and tear: working a beam beside a man softens a grudge; a
//! neighbor who jumps your claim starts one (the Dow killing that set off the
//! Wakarusa War in November 1855 began as a claim dispute).

use super::calendar::Day;
use super::events::{EventKind, WorldEvent};
use super::psyche::LifeStage;
use super::world::{Faction, FamilyId, NpcId, PLAYER, World, distance};

/// Man-days to finish a project.
pub const WORK: f32 = 150.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Project {
    /// Children learn letters faster; the player studies faster.
    Schoolhouse,
    /// Over the Wakarusa: freight moves easier (market).
    Bridge,
    /// Debates on winter nights: opposite factions meet; speeches carry.
    Lyceum,
}

impl Project {
    pub const ALL: [Project; 3] = [Project::Schoolhouse, Project::Bridge, Project::Lyceum];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            Project::Schoolhouse => "schoolhouse",
            Project::Bridge => "Wakarusa bridge",
            Project::Lyceum => "lyceum hall",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holiday {
    NewYear,
    Independence,
    HarvestHome,
    Christmas,
}

impl Holiday {
    pub fn on(month: u32, day: u32) -> Option<Holiday> {
        match (month, day) {
            (1, 1) => Some(Holiday::NewYear),
            (7, 4) => Some(Holiday::Independence),
            (10, 15) => Some(Holiday::HarvestHome),
            (12, 25) => Some(Holiday::Christmas),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Holiday::NewYear => "New Year's calls",
            Holiday::Independence => "the Fourth of July",
            Holiday::HarvestHome => "harvest home",
            Holiday::Christmas => "Christmas",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Civic {
    pub progress: [f32; 3],
    pub built: [Option<Day>; 3],
    /// Preemption filed at the Lecompton land office, by family.
    pub filed: Vec<bool>,
    /// Days of work each person has given the county.
    pub volunteered: Vec<(NpcId, u16)>,
}

impl Civic {
    pub fn founding(world: &mut World) -> Self {
        let filed = (0..world.families.len())
            .map(|f| {
                let lettered = world
                    .head_of(f as FamilyId)
                    .map_or(0.0, |h| world.npc(h).temperament.literacy);
                f != 0 && world.rng.chance(0.2 + 0.3 * lettered)
            })
            .collect();
        Civic {
            filed,
            ..Default::default()
        }
    }

    pub fn built(&self, p: Project) -> bool {
        self.built[p.index()].is_some()
    }
}

/// Someone gives a day to a project.
pub fn work(world: &mut World, who: NpcId, p: Project, amount: f32) {
    if world.civic.built(p) {
        return;
    }
    let i = p.index();
    world.civic.progress[i] += amount;
    match world.civic.volunteered.iter_mut().find(|v| v.0 == who) {
        Some(v) => v.1 += 1,
        None => world.civic.volunteered.push((who, 1)),
    }
    if world.civic.progress[i] >= WORK {
        world.civic.built[i] = Some(world.day);
        world.emit_root(EventKind::Built { project: p }, None);
    }
}

/// Ride to Lecompton and file. The register is a pro-slavery appointee; a
/// Free-State man's papers have a way of getting lost unless he knows the
/// forms cold. Returns whether the trip was made.
pub fn file_claim(world: &mut World, family: FamilyId, letters: f32) -> bool {
    if world
        .civic
        .filed
        .get(family as usize)
        .copied()
        .unwrap_or(true)
    {
        return false;
    }
    let hh = &mut world.families[family as usize].stores;
    if hh.cash < 2 {
        return false;
    }
    hh.cash -= 2;
    let free_state = world.families[family as usize].faction == Faction::FreeState;
    let lost = if free_state {
        0.4 - 0.3 * letters
    } else {
        0.05
    };
    let delayed = world.rng.chance(lost.max(0.02));
    world.emit_root(EventKind::ClaimFiled { family, delayed }, None);
    true
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::ClaimFiled { family, delayed } if !delayed => {
            world.civic.filed[family as usize] = true;
        }
        EventKind::ClaimJumped {
            family,
            lost,
            neighbor,
        } => {
            let hh = &mut world.families[family as usize].stores;
            hh.acres = hh.acres.saturating_sub(lost as u32);
            let kin: Vec<NpcId> = world
                .living()
                .filter(|n| n.family == family)
                .map(|n| n.id)
                .collect();
            for &k in &kin {
                world.npc_mut(k).emotions.anger += 25.0;
            }
            if let Some(jumper) = neighbor {
                for k in kin {
                    world.emit_child(
                        ev,
                        EventKind::OpinionChange {
                            holder: k,
                            target: jumper,
                            delta: -35,
                            after: 0,
                        },
                    );
                }
            }
        }
        _ => {}
    }
}

pub fn daily(world: &mut World) {
    let (_, month, day) = world.day.date();
    if let Some(h) = Holiday::on(month, day) {
        festival(world, h);
    }
    // Saturdays: the generous give a day to the county.
    if world.day.0 % 7 == 2 {
        volunteers(world);
    }
}

fn festival(world: &mut World, h: Holiday) {
    let people: Vec<NpcId> = world
        .living()
        .filter(|n| n.id != PLAYER)
        .map(|n| n.id)
        .collect();
    let mut crowd = Vec::new();
    for id in people {
        let p = 0.3 + 0.5 * world.npc(id).temperament.sociability;
        if world.rng.chance(p) {
            crowd.push(id);
        }
    }
    if super::family::away(world).is_none() && world.player_alive() {
        crowd.push(PLAYER);
        world.life.spirits = (world.life.spirits + 12.0).min(100.0);
    }
    for &c in &crowd {
        let e = &mut world.npc_mut(c).emotions;
        e.fear *= 0.8;
        e.grief *= 0.9;
    }
    // A few conversations across the fence.
    for _ in 0..crowd.len() / 2 {
        if crowd.len() < 2 {
            break;
        }
        let a = crowd[world.rng.range(0, crowd.len() as u32) as usize];
        let b = crowd[world.rng.range(0, crowd.len() as u32) as usize];
        if a != b {
            world.adjust_opinion(a, b, 2);
            world.adjust_opinion(b, a, 2);
        }
    }
    world.emit_root(
        EventKind::Festival {
            holiday: h,
            crowd: crowd.len() as u8,
        },
        None,
    );
}

fn volunteers(world: &mut World) {
    let Some(project) = Project::ALL.into_iter().find(|&p| !world.civic.built(p)) else {
        return;
    };
    let able: Vec<NpcId> = world
        .living()
        .filter(|n| {
            n.id != PLAYER
                && LifeStage::of(n.age) == LifeStage::Adult
                && !n.wounded
                && n.temperament.generosity > 0.5
        })
        .map(|n| n.id)
        .collect();
    let mut crew = Vec::new();
    for id in able {
        let g = world.npc(id).temperament.generosity;
        if world.rng.chance(0.35 * g) {
            crew.push(id);
        }
    }
    for &c in &crew {
        work(world, c, project, 1.0);
    }
    // Working beside a man softens a grudge, more so across a feud.
    for (i, &a) in crew.iter().enumerate() {
        for &b in &crew[i + 1..] {
            let (fa, fb) = (world.npc(a).family, world.npc(b).family);
            if fa == fb {
                continue;
            }
            let warm = if world.feud_between(fa, fb) { 3 } else { 1 };
            world.adjust_opinion(a, b, warm);
            world.adjust_opinion(b, a, warm);
        }
    }
}

pub fn monthly(world: &mut World) {
    // Children in reach of a schoolhouse learn their letters.
    if world.civic.built(Project::Schoolhouse) {
        let kids: Vec<NpcId> = world
            .living()
            .filter(|n| LifeStage::of(n.age) == LifeStage::Child && n.age >= 5)
            .map(|n| n.id)
            .collect();
        for k in kids {
            let t = &mut world.npc_mut(k).temperament;
            t.literacy = (t.literacy + 0.03).min(1.0);
        }
    }

    // Lyceum debates: people from both sides in one room, arguing in turn.
    if world.civic.built(Project::Lyceum) {
        let free: Vec<NpcId> = debaters(world, Faction::FreeState);
        let slave: Vec<NpcId> = debaters(world, Faction::ProSlavery);
        for (&a, &b) in free.iter().zip(slave.iter()) {
            world.adjust_opinion(a, b, 4);
            world.adjust_opinion(b, a, 4);
        }
    }

    // Unfiled families file, if they can read the forms and spare the trip.
    for f in 1..world.families.len() {
        let fid = f as FamilyId;
        if world.civic.filed[f] || !world.families[f].farms() {
            continue;
        }
        let Some(head) = world.head_of(fid) else {
            continue;
        };
        let letters = world.npc(head).temperament.literacy;
        let prudent = world.families[f].stores.prudence;
        if world.rng.chance(0.05 + 0.1 * letters + 0.1 * prudent) {
            file_claim(world, fid, letters * 0.5);
        }
    }

    // Claim jumpers: emigrant season is worst. Filed claims hold at law.
    let spring = (3..=6).contains(&world.day.month());
    for f in 0..world.families.len() {
        let fid = f as FamilyId;
        if !world.families[f].farms() || world.head_of(fid).is_none() {
            continue;
        }
        let p = if world.civic.filed[f] { 0.004 } else { 0.02 } * if spring { 2.0 } else { 1.0 };
        if !world.rng.chance(p) {
            continue;
        }
        let head = world.head_of(fid).unwrap();
        // Half the time it's the man next door moving a stake.
        let origin = world.families[f].farm;
        let neighbor = if world.rng.chance(0.5) {
            world
                .families
                .iter()
                .filter(|o| o.id != fid && o.id != 0 && o.farms())
                .min_by(|a, b| distance(a.farm, origin).total_cmp(&distance(b.farm, origin)))
                .and_then(|o| world.head_of(o.id))
        } else {
            None
        };
        // A bold family runs a stranger off; a neighbor, you take to law.
        let bold = world.npc(head).temperament.courage > 0.6;
        let acres = world.families[f].stores.acres;
        let lost = if neighbor.is_none() && bold {
            0
        } else {
            (acres / 4).max(1) as u8
        };
        world.emit_root(
            EventKind::ClaimJumped {
                family: fid,
                lost,
                neighbor,
            },
            None,
        );
    }
}

fn debaters(world: &World, faction: Faction) -> Vec<NpcId> {
    world
        .living()
        .filter(|n| {
            n.id != PLAYER
                && n.faction == faction
                && LifeStage::of(n.age) != LifeStage::Child
                && n.temperament.sociability > 0.5
        })
        .map(|n| n.id)
        .take(3)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enough_hands_finish_a_project() {
        let mut w = World::new(3);
        for _ in 0..(WORK as usize) {
            work(&mut w, 1, Project::Bridge, 1.0);
        }
        w.run_cascades();
        assert!(w.civic.built(Project::Bridge));
        assert!(w.events.iter().any(|e| matches!(
            e.kind,
            EventKind::Built {
                project: Project::Bridge
            }
        )));
    }

    #[test]
    fn free_state_papers_get_lost_more_often() {
        let delays = |faction: Faction, letters: f32| {
            (0..200)
                .filter(|&s| {
                    let mut w = World::new(s);
                    w.families[0].faction = faction;
                    w.families[0].stores.cash = 10;
                    file_claim(&mut w, 0, letters);
                    w.run_cascades();
                    !w.civic.filed[0]
                })
                .count()
        };
        let lost_free = delays(Faction::FreeState, 0.0);
        let lost_slave = delays(Faction::ProSlavery, 0.0);
        let lost_lettered = delays(Faction::FreeState, 1.0);
        assert!(lost_free > lost_slave * 3, "{lost_free} vs {lost_slave}");
        assert!(lost_lettered < lost_free);
    }

    #[test]
    fn a_neighbor_who_jumps_your_claim_is_hated() {
        let mut w = World::new(5);
        let jumper = w.head_of(2).unwrap();
        let victim = w.head_of(1).unwrap();
        let acres = w.families[1].stores.acres;
        let before = w.opinion(victim, jumper);
        w.emit_root(
            EventKind::ClaimJumped {
                family: 1,
                lost: 3,
                neighbor: Some(jumper),
            },
            None,
        );
        w.run_cascades();
        assert_eq!(w.families[1].stores.acres, acres - 3);
        assert!(w.opinion(victim, jumper) <= before - 30);
    }

    #[test]
    fn holidays_fall_on_their_days() {
        assert_eq!(Holiday::on(7, 4), Some(Holiday::Independence));
        assert_eq!(Holiday::on(7, 5), None);
    }
}
