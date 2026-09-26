//! Bees, singing school, and the meeting that split. Frontier work was social:
//! a husking bee shucked a neighbor's crop in a night (find a red ear, claim
//! a kiss), a quilting bee put covers on a family's beds before winter, a
//! spelling bee crowned the county's best speller. They mixed people who
//! otherwise kept to their own side.
//!
//! The union meeting — Methodists, mostly, before the county had a church
//! for each side — held both sides on Sunday until it couldn't. When it
//! split (North and South, as the Methodists had nationally in 1844), the
//! one room where both factions sat together was gone.

use super::events::{EventKind, WorldEvent};
use super::psyche::LifeStage;
use super::romance;
use super::world::{FamilyId, NpcId, PLAYER, World, distance, is_woman};

/// Tension (both grievances summed) at which the meeting splits.
pub const SPLIT_AT: i32 = 150;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bee {
    /// October–November: the host's standing corn gets shucked.
    Husking,
    /// Winter: quilts on the host's beds against the cold.
    Quilting,
    /// Winter: the best speller in the county.
    Spelling,
    /// Winter weeknights: shape-note singing, and courting.
    SingingSchool,
}

impl Bee {
    pub fn label(self) -> &'static str {
        match self {
            Bee::Husking => "husking bee",
            Bee::Quilting => "quilting bee",
            Bee::Spelling => "spelling bee",
            Bee::SingingSchool => "singing school",
        }
    }

    /// What's on this week, if anything.
    pub fn season(month: u32, weekday: u32) -> Option<Bee> {
        match (month, weekday) {
            (10 | 11, 1) => Some(Bee::Husking),
            (1..=3, 1) => Some(Bee::Quilting),
            (12 | 1 | 2, 5) => Some(Bee::Spelling),
            (11 | 12 | 1 | 2 | 3, 6) => Some(Bee::SingingSchool),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// Methodist Episcopal: antislavery.
    North,
    /// Methodist Episcopal, South.
    South,
}

#[derive(Clone, Debug, Default)]
pub struct Gatherings {
    /// Today's bee: (kind, host family, who came).
    pub today: Option<(Bee, FamilyId, Vec<NpcId>)>,
    /// None while the union meeting holds.
    pub split: Option<super::calendar::Day>,
    /// Who went where after the split.
    pub sides: Vec<(NpcId, Side)>,
    /// Quilting bees each family has hosted this winter.
    pub quilts: Vec<(FamilyId, u8)>,
}

impl Gatherings {
    pub fn side_of(&self, id: NpcId) -> Option<Side> {
        self.sides.iter().find(|s| s.0 == id).map(|s| s.1)
    }

    pub fn quilts(&self, family: FamilyId) -> u8 {
        self.quilts
            .iter()
            .find(|q| q.0 == family)
            .map_or(0, |q| q.1)
    }
}

/// Where your heart is on slavery decides which meeting you go to.
fn leaning(world: &World, id: NpcId) -> Side {
    if world.npc(id).ideology.private >= 0.0 {
        Side::North
    } else {
        Side::South
    }
}

pub fn daily(world: &mut World) {
    world.gatherings.today = None;
    let weekday = world.day.0 % 7;
    let month = world.day.month();
    if let Some(bee) = Bee::season(month, weekday) {
        hold(world, bee);
    }
    if weekday == 3 {
        sunday_meeting(world);
    }
    if world.gatherings.split.is_none() && world.grievance[0] + world.grievance[1] >= SPLIT_AT {
        world.emit_root(EventKind::ChurchSplit, None);
    }
    if month == 4 {
        world.gatherings.quilts.clear();
    }
}

fn hold(world: &mut World, bee: Bee) {
    // A host: a sociable household that farms.
    let hosts: Vec<FamilyId> = world
        .families
        .iter()
        .filter(|f| f.farms() && f.id != 0)
        .filter(|f| {
            world
                .head_of(f.id)
                .is_some_and(|h| world.npc(h).temperament.sociability > 0.4)
        })
        .map(|f| f.id)
        .collect();
    if hosts.is_empty() || !world.rng.chance(0.6) {
        return;
    }
    let host = hosts[world.rng.range(0, hosts.len() as u32) as usize];
    let home = world.families[host as usize].farm;
    let people: Vec<NpcId> = world
        .living()
        .filter(|n| n.id != PLAYER && LifeStage::of(n.age) != LifeStage::Child)
        .filter(|n| distance(world.farm_of(n.id), home) <= 14.0)
        .map(|n| n.id)
        .collect();
    let mut came = Vec::new();
    for id in people {
        let n = world.npc(id);
        let fam = n.family;
        if fam != host && world.feud_between(fam, host) {
            continue;
        }
        let other_side = n.faction != world.families[host as usize].faction;
        let mut p = 0.2 + 0.5 * n.temperament.sociability - if other_side { 0.15 } else { 0.0 };
        if bee == Bee::Quilting && !is_woman(&n.name) {
            p *= 0.3;
        }
        if world.rng.chance(p.max(0.02)) {
            came.push(id);
        }
    }
    if came.len() < 3 {
        return;
    }
    mingle(world, &came);
    match bee {
        Bee::Husking => {
            let w = &mut world.families[host as usize].stores.work;
            let shucked = (came.len() as f32 * 12.0).min(w.standing);
            w.standing -= shucked;
            world.families[host as usize].stores.food += shucked;
            red_ear(world, &came);
        }
        Bee::Quilting => {
            let q = &mut world.gatherings.quilts;
            match q.iter_mut().find(|q| q.0 == host) {
                Some(e) => e.1 = e.1.saturating_add(1),
                None => q.push((host, 1)),
            }
        }
        Bee::Spelling => {
            // The best speller, with a little luck.
            let mut best = (came[0], f32::MIN);
            for &c in &came {
                let roll = world.rng.unit();
                let n = world.npc(c);
                let s = n.temperament.literacy + 0.3 * roll * n.hidden.luck;
                if s > best.1 {
                    best = (c, s);
                }
            }
            for &c in &came {
                if c != best.0 {
                    world.adjust_opinion(c, best.0, 4);
                }
            }
        }
        Bee::SingingSchool => {
            for &c in &came {
                let t = &mut world.npc_mut(c).temperament;
                t.piety = (t.piety + 0.005).min(1.0);
            }
            red_ear(world, &came);
        }
    }
    world.emit_root(
        EventKind::BeeHeld {
            bee,
            host,
            crowd: came.len() as u8,
        },
        None,
    );
    world.gatherings.today = Some((bee, host, came));
}

/// Warm words across the room, more across the line.
fn mingle(world: &mut World, came: &[NpcId]) {
    for _ in 0..came.len() {
        let a = came[world.rng.range(0, came.len() as u32) as usize];
        let b = came[world.rng.range(0, came.len() as u32) as usize];
        if a == b || world.npc(a).family == world.npc(b).family {
            continue;
        }
        let warm = if world.npc(a).faction != world.npc(b).faction {
            4
        } else {
            2
        };
        world.adjust_opinion(a, b, warm);
        world.adjust_opinion(b, a, warm);
    }
}

/// The red ear, or the walk home after singing: somebody courts somebody.
fn red_ear(world: &mut World, came: &[NpcId]) {
    let finder = came[world.rng.range(0, came.len() as u32) as usize];
    let fancy = came
        .iter()
        .copied()
        .filter(|&o| o != finder && world.npc(o).family != world.npc(finder).family)
        .max_by_key(|&o| world.opinion(finder, o));
    if let Some(o) = fancy {
        romance::court(world, finder, o, 0.0);
    }
}

/// Sunday. Before the split, the union meeting mixes the pious of both
/// sides; after, each side's meeting hardens it.
fn sunday_meeting(world: &mut World) {
    let pious: Vec<NpcId> = world
        .living()
        .filter(|n| n.id != PLAYER && n.temperament.piety > 0.45)
        .filter(|n| LifeStage::of(n.age) != LifeStage::Child)
        .map(|n| n.id)
        .collect();
    if pious.len() < 2 {
        return;
    }
    let a = pious[world.rng.range(0, pious.len() as u32) as usize];
    let b = pious[world.rng.range(0, pious.len() as u32) as usize];
    if a == b || world.npc(a).family == world.npc(b).family {
        return;
    }
    let delta = match world.gatherings.split {
        None => 2,
        Some(_) => match (world.gatherings.side_of(a), world.gatherings.side_of(b)) {
            (Some(x), Some(y)) if x == y => 2,
            (Some(_), Some(_)) => -2,
            _ => 0,
        },
    };
    world.adjust_opinion(a, b, delta);
    world.adjust_opinion(b, a, delta);
}

/// The player's evening at today's bee.
pub fn attend(world: &mut World) -> Option<Bee> {
    let (bee, _, came) = world.gatherings.today.clone()?;
    for &c in &came {
        world.adjust_opinion(c, PLAYER, 3);
        world.adjust_opinion(PLAYER, c, 2);
    }
    if matches!(bee, Bee::Husking | Bee::SingingSchool) {
        // Somebody finds a red ear and looks your way.
        if let Some(&o) = came.iter().max_by_key(|&&o| world.opinion(o, PLAYER)) {
            world.hearts.add_affection(o, PLAYER, 5.0);
        }
    }
    Some(bee)
}

/// Pick your meeting after the split.
pub fn choose(world: &mut World, side: Side) -> bool {
    if world.gatherings.split.is_none() || world.gatherings.side_of(PLAYER).is_some() {
        return false;
    }
    world.gatherings.sides.push((PLAYER, side));
    let members: Vec<(NpcId, Side)> = world.gatherings.sides.clone();
    for (m, s) in members {
        if m != PLAYER {
            world.adjust_opinion(m, PLAYER, if s == side { 6 } else { -8 });
        }
    }
    true
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    if let EventKind::ChurchSplit = ev.kind {
        if world.gatherings.split.is_some() {
            return;
        }
        world.gatherings.split = Some(ev.day);
        let members: Vec<NpcId> = world
            .living()
            .filter(|n| n.id != PLAYER && n.temperament.piety > 0.35)
            .map(|n| n.id)
            .collect();
        let sides: Vec<(NpcId, Side)> = members.iter().map(|&m| (m, leaning(world, m))).collect();
        // Old pew-mates on the other side: a bitter parting.
        for &(a, sa) in &sides {
            for &(b, sb) in &sides {
                if a != b && sa != sb {
                    world.adjust_opinion(a, b, -6);
                }
            }
        }
        world.gatherings.sides = sides;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bees_keep_their_seasons() {
        assert_eq!(Bee::season(10, 1), Some(Bee::Husking));
        assert_eq!(Bee::season(7, 1), None);
        assert_eq!(Bee::season(1, 5), Some(Bee::Spelling));
    }

    #[test]
    fn a_husking_bee_shucks_the_hosts_corn() {
        let mut w = World::new(3);
        for f in &mut w.families {
            f.stores.work.standing = 400.0;
        }
        for _ in 0..40 {
            hold(&mut w, Bee::Husking);
            if w.gatherings.today.is_some() {
                break;
            }
        }
        let (_, host, came) = w.gatherings.today.clone().expect("a bee was held");
        assert!(
            w.families[host as usize].stores.work.standing
                <= 400.0 - came.len() as f32 * 12.0 + 0.1
        );
    }

    #[test]
    fn the_split_sours_old_pew_mates() {
        let mut w = World::new(5);
        for n in w.npcs.iter_mut() {
            n.temperament.piety = 0.8;
        }
        let north = w
            .living()
            .find(|n| n.ideology.private > 0.0 && n.id != PLAYER)
            .unwrap()
            .id;
        let south = w.living().find(|n| n.ideology.private < 0.0).unwrap().id;
        let before = w.opinion(north, south);
        w.emit_root(EventKind::ChurchSplit, None);
        w.run_cascades();
        assert!(w.gatherings.split.is_some());
        assert!(w.opinion(north, south) < before);
        assert!(choose(&mut w, Side::North));
        assert!(!choose(&mut w, Side::South), "one meeting per soul");
    }
}
