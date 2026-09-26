//! Slow capital (§10): food, seed corn, breeding stock, oxen, and debt.
//!
//! The rule: nothing here is free. Every way out of a hungry February spends
//! something the household needed for next year, or puts it in someone's debt.

use super::calendar::{Day, Season};
use super::character::{self, Archetype};
use super::events::{Desperate, EventKind, Hardship, Loot};
use super::market::{self, GOODS, Good};
use super::psyche::{self, LifeStage};
use super::world::{Faction, FamilyId, NpcId, PLAYER, World, distance};

/// Days of food one bushel of corn is worth.
pub const SEED_FOOD: f32 = 12.0;
/// Days of food in a butchered cow / ox.
const COW_FOOD: f32 = 90.0;
const OX_FOOD: f32 = 160.0;
/// Most the store will lend before it stops extending credit.
const CREDIT_LIMIT: i32 = 60;
/// Days a family waits between desperate choices.
const DECISION_COOLDOWN: u32 = 5;
/// Start deciding when stores fall below this many days of eating.
const LOW_WATER_DAYS: f32 = 8.0;
/// Days of food stolen with a sack of grain.
const GRAIN_SACK: f32 = 30.0;
/// Days of food a neighbor hands over when begged.
const CHARITY: f32 = 30.0;

#[derive(Clone, Debug, Default)]
pub struct Household {
    /// Days of food for one person.
    pub food: f32,
    /// Bushels held back to plant.
    pub seed: u32,
    /// Land under plow.
    pub acres: u32,
    pub planted: u32,
    /// Breeding stock.
    pub cattle: u32,
    pub oxen: u32,
    pub cash: i32,
    pub debt: i32,
    pub creditor: Option<NpcId>,
    /// Consecutive days with nothing to eat.
    pub hungry_days: u32,
    pub last_decision: Option<Day>,
    /// Last day this family was seen begging. Hunger is public knowledge.
    pub begged_on: Option<Day>,
    /// 0 eats the future first .. 1 holds out and borrows first.
    pub prudence: f32,
    /// Proud families would rather steal than be seen begging.
    pub proud: bool,
    /// Impulsive families reach for the neighbor's cow before the seed corn.
    pub impulsive: bool,
    /// Which desperate acts the county has already heard about this winter.
    pub announced: u8,
    /// Salt, powder, timber, whiskey, hides (indexed by `Good`).
    pub goods: [f32; GOODS],
}

impl Household {
    /// Money trouble and hunger make people dangerous: nothing left to lose.
    pub fn desperate(&self) -> bool {
        self.hungry_days > 0 || self.debt >= 30
    }

    /// Everyone can see a family is starving: they come begging, they look thin.
    pub fn visibly_hungry(&self, today: Day) -> bool {
        self.hungry_days > 0 || self.begged_on.is_some_and(|d| today.0 < d.0 + 30)
    }
}

fn members(world: &World, family: FamilyId) -> Vec<NpcId> {
    world
        .living()
        .filter(|n| n.family == family)
        .map(|n| n.id)
        .collect()
}

/// Who holds the store's ledger.
pub fn storekeeper(world: &World) -> Option<NpcId> {
    world
        .families
        .iter()
        .find(|f| f.store)
        .and_then(|f| world.head_of(f.id))
}

pub fn daily(world: &mut World) {
    let today = world.day;
    let weather = world.weather_on(today);
    let winter = today.season() == Season::Winter;
    let (_, month, dom) = today.date();

    for f in 0..world.families.len() {
        if world.families[f].store {
            continue;
        }
        let family = f as FamilyId;
        let people = members(world, family);
        if people.is_empty() {
            continue;
        }
        let m = people.len() as f32;

        // Eating. Cold makes you eat more; a blizzard makes it worse.
        let mut need = m;
        // Gardens, game, greens: spring and summer feed you partway.
        if (4..=9).contains(&month) {
            need -= 0.6 * m;
        }
        if winter {
            need += 0.3 * m * world.winter_severity;
        }
        if weather.blizzard {
            need += 0.4 * m;
        }
        let hh = &mut world.families[f].stores;
        hh.food -= need;
        let starving = hh.food <= 0.0;
        if starving {
            hh.food = 0.0;
            hh.hungry_days += 1;
        } else {
            hh.hungry_days = 0;
        }

        // Cattle with no barn freeze in a blizzard.
        if weather.blizzard
            && !world.families[f].barn_standing
            && world.families[f].stores.cattle > 0
            && world.rng.chance(0.2)
        {
            world.families[f].stores.cattle -= 1;
        }

        let hh = &world.families[f].stores;
        let low = hh.food < LOW_WATER_DAYS * m;
        let rested = hh
            .last_decision
            .is_none_or(|d| today.0 >= d.0 + DECISION_COOLDOWN);
        if low && rested && (family != 0 || world.autopilot_player) {
            world.families[f].stores.last_decision = Some(today);
            decide(world, family);
        }

        // Bodies give out one at a time; children first (see psyche).
        let hungry = world.families[f].stores.hungry_days > 0;
        let cold = weather.blizzard && !world.families[f].barn_standing;
        for &p in &people {
            if world.npc(p).health <= 0 {
                let cause = if hungry {
                    Hardship::Hunger
                } else if cold {
                    Hardship::Cold
                } else {
                    Hardship::Fever
                };
                world.emit_root(EventKind::Perished { victim: p, cause }, None);
            }
        }
    }

    match (month, dom) {
        (11, 1) => {
            for f in &mut world.families {
                f.stores.announced = 0;
            }
        }
        (4, 15) => plant(world),
        (5, 1) => breed(world),
        (9, 20) => harvest(world),
        _ => {}
    }
    if dom == 1 {
        monthly(world);
    }
}

/// What a hungry household tries, in the order its temperament suggests.
fn decide(world: &mut World, family: FamilyId) {
    let hungry = world.families[family as usize].stores.hungry_days > 0;
    let free_state = world.families[family as usize].faction == Faction::FreeState;

    // Buying with cash on hand is just shopping. Hunting is just work.
    // In the cold months the robes are prime: a family with a team and powder
    // tries the range before the rabbits.
    let range_season = matches!(world.day.month(), 10..=12 | 1 | 2);
    if buy(world, family)
        || (range_season && super::bison::go_west(world, family))
        || hunt(world, family)
    {
        return;
    }

    let hh = &world.families[family as usize].stores;
    let (prudent, proud, impulsive) = (hh.prudence > 0.5, hh.proud, hh.impulsive);
    let skinflint = world
        .head_of(family)
        .is_some_and(|h| character::is(world.npc(h), Archetype::Skinflint));
    let order: &[Choice] = if impulsive {
        &[
            Choice::SpareCow,
            Choice::Steal,
            Choice::EatSeed,
            Choice::Borrow,
            Choice::Beg,
            Choice::Ox,
            Choice::LastCow,
        ]
    } else if prudent {
        &[
            Choice::GoWest,
            Choice::SpareCow,
            Choice::Beg,
            Choice::Borrow,
            Choice::EatSeed,
            Choice::Ox,
            Choice::Steal,
            Choice::LastCow,
        ]
    } else {
        &[
            Choice::EatSeed,
            Choice::SpareCow,
            Choice::Borrow,
            Choice::Beg,
            Choice::Steal,
            Choice::Ox,
            Choice::LastCow,
        ]
    };
    for choice in order {
        let done = match choice {
            // A Free-State family goes to the pro-slavery store only when it has to.
            Choice::Borrow if free_state && !hungry => false,
            Choice::Steal if !hungry && !impulsive && !skinflint => false,
            Choice::Beg if proud => false,
            c => act(world, family, *c),
        };
        if done {
            return;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Go out with the rifle. Free, if you can shoot and the game is there.
    Hunt,
    /// Weeks west on the buffalo range with a wagon. Needs oxen and powder.
    GoWest,
    EatSeed,
    /// Butcher one of the herd, keeping a breeding pair.
    SpareCow,
    /// Butcher the last of the herd.
    LastCow,
    Ox,
    Borrow,
    Beg,
    Steal,
}

/// The first time each winter a family does something desperate, the county
/// hears about it. After that it's just how they live now.
fn desperation(world: &mut World, family: FamilyId, act: Desperate) {
    let bit = match act {
        Desperate::EatSeed { .. } => 1,
        Desperate::Slaughter => 2,
        Desperate::SlaughterOx => 4,
        Desperate::Borrow { .. } => 8,
        Desperate::Beg { .. } => 0,
    };
    let hh = &mut world.families[family as usize].stores;
    if bit != 0 && hh.announced & bit != 0 {
        return;
    }
    hh.announced |= bit;
    world.emit_root(EventKind::Desperation { family, act }, None);
}

/// Buy corn at market price. If the store is bare, look at who's hoarding.
fn buy(world: &mut World, family: FamilyId) -> bool {
    let got = if world.families[family as usize].stores.cash >= 2 {
        market::buy(world, family, Good::Corn, 15.0)
    } else {
        0.0
    };
    // Bare shelves or robbery prices: either way, someone's sitting on corn.
    market::resent_hoarders(world, family);
    got > 0.0
}

/// Meat from a butchered animal: half of it spoils without salt.
fn butcher(world: &mut World, family: FamilyId, meat: f32) {
    let salted = market::consume(world, family, Good::Salt);
    let hh = &mut world.families[family as usize].stores;
    hh.food += if salted { meat } else { meat * 0.5 };
    hh.goods[Good::Hides.index()] += 1.0;
}

/// Try one way out. Returns whether it produced food (or was at least tried
/// publicly, like begging and being refused).
pub fn act(world: &mut World, family: FamilyId, choice: Choice) -> bool {
    let f = family as usize;
    match choice {
        Choice::EatSeed => {
            let hh = &mut world.families[f].stores;
            if hh.seed == 0 {
                return false;
            }
            let bushels = hh.seed.min(4);
            hh.seed -= bushels;
            hh.food += bushels as f32 * SEED_FOOD;
            desperation(world, family, Desperate::EatSeed { bushels });
            true
        }
        Choice::SpareCow | Choice::LastCow => {
            let keep = if choice == Choice::SpareCow { 2 } else { 0 };
            let hh = &mut world.families[f].stores;
            if hh.cattle <= keep {
                return false;
            }
            hh.cattle -= 1;
            butcher(world, family, COW_FOOD);
            desperation(world, family, Desperate::Slaughter);
            true
        }
        Choice::Ox => {
            let hh = &mut world.families[f].stores;
            if hh.oxen == 0 {
                return false;
            }
            hh.oxen -= 1;
            butcher(world, family, OX_FOOD);
            desperation(world, family, Desperate::SlaughterOx);
            true
        }
        Choice::Borrow => {
            let Some(lender) = storekeeper(world) else {
                return false;
            };
            let limit = if world.credit_crunch {
                CREDIT_LIMIT / 2
            } else {
                CREDIT_LIMIT
            };
            let hh = &mut world.families[f].stores;
            if hh.debt >= limit {
                return false;
            }
            let dollars = 10;
            hh.debt += dollars;
            hh.creditor = Some(lender);
            if market::buy_on_credit(world, family, Good::Corn, dollars) <= 0.0 {
                // The store's bare too. The debt stands anyway: it's a promise.
                market::resent_hoarders(world, family);
            }
            desperation(world, family, Desperate::Borrow { lender, dollars });
            true
        }
        Choice::Beg => beg(world, family),
        Choice::Hunt => hunt(world, family),
        Choice::GoWest => super::bison::go_west(world, family),
        Choice::Steal => steal(world, family),
    }
}

/// The best shot in the family goes out. Winter game is thin and the cold bites.
fn hunt(world: &mut World, family: FamilyId) -> bool {
    let Some(hunter) = world
        .living()
        .filter(|n| n.family == family && LifeStage::of(n.age) == LifeStage::Adult && !n.wounded)
        .max_by(|a, b| a.body.marksmanship.total_cmp(&b.body.marksmanship))
        .map(|n| n.id)
    else {
        return false;
    };
    // No powder, no hunt.
    if !market::consume(world, family, Good::Powder) {
        return false;
    }
    let n = world.npc(hunter);
    let winter = world.day.season() == super::calendar::Season::Winter;
    let game = if winter { 0.5 } else { 1.0 };
    let p = (0.1 + 0.5 * n.body.marksmanship) * game * n.hidden.luck;
    let frailty = n.body.frailty();
    if winter {
        world.npc_mut(hunter).health -= (3.0 * frailty) as i32;
    }
    if world.rng.chance(p) {
        let hh = &mut world.families[family as usize].stores;
        hh.food += 25.0;
        hh.goods[Good::Hides.index()] += 1.0;
        true
    } else {
        false
    }
}

/// Ask the neighbor you're on best terms with. Whoever helps, you now owe.
fn beg(world: &mut World, family: FamilyId) -> bool {
    let Some(asker) = world.head_of(family) else {
        return false;
    };
    let today = world.day;
    let candidates: Vec<(FamilyId, NpcId, i16)> = world
        .families
        .iter()
        .filter(|g| g.id != family && !g.store)
        .filter(|g| g.stores.food > CHARITY * 3.0)
        .filter_map(|g| world.head_of(g.id).map(|h| (g.id, h)))
        .map(|(g, h)| (g, h, world.opinion(asker, h)))
        .collect();
    let Some(&(giver_family, giver, _)) = candidates.iter().max_by_key(|c| c.2) else {
        return false;
    };
    world.families[family as usize].stores.begged_on = Some(today);

    let regard = world.opinion(giver, asker) as f32;
    let g = world.npc(giver);
    let heart = (g.temperament.generosity - 0.5) * 0.6;
    let granted = character::is(g, Archetype::Deacon)
        || world
            .rng
            .chance((0.35 + regard / 100.0 + heart).clamp(0.05, 0.95));
    if granted {
        world.families[giver_family as usize].stores.food -= CHARITY;
        world.families[family as usize].stores.food += CHARITY;
    }
    let id = world.emit_root(
        EventKind::Desperation {
            family,
            act: Desperate::Beg {
                neighbor: giver,
                granted,
            },
        },
        None,
    );
    // Gratitude and resentment both last.
    let (to_giver, to_asker) = if granted { (20, 5) } else { (-15, 0) };
    world.emit_root(
        EventKind::OpinionChange {
            holder: asker,
            target: giver,
            delta: to_giver,
            after: 0,
        },
        Some(id),
    );
    if to_asker != 0 {
        world.emit_root(
            EventKind::OpinionChange {
                holder: giver,
                target: asker,
                delta: to_asker,
                after: 0,
            },
            Some(id),
        );
    }
    granted
}

/// Take a cow or a sack of grain from someone who has one. Preferably someone
/// you don't like, and not too far to drive a cow at night.
fn steal(world: &mut World, family: FamilyId) -> bool {
    let Some(thief) = world.head_of(family) else {
        return false;
    };
    let momentum = world.npc(thief).violence as f32;
    let nerve = psyche::would_steal(world, thief) * (1.0 + 0.15 * momentum);
    if thief != PLAYER && !world.rng.chance(nerve.min(0.95)) {
        return false;
    }
    let home = world.families[family as usize].farm;
    let target = world
        .families
        .iter()
        .filter(|g| g.id != family && (g.stores.cattle > 0 || g.stores.food > GRAIN_SACK))
        .filter_map(|g| world.head_of(g.id).map(|h| (g, h)))
        .max_by_key(|(g, h)| {
            // Who you dislike, who's close, and who's got the most.
            -(world.opinion(thief, *h) as i32) - distance(g.farm, home) as i32 * 3
                + (g.stores.food / 60.0) as i32
        })
        .map(|(g, h)| (g.id, h));
    let Some((victim_family, victim)) = target else {
        return false;
    };
    steal_from(world, thief, victim_family, victim);
    true
}

pub fn steal_from(world: &mut World, thief: NpcId, victim_family: FamilyId, victim: NpcId) {
    let thief_family = world.npc(thief).family as usize;
    let v = &mut world.families[victim_family as usize].stores;
    let loot = if v.cattle > 0 {
        v.cattle -= 1;
        Loot::Cow
    } else {
        v.food = (v.food - GRAIN_SACK).max(0.0);
        Loot::Grain
    };
    world.families[thief_family].stores.food += match loot {
        Loot::Cow => COW_FOOD,
        Loot::Grain => GRAIN_SACK * (0.5 + world.npc(thief).body.strength),
    };
    world.npc_mut(thief).alibi = None;
    world.emit_root(
        EventKind::Theft {
            thief,
            victim,
            loot,
        },
        None,
    );
}

fn plant(world: &mut World) {
    for f in &mut world.families {
        if f.store {
            continue;
        }
        let hh = &mut f.stores;
        // Oxen break new sod every spring. Without them you're frozen at this size.
        if hh.oxen > 0 {
            hh.acres += 1;
        }
        let capacity = if hh.oxen > 0 {
            hh.acres
        } else {
            hh.acres.min(3)
        };
        hh.planted = hh.seed.min(capacity);
        hh.seed -= hh.planted;
    }
}

fn breed(world: &mut World) {
    for f in &mut world.families {
        let hh = &mut f.stores;
        if hh.cattle >= 2 {
            hh.cattle += (hh.cattle / 3).max(1);
        }
    }
}

fn harvest(world: &mut World) {
    // A dry summer means a thin crop.
    let summer: Vec<f32> = world
        .weather
        .iter()
        .enumerate()
        .filter(|(d, _)| matches!(Day(*d as u32).month(), 6..=8) && *d + 120 > world.day.0 as usize)
        .map(|(_, w)| if w.rain { 1.0 } else { 0.0 })
        .collect();
    let wet = if summer.is_empty() {
        0.2
    } else {
        summer.iter().sum::<f32>() / summer.len() as f32
    };
    let rain_factor = (0.4 + wet * 3.5).min(1.3);

    // The whole county had the same summer: a drought means a thin glut this
    // fall and dear corn all winter.
    world.market.county_harvest = 90.0 * rain_factor;
    let bid = market::bid_price(world, Good::Corn).max(0.05);
    for f in 0..world.families.len() {
        if world.families[f].store || world.head_of(f as FamilyId).is_none() {
            continue;
        }
        let labor = psyche::labor(world, f as FamilyId);
        let luck = world
            .head_of(f as FamilyId)
            .map_or(1.0, |h| world.npc(h).hidden.luck);
        let hh = &mut world.families[f].stores;
        let mut bushels =
            (hh.planted as f32 * 14.0 * rain_factor * (0.6 + 0.5 * labor) * luck) as u32;
        // Hold back next year's seed first.
        let keep = hh.acres.saturating_sub(hh.seed).min(bushels);
        hh.seed += keep;
        bushels -= keep;
        let food = bushels as f32 * SEED_FOOD;
        hh.food += food;
        // Surplus pays down the store, at whatever corn fetches this fall.
        if hh.debt > 0 && hh.food > 250.0 {
            let per_dollar = SEED_FOOD / bid;
            let dollars = (((hh.food - 250.0) / per_dollar) as i32).min(hh.debt);
            hh.debt -= dollars;
            hh.food -= dollars as f32 * per_dollar;
        }
        let planted = hh.planted;
        hh.planted = 0;
        world.emit_root(
            EventKind::Harvest {
                family: f as FamilyId,
                planted,
                food: food as u32,
            },
            None,
        );
    }
}

fn monthly(world: &mut World) {
    let Some(creditor) = storekeeper(world) else {
        return;
    };
    for f in 0..world.families.len() {
        let hh = &mut world.families[f].stores;
        if hh.debt <= 0 {
            continue;
        }
        hh.debt += (hh.debt as f32 * 0.04).ceil() as i32;
        let debt = hh.debt;
        let family = f as FamilyId;
        // The storekeeper's politics come due with the interest.
        if world.families[f].faction == Faction::FreeState
            && debt >= 20
            && world.rng.chance(0.35)
            && let Some(debtor) = world.head_of(family)
        {
            if debtor == PLAYER && !world.autopilot_player {
                world.pending_favor = Some(creditor);
                continue;
            }
            let hungry = world.families[f].stores.hungry_days > 0;
            let spite = (-world.opinion(debtor, creditor)).max(0) as f32 / 100.0;
            let d = world.npc(debtor);
            let complied = if character::is(d, Archetype::Zealot) {
                false
            } else if character::is(d, Archetype::Coward) {
                true
            } else {
                let p = 0.45 + if hungry { 0.2 } else { 0.0 } - spite
                    + psyche::pliability(world, debtor);
                world.rng.chance(p.clamp(0.05, 0.95))
            };
            world.emit_root(
                EventKind::Favor {
                    creditor,
                    debtor,
                    complied,
                },
                None,
            );
        }
    }
}

/// Settles a favor: sign the petition and be forgiven half, or refuse and
/// lose what the store can carry off.
pub fn settle_favor(world: &mut World, debtor: NpcId, complied: bool) {
    let f = world.npc(debtor).family as usize;
    let hh = &mut world.families[f].stores;
    if complied {
        hh.debt /= 2;
    } else if hh.oxen > 0 {
        hh.oxen -= 1;
        hh.debt = (hh.debt - 40).max(0);
    } else {
        let taken = hh.cattle.min(2);
        hh.cattle -= taken;
        hh.debt = (hh.debt - 20 * taken as i32).max(0);
    }
    if hh.debt == 0 {
        hh.creditor = None;
    }
}
