//! The county market: Dunmore's store, the Westport freight wagons, and
//! everyone's buying and selling.
//!
//! Prices come from stock against a target level, so anyone who buys enough
//! moves the price. The player can corner corn in a hungry winter. The county
//! will notice, and the hungry will come for the granary.

use super::calendar::Season;
use super::character::{self, Archetype};
use super::events::EventKind;
use super::world::{Faction, FamilyId, NpcId, PLAYER, World};

pub const GOODS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Good {
    /// Bushels. Food: 12 days a bushel.
    Corn,
    /// Bushels held for planting. Scarce and dear in April.
    SeedCorn,
    /// Head. Breeding stock.
    Cattle,
    /// Sacks. Without it, butchered meat half spoils.
    Salt,
    /// Pounds of powder and lead. Every hunt and every ambush uses some.
    Powder,
    /// Wagon loads. A burned barn needs four to rebuild.
    Timber,
    /// Gallons. Anger up, fear down, and you're seen at the tavern.
    Whiskey,
    /// Hides from hunting and butchering. Cash crop of the poor.
    Hides,
}

impl Good {
    pub const ALL: [Good; GOODS] = [
        Good::Corn,
        Good::SeedCorn,
        Good::Cattle,
        Good::Salt,
        Good::Powder,
        Good::Timber,
        Good::Whiskey,
        Good::Hides,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            Good::Corn => "corn",
            Good::SeedCorn => "seed corn",
            Good::Cattle => "cattle",
            Good::Salt => "salt",
            Good::Powder => "powder & lead",
            Good::Timber => "timber",
            Good::Whiskey => "whiskey",
            Good::Hides => "hides",
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Good::Corn | Good::SeedCorn => "bushel",
            Good::Cattle => "head",
            Good::Salt => "sack",
            Good::Powder => "lb",
            Good::Timber => "load",
            Good::Whiskey => "gallon",
            Good::Hides => "hide",
        }
    }

    /// 1855 dollars, roughly what things cost at a Kansas store.
    fn base_price(self) -> f32 {
        match self {
            Good::Corn => 0.60,
            Good::SeedCorn => 1.00,
            Good::Cattle => 16.0,
            Good::Salt => 1.50,
            Good::Powder => 0.50,
            Good::Timber => 3.00,
            Good::Whiskey => 0.40,
            Good::Hides => 1.50,
        }
    }

    /// Stock the store tries to keep on hand.
    fn target_stock(self) -> f32 {
        match self {
            Good::Corn => 400.0,
            Good::SeedCorn => 80.0,
            Good::Cattle => 8.0,
            Good::Salt => 50.0,
            Good::Powder => 60.0,
            Good::Timber => 20.0,
            Good::Whiskey => 40.0,
            Good::Hides => 40.0,
        }
    }

    /// What the Westport wagons bring each week, before any blockade.
    fn weekly_freight(self) -> f32 {
        match self {
            Good::Corn => 60.0,
            Good::SeedCorn => 6.0,
            Good::Cattle => 0.5,
            Good::Salt => 7.0,
            Good::Powder => 8.0,
            Good::Timber => 3.0,
            Good::Whiskey => 8.0,
            // The store ships hides east rather than receiving them.
            Good::Hides => -6.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GoodState {
    pub stock: f32,
    pub price: f32,
    /// Last price band announced, so the chronicle only hears about big moves.
    pub band: i8,
}

#[derive(Clone, Debug)]
pub struct Market {
    pub goods: [GoodState; GOODS],
    /// Freight is cut: river closed, roads watched.
    pub blockade: bool,
    /// Multiplier on freight from history (a blockade year, a bumper year back east).
    pub freight_factor: f32,
    /// Weekly price snapshots, for charts and the headless report.
    pub history: Vec<[f32; GOODS]>,
    /// What the player has bought and sold this season, in dollars.
    pub player_bought: f32,
    /// The county's other farms: weekly corn surplus they bring in each fall.
    pub county_harvest: f32,
    /// 0..1: how safe the Santa Fe and Platte roads are for freighters.
    pub trail_safety: f32,
}

impl Market {
    pub fn new() -> Self {
        let goods = Good::ALL.map(|g| GoodState {
            stock: g.target_stock(),
            price: g.base_price(),
            band: 0,
        });
        Self {
            goods,
            blockade: false,
            freight_factor: 1.0,
            history: Vec::new(),
            player_bought: 0.0,
            county_harvest: 70.0,
            trail_safety: 1.0,
        }
    }

    pub fn price(&self, g: Good) -> f32 {
        self.goods[g.index()].price
    }

    pub fn stock(&self, g: Good) -> f32 {
        self.goods[g.index()].stock
    }

    /// Price as a multiple of normal. 1.0 is an ordinary week.
    pub fn pressure(&self, g: Good) -> f32 {
        self.price(g) / g.base_price()
    }

    fn reprice(&mut self, g: Good) {
        let s = &mut self.goods[g.index()];
        let target = g.target_stock();
        let scarcity = target / s.stock.max(target * 0.04);
        let fair = g.base_price() * scarcity.powf(0.8);
        let fair = fair.clamp(g.base_price() * 0.3, g.base_price() * 10.0);
        // Prices move toward fair, not all at once.
        s.price += (fair - s.price) * 0.35;
    }
}

impl Default for Market {
    fn default() -> Self {
        Self::new()
    }
}

/// What a buyer pays per unit: Dunmore marks up for Free-State customers
/// once the county's gone bad (Pro-Slavery grievance high).
pub fn ask_price(world: &World, buyer: NpcId, g: Good) -> f32 {
    let base = world.market.price(g);
    let free_state = world.npc(buyer).faction == Faction::FreeState;
    if free_state && world.grievance[Faction::ProSlavery.index()] >= 40 {
        base * 1.25
    } else {
        base
    }
}

/// What the store pays you: a trader's spread.
pub fn bid_price(world: &World, g: Good) -> f32 {
    world.market.price(g) * 0.7
}

/// Buy up to `qty` units with cash. Returns units bought.
pub fn buy(world: &mut World, family: FamilyId, g: Good, qty: f32) -> f32 {
    let Some(buyer) = world.head_of(family) else {
        return 0.0;
    };
    let price = ask_price(world, buyer, g);
    let cash = world.families[family as usize].stores.cash as f32;
    let affordable = (cash / price).floor();
    let units = qty.min(affordable).min(world.market.stock(g).floor());
    if units <= 0.0 {
        return 0.0;
    }
    let cost = (units * price).ceil() as i32;
    world.families[family as usize].stores.cash -= cost;
    world.market.goods[g.index()].stock -= units;
    receive(world, family, g, units);
    if family == 0 {
        world.market.player_bought += cost as f32;
    }
    world.market.reprice(g);
    units
}

/// Buy on the store's credit. Returns units bought.
pub fn buy_on_credit(world: &mut World, family: FamilyId, g: Good, dollars: i32) -> f32 {
    let Some(buyer) = world.head_of(family) else {
        return 0.0;
    };
    let price = ask_price(world, buyer, g);
    let units = (dollars as f32 / price)
        .floor()
        .min(world.market.stock(g).floor());
    if units <= 0.0 {
        return 0.0;
    }
    world.market.goods[g.index()].stock -= units;
    receive(world, family, g, units);
    world.market.reprice(g);
    units
}

/// Sell up to `qty` units to the store. Returns dollars received.
pub fn sell(world: &mut World, family: FamilyId, g: Good, qty: f32) -> i32 {
    let held = holding(world, family, g);
    let units = qty.min(held).floor();
    if units <= 0.0 {
        return 0;
    }
    let dollars = (units * bid_price(world, g)).floor() as i32;
    take(world, family, g, units);
    world.market.goods[g.index()].stock += units;
    world.families[family as usize].stores.cash += dollars;
    world.market.reprice(g);
    dollars
}

/// How much of a good a household holds, in market units.
pub fn holding(world: &World, family: FamilyId, g: Good) -> f32 {
    let hh = &world.families[family as usize].stores;
    match g {
        Good::Corn => hh.food / super::economy::SEED_FOOD,
        Good::SeedCorn => hh.seed as f32,
        Good::Cattle => hh.cattle as f32,
        _ => hh.goods[g.index()],
    }
}

fn receive(world: &mut World, family: FamilyId, g: Good, units: f32) {
    let hh = &mut world.families[family as usize].stores;
    match g {
        Good::Corn => hh.food += units * super::economy::SEED_FOOD,
        Good::SeedCorn => hh.seed += units as u32,
        Good::Cattle => hh.cattle += units as u32,
        _ => hh.goods[g.index()] += units,
    }
}

fn take(world: &mut World, family: FamilyId, g: Good, units: f32) {
    let hh = &mut world.families[family as usize].stores;
    match g {
        Good::Corn => hh.food = (hh.food - units * super::economy::SEED_FOOD).max(0.0),
        Good::SeedCorn => hh.seed = hh.seed.saturating_sub(units as u32),
        Good::Cattle => hh.cattle = hh.cattle.saturating_sub(units as u32),
        _ => hh.goods[g.index()] = (hh.goods[g.index()] - units).max(0.0),
    }
}

/// Use up one unit of a household good. Returns whether they had it.
pub fn consume(world: &mut World, family: FamilyId, g: Good) -> bool {
    let hh = &mut world.families[family as usize].stores;
    let slot = &mut hh.goods[g.index()];
    if *slot >= 1.0 {
        *slot -= 1.0;
        true
    } else {
        false
    }
}

/// Daily market life: freight, household trading, speculation, prices.
pub fn daily(world: &mut World) {
    let today = world.day;
    let (_, month, dom) = today.date();

    // Blockade: the river closes when the county's at war with itself.
    let tension = world.grievance[0] + world.grievance[1];
    world.market.blockade = tension >= 200 || world.market.freight_factor < 0.5;

    // Monday: the Westport wagons, and the rest of the county's week.
    if today.0.is_multiple_of(7) {
        weekly(world);
    }

    for f in 0..world.families.len() {
        if world.families[f].store {
            continue;
        }
        let family = f as FamilyId;
        let Some(head) = world.head_of(family) else {
            continue;
        };
        if family == 0 && !world.autopilot_player {
            continue;
        }
        routine_trade(world, family, head, month, dom);
    }

    for g in Good::ALL {
        world.market.reprice(g);
        announce(world, g);
    }
    let _ = PLAYER;
}

/// The county beyond our nine farms: Lawrence, emigrant trains, freighters,
/// the army. Their buying and the wagons' arrivals keep the market alive.
fn weekly(world: &mut World) {
    let season = world.day.season();
    let month = world.day.month();
    // Winter roads are mud and ice; the river runs spring to fall.
    let roads = match season {
        Season::Winter => 0.35,
        Season::Spring => 1.0,
        Season::Summer => 1.2,
        Season::Autumn => 1.0,
    };
    let tension = (world.grievance[0] + world.grievance[1]) as f32;
    let blockade = if world.market.blockade { 0.25 } else { 1.0 };
    let freight = world.market.freight_factor * roads * blockade;
    let emigrants = if (4..=6).contains(&month) { 1.0 } else { 0.0 };
    let glut = if (9..=10).contains(&month) {
        world.market.county_harvest
    } else {
        0.0
    };

    for g in Good::ALL {
        let noise = 0.6 + world.rng.unit() * 0.8;
        let demand_base = match g {
            Good::Corn => {
                40.0 * (1.0 + 0.4 * emigrants) + if season == Season::Winter { 20.0 } else { 0.0 }
            }
            Good::SeedCorn => {
                if (3..=4).contains(&month) {
                    18.0
                } else {
                    0.5
                }
            }
            Good::Cattle => 0.3 + 0.8 * emigrants,
            Good::Salt => {
                if (11..=12).contains(&month) {
                    7.0
                } else {
                    2.5
                }
            }
            // An arms race: powder sells as the county turns on itself.
            Good::Powder => 5.0 + tension / 12.0,
            Good::Timber => 2.5 + 3.0 * emigrants,
            Good::Whiskey => 7.0 + tension / 60.0,
            // Eastern buyers take hides off the store's hands.
            Good::Hides => 6.0,
        };
        let supply_base = match g {
            Good::Corn => glut,
            // Hunters and Santa Fe freighters bring hides in.
            Good::Hides => 6.0 * world.market.trail_safety,
            _ => 0.0,
        };
        let s = &mut world.market.goods[g.index()];
        s.stock = (s.stock - demand_base * noise).max(0.0);
        s.stock += supply_base * (0.6 + 0.8 * world.rng.unit());
        // The store orders what it's short, as far as the wagons can carry.
        if g != Good::Hides {
            let capacity = g.weekly_freight().max(0.0) * 1.6 * freight * noise;
            let short = (g.target_stock() - s.stock).max(0.0);
            s.stock += short.min(capacity);
        }
    }
    let snapshot = Good::ALL.map(|g| world.market.price(g));
    world.market.history.push(snapshot);
}

/// What an ordinary household buys and sells in an ordinary week.
fn routine_trade(world: &mut World, family: FamilyId, head: NpcId, month: u32, dom: u32) {
    let f = family as usize;
    let people = world.living().filter(|n| n.family == family).count() as f32;

    // Autumn: lay in salt for butchering, powder for the winter hunt.
    if month == 11 && dom == 5 {
        if world.families[f].stores.goods[Good::Salt.index()] < 2.0 {
            buy(world, family, Good::Salt, 2.0);
        }
        if world.families[f].stores.goods[Good::Powder.index()] < 3.0 {
            buy(world, family, Good::Powder, 3.0);
        }
    }

    // Early April: buy seed if the winter ate it.
    if month == 4 && (1..=10).contains(&dom) {
        let hh = &world.families[f].stores;
        let short = hh.acres.saturating_sub(hh.seed) as f32;
        if short > 0.0 {
            buy(world, family, Good::SeedCorn, short);
        }
    }

    // Burned barn: buy timber to rebuild.
    if !world.families[f].barn_standing
        && world.families[f].stores.goods[Good::Timber.index()] < 4.0
    {
        buy(world, family, Good::Timber, 4.0);
    }

    // Surplus corn after harvest goes to the store.
    let reserve = people * 280.0;
    if world.families[f].stores.food > reserve * 1.2 && dom.is_multiple_of(7) {
        let extra = (world.families[f].stores.food - reserve) / super::economy::SEED_FOOD;
        sell(world, family, Good::Corn, extra * 0.5);
    }

    // Hides are for selling.
    if world.families[f].stores.goods[Good::Hides.index()] >= 2.0 && dom % 7 == 1 {
        sell(world, family, Good::Hides, 99.0);
    }

    // Saturday night. The quick-tempered drink; drink makes them bolder and angrier.
    if world.day.0 % 7 == 6 {
        let drinkers: Vec<NpcId> = world
            .living()
            .filter(|n| n.family == family && n.age >= 16 && n.temperament.temper > 0.6)
            .map(|n| n.id)
            .collect();
        for d in drinkers {
            if buy(world, family, Good::Whiskey, 1.0) > 0.0 {
                consume(world, family, Good::Whiskey);
                let day = world.day;
                let n = world.npc_mut(d);
                n.emotions.anger = (n.emotions.anger + 6.0).min(100.0);
                n.emotions.fear = (n.emotions.fear - 10.0).max(0.0);
                n.alibi = Some(day);
            }
        }
    }

    // Speculators buy cheap and sell dear. Skinflints are speculators.
    if character::is(world.npc(head), Archetype::Skinflint) && dom % 7 == 2 {
        for g in [Good::Corn, Good::Salt, Good::Powder] {
            let pressure = world.market.pressure(g);
            if pressure < 0.8 {
                buy(world, family, g, 20.0);
            } else if pressure > 1.6 {
                sell(world, family, g, 20.0);
            }
        }
    }
}

/// Big price moves make the news: "Corn is $1.40 a bushel at Dunmore's."
fn announce(world: &mut World, g: Good) {
    let pressure = world.market.pressure(g);
    let band = if pressure >= 3.0 {
        3
    } else if pressure >= 2.0 {
        2
    } else if pressure >= 1.5 {
        1
    } else if pressure <= 0.6 {
        -1
    } else {
        0
    };
    let s = &mut world.market.goods[g.index()];
    if band != s.band {
        let rising = band > s.band;
        s.band = band;
        if band != 0 || !rising {
            let cents = (s.price * 100.0).round() as u32;
            world.emit_root(
                EventKind::PriceMove {
                    good: g,
                    cents,
                    rising,
                },
                None,
            );
        }
    }
}

/// Someone couldn't buy corn because there was none. If a household is
/// sitting on a mountain of it, the county knows who.
pub fn resent_hoarders(world: &mut World, hungry_family: FamilyId) {
    let bare = world.market.stock(Good::Corn) <= 20.0;
    let dear = world.market.pressure(Good::Corn) >= 1.8;
    if !bare && !dear {
        return;
    }
    let Some(hungry) = world.head_of(hungry_family) else {
        return;
    };
    let hoarder = world
        .families
        .iter()
        .filter(|f| !f.store && f.id != hungry_family)
        .max_by(|a, b| a.stores.food.total_cmp(&b.stores.food))
        .map(|f| f.id);
    let Some(hf) = hoarder else {
        return;
    };
    let people = world.living().filter(|n| n.family == hf).count().max(1) as f32;
    if world.families[hf as usize].stores.food < people * 400.0 {
        return;
    }
    if let Some(h) = world.head_of(hf) {
        world.adjust_opinion(hungry, h, -8);
    }
    let _ = Season::Winter;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buying_out_stock_raises_the_price() {
        let mut w = World::new(1);
        w.families[0].stores.cash = 100_000;
        let before = w.market.price(Good::Salt);
        buy(&mut w, 0, Good::Salt, 1_000.0);
        for _ in 0..10 {
            w.market.reprice(Good::Salt);
        }
        assert!(w.market.price(Good::Salt) > before * 3.0);
    }

    #[test]
    fn buy_spends_cash_and_delivers_goods() {
        let mut w = World::new(1);
        w.families[0].stores.cash = 100;
        let got = buy(&mut w, 0, Good::Powder, 10.0);
        assert_eq!(got, 10.0);
        assert!(w.families[0].stores.cash < 100);
        assert_eq!(holding(&w, 0, Good::Powder), 10.0);
    }

    #[test]
    fn you_cannot_buy_without_cash_or_sell_what_you_lack() {
        let mut w = World::new(1);
        w.families[0].stores.cash = 0;
        assert_eq!(buy(&mut w, 0, Good::Corn, 10.0), 0.0);
        w.families[0].stores.goods[Good::Whiskey.index()] = 0.0;
        assert_eq!(sell(&mut w, 0, Good::Whiskey, 10.0), 0);
    }

    #[test]
    fn dunmore_marks_up_for_free_state_buyers_when_things_are_bad() {
        let mut w = World::new(1);
        let calm = ask_price(&w, 0, Good::Corn);
        w.grievance[Faction::ProSlavery.index()] = 80;
        assert!(ask_price(&w, 0, Good::Corn) > calm);
    }

    #[test]
    fn the_store_buys_below_what_it_sells() {
        let w = World::new(1);
        assert!(bid_price(&w, Good::Corn) < ask_price(&w, 0, Good::Corn));
    }
}
