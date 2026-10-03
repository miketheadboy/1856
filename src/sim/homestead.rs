//! What a claim becomes: a well, a smokehouse, a corn crib, a root cellar,
//! rail fence. Each takes days of work and loads of timber, and each changes
//! how the family weathers the county. The timber comes off the creek
//! bottoms, a load at a time, until the belts are stumps (§7.3's ring of
//! scarcity, made visible). Spring burns green the pasture and sometimes get
//! away.

use super::events::{EventKind, FireCause};
use super::geography::MILES_PER_TILE;
use super::market::Good;
use super::world::{FamilyId, PLAYER, World, distance};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Improvement {
    /// Clean water: children sicken less in the fever months (`mortality`).
    Well,
    /// Meat keeps without salt (`farmwork` butchering, `economy::butcher`).
    Smokehouse,
    /// Corn stored off the ground: thieves carry off less (`economy`).
    Crib,
    /// Roots through the winter: less food eaten in the cold (`economy`).
    Cellar,
    /// Split rails instead of brush: fences rot half as fast (`farmwork`).
    RailFence,
}

impl Improvement {
    pub const ALL: [Improvement; 5] = [
        Improvement::Well,
        Improvement::Smokehouse,
        Improvement::Crib,
        Improvement::Cellar,
        Improvement::RailFence,
    ];

    pub fn bit(self) -> u8 {
        1 << (self as u8)
    }

    /// (days of work, loads of timber)
    pub fn cost(self) -> (f32, f32) {
        match self {
            Improvement::Well => (8.0, 1.0),
            Improvement::Smokehouse => (5.0, 3.0),
            Improvement::Crib => (4.0, 3.0),
            Improvement::Cellar => (7.0, 1.0),
            Improvement::RailFence => (10.0, 6.0),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Improvement::Well => "well",
            Improvement::Smokehouse => "smokehouse",
            Improvement::Crib => "corn crib",
            Improvement::Cellar => "root cellar",
            Improvement::RailFence => "rail fence",
        }
    }
}

/// Built improvements (a bitset) and work in progress.
#[derive(Clone, Debug, Default)]
pub struct Improvements {
    pub built: u8,
    pub building: Option<(Improvement, f32)>,
    /// Burned off this spring: the pasture greens early.
    pub burned_pasture: bool,
}

pub fn has(world: &World, family: FamilyId, imp: Improvement) -> bool {
    world.families[family as usize].stores.improvements.built & imp.bit() != 0
}

/// A day's work on an improvement. Timber is spent when the work starts.
/// Returns whether any work was done.
pub fn build(world: &mut World, family: FamilyId, imp: Improvement, hands: f32) -> bool {
    if has(world, family, imp) {
        return false;
    }
    let (days, timber) = imp.cost();
    let hh = &mut world.families[family as usize].stores;
    let started = matches!(hh.improvements.building, Some((i, _)) if i == imp);
    if !started {
        if hh.goods[Good::Timber.index()] < timber {
            return false;
        }
        hh.goods[Good::Timber.index()] -= timber;
        hh.improvements.building = Some((imp, 0.0));
    }
    // Nails and a bit of iron, or pegs and patience.
    let iron = &mut hh.goods[Good::Iron.index()];
    let nailed = *iron >= 0.5;
    if nailed {
        *iron -= 0.5;
    }
    let Some((_, done)) = hh.improvements.building.as_mut() else {
        return false;
    };
    *done += hands * if nailed { 1.0 } else { 0.6 };
    if *done >= days {
        hh.improvements.building = None;
        hh.improvements.built |= imp.bit();
        world.emit_root(EventKind::Improved { family, what: imp }, None);
    }
    true
}

/// Cut a load of timber from the nearest stand. Off the reserve if there's
/// any within reach; otherwise, across the line. Returns loads cut.
pub fn cut_timber(world: &mut World, family: FamilyId) -> f32 {
    let farm = world.families[family as usize].farm;
    let Some(stand) = world.map.nearest_timber(farm) else {
        return 0.0;
    };
    let miles = distance(stand, farm) * MILES_PER_TILE;
    if miles > 4.0 {
        // Nothing close: the Delaware's timber is right there.
        super::nations::cut_reserve_timber(world, family);
        return 0.0;
    }
    if !world.map.cut(stand) {
        return 0.0;
    }
    world.families[family as usize].stores.goods[Good::Timber.index()] += 1.0;
    if world.map.loads_left(stand) == 0 {
        world.emit_root(
            EventKind::StandCut {
                family,
                tile: stand,
            },
            None,
        );
    }
    1.0
}

/// Burn off the pasture in spring. Greener grass means better calves; a dry,
/// windy day means the fire goes where it likes.
pub fn burn_pasture(world: &mut World, family: FamilyId) -> bool {
    if !matches!(world.day.month(), 3 | 4) {
        return false;
    }
    let farm = world.families[family as usize].farm;
    world.families[family as usize]
        .stores
        .improvements
        .burned_pasture = true;
    for dx in -1..=1 {
        for dy in -1..=1 {
            world.map.scorch((farm.0 + dx, farm.1 + dy), world.day.0);
        }
    }
    let w = world.weather_on(world.day);
    let escape = w.wind * world.dryness * 0.8;
    if world.rng.chance(escape) {
        // It got away. The nearest neighbor's barn is in its path.
        let target = world
            .families
            .iter()
            .filter(|f| f.id != family && f.barn_standing && f.farms())
            .min_by(|a, b| distance(a.farm, farm).total_cmp(&distance(b.farm, farm)))
            .and_then(|f| world.head_of(f.id));
        if let Some(owner) = target {
            let setter = world.head_of(family).unwrap_or(PLAYER);
            world.emit_root(
                EventKind::Fire {
                    owner,
                    cause: FireCause::EscapedBurn,
                    spread_from: Some(setter),
                },
                None,
            );
        }
    }
    true
}

/// Winter firewood: a load a week per household from the nearest stand.
pub fn daily(world: &mut World) {
    let winter = matches!(world.day.month(), 12 | 1 | 2);
    if winter && world.day.0 % 7 == 5 {
        for f in 0..world.families.len() {
            if world.head_of(f as FamilyId).is_none() {
                continue;
            }
            let farm = world.families[f].farm;
            if let Some(stand) = world.map.nearest_timber(farm)
                && distance(stand, farm) * MILES_PER_TILE <= 4.0
            {
                world.map.cut(stand);
            }
        }
    }
}

/// NPC households improve their claims as work and timber allow; the timber
/// comes out of the creek bottoms.
pub fn monthly(world: &mut World) {
    if world.day.month() == 3 {
        for f in &mut world.families {
            f.stores.improvements.burned_pasture = false;
        }
    }
    for f in 0..world.families.len() {
        let family = f as FamilyId;
        if !world.families[f].farms() || world.head_of(family).is_none() {
            continue;
        }
        let farm = world.families[f].farm;
        world.families[f].timber_miles = world.map.miles_to_timber(farm);
        if family == 0 && !world.autopilot_player {
            continue;
        }
        let Some(next) = Improvement::ALL
            .into_iter()
            .find(|&i| !has(world, family, i))
        else {
            continue;
        };
        let prudence = world.families[f].stores.prudence;
        let labor = super::psyche::labor(world, family);
        if !world.rng.chance(0.25 * prudence + 0.1 * labor) {
            continue;
        }
        let (_, timber) = next.cost();
        for _ in 0..(timber.ceil() as u32) {
            cut_timber(world, family);
        }
        for _ in 0..4 {
            build(world, family, next, labor.max(0.5));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::geography::Terrain;

    #[test]
    fn a_stand_cut_to_the_last_load_is_stumps() {
        let mut w = World::new(1);
        let farm = w.families[1].farm;
        let stand = w.map.nearest_timber(farm).unwrap();
        while w.map.loads_left(stand) > 0 {
            assert!(w.map.cut(stand));
        }
        assert_eq!(w.map.at(stand), Terrain::Stumps);
        assert!(!w.map.cut(stand));
        assert_ne!(w.map.nearest_timber(farm), Some(stand));
    }

    #[test]
    fn improvements_take_timber_and_days() {
        let mut w = World::new(2);
        w.families[0].stores.goods[Good::Timber.index()] = 0.0;
        assert!(
            !build(&mut w, 0, Improvement::Crib, 1.0),
            "no timber, no crib"
        );
        w.families[0].stores.goods[Good::Timber.index()] = 3.0;
        for _ in 0..4 {
            build(&mut w, 0, Improvement::Crib, 1.0);
        }
        assert!(has(&w, 0, Improvement::Crib));
        assert_eq!(w.families[0].stores.goods[Good::Timber.index()], 0.0);
    }

    #[test]
    fn a_windy_dry_burn_gets_away() {
        let escaped = (0..60).any(|s| {
            let mut w = World::new(s);
            while w.day.month() != 3 {
                w.advance_day();
            }
            w.dryness = 1.0;
            burn_pasture(&mut w, 0);
            w.run_cascades();
            w.events.iter().any(|e| {
                matches!(
                    e.kind,
                    EventKind::Fire {
                        cause: FireCause::EscapedBurn,
                        ..
                    }
                )
            })
        });
        assert!(escaped);
    }
}
