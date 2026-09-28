//! The player's dark verbs. Secrets are learned at the groggery or out at
//! night; blackmail sells your silence; exposure spends it. Sabotage is done
//! in the dark and read by the victim through the same attribution engine as
//! everything else — they may blame you, or the neighbor they already hated.
//! Slander points a family at somebody for a harm they already suffered.

use super::events::{Cruelty, EventKind};
use super::institutions::{self, Secret};
use super::world::{NpcId, PLAYER, World};

/// Dollars asked for silence.
pub const PRICE: i32 = 10;

/// The player now knows something about `about`.
pub fn learn(world: &mut World, about: NpcId) {
    let secrets = &mut world.institutions.secrets;
    match secrets.iter_mut().find(|s| s.about == about) {
        Some(s) if !s.known_by.contains(&PLAYER) => s.known_by.push(PLAYER),
        Some(_) => {}
        None => secrets.push(Secret {
            about,
            known_by: vec![PLAYER],
            exposed: false,
        }),
    }
}

/// An unexposed secret the player holds about `about`.
pub fn held(world: &World, about: NpcId) -> Option<usize> {
    world
        .institutions
        .secrets
        .iter()
        .position(|s| s.about == about && !s.exposed && s.known_by.contains(&PLAYER))
}

/// Everyone the player could lean on.
pub fn leverage(world: &World) -> Vec<NpcId> {
    world
        .institutions
        .secrets
        .iter()
        .filter(|s| !s.exposed && s.known_by.contains(&PLAYER) && world.npc(s.about).alive)
        .map(|s| s.about)
        .collect()
}

/// Ask for money to keep quiet. Returns whether they paid.
pub fn blackmail(world: &mut World, target: NpcId) -> Option<bool> {
    held(world, target)?;
    let family = world.npc(target).family as usize;
    let n = world.npc(target);
    let scared = 0.3 + n.emotions.fear / 150.0 + 0.3 * (1.0 - n.temperament.courage);
    let pays = world.families[family].stores.cash >= PRICE && world.rng.chance(scared.min(0.9));
    if pays {
        world.families[family].stores.cash -= PRICE;
        world.families[0].stores.cash += PRICE;
        world.adjust_opinion(target, PLAYER, -30);
    } else {
        // Call your bluff, and hate you for trying.
        world.adjust_opinion(target, PLAYER, -40);
    }
    world.emit_root(
        EventKind::Blackmail {
            victim: target,
            extorter: PLAYER,
            paid: pays,
        },
        None,
    );
    Some(pays)
}

/// Tell the county. Your leverage is spent, and they know who talked.
pub fn expose(world: &mut World, target: NpcId) -> bool {
    let Some(i) = held(world, target) else {
        return false;
    };
    institutions::expose(world, i, PLAYER);
    world.adjust_opinion(target, PLAYER, -60);
    true
}

/// Do harm at night. No alibi; the victim reads it as they read anything.
pub fn sabotage(world: &mut World, target: NpcId, act: Cruelty) -> bool {
    if target == PLAYER || !world.npc(target).alive || world.npc(target).family == 0 {
        return false;
    }
    world.npc_mut(PLAYER).alibi = None;
    world.emit_root(
        EventKind::Cruelty {
            actor: PLAYER,
            victim: target,
            act,
        },
        None,
    );
    true
}

/// Tell the family that suffered the most recent harm that `target` did it.
pub fn slander(world: &mut World, target: NpcId) -> bool {
    let recent = world
        .events
        .iter()
        .rev()
        .take_while(|e| e.day.0 + 60 >= world.day.0)
        .find_map(|e| {
            let v = super::attribution::victim_of(world, e.id)?;
            (world.npc(v).family != world.npc(target).family
                && world.npc(v).family != 0
                && world.npc(v).alive)
                .then_some((e.id, v))
        });
    let Some((about, listener)) = recent else {
        return false;
    };
    world.emit_root(
        EventKind::Cruelty {
            actor: PLAYER,
            victim: target,
            act: Cruelty::Slander { listener, about },
        },
        None,
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_secret_no_leverage() {
        let mut w = World::new(2);
        let t = w.head_of(3).unwrap();
        assert_eq!(blackmail(&mut w, t), None);
        learn(&mut w, t);
        assert!(leverage(&w).contains(&t));
        assert!(blackmail(&mut w, t).is_some());
    }

    #[test]
    fn exposure_spends_the_secret() {
        let mut w = World::new(2);
        let t = w.head_of(3).unwrap();
        learn(&mut w, t);
        assert!(expose(&mut w, t));
        w.run_cascades();
        assert!(!expose(&mut w, t));
        assert!(w.opinion(t, PLAYER) <= -50);
    }

    #[test]
    fn a_cut_fence_lets_stock_wander_and_someone_gets_blamed() {
        let mut w = World::new(5);
        let t = w.head_of(2).unwrap();
        let f = w.npc(t).family as usize;
        w.families[f].stores.work.fences = 1.0;
        assert!(sabotage(&mut w, t, Cruelty::CutFence));
        w.run_cascades();
        assert_eq!(w.families[f].stores.work.fences, 0.0);
        assert!(
            w.events
                .iter()
                .any(|e| matches!(e.kind, EventKind::Belief { holder, .. } if holder == t))
        );
    }
}
