//! The buffalo (§3 of the systems framework: the ecological-political loop).
//!
//! By 1855 the herds were gone from Douglas County; the range began around
//! Council Grove and the Smoky Hill and ran west. So bison enter the county
//! through what they feed: robes and hides at Dunmore's, the Kaw's fall hunt,
//! the plains nations' defense of their range against trail traffic, and the
//! long hunting trips settlers made west when the corn ran out.
//!
//! The loop: scarcity raises robe prices; high prices bring more hunters;
//! more hunters make the herd scarcer. Nobody has to decide to end the
//! buffalo. They just have to keep eating.

use super::events::EventKind;
use super::market::Good;
use super::nations::NationId;
use super::psyche::LifeStage;
use super::world::{FamilyId, NpcId, World};

/// The herd within reach of eastern Kansas hunters, in animals.
pub const RANGE_CAPACITY: f32 = 60_000.0;

#[derive(Clone, Debug)]
pub struct Herd {
    pub population: f32,
    /// Hunters on the range this year (hide men, emigrants, trail crews).
    pub pressure: f32,
    /// Animals taken this season.
    pub taken: f32,
}

impl Default for Herd {
    fn default() -> Self {
        Self {
            population: RANGE_CAPACITY * 0.7,
            pressure: 1.0,
            taken: 0.0,
        }
    }
}

impl Herd {
    /// 0..1: how easy it is to find buffalo.
    pub fn abundance(&self) -> f32 {
        (self.population / RANGE_CAPACITY).clamp(0.0, 1.0)
    }
}

/// Monthly: calving in spring, hunting pressure driven by robe prices,
/// the Kaw fall hunt, and the plains nations' response to trail traffic.
pub fn monthly(world: &mut World) {
    let month = world.day.month();
    let robe_price = world.market.pressure(Good::Hides);
    let herd = &mut world.bison;

    // Calving: May and June. Growth slows as the range fills.
    if month == 5 || month == 6 {
        let room = 1.0 - herd.population / RANGE_CAPACITY;
        herd.population += herd.population * 0.09 * room.max(0.0);
    }

    // Hide men follow the price. Dear robes bring more guns onto the range.
    herd.pressure = (0.6 + 0.6 * robe_price).clamp(0.5, 3.0);
    let season = match month {
        10..=12 | 1 | 2 => 1.4, // robes are prime in winter
        _ => 0.6,
    };
    let kill = 700.0 * herd.pressure * season * herd.abundance().sqrt();
    herd.population = (herd.population - kill).max(0.0);
    herd.taken += kill;

    // Robes reach Westport, then the store. A thinner herd, fewer robes.
    let abundance = herd.abundance();
    world.market.goods[Good::Hides.index()].stock += 20.0 * abundance * season;

    // The Kaw hunt west each fall. Their winter rides on it.
    if month == 10 {
        let kaw = &mut world.nations[NationId::Kaw.index()];
        let hunt = 0.35 * abundance;
        kaw.food_security = (kaw.food_security + hunt).min(1.0);
        world.emit_root(
            EventKind::KawHunt {
                good: abundance > 0.4,
            },
            None,
        );
    }

    // The plains nations defend a range they can see being emptied. Trail
    // safety falls as the herd thins and traffic grows.
    let target = 0.5 + 0.5 * abundance;
    world.market.trail_safety += (target - world.market.trail_safety) * 0.2;

    if month == 1 {
        world.bison.taken = 0.0;
    }
}

/// A family sends its best shot west for a few weeks. Big meat, robes to sell,
/// no seed corn eaten. Costs powder, weeks of labor, and the cold.
pub fn go_west(world: &mut World, family: FamilyId) -> bool {
    let Some(hunter) = world
        .living()
        .filter(|n| n.family == family && LifeStage::of(n.age) == LifeStage::Adult && !n.wounded)
        .max_by(|a, b| a.body.marksmanship.total_cmp(&b.body.marksmanship))
        .map(|n| n.id)
    else {
        return false;
    };
    let hh = &mut world.families[family as usize].stores;
    if hh.goods[Good::Powder.index()] < 3.0 || hh.oxen == 0 {
        // You need powder and a wagon team to bring meat home.
        return false;
    }
    hh.goods[Good::Powder.index()] -= 3.0;
    resolve_trip(world, family, hunter);
    true
}

fn resolve_trip(world: &mut World, family: FamilyId, hunter: NpcId) {
    let n = world.npc(hunter);
    let skill = 0.3 + 0.7 * n.body.marksmanship;
    let luck = n.hidden.luck;
    let frailty = n.body.frailty();
    let winter = world.day.season() == super::calendar::Season::Winter;
    let abundance = world.bison.abundance();
    let animals = (4.0 * skill * luck * abundance * world.rng.unit().max(0.3)).round();
    // Weeks away: the family eats without them, and the plains are cold.
    world.npc_mut(hunter).health -= ((if winter { 12.0 } else { 5.0 }) * frailty) as i32;
    super::larder::add_meat(world, family, animals * 120.0);
    let hh = &mut world.families[family as usize].stores;
    hh.goods[Good::Hides.index()] += animals;
    world.bison.population = (world.bison.population - animals).max(0.0);
    world.npc_mut(hunter).alibi = None;
    world.emit_root(
        EventKind::BuffaloHunt {
            hunter,
            animals: animals as u32,
        },
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_robe_prices_drive_the_herd_down() {
        let mut cheap = World::new(5);
        let mut dear = World::new(5);
        dear.market.goods[Good::Hides.index()].price *= 4.0;
        // Pin the price: the scarcity loop is what's under test.
        for _ in 0..12 {
            dear.market.goods[Good::Hides.index()].price = 6.0;
            monthly(&mut cheap);
            monthly(&mut dear);
        }
        assert!(dear.bison.population < cheap.bison.population);
    }

    #[test]
    fn a_thin_herd_means_a_hungry_kaw_winter() {
        let mut fat = World::new(5);
        let mut thin = World::new(5);
        thin.bison.population = RANGE_CAPACITY * 0.05;
        fat.day = super::super::Day(335); // 1 October 1856
        thin.day = fat.day;
        monthly(&mut fat);
        monthly(&mut thin);
        assert!(
            thin.nations[NationId::Kaw.index()].food_security
                < fat.nations[NationId::Kaw.index()].food_security
        );
    }

    #[test]
    fn going_west_needs_powder_and_oxen() {
        let mut w = World::new(5);
        w.families[0].stores.goods[Good::Powder.index()] = 0.0;
        assert!(!go_west(&mut w, 0));
        w.families[0].stores.goods[Good::Powder.index()] = 5.0;
        w.families[0].stores.oxen = 2;
        assert!(go_west(&mut w, 0));
    }
}
