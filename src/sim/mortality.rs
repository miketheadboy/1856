//! Childhood mortality. On the 1850s frontier perhaps one child in four did
//! not live to adulthood: summer complaint and ague in the hot months, croup
//! and scarlet fever in the cold, cholera up the river in 1855. Hunger, a
//! frail constitution and a burned-out house make each of them likelier.

use super::events::{EventKind, Hardship};
use super::psyche::LifeStage;
use super::world::{NpcId, World};

/// Annual risk of dying of sickness, by age.
pub fn annual_risk(age: u8) -> f32 {
    match age {
        0 => 0.15,
        1..=4 => 0.06,
        5..=9 => 0.015,
        _ => 0.008,
    }
}

/// Sickly seasons: July–September for fevers and the flux; winter for croup.
pub fn season_factor(month: u32) -> f32 {
    match month {
        7..=9 => 1.8,
        12 | 1 | 2 => 1.3,
        _ => 0.6,
    }
}

/// One day's chance for this child.
pub fn daily_risk(world: &World, id: NpcId) -> f32 {
    let n = world.npc(id);
    let f = &world.families[n.family as usize];
    let hungry = if f.stores.hungry_days > 0 { 2.0 } else { 1.0 };
    let exposed = if f.barn_standing { 1.0 } else { 1.3 };
    let weak = 1.0 + (100 - n.health.clamp(0, 100)) as f32 / 100.0;
    // Well water instead of the creek: fewer fevers in the hot months.
    let well = if super::homestead::has(world, n.family, super::homestead::Improvement::Well) {
        0.8
    } else {
        1.0
    };
    annual_risk(n.age)
        * season_factor(world.day.month())
        * n.body.frailty()
        * hungry
        * exposed
        * well
        * weak
        / 365.0
}

pub fn daily(world: &mut World) {
    let children: Vec<NpcId> = world
        .living()
        .filter(|n| LifeStage::of(n.age) == LifeStage::Child)
        .map(|n| n.id)
        .collect();
    for c in children {
        let p = daily_risk(world, c);
        if world.rng.chance(p) {
            world.emit_root(
                EventKind::Perished {
                    victim: c,
                    cause: Hardship::Fever,
                },
                None,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn little_ones_and_summer_are_deadliest() {
        assert!(annual_risk(2) > annual_risk(7) && annual_risk(7) > annual_risk(13));
        assert!(season_factor(8) > season_factor(1) && season_factor(1) > season_factor(4));
    }

    #[test]
    fn hunger_doubles_the_risk() {
        let mut w = World::new(3);
        let child = w
            .living()
            .find(|n| LifeStage::of(n.age) == LifeStage::Child)
            .unwrap()
            .id;
        let f = w.npc(child).family as usize;
        w.families[f].stores.hungry_days = 0;
        let fed = daily_risk(&w, child);
        w.families[f].stores.hungry_days = 5;
        assert!((daily_risk(&w, child) - 2.0 * fed).abs() < 1e-6);
    }

    #[test]
    fn about_one_in_ten_children_die_over_three_years() {
        let (mut kids, mut dead) = (0, 0);
        for seed in 1..=10 {
            let mut w = World::new(seed);
            let start: Vec<NpcId> = w
                .living()
                .filter(|n| LifeStage::of(n.age) == LifeStage::Child)
                .map(|n| n.id)
                .collect();
            w.run_days(3 * 365);
            kids += start.len();
            dead += start
                .iter()
                .filter(|&&c| {
                    w.events.iter().any(
                        |e| matches!(e.kind, EventKind::Perished { victim, .. } if victim == c),
                    )
                })
                .count();
        }
        let rate = dead as f32 / kids as f32;
        assert!((0.03..0.25).contains(&rate), "{dead}/{kids} = {rate}");
    }
}
