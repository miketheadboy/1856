//! What your life leaves behind: the papers writing you up, the store
//! closing its book to you, and children who grow up in your shadow.
//!
//! A speech makes the Herald or the Sovereign depending on which side you
//! spoke for; readers of each paper come to think of you the paper's way.
//! Dunmore extends credit on character as much as collateral, and a scandal
//! shuts the book. Your children absorb what you do all day: a preacher's
//! kid mostly takes after you and sometimes rebels hard; the souse's kid
//! either drinks or never touches it.

use super::calendar::Day;
use super::events::{EventId, EventKind, WorldEvent};
use super::history::Paper;
use super::life;
use super::psyche::LifeStage;
use super::world::{Faction, NpcId, PLAYER, World};

/// Days the store's book stays shut after a scandal.
pub const CREDIT_CUT_DAYS: u32 = 120;

/// The paper that takes your side, and the one that doesn't.
fn papers_for(faction: Faction) -> (Paper, Paper) {
    match faction {
        Faction::FreeState => (Paper::HeraldOfFreedom, Paper::SquatterSovereign),
        Faction::ProSlavery => (Paper::SquatterSovereign, Paper::HeraldOfFreedom),
    }
}

fn notice(world: &mut World, ev: &WorldEvent, paper: Paper, praise: bool) {
    if world.press.silenced[paper.index()] {
        return;
    }
    world.emit_child(
        ev,
        EventKind::Notice {
            paper,
            about: ev.id,
            praise,
        },
    );
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    let mine = world.npc(PLAYER).faction;
    let (ours, theirs) = papers_for(mine);
    match ev.kind {
        EventKind::Speech {
            speaker: PLAYER,
            calm,
            heard,
        } if heard >= 3 => {
            notice(world, ev, ours, true);
            if !calm {
                notice(world, ev, theirs, false);
            }
        }
        EventKind::Scandal { a, b, .. } => {
            if a == PLAYER || b == PLAYER {
                notice(world, ev, theirs, false);
            }
            // The store shuts its book to both.
            let until = Day(ev.day.0 + CREDIT_CUT_DAYS);
            for p in [a, b] {
                let f = world.npc(p).family as usize;
                world.families[f].stores.credit_cut_until = Some(until);
            }
        }
        EventKind::Lawsuit {
            plaintiff: PLAYER,
            judge,
            won: false,
            ..
        } if world.npc(judge).faction != mine => {
            // Wronged by the bogus court: a martyr in your own paper.
            notice(world, ev, ours, true);
        }
        EventKind::Sermon {
            preacher: PLAYER,
            flock,
        } if flock >= 8 => notice(world, ev, ours, true),
        EventKind::Muster { you: true, .. } => {
            notice(world, ev, ours, true);
            notice(world, ev, theirs, false);
        }
        EventKind::Legacy { .. } => on_legacy(world, ev),
        EventKind::Notice { paper, praise, .. } => {
            // Readers take the paper's view of you.
            let readers: Vec<NpcId> = world
                .living()
                .filter(|n| {
                    n.id != PLAYER && n.faction == paper.faction() && n.temperament.literacy > 0.4
                })
                .map(|n| n.id)
                .collect();
            let delta = if praise { 6 } else { -8 };
            for r in readers {
                world.adjust_opinion(r, PLAYER, delta);
            }
        }
        _ => {}
    }
}

/// The store won't carry you.
pub fn credit_cut(world: &World, family: super::world::FamilyId) -> bool {
    world.families[family as usize]
        .stores
        .credit_cut_until
        .is_some_and(|d| world.day < d)
}

/// Monthly: the player's children lean toward what the player does.
pub fn monthly(world: &mut World) {
    let paths = life::paths(world);
    let kids: Vec<NpcId> = world
        .living()
        .filter(|n| n.family == 0 && n.id != PLAYER && LifeStage::of(n.age) == LifeStage::Child)
        .map(|n| n.id)
        .collect();
    for k in kids {
        for &p in &paths {
            let t = &mut world.npc_mut(k).temperament;
            match p {
                "exalted preacher" | "lay preacher" => t.piety = (t.piety + 0.02).min(1.0),
                "town souse" => t.temper = (t.temper + 0.02).min(1.0),
                "silver tongue" | "big shot" => t.sociability = (t.sociability + 0.02).min(1.0),
                "pillar of the community" => t.generosity = (t.generosity + 0.02).min(1.0),
                "ruffian" => t.courage = (t.courage + 0.02).min(1.0),
                "rake" => t.honesty = (t.honesty - 0.02).max(0.0),
                "vagabond" => t.skepticism = (t.skepticism + 0.02).min(1.0),
                _ => {}
            }
        }
        let n = world.npc(k);
        if n.age >= super::ghosts::COMING_OF_AGE - 1
            && !world.life.legacies.contains(&k)
            && let Some(&path) = paths.first()
        {
            // At fifteen: take after you, or turn hard the other way.
            let rebel = world.rng.chance(0.25 + 0.4 * n.temperament.temper);
            world.life.legacies.push(k);
            world.emit_root(
                EventKind::Legacy {
                    child: k,
                    path,
                    rebelled: rebel,
                },
                None,
            );
        }
    }
}

fn on_legacy(world: &mut World, ev: &WorldEvent) {
    let EventKind::Legacy {
        child,
        path,
        rebelled,
    } = ev.kind
    else {
        return;
    };
    let t = &mut world.npc_mut(child).temperament;
    let s = if rebelled { -0.3 } else { 0.3 };
    let shift = |x: &mut f32| *x = (*x + s).clamp(0.0, 1.0);
    match path {
        "exalted preacher" | "lay preacher" => shift(&mut t.piety),
        "town souse" => shift(&mut t.temper),
        "silver tongue" | "big shot" => shift(&mut t.sociability),
        "pillar of the community" => shift(&mut t.generosity),
        "ruffian" => {
            if !rebelled {
                world.npc_mut(child).violence = 1;
            } else {
                shift(&mut t.humility);
            }
        }
        _ => shift(&mut t.loyalty),
    }
}

/// What the view can show: your latest press, if any.
pub fn last_notice(world: &World) -> Option<(Paper, bool, EventId)> {
    world.events.iter().rev().find_map(|e| match e.kind {
        EventKind::Notice {
            paper,
            about,
            praise,
        } => Some((paper, praise, about)),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::life::Activity;

    #[test]
    fn a_hot_speech_makes_both_papers() {
        let mut w = World::new(4);
        w.autopilot_player = false;
        for n in w.npcs.iter_mut() {
            n.temperament.sociability = 1.0;
            n.temperament.literacy = 1.0;
        }
        assert!(w.player_do(Activity::Speech { calm: false }));
        let notices: Vec<bool> = w
            .events
            .iter()
            .filter_map(|e| match e.kind {
                EventKind::Notice { praise, .. } => Some(praise),
                _ => None,
            })
            .collect();
        assert!(notices.contains(&true) && notices.contains(&false));
    }

    #[test]
    fn a_scandal_shuts_the_book() {
        let mut w = World::new(4);
        let other = w.head_of(3).unwrap();
        let spouse = super::super::romance::married_to(&w, PLAYER).unwrap();
        w.emit_root(
            EventKind::Scandal {
                a: PLAYER,
                b: other,
                wronged: spouse,
            },
            None,
        );
        w.run_cascades();
        assert!(credit_cut(&w, 0));
        assert!(credit_cut(&w, w.npc(other).family));
        w.run_days(CREDIT_CUT_DAYS + 1);
        assert!(!credit_cut(&w, 0));
    }

    #[test]
    fn the_preachers_kid_grows_up_one_way_or_the_other() {
        let mut w = World::new(4);
        let kid = w
            .living()
            .find(|n| n.family == 0 && n.id != PLAYER && LifeStage::of(n.age) == LifeStage::Child)
            .unwrap()
            .id;
        let before = w.npc(kid).temperament.piety;
        w.emit_root(
            EventKind::Legacy {
                child: kid,
                path: "exalted preacher",
                rebelled: false,
            },
            None,
        );
        w.run_cascades();
        assert!(w.npc(kid).temperament.piety > before);
    }
}
