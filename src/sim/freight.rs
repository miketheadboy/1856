//! Logistics: the wagon to the river (§10).
//!
//! Everything Dunmore sells came up from the Missouri by ox team, and cost
//! the freight. A family with a yoke of oxen can go get it themselves, and
//! carry for the neighbors while they're at it:
//! - **Westport**: three days each way by ox wagon, cheap goods off the
//!   levee. In the blockade summer of 1856, Missourians on the road
//!   stopped Free-State wagons and took the load (`WagonStopped`).
//! - **The Lane Trail**: through Iowa and Nebraska, the road the Free-State
//!   emigrants cut when the Missouri was closed to them. It takes weeks,
//!   costs more, and gets through.
//!
//! On the road a man is out of the county: that is an alibi no neighbor can
//! argue with (`attribution`), and he isn't home to be shot (`systems`).
//! Neighbors of his side send orders along with cash. A dishonest teamster
//! skims them, and the short weight is noticed (`ShortWeight`).
//!
//! A wagon hauls about a ton. The oxen set the capacity; the budget sets
//! the rest.

use super::calendar::Day;
use super::events::{EventKind, Source, Suspect, WorldEvent};
use super::market::{self, Good};
use super::world::{Faction, FamilyId, NpcId, PLAYER, World};

/// Below this `standing::word`, neighbors don't send money along.
pub const TRUSTED_WORD: f32 = 0.6;

/// Pounds a wagon and one yoke can haul over prairie roads.
pub const WAGON_POUNDS: f32 = 2000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Westport,
    LaneTrail,
}

impl Route {
    pub const ALL: [Route; 2] = [Route::Westport, Route::LaneTrail];

    pub fn label(self) -> &'static str {
        match self {
            Route::Westport => "Westport",
            Route::LaneTrail => "the Lane Trail",
        }
    }

    /// Days there and back, with a day in town.
    pub fn days(self) -> u32 {
        match self {
            Route::Westport => 7,
            Route::LaneTrail => 30,
        }
    }

    /// Off the levee, before the store's markup and the freight up the
    /// road; the northern road's goods came overland from Iowa.
    fn price_factor(self) -> f32 {
        match self {
            Route::Westport => 0.65,
            Route::LaneTrail => 0.85,
        }
    }
}

/// Pounds to a market unit, for loading the wagon. Cattle walk; timber
/// is cut in the county.
pub fn pounds(g: Good) -> Option<f32> {
    match g {
        Good::Corn | Good::SeedCorn => Some(56.0),
        Good::Salt => Some(50.0),
        Good::Whiskey => Some(9.0),
        Good::Hides => Some(10.0),
        Good::Powder | Good::Coffee | Good::Sugar | Good::Bacon | Good::Iron => Some(1.0),
        Good::Cloth => Some(0.5),
        Good::Cattle | Good::Timber => None,
    }
}

/// What a house asks the wagon to bring: the year's store goods.
const BASKET: [(Good, f32); 7] = [
    (Good::Coffee, 15.0),
    (Good::Sugar, 15.0),
    (Good::Bacon, 60.0),
    (Good::Cloth, 10.0),
    (Good::Iron, 15.0),
    (Good::Salt, 2.0),
    (Good::Powder, 5.0),
];

#[derive(Clone, Debug)]
pub struct Trip {
    pub teamster: NpcId,
    pub family: FamilyId,
    pub route: Route,
    pub left: Day,
    pub back: Day,
    /// (for whom, what, how many units), bought and paid for at the river.
    pub load: Vec<(FamilyId, Good, f32)>,
}

#[derive(Clone, Debug, Default)]
pub struct Freight {
    pub trips: Vec<Trip>,
    /// (teamster, day): a neighbor said his order came back light. The
    /// county remembers it a year (`standing`: "gave short weight").
    pub shorted: Vec<(NpcId, Day)>,
}

/// How long a short-weight name sticks.
pub const SHORT_MEMORY: u32 = 365;

/// Whether the county still says this man gives short weight.
pub fn short_weight(world: &World, id: NpcId) -> bool {
    world
        .freight
        .shorted
        .iter()
        .any(|&(t, d)| t == id && world.day.0 < d.0 + SHORT_MEMORY)
}

/// On the road (and so not in the county).
pub fn away(world: &World, id: NpcId) -> bool {
    world.freight.trips.iter().any(|t| t.teamster == id)
}

/// Whether the Missouri road is shut to this man.
pub fn road_closed(world: &World, teamster: NpcId, route: Route) -> bool {
    route == Route::Westport
        && world.market.blockade
        && world.npc(teamster).faction == Faction::FreeState
}

/// The chance the Missourians stop this wagon on the way home.
pub fn stop_odds(world: &World, teamster: NpcId, route: Route) -> f32 {
    match route {
        Route::LaneTrail => 0.0,
        Route::Westport if road_closed(world, teamster, route) => 0.5,
        // Even in a quiet year a Free-State wagon got looked over.
        Route::Westport if world.npc(teamster).faction == Faction::FreeState => {
            0.02 + (world.grievance[1] as f32 / 2000.0).clamp(0.0, 0.1)
        }
        Route::Westport => 0.0,
    }
}

/// River price for a unit of a good.
pub fn river_price(g: Good, route: Route) -> f32 {
    market::base_price(g) * route.price_factor()
}

/// Buy a basket at the river with `budget` dollars into `room` pounds.
/// Returns what was bought, what it cost, and the pounds it takes.
fn fill(route: Route, budget: i32, room: f32) -> (Vec<(Good, f32)>, i32, f32) {
    let mut bought = Vec::new();
    let (mut spent, mut used) = (0.0f32, 0.0f32);
    for (g, want) in BASKET {
        let price = river_price(g, route);
        let lb = pounds(g).unwrap_or(1.0);
        let by_cash = ((budget as f32 - spent) / price).floor();
        let by_room = ((room - used) / lb).floor();
        let units = want.min(by_cash).min(by_room);
        if units >= 1.0 {
            bought.push((g, units));
            spent += units * price;
            used += units * lb;
        }
    }
    (bought, spent.ceil() as i32, used)
}

/// Daily chance, times conviction, that a house hiding a freedom seeker
/// takes them north itself.
pub const NORTHBOUND: f32 = 0.3;

/// Dollars per hundredweight a neighbor pays for the haul (Westport to
/// Lawrence ran a dollar to a dollar and a half the hundred in 1856).
pub const HAUL_FEE: i32 = 1;

/// Neighbors who'd send money with this teamster: his side, with cash,
/// who think well of him, if his word is good.
fn customers(world: &World, family: FamilyId, teamster: NpcId) -> Vec<FamilyId> {
    if super::standing::word(world, teamster) < TRUSTED_WORD {
        return Vec::new();
    }
    let side = world.npc(teamster).faction;
    world
        .families
        .iter()
        .filter(|f| f.id != family && !f.store && f.faction == side)
        .filter(|f| f.stores.cash >= 10)
        .filter_map(|f| world.head_of(f.id).map(|h| (f.id, h)))
        .filter(|&(_, h)| world.opinion(h, teamster) >= 15)
        .map(|(f, _)| f)
        .collect()
}

fn orders_waiting(world: &World, family: FamilyId) -> bool {
    world
        .head_of(family)
        .is_some_and(|t| !customers(world, family, t).is_empty())
}

/// Hitch up and go. The house spends up to `budget` of its cash; neighbors
/// of its side who think well of the teamster send orders. Returns whether
/// the wagon left.
pub fn set_out(world: &mut World, family: FamilyId, route: Route, budget: i32) -> bool {
    let Some(teamster) = world.head_of(family) else {
        return false;
    };
    let hh = &world.families[family as usize].stores;
    if hh.oxen < 2
        || away(world, teamster)
        || super::warrant::held(world, teamster)
        || world.freight.trips.iter().any(|t| t.family == family)
    {
        return false;
    }
    let mut room = WAGON_POUNDS;
    let mut load = Vec::new();
    // Hides go down to the river and sell better there than at Dunmore's.
    let hh = &mut world.families[family as usize].stores;
    let hides = hh.goods[Good::Hides.index()].floor();
    hh.goods[Good::Hides.index()] -= hides;
    let hide_money = (hides * market::base_price(Good::Hides) * 1.1) as i32;
    hh.cash += hide_money;
    let budget = (budget + hide_money).min(hh.cash).max(0);
    let (mine, spent, used) = fill(route, budget, room);
    world.families[family as usize].stores.cash -= spent;
    room -= used;
    load.extend(mine.into_iter().map(|(g, u)| (family, g, u)));
    // "Going to the river: need anything?"
    // Cash goes with a man whose word is good: a jailbird or a short-weight
    // man carries his own order and nobody else's (`standing`).
    let others = customers(world, family, teamster);
    let mut orders = 0u8;
    for f in others {
        if room < 50.0 {
            break;
        }
        let send = (world.families[f as usize].stores.cash / 2).min(15);
        let (theirs, spent, used) = fill(route, send, room);
        if theirs.is_empty() {
            continue;
        }
        // A dollar a hundredweight for the haul, to the teamster's house.
        let fee = ((used / 100.0).ceil() as i32 * HAUL_FEE)
            .min(world.families[f as usize].stores.cash - spent);
        world.families[f as usize].stores.cash -= spent + fee.max(0);
        world.families[family as usize].stores.cash += fee.max(0);
        room -= used;
        orders += 1;
        load.extend(theirs.into_iter().map(|(g, u)| (f, g, u)));
    }
    let left = world.day;
    world.freight.trips.push(Trip {
        teamster,
        family,
        route,
        left,
        back: Day(left.0 + route.days()),
        load,
    });
    if teamster == PLAYER {
        super::family::leave_for(world, super::family::Errand::Freighting, route.days());
    }
    world.emit_root(
        EventKind::WagonOut {
            teamster,
            route_west: route == Route::Westport,
            orders,
        },
        None,
    );
    true
}

pub fn daily(world: &mut World) {
    let today = world.day;
    // On the road: out of the county, every day of it.
    for t in &world.freight.trips {
        world.npcs[t.teamster as usize].alibi = Some(today);
    }
    let due: Vec<usize> = world
        .freight
        .trips
        .iter()
        .enumerate()
        .filter(|(_, t)| t.back <= today)
        .map(|(i, _)| i)
        .collect();
    for i in due.into_iter().rev() {
        let trip = world.freight.trips.remove(i);
        come_home(world, trip);
    }
    // A house hiding someone, with a yoke and the conviction, doesn't wait
    // for the season: it hitches up for Iowa (Dr. Doy's wagon left Lawrence
    // in January 1859). The load is the excuse.
    let northbound: Vec<FamilyId> = world
        .railroad
        .seekers
        .iter()
        .filter_map(|s| match s.status {
            super::railroad::Status::Hidden(f) => Some(f),
            _ => None,
        })
        .collect();
    for family in northbound {
        if family == 0 && !world.autopilot_player {
            continue;
        }
        let fam = &world.families[family as usize];
        let Some(head) = world.head_of(family) else {
            continue;
        };
        if fam.stores.oxen < 2 || fam.faction != Faction::FreeState {
            continue;
        }
        let zeal = world.npc(head).ideology.private;
        if zeal > 0.3 && world.rng.chance(NORTHBOUND * zeal) {
            let budget = world.families[family as usize].stores.cash / 2;
            set_out(world, family, Route::LaneTrail, budget);
        }
    }
    // April to October, when the roads are fit and the boats are running:
    // households with a yoke and a reason go down to the river.
    let month = today.month();
    if !(4..=10).contains(&month) {
        return;
    }
    for f in 0..world.families.len() {
        let family = f as FamilyId;
        let hh = &world.families[f].stores;
        if hh.oxen < 2 || world.families[f].store {
            continue;
        }
        // Money in the jar, hides to sell, or neighbors who'll pay the
        // haul: any of them puts a wagon on the road.
        let hides = hh.goods[Good::Hides.index()] >= 3.0;
        if hh.cash < 15 && !hides && !orders_waiting(world, family) {
            continue;
        }
        if family == 0 && !world.autopilot_player {
            continue;
        }
        let Some(head) = world.head_of(family) else {
            continue;
        };
        if !world.rng.chance(0.02) {
            continue;
        }
        // A Free-State man with sense takes the long road in a closed year;
        // a careless or a hungry one chances Westport.
        let careful = world.families[f].stores.prudence;
        let route =
            if road_closed(world, head, Route::Westport) && world.rng.chance(0.3 + 0.6 * careful) {
                Route::LaneTrail
            } else {
                Route::Westport
            };
        let budget = world.families[f].stores.cash / 2;
        set_out(world, family, route, budget);
    }
}

fn come_home(world: &mut World, trip: Trip) {
    let Trip {
        teamster,
        family,
        route,
        load,
        ..
    } = trip;
    let pounds_of = |l: &[(FamilyId, Good, f32)]| {
        l.iter()
            .map(|&(_, g, u)| u * pounds(g).unwrap_or(1.0))
            .sum::<f32>()
    };
    if world.rng.chance(stop_odds(world, teamster, route)) {
        let lost: f32 = load
            .iter()
            .map(|&(_, g, u)| u * river_price(g, route))
            .sum();
        world.emit_root(
            EventKind::WagonStopped {
                teamster,
                seized: lost.round() as u16,
            },
            None,
        );
        return;
    }
    // A light-fingered teamster keeps a fifth of the neighbors' goods.
    let skims = world.npc(teamster).temperament.honesty < 0.25;
    let mut shorted: Vec<FamilyId> = Vec::new();
    let tons = pounds_of(&load) / 2000.0;
    for (to, g, units) in load {
        let mut got = units;
        if skims && to != family {
            let kept = (units * 0.2).floor();
            got -= kept;
            market::receive(world, family, g, kept);
            if kept > 0.0 && !shorted.contains(&to) {
                shorted.push(to);
            }
        }
        market::receive(world, to, g, got);
    }
    for to in shorted {
        if let Some(h) = world.head_of(to)
            && world.rng.chance(0.5)
        {
            world.emit_root(
                EventKind::ShortWeight {
                    teamster,
                    noticed_by: h,
                },
                None,
            );
        }
    }
    world.emit_root(
        EventKind::WagonBack {
            teamster,
            route_west: route == Route::Westport,
            tenths_of_a_ton: (tons * 10.0).round() as u8,
        },
        None,
    );
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::WagonStopped { teamster, .. } => {
            let n = world.npc_mut(teamster);
            n.emotions.fear += 25.0;
            n.emotions.anger += 30.0;
            world.add_grievance(Faction::FreeState, 3);
            // Strangers in the road, masked or near enough. But one of them
            // sat his horse like that Missouri neighbor he never could
            // abide: he'd swear to it. The paper will print who he swears to.
            if let Some(x) = rode_with_them(world, teamster) {
                let anger = world.npc(teamster).emotions.anger;
                world.emit_child(
                    ev,
                    EventKind::Belief {
                        holder: teamster,
                        about: ev.id,
                        blamed: Suspect::Person(x),
                        confidence: (40.0 + anger * 0.3).min(80.0) as u8,
                        source: Source::Victim,
                        reason: RODE_WITH_THEM,
                    },
                );
            }
        }
        EventKind::WagonBack { teamster, .. } => {
            // The neighbors' goods came through: that's a favor owed.
            let side = world.npc(teamster).faction;
            let glad: Vec<NpcId> = world
                .living()
                .filter(|n| n.faction == side && n.id != teamster)
                .map(|n| n.id)
                .collect();
            for g in glad {
                if world.opinion(g, teamster) >= 15 {
                    world.adjust_opinion(g, teamster, 2);
                }
            }
        }
        EventKind::ShortWeight {
            teamster,
            noticed_by,
        } => {
            world.adjust_opinion(noticed_by, teamster, -15);
            world.freight.shorted.retain(|&(t, _)| t != teamster);
            world.freight.shorted.push((teamster, ev.day));
        }
        _ => {}
    }
}

/// Why a stopped teamster blames a neighbor for the Missourians on the road.
pub const RODE_WITH_THEM: &str = "he was riding with them, I'd swear it";

/// The man of the other side the teamster can least abide, if he can't
/// abide any: grown, living, in the county.
fn rode_with_them(world: &World, teamster: NpcId) -> Option<NpcId> {
    let side = world.npc(teamster).faction;
    world
        .living()
        .filter(|n| n.faction != side && n.family != 0 && !n.departed)
        .filter(|n| n.age >= 16 && !super::world::is_woman(&n.name))
        .filter(|n| !away(world, n.id))
        .map(|n| (world.opinion(teamster, n.id), n.id))
        .filter(|&(o, _)| o < 0)
        .min()
        .map(|(_, id)| id)
}

/// Wagons on the road, for the lab and `debug`.
pub fn describe(world: &World) -> Vec<String> {
    world
        .freight
        .trips
        .iter()
        .map(|t| {
            format!(
                "{} to {} ({} orders), back {}",
                world.name(t.teamster),
                t.route.label(),
                t.load
                    .iter()
                    .map(|l| l.0)
                    .filter(|&f| f != t.family)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                t.back
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_house(w: &World, side: Faction) -> FamilyId {
        (1..w.families.len() as FamilyId)
            .find(|&f| {
                let fam = &w.families[f as usize];
                fam.faction == side && fam.farms() && w.head_of(f).is_some()
            })
            .unwrap()
    }

    #[test]
    fn a_wagon_brings_the_river_cheaper_than_the_store() {
        let mut w = World::new(5);
        let f = a_house(&w, Faction::ProSlavery);
        w.families[f as usize].stores.oxen = 2;
        w.families[f as usize].stores.cash = 40;
        let coffee = w.families[f as usize].stores.goods[Good::Coffee.index()];
        assert!(set_out(&mut w, f, Route::Westport, 40));
        let head = w.head_of(f).unwrap();
        assert!(away(&w, head));
        w.run_days(Route::Westport.days() + 1);
        assert!(!away(&w, head));
        let got = w.families[f as usize].stores.goods[Good::Coffee.index()] - coffee;
        assert!(got >= 14.0, "{got} lb of coffee");
        assert!(river_price(Good::Coffee, Route::Westport) < market::base_price(Good::Coffee));
    }

    #[test]
    fn no_oxen_no_wagon() {
        let mut w = World::new(5);
        let f = a_house(&w, Faction::FreeState);
        w.families[f as usize].stores.oxen = 1;
        w.families[f as usize].stores.cash = 40;
        assert!(!set_out(&mut w, f, Route::Westport, 40));
    }

    #[test]
    fn the_missouri_road_is_shut_to_free_state_wagons_in_a_blockade() {
        let mut w = World::new(5);
        let fs = a_house(&w, Faction::FreeState);
        let ps = a_house(&w, Faction::ProSlavery);
        let (a, b) = (w.head_of(fs).unwrap(), w.head_of(ps).unwrap());
        w.market.blockade = true;
        assert!(stop_odds(&w, a, Route::Westport) >= 0.5);
        assert_eq!(stop_odds(&w, b, Route::Westport), 0.0);
        assert_eq!(stop_odds(&w, a, Route::LaneTrail), 0.0);
        assert!(Route::LaneTrail.days() > 3 * Route::Westport.days());
    }

    #[test]
    fn a_man_on_the_road_has_an_alibi() {
        let mut w = World::new(5);
        let f = a_house(&w, Faction::ProSlavery);
        w.families[f as usize].stores.oxen = 2;
        w.families[f as usize].stores.cash = 30;
        assert!(set_out(&mut w, f, Route::Westport, 30));
        let head = w.head_of(f).unwrap();
        w.advance_day();
        assert_eq!(w.npc(head).alibi, Some(w.day));
    }

    #[test]
    fn a_wagon_never_carries_more_than_a_ton() {
        let (goods, _, used) = fill(Route::Westport, 10_000, WAGON_POUNDS);
        assert!(used <= WAGON_POUNDS);
        assert!(!goods.is_empty());
    }
}
