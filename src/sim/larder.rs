//! The larder and the water: what a household eats and drinks beyond the
//! corn (§10).
//!
//! `Household::food` is still the calories: days of eating for one person.
//! The larder says what those days are made of, and each part moves
//! something:
//! - **Meat** is a share of `food` (butchering, the buffalo, game, Missouri
//!   bacon). Hog and hominy is a working diet; hominy alone is not
//!   (`psyche::labor`).
//! - **Garden** stores (potatoes, turnips, squash, beans) sit beside the
//!   corn. They're dug in September, eaten through the cold half-year,
//!   and they rot without a cellar. Sugar puts them up as preserves and
//!   dried apples. No garden in late winter means scurvy (`sickness`).
//! - **Milk**, from a family cow in the grass months, keeps children alive
//!   (`mortality`).
//! - **Coffee**: a pound a week. Run out for a fortnight and tempers fray.
//!   The blockade summer of 1856 was a coffee famine.
//! - **Cloth** mends the family's clothes before they're rags (`wardrobe`
//!   wear: warmth and lice).
//! - **Water**: a well; else the creek, which runs low and foul in a dry
//!   August; else hauled by the barrel. Hauling costs a hand's day
//!   (`psyche::labor`), and a low creek breeds the flux (`sickness`).
//!
//! Whiskey stays in `market` and `life`.

use super::geography::Terrain;
use super::homestead::{self, Improvement};
use super::market::{self, Good};
use super::world::{FamilyId, NpcId, World, distance};

/// Days of meat a pound of side bacon makes for one person.
pub const BACON_DAYS: f32 = 1.5;
/// Days of garden per person dug in a fair year.
const HARVEST_DAYS: f32 = 40.0;
/// Roots eaten per person-day through the cold half-year.
const ROOTS_A_DAY: f32 = 0.3;

#[derive(Clone, Debug, Default)]
pub struct Larder {
    /// The part of `food` that is meat, 0..1. A share, not a count: meat
    /// goes wherever the food goes (eaten, stolen, given away, sold).
    pub meat_share: f32,
    /// Days of roots and put-up garden for one person, apart from `food`.
    pub garden: f32,
    /// Put up with sugar this fall: stores keep twice as long.
    pub preserved: bool,
    /// Days since the coffee ran out (0 while there's coffee).
    pub dry: u16,
}

/// Where a household's water comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Water {
    Well,
    /// A creek or the river within a mile and a half.
    Creek,
    /// A barrel on the wagon, every other day.
    Hauled,
}

impl Water {
    pub fn label(self) -> &'static str {
        match self {
            Water::Well => "well",
            Water::Creek => "creek",
            Water::Hauled => "hauled",
        }
    }
}

pub fn water(world: &World, family: FamilyId) -> Water {
    if homestead::has(world, family, Improvement::Well) {
        return Water::Well;
    }
    let (x, y) = world.families[family as usize].farm;
    for dy in -3..=3 {
        for dx in -3..=3 {
            let t = (x + dx, y + dy);
            if distance(t, (x, y)) <= 3.0 && world.map.at(t) == Terrain::River {
                return Water::Creek;
            }
        }
    }
    Water::Hauled
}

/// The creek runs low and warm: late summer after a dry spell.
pub fn creek_low(world: &World) -> bool {
    (7..=9).contains(&world.day.month()) && world.dryness > 0.7
}

/// Somebody spends the day with a barrel: no creek and no well, or a creek
/// gone to mud.
pub fn hauling(world: &World, family: FamilyId) -> bool {
    match water(world, family) {
        Water::Well => false,
        Water::Creek => creek_low(world),
        Water::Hauled => true,
    }
}

/// Meat and garden in the diet: `psyche::labor`'s multiplier.
pub fn fed_for_work(world: &World, family: FamilyId) -> f32 {
    let meat = if meat(world, family) > 0.5 { 1.0 } else { 0.92 };
    let water = if hauling(world, family) { 0.93 } else { 1.0 };
    meat * water
}

/// Fresh garden in season, or roots in the cellar.
pub fn greens(world: &World, family: FamilyId) -> bool {
    (5..=9).contains(&world.day.month())
        || world.families[family as usize].stores.larder.garden > 0.5
}

/// A family cow in milk: the grass months, and a cow to milk.
pub fn milk(world: &World, family: FamilyId) -> bool {
    (4..=10).contains(&world.day.month()) && world.families[family as usize].stores.cattle > 0
}

/// Days of meat for one person in the house.
pub fn meat(world: &World, family: FamilyId) -> f32 {
    let hh = &world.families[family as usize].stores;
    hh.food.max(0.0) * hh.larder.meat_share
}

/// Meat onto the table: adds to the food and to meat's share of it.
pub fn add_meat(world: &mut World, family: FamilyId, days: f32) {
    let hh = &mut world.families[family as usize].stores;
    let before = hh.food.max(0.0) * hh.larder.meat_share;
    hh.food += days;
    hh.larder.meat_share = if hh.food > 0.0 {
        ((before + days) / hh.food).clamp(0.0, 1.0)
    } else {
        0.0
    };
}

/// Roots off the cellar shelf through the cold half-year: what they spare
/// of the corn today.
pub fn eat_garden(world: &mut World, family: FamilyId, people: f32) -> f32 {
    if (4..=9).contains(&world.day.month()) {
        return 0.0;
    }
    let hh = &mut world.families[family as usize].stores;
    let ate = hh.larder.garden.min(ROOTS_A_DAY * people);
    hh.larder.garden -= ate;
    ate
}

pub fn daily(world: &mut World) {
    let today = world.day;
    let (_, month, dom) = today.date();
    let monday = today.0.is_multiple_of(7);
    for f in 0..world.families.len() {
        let family = f as FamilyId;
        if world.families[f].store || world.head_of(family).is_none() {
            continue;
        }
        let npc_house = family != 0 || world.autopilot_player;
        // Bacon off the wagon hangs in the smokehouse as meat.
        let bacon = world.families[f].stores.goods[Good::Bacon.index()];
        if bacon > 0.0 {
            world.families[f].stores.goods[Good::Bacon.index()] = 0.0;
            add_meat(world, family, bacon * BACON_DAYS);
        }
        // Digging the garden, the middle of September.
        if month == 9 && dom == 15 {
            harvest(world, family);
        }
        // Roots rot in a crib; a cellar keeps them; preserves keep best.
        let cellar = homestead::has(world, family, Improvement::Cellar);
        let l = &mut world.families[f].stores.larder;
        let rot = if cellar { 0.002 } else { 0.01 } * if l.preserved { 0.5 } else { 1.0 };
        l.garden *= 1.0 - rot;
        if monday {
            coffee(world, family);
            mend(world, family);
            if npc_house {
                shop(world, family, month);
            }
        }
        // A fortnight without coffee: short words at the table. After a
        // month or so they parch corn and rye and call it coffee, and get
        // used to it.
        let dry = &mut world.families[f].stores.larder.dry;
        if *dry > 0 {
            *dry = dry.saturating_add(1);
        }
        if (14..45).contains(&*dry) {
            let kin: Vec<NpcId> = world
                .living()
                .filter(|n| n.family == family)
                .map(|n| n.id)
                .collect();
            for k in kin {
                world.npc_mut(k).emotions.anger += 0.15;
            }
        }
    }
}

/// Dig the garden. A dry August shrinks it; sugar puts it up.
fn harvest(world: &mut World, family: FamilyId) {
    let people = world.living().filter(|n| n.family == family).count() as f32;
    let luck = world
        .head_of(family)
        .map_or(1.0, |h| world.npc(h).hidden.luck);
    let drought = 1.0 - 0.5 * (world.dryness - 0.6).max(0.0);
    let dug = HARVEST_DAYS * people * luck * drought * (0.8 + 0.4 * world.rng.unit());
    let hh = &mut world.families[family as usize].stores;
    let sugar = hh.goods[Good::Sugar.index()] >= 5.0;
    if sugar {
        hh.goods[Good::Sugar.index()] -= 5.0;
    }
    hh.larder.preserved = sugar;
    hh.larder.garden += dug;
}

/// A pound of coffee a week, while it lasts.
fn coffee(world: &mut World, family: FamilyId) {
    let had = market::consume(world, family, Good::Coffee);
    let l = &mut world.families[family as usize].stores.larder;
    if had {
        l.dry = 0;
    } else if l.dry == 0 {
        l.dry = 1;
    }
}

/// Sitting up with the mending: two yards patch one person's clothes.
fn mend(world: &mut World, family: FamilyId) {
    let worn: Vec<NpcId> = world
        .living()
        .filter(|n| n.family == family && n.outfit.wear > 0.6)
        .map(|n| n.id)
        .collect();
    for id in worn {
        let cloth = &mut world.families[family as usize].stores.goods[Good::Cloth.index()];
        if *cloth < 2.0 {
            break;
        }
        *cloth -= 2.0;
        let o = &mut world.npc_mut(id).outfit;
        o.wear = (o.wear - 0.35).max(0.0);
    }
}

/// What an ordinary house buys at Dunmore's for the table and the mending.
fn shop(world: &mut World, family: FamilyId, month: u32) {
    let hh = &world.families[family as usize].stores;
    let cash = hh.cash;
    let coffee = hh.goods[Good::Coffee.index()];
    let meat = meat(world, family);
    let cloth = hh.goods[Good::Cloth.index()];
    let sugar = hh.goods[Good::Sugar.index()];
    let people = world.living().filter(|n| n.family == family).count() as f32;
    if coffee < 1.0 && cash >= 4 {
        market::buy(world, family, Good::Coffee, 4.0);
    }
    // Nothing to butcher yet and the smokehouse bare: a side of bacon.
    if meat < 2.0 * people && !(5..=9).contains(&month) && cash >= 8 {
        market::buy(world, family, Good::Bacon, 10.0 * people.min(4.0));
    }
    if month == 8 && sugar < 5.0 && cash >= 6 {
        market::buy(world, family, Good::Sugar, 5.0);
    }
    let ragged = world
        .living()
        .any(|n| n.family == family && n.outfit.wear > 0.5);
    if ragged && cloth < 2.0 && cash >= 6 {
        market::buy(world, family, Good::Cloth, 4.0);
    }
}

/// One household's table, for the lab and `debug`.
pub fn describe(world: &World, family: FamilyId) -> String {
    let hh = &world.families[family as usize].stores;
    let l = &hh.larder;
    let g = |x: Good| hh.goods[x.index()];
    format!(
        "food {:.0}d (meat {:.0}d)  garden {:.0}d{}  milk {}  coffee {:.0}lb{}  cloth {:.0}yd  iron {:.0}lb  water {}{}",
        hh.food,
        meat(world, family),
        l.garden,
        if l.preserved { " put up" } else { "" },
        if milk(world, family) { "yes" } else { "no" },
        g(Good::Coffee),
        if l.dry > 0 {
            format!(" (out {} days)", l.dry)
        } else {
            String::new()
        },
        g(Good::Cloth),
        g(Good::Iron),
        water(world, family).label(),
        if hauling(world, family) {
            ", hauling"
        } else {
            ""
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_family(w: &World) -> FamilyId {
        (1..w.families.len() as FamilyId)
            .find(|&f| w.families[f as usize].farms() && w.head_of(f).is_some())
            .unwrap()
    }

    #[test]
    fn meat_goes_wherever_the_food_goes() {
        let mut w = World::new(3);
        let f = a_family(&w);
        w.families[f as usize].stores.food = 50.0;
        w.families[f as usize].stores.larder.meat_share = 0.0;
        add_meat(&mut w, f, 50.0);
        assert_eq!(w.families[f as usize].stores.food, 100.0);
        assert!((meat(&w, f) - 50.0).abs() < 0.01);
        // Eaten, stolen or sold, the rest is still half meat.
        w.families[f as usize].stores.food = 60.0;
        assert!((meat(&w, f) - 30.0).abs() < 0.01);
    }

    #[test]
    fn a_cellar_keeps_the_roots_and_sugar_keeps_them_longer() {
        let keep = |cellar: bool, sugar: bool| {
            let mut w = World::new(3);
            let f = a_family(&w);
            if cellar {
                w.families[f as usize].stores.improvements.built |= Improvement::Cellar.bit();
            } else {
                w.families[f as usize].stores.improvements.built &= !Improvement::Cellar.bit();
            }
            w.families[f as usize].stores.goods[Good::Sugar.index()] =
                if sugar { 5.0 } else { 0.0 };
            // Into the middle of September, dig, then sit on it a month.
            use crate::sim::Day;
            while !(w.day.month() == 9 && w.day.date().2 == 16) {
                w.day = Day(w.day.0 + 1);
            }
            w.day = Day(w.day.0 - 1);
            harvest(&mut w, f);
            w.families[f as usize].stores.larder.garden = 100.0;
            for _ in 0..30 {
                let cellar = homestead::has(&w, f, Improvement::Cellar);
                let l = &mut w.families[f as usize].stores.larder;
                let rot = if cellar { 0.002 } else { 0.01 } * if l.preserved { 0.5 } else { 1.0 };
                l.garden *= 1.0 - rot;
            }
            w.families[f as usize].stores.larder.garden
        };
        assert!(keep(true, false) > keep(false, false) + 15.0);
        assert!(keep(false, true) > keep(false, false));
    }

    #[test]
    fn no_coffee_frays_tempers() {
        let mut w = World::new(3);
        let f = a_family(&w);
        w.families[f as usize].stores.goods[Good::Coffee.index()] = 0.0;
        w.families[f as usize].stores.cash = 0;
        let head = w.head_of(f).unwrap();
        w.run_days(30);
        assert!(w.families[f as usize].stores.larder.dry >= 14 || w.npc(head).emotions.anger > 0.0);
    }

    #[test]
    fn cloth_mends_rags() {
        let mut w = World::new(3);
        let f = a_family(&w);
        let head = w.head_of(f).unwrap();
        w.npc_mut(head).outfit.wear = 0.9;
        w.families[f as usize].stores.goods[Good::Cloth.index()] = 4.0;
        mend(&mut w, f);
        assert!(w.npc(head).outfit.wear < 0.6);
    }

    #[test]
    fn hauling_water_costs_a_hand() {
        let mut w = World::new(3);
        let f = a_family(&w);
        w.families[f as usize].stores.improvements.built &= !Improvement::Well.bit();
        if water(&w, f) == Water::Creek {
            w.dryness = 1.0;
            while !(7..=9).contains(&w.day.month()) {
                w.day = crate::sim::Day(w.day.0 + 1);
            }
        }
        assert!(hauling(&w, f));
        let with = fed_for_work(&w, f);
        w.families[f as usize].stores.improvements.built |= Improvement::Well.bit();
        assert!(fed_for_work(&w, f) > with);
    }
}
