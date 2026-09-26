//! The better way (§13). Feuds are loud; peace is usually quiet and sideways:
//! a man who lowers the rifle, a rival who shows up to raise your barn, an
//! enemy at a child's grave, two families who notice they hate the same third
//! one, or plain exhaustion. None of it is guaranteed and all of it is
//! reversible — the next fire starts it over.

use super::calendar::Day;
use super::character::{self, Archetype};
use super::events::{EventKind, Retaliation, WorldEvent};
use super::psyche::LifeStage;
use super::world::{FamilyId, NpcId, PLAYER, World, distance};

/// Opinion at or below which hatred is hatred (matches systems::HATRED).
const HATRED: i16 = -50;
/// Days a feud must go without blood before it can burn out.
pub const QUIET_DAYS: u32 = 180;
/// Days after a fire before the neighbors come with lumber.
pub const RAISING_AFTER: u32 = 10;
/// Tiles (half-miles) a neighbor will travel to lift a beam: five miles.
const RAISING_RANGE: f32 = 10.0;
/// Days a truce holds against hot words; new blood breaks it at once.
pub const TRUCE_DAYS: u32 = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mercy {
    /// Children at the window.
    Children,
    /// Thou shalt not.
    Faith,
    /// Tired of it.
    Weariness,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Peace {
    /// Nobody has bled in half a year; nobody remembers why it mattered.
    Exhaustion,
    /// They found they hate the same people more.
    CommonEnemy,
    /// A grave, or a barn, did it.
    Calamity,
    /// Someone talked them down.
    Brokered,
    /// A wedding made them kin.
    Marriage,
}

/// At the moment of the act: does the plotter go through with it?
/// Humility, faith and generosity argue against; malice and zeal for.
pub fn mercy(world: &mut World, actor: NpcId, target: NpcId, method: Retaliation) -> Option<Mercy> {
    let a = world.npc(actor);
    if character::is(a, Archetype::Bushwhacker) || actor == PLAYER {
        return None;
    }
    let t = &a.temperament;
    let children = world
        .living()
        .any(|n| n.family == world.npc(target).family && LifeStage::of(n.age) == LifeStage::Child);
    let mut p = 0.22 * t.humility + 0.12 * t.piety + 0.08 * t.generosity;
    if children && method == Retaliation::Ambush {
        // You see them through the window at supper.
        p += 0.12;
    }
    if a.violence == 0 {
        p += 0.08; // the first time is the hardest
    }
    if super::ghosts::ashamed(world, actor) {
        p += 0.1;
    }
    p -= 0.5 * a.hidden.malice + a.emotions.zeal / 400.0;
    let p = p.clamp(0.0, 0.6);
    if !world.rng.chance(p) {
        return None;
    }
    let t = &world.npc(actor).temperament;
    Some(if children && method == Retaliation::Ambush {
        Mercy::Children
    } else if t.piety > t.humility {
        Mercy::Faith
    } else {
        Mercy::Weariness
    })
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::Spared { actor, target, .. } => {
            world.adjust_opinion(actor, target, 25);
            let day = ev.day;
            let a = world.npc_mut(actor);
            a.emotions.anger *= 0.5;
            a.last_revenge = Some(day);
            // Maybe the target saw him out there, and saw him leave.
            let noticed = 0.6 * world.npc(target).body.alertness;
            if world.rng.chance(noticed) {
                world.adjust_opinion(target, actor, 30);
                world.npc_mut(target).emotions.fear += 10.0;
            }
        }
        EventKind::Fire { owner, .. } => plan_raising(world, ev, owner),
        EventKind::BarnRaising { owner, rival, .. } => raise(world, ev, owner, rival),
        EventKind::Perished { victim, .. }
            if LifeStage::of(world.npc(victim).age) == LifeStage::Child =>
        {
            condolences(world, ev, victim)
        }
        EventKind::Condolence { visitor, mourner } => {
            let (vf, mf) = (world.npc(visitor).family, world.npc(mourner).family);
            for k in kin(world, mf) {
                world.adjust_opinion(k, visitor, 30);
            }
            world.adjust_opinion(visitor, mourner, 20);
            try_end(world, Some(ev), vf, mf, Peace::Calamity);
        }
        EventKind::Apology { actor, to } => {
            // The truth costs before forgiveness pays: net depends on faith.
            let family = world.npc(to).family;
            for k in kin(world, family) {
                let delta = (45.0 * world.npc(k).temperament.piety) as i16 - 20;
                world.adjust_opinion(k, actor, delta);
            }
        }
        EventKind::FeudEnded { a, b, .. } => {
            world.feuds.remove(&(a.min(b), a.max(b)));
            let until = Day(ev.day.0 + TRUCE_DAYS);
            world.truces.insert((a.min(b), a.max(b)), until);
        }
        EventKind::Retaliation { actor, target, .. } => {
            let (a, b) = (world.npc(actor).family, world.npc(target).family);
            world.truces.remove(&(a.min(b), a.max(b)));
        }
        _ => {}
    }
}

fn kin(world: &World, family: FamilyId) -> Vec<NpcId> {
    world
        .living()
        .filter(|n| n.family == family)
        .map(|n| n.id)
        .collect()
}

/// Neighbors in range who'll come, and maybe one who hates you.
fn plan_raising(world: &mut World, ev: &WorldEvent, owner: NpcId) {
    let family = world.npc(owner).family;
    let f = &world.families[family as usize];
    let already = world.is_scheduled(
        |k| matches!(*k, EventKind::BarnRaising { owner: o, .. } if world.npc(o).family == family),
    );
    if !f.farms() || already {
        return;
    }
    let origin = f.farm;
    let neighbors: Vec<(FamilyId, NpcId)> = world
        .families
        .iter()
        .filter(|n| n.id != family && n.farms() && distance(n.farm, origin) <= RAISING_RANGE)
        .filter_map(|n| world.head_of(n.id).map(|h| (n.id, h)))
        .collect();
    let (mut hands, mut rival) = (0u8, None);
    for (nf, head) in neighbors {
        let h = world.npc(head);
        let t = h.temperament;
        let feel = world.opinion(head, owner);
        let enemy = feel <= -30 || world.feud_between(nf, family);
        let p = if enemy {
            // The rare, loud kindness.
            0.25 * (t.generosity * t.humility).sqrt() - h.hidden.malice
        } else if feel >= -20 {
            0.2 + 0.5 * t.generosity + 0.2 * t.sociability
        } else {
            0.0
        };
        if world.rng.chance(p.max(0.0)) {
            hands += 1;
            if enemy && rival.is_none() {
                rival = Some(head);
            }
        }
    }
    if hands >= 2 || rival.is_some() {
        world.schedule(
            RAISING_AFTER,
            EventKind::BarnRaising {
                owner,
                hands,
                rival,
            },
            ev.id,
        );
    }
}

fn raise(world: &mut World, ev: &WorldEvent, owner: NpcId, rival: Option<NpcId>) {
    let family = world.npc(owner).family;
    let f = &mut world.families[family as usize];
    if f.barn_standing {
        return;
    }
    f.barn_standing = true;
    f.barn_burned_on = None;
    let household = kin(world, family);
    for &k in &household {
        world.npc_mut(k).emotions.fear *= 0.7;
    }
    if let Some(r) = rival {
        for &k in &household {
            world.adjust_opinion(k, r, 35);
        }
        // Swinging a hammer for a man is hard to square with hating him.
        world.adjust_opinion(r, owner, 30);
        let rf = world.npc(r).family;
        try_end(world, Some(ev), family, rf, Peace::Calamity);
    }
}

/// An enemy at a child's burying.
fn condolences(world: &mut World, ev: &WorldEvent, child: NpcId) {
    let family = world.npc(child).family;
    let Some(mourner) = world.head_of(family) else {
        return;
    };
    let enemies: Vec<FamilyId> = world
        .feuds
        .iter()
        .filter_map(|&(a, b)| match (a == family, b == family) {
            (true, _) => Some(b),
            (_, true) => Some(a),
            _ => None,
        })
        .collect();
    for e in enemies {
        let Some(visitor) = world.head_of(e) else {
            continue;
        };
        let t = world.npc(visitor).temperament;
        let p = 0.15 + 0.3 * t.piety + 0.25 * t.humility - world.npc(visitor).hidden.malice;
        if world.rng.chance(p.max(0.0)) {
            world.emit_child(ev, EventKind::Condolence { visitor, mourner });
        }
    }
}

/// End a feud if neither head still hates the other.
fn try_end(
    world: &mut World,
    ev: Option<&WorldEvent>,
    a: FamilyId,
    b: FamilyId,
    how: Peace,
) -> bool {
    if a == b || !world.feud_between(a, b) {
        return false;
    }
    let (Some(ha), Some(hb)) = (world.head_of(a), world.head_of(b)) else {
        return false;
    };
    if world.opinion(ha, hb) > HATRED && world.opinion(hb, ha) > HATRED {
        let kind = EventKind::FeudEnded { a, b, how };
        match ev {
            Some(ev) => {
                world.emit_child(ev, kind);
            }
            None => {
                world.emit_root(kind, None);
            }
        }
        return true;
    }
    false
}

/// Who drew whose blood, family to family, since `since`.
fn blood_since(world: &World, since: Day) -> Vec<(FamilyId, FamilyId, NpcId, NpcId)> {
    world
        .events
        .iter()
        .rev()
        .take_while(|e| e.day >= since)
        .filter_map(|e| match e.kind {
            EventKind::Retaliation { actor, target, .. } => Some((
                world.npc(actor).family,
                world.npc(target).family,
                actor,
                target,
            )),
            _ => None,
        })
        .collect()
}

pub fn monthly(world: &mut World) {
    let today = world.day.0;

    // The humble own up to what they did lately.
    let recent = blood_since(world, Day(today.saturating_sub(60)));
    let mut owned: Vec<NpcId> = Vec::new();
    for &(_, tf, actor, target) in &recent {
        let a = world.npc(actor);
        if !a.alive || owned.contains(&actor) || actor == PLAYER {
            continue;
        }
        let p = 0.2 * a.temperament.humility * (1.0 - 0.5 * a.temperament.temper);
        if world.rng.chance(p) {
            owned.push(actor);
            let to = world.head_of(tf).unwrap_or(target);
            world.emit_root(EventKind::Apology { actor, to }, None);
        }
    }

    // Feuds nobody feeds starve.
    if today >= QUIET_DAYS {
        let bled = blood_since(world, Day(today - QUIET_DAYS));
        let feuds: Vec<_> = world.feuds.iter().copied().collect();
        for (a, b) in feuds {
            let quiet = !bled
                .iter()
                .any(|&(x, y, ..)| (x == a && y == b) || (x == b && y == a));
            if quiet {
                try_end(world, None, a, b, Peace::Exhaustion);
            }
        }
    }

    // The enemy of my enemy.
    let feuds: Vec<_> = world.feuds.iter().copied().collect();
    let enemies = |f: FamilyId| -> Vec<FamilyId> {
        feuds
            .iter()
            .filter_map(|&(a, b)| {
                if a == f {
                    Some(b)
                } else if b == f {
                    Some(a)
                } else {
                    None
                }
            })
            .collect()
    };
    let n = world.families.len() as FamilyId;
    for x in 1..n {
        let ex = enemies(x);
        if ex.is_empty() {
            continue;
        }
        for y in (x + 1)..n {
            if !enemies(y).iter().any(|z| ex.contains(z)) {
                continue;
            }
            let (Some(hx), Some(hy)) = (world.head_of(x), world.head_of(y)) else {
                continue;
            };
            let warm = if world.feud_between(x, y) { 15 } else { 8 };
            for (holder, target) in [(hx, hy), (hy, hx)] {
                world.emit_root(
                    EventKind::OpinionChange {
                        holder,
                        target,
                        delta: warm,
                        after: 0,
                    },
                    None,
                );
            }
            if world.feud_between(x, y) {
                // Opinions apply as those events run; judge on the near future.
                let (o1, o2) = (world.opinion(hx, hy) + warm, world.opinion(hy, hx) + warm);
                if o1 > HATRED && o2 > HATRED {
                    world.emit_root(
                        EventKind::FeudEnded {
                            a: x,
                            b: y,
                            how: Peace::CommonEnemy,
                        },
                        None,
                    );
                }
            }
        }
    }
}

/// The player walks between two families and tries to talk them down.
/// Works on reputation and a silver tongue; fails loud.
pub fn broker(world: &mut World, a: FamilyId, b: FamilyId) -> bool {
    if !world.feud_between(a, b) {
        return false;
    }
    let (Some(ha), Some(hb)) = (world.head_of(a), world.head_of(b)) else {
        return false;
    };
    let trust = (world.opinion(ha, PLAYER) + world.opinion(hb, PLAYER)) as f32 / 200.0;
    let tongue = world.npc(PLAYER).temperament.sociability;
    let p = (0.15 + 0.35 * tongue + 0.5 * trust).clamp(0.02, 0.8);
    if world.rng.chance(p) {
        world.adjust_opinion(ha, hb, 30);
        world.adjust_opinion(hb, ha, 30);
        world.emit_root(
            EventKind::FeudEnded {
                a,
                b,
                how: Peace::Brokered,
            },
            None,
        );
        world.adjust_opinion(ha, PLAYER, 10);
        world.adjust_opinion(hb, PLAYER, 10);
        true
    } else {
        // Meddler.
        world.adjust_opinion(ha, PLAYER, -10);
        world.adjust_opinion(hb, PLAYER, -10);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::events::FireCause;

    fn feuding_pair(w: &mut World) -> (FamilyId, FamilyId, NpcId, NpcId) {
        let (a, b) = (1, 2);
        let (ha, hb) = (w.head_of(a).unwrap(), w.head_of(b).unwrap());
        w.feuds.insert((a, b));
        w.set_opinion(ha, hb, -80);
        w.set_opinion(hb, ha, -80);
        (a, b, ha, hb)
    }

    #[test]
    fn the_humble_spare_more_often_than_the_hard() {
        let spared = |humility: f32| {
            (0..400)
                .filter(|&s| {
                    let mut w = World::new(s);
                    let (_, _, ha, hb) = feuding_pair(&mut w);
                    let n = w.npc_mut(ha);
                    n.temperament.humility = humility;
                    n.temperament.piety = humility;
                    n.hidden.malice = 0.0;
                    mercy(&mut w, ha, hb, Retaliation::Arson).is_some()
                })
                .count()
        };
        assert!(spared(1.0) > spared(0.0) * 3 + 20);
    }

    #[test]
    fn a_rival_at_the_raising_can_end_a_feud() {
        let mut w = World::new(5);
        let (a, b, ha, hb) = feuding_pair(&mut w);
        let fire = w.emit_root(
            EventKind::Fire {
                owner: ha,
                cause: FireCause::Hearth,
                spread_from: None,
            },
            None,
        );
        w.run_cascades();
        // Whatever the fire made them think, they're just past hating.
        w.set_opinion(ha, hb, -60);
        w.set_opinion(hb, ha, -60);
        let ev = w.events[fire as usize].clone();
        raise(&mut w, &ev, ha, Some(hb));
        w.run_cascades();
        assert!(w.families[a as usize].barn_standing);
        assert!(!w.feud_between(a, b), "the feud should end");
    }

    #[test]
    fn feuds_starve_without_blood() {
        let mut w = World::new(9);
        let (a, b, ha, hb) = feuding_pair(&mut w);
        w.day = Day(QUIET_DAYS + 1);
        w.set_opinion(ha, hb, -30);
        w.set_opinion(hb, ha, -30);
        monthly(&mut w);
        w.run_cascades();
        assert!(!w.feud_between(a, b));
    }

    #[test]
    fn a_feud_stays_hot_while_they_hate() {
        let mut w = World::new(9);
        let (a, b, ..) = feuding_pair(&mut w);
        w.day = Day(QUIET_DAYS + 1);
        monthly(&mut w);
        w.run_cascades();
        assert!(w.feud_between(a, b));
    }

    #[test]
    fn the_enemy_of_my_enemy_warms_up() {
        let mut w = World::new(12);
        let (x, y, z) = (1, 2, 3);
        w.feuds.insert((x, z));
        w.feuds.insert((y, z));
        let (hx, hy) = (w.head_of(x).unwrap(), w.head_of(y).unwrap());
        w.set_opinion(hx, hy, -20);
        monthly(&mut w);
        w.run_cascades();
        assert!(w.opinion(hx, hy) > -20);
    }

    #[test]
    fn brokering_moves_opinions_one_way_or_the_other() {
        let mut w = World::new(4);
        let (a, b, ha, hb) = feuding_pair(&mut w);
        let before = w.opinion(ha, PLAYER);
        let ok = w.player_broker_peace(a, b);
        assert_eq!(ok, !w.feud_between(a, b));
        assert_ne!(w.opinion(ha, PLAYER), before);
        let _ = hb;
    }
}
