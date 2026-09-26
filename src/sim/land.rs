//! Real estate. Lawrence town lots went for a few dollars in 1855 and for
//! hundreds by the spring of 1857, when every emigrant was a speculator;
//! then the Ohio Life failed in August 1857 and the bottom fell out. Claims
//! changed hands too: a family broken by a winter would sell its
//! relinquishment for enough to get back to the States.

use super::calendar::Day;
use super::events::{EventKind, WorldEvent};
use super::world::{FamilyId, NpcId, PLAYER, World};

/// Price per acre at the land sale, and what a relinquishment went for on top.
const PER_ACRE: f32 = 1.25;
const BARN: f32 = 20.0;

#[derive(Clone, Debug)]
pub struct Land {
    /// Dollars for a Lawrence town lot.
    pub lot_price: f32,
    /// Lots the player holds.
    pub lots: u8,
    /// What the player paid for them, in total.
    pub paid: f32,
    /// Claims the player has bought out.
    pub claims_bought: u8,
}

impl Default for Land {
    fn default() -> Self {
        Land {
            lot_price: 25.0,
            lots: 0,
            paid: 0.0,
            claims_bought: 0,
        }
    }
}

fn after(day: Day, date: (i32, u32, u32)) -> bool {
    day.date() >= date
}

/// The month's move in lot prices.
pub fn monthly(world: &mut World) {
    let day = world.day;
    let month = day.month();
    let mut drift = if (4..=6).contains(&month) {
        0.06
    } else {
        0.015
    };
    // The spring of 1857: every emigrant a speculator.
    if after(day, (1857, 3, 1)) && !after(day, (1857, 8, 24)) {
        drift += 0.12;
    }
    // The Panic.
    if after(day, (1857, 9, 1)) {
        drift = -0.15;
    }
    // The month after the Sack of Lawrence.
    if day.date().0 == 1856 && month == 6 {
        drift -= 0.25;
    }
    let noise = (world.rng.unit() - 0.5) * 0.06;
    let l = &mut world.land;
    l.lot_price = (l.lot_price * (1.0 + drift + noise)).max(3.0);
}

pub fn buy_lot(world: &mut World) -> bool {
    let price = world.land.lot_price.round() as i32;
    let hh = &mut world.families[0].stores;
    if hh.cash < price {
        return false;
    }
    hh.cash -= price;
    world.land.lots += 1;
    world.land.paid += price as f32;
    true
}

pub fn sell_lot(world: &mut World) -> bool {
    if world.land.lots == 0 {
        return false;
    }
    let price = (world.land.lot_price * 0.95).round() as i32;
    let avg = world.land.paid / world.land.lots as f32;
    world.land.lots -= 1;
    world.land.paid -= avg;
    world.families[0].stores.cash += price;
    true
}

/// Is this family broken enough to sell?
pub fn would_sell(world: &World, family: FamilyId) -> bool {
    let f = &world.families[family as usize];
    let hh = &f.stores;
    family != 0
        && f.farms()
        && world.head_of(family).is_some()
        && (hh.hungry_days > 0 || hh.debt >= 20 || hh.food < 30.0)
}

pub fn claim_price(world: &World, family: FamilyId) -> i32 {
    let f = &world.families[family as usize];
    (f.stores.acres as f32 * PER_ACRE + if f.barn_standing { BARN } else { 0.0 }).round() as i32
}

/// Buy a broken family's relinquishment. They go back east; their plowed
/// acres are yours to work.
pub fn buy_claim(world: &mut World, family: FamilyId) -> bool {
    if !would_sell(world, family) {
        return false;
    }
    let head = world.head_of(family).unwrap();
    if world.opinion(head, PLAYER) < -20 {
        return false; // not to you
    }
    let price = claim_price(world, family);
    if world.families[0].stores.cash < price {
        return false;
    }
    world.families[0].stores.cash -= price;
    world.emit_root(
        EventKind::ClaimBought {
            family,
            seller: head,
            price: price as u16,
        },
        None,
    );
    true
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    if let EventKind::ClaimBought { family, price, .. } = ev.kind {
        let acres = world.families[family as usize].stores.acres;
        world.families[0].stores.acres += acres;
        world.families[family as usize].stores.cash += price as i32;
        world.families[family as usize].stores.acres = 0;
        let gone: Vec<NpcId> = world
            .living()
            .filter(|n| n.family == family)
            .map(|n| n.id)
            .collect();
        for g in gone {
            world.npc_mut(g).departed = true;
        }
        world.land.claims_bought += 1;
    }
}

/// Paper wealth in lots at today's price.
pub fn holdings(world: &World) -> f32 {
    world.land.lots as f32 * world.land.lot_price
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_boom_and_the_panic() {
        let mut w = World::new(2);
        let at = |days: u32, w: &mut World| {
            w.run_days(days);
            w.land.lot_price
        };
        let start = w.land.lot_price;
        let spring57 = at(640, &mut w); // August 1857
        let winter57 = at(120, &mut w); // December 1857
        assert!(spring57 > start * 2.0, "boom: {start} -> {spring57}");
        assert!(winter57 < spring57 * 0.7, "panic: {spring57} -> {winter57}");
    }

    #[test]
    fn a_broken_family_sells_and_goes_east() {
        let mut w = World::new(3);
        let f = 2;
        w.families[f].stores.hungry_days = 5;
        w.families[0].stores.cash = 500;
        let head = w.head_of(f as FamilyId).unwrap();
        w.set_opinion(head, PLAYER, 10);
        let acres = w.families[0].stores.acres + w.families[f].stores.acres;
        assert!(buy_claim(&mut w, f as FamilyId));
        w.run_cascades();
        assert_eq!(w.families[0].stores.acres, acres);
        assert!(w.head_of(f as FamilyId).is_none(), "they left");
    }

    #[test]
    fn lots_round_trip() {
        let mut w = World::new(2);
        w.families[0].stores.cash = 100;
        assert!(buy_lot(&mut w));
        assert!(sell_lot(&mut w));
        assert!(!sell_lot(&mut w));
        assert!(w.families[0].stores.cash <= 100);
    }
}
