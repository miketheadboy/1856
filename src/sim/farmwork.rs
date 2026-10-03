//! The farm year (§10). Plow in April, plant in the planting window, cut hay
//! in July, pick corn from late September, butcher at the first hard cold,
//! and mend fence whenever nothing else is due. Every job has a window; miss
//! it and you pay all winter. Neighbors work their own places by their
//! labor; the player works theirs one day at a time (`life::Activity::Chores`).
//!
//! The almanac says plant corn in the light of the moon. The moon does nothing
//! for corn here — but the families who wait for it plant late, and late corn
//! yields less. Belief has costs even when it's wrong.

use super::calendar::Day;
use super::economy;
use super::events::EventKind;
use super::market::{self, Good};
use super::psyche;
use super::world::{FamilyId, World};

/// Days of work each job takes a household of average labor.
const PLOW_DAYS: f32 = 4.0;
const PLANT_DAYS: f32 = 3.0;
/// Planting after this (21 May) costs yield.
const LATE_AFTER: (u32, u32) = (5, 20);
const LATE_YIELD: f32 = 0.7;
/// Tons of prairie hay a head of stock eats over a winter.
pub const HAY_PER_HEAD: f32 = 1.0;
/// Share of the standing crop a day of picking brings in.
const PICK_RATE: f32 = 0.12;
/// Share of unpicked corn still worth anything by December: hogs, deer and
/// weather take the rest.
const LEFT_IN_FIELD: f32 = 0.3;
/// Days of food in a winter hog's worth of salt pork and lard.
const BUTCHER_FOOD: f32 = 70.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Task {
    Plow,
    Plant,
    Hay,
    Pick,
    Butcher,
    Fences,
}

impl Task {
    pub fn label(self) -> &'static str {
        match self {
            Task::Plow => "plowing",
            Task::Plant => "planting",
            Task::Hay => "haying",
            Task::Pick => "picking corn",
            Task::Butcher => "butchering",
            Task::Fences => "mending fence",
        }
    }
}

/// What the season asks for today. Fences when nothing else does.
pub fn due(day: Day) -> Task {
    let (_, m, d) = day.date();
    match (m, d) {
        (4, 1..=20) => Task::Plow,
        (4, _) | (5, _) => Task::Plant,
        (7, _) | (8, 1..=15) => Task::Hay,
        (9, 20..) | (10, _) | (11, _) => Task::Pick,
        (12, 1..=20) => Task::Butcher,
        _ => Task::Fences,
    }
}

#[derive(Clone, Debug)]
pub struct Farm {
    pub plowed: f32,
    pub plant_work: f32,
    pub planted_on: Option<Day>,
    /// Tons put up for the winter.
    pub hay: f32,
    /// Days of food still standing in the field, of `crop` at harvest.
    pub standing: f32,
    pub crop: f32,
    pub butchered: bool,
    /// 0 down .. 1 tight. Stock strays through bad fence.
    pub fences: f32,
}

impl Default for Farm {
    fn default() -> Self {
        Farm {
            plowed: 0.0,
            plant_work: 0.0,
            planted_on: None,
            hay: 0.0,
            standing: 0.0,
            crop: 0.0,
            butchered: false,
            fences: 0.7,
        }
    }
}

/// Almanac folk wait for the waxing moon to plant.
fn waits_for_moon(world: &World, family: FamilyId) -> bool {
    world.head_of(family).is_some_and(|h| {
        let t = &world.npc(h).temperament;
        t.piety > 0.6 && t.skepticism < 0.4
    })
}

/// One day's work on whatever is due. `hands` is a day's labor (1.0 = an
/// average household). Returns the task worked.
pub fn work(world: &mut World, family: FamilyId, hands: f32) -> Task {
    let today = world.day;
    let task = due(today);
    let f = family as usize;
    match task {
        Task::Plow => world.families[f].stores.work.plowed += hands,
        Task::Plant => {
            let w = &mut world.families[f].stores.work;
            if w.planted_on.is_none() {
                w.plant_work += hands;
                if w.plant_work >= PLANT_DAYS {
                    w.planted_on = Some(today);
                    economy::sow(
                        world,
                        family,
                        world.families[f].stores.work.plowed >= PLOW_DAYS,
                    );
                }
            }
        }
        Task::Hay => {
            // Cut what the stock will need, and a margin; no more.
            let hh = &mut world.families[f].stores;
            let want = (hh.cattle + hh.oxen) as f32 * HAY_PER_HEAD * 1.3;
            if hh.work.hay < want {
                hh.work.hay += 0.15 * hands;
            }
        }
        Task::Pick => {
            let w = &mut world.families[f].stores.work;
            let picked = (w.crop * PICK_RATE * hands).max(5.0).min(w.standing);
            w.standing -= picked;
            world.families[f].stores.food += picked;
        }
        Task::Butcher => {
            let hh = &world.families[f].stores;
            if !hh.work.butchered && hh.cattle >= 3 {
                let salted = market::consume(world, family, Good::Salt);
                let hh = &mut world.families[f].stores;
                hh.cattle -= 1;
                hh.work.butchered = true;
                let smoked =
                    hh.improvements.built & super::homestead::Improvement::Smokehouse.bit() != 0;
                hh.goods[Good::Hides.index()] += 1.0;
                let kept = if salted {
                    BUTCHER_FOOD
                } else if smoked {
                    BUTCHER_FOOD * 0.8
                } else {
                    BUTCHER_FOOD * 0.5
                };
                super::larder::add_meat(world, family, kept);
            }
        }
        Task::Fences => {
            let w = &mut world.families[f].stores.work;
            w.fences = (w.fences + 0.1 * hands).min(1.0);
        }
    }
    task
}

/// Mend fence whatever the season asks: a day on the rails.
pub fn mend(world: &mut World, family: FamilyId, hands: f32) {
    let h = &mut world.families[family as usize].stores;
    // Rails already split go up three times as fast as rails you split today.
    let split = if h.arms.rails >= 10 {
        h.arms.rails -= 10;
        3.0
    } else {
        1.0
    };
    let w = &mut h.work;
    w.fences = (w.fences + 0.1 * hands * split).min(1.0);
}

/// Yield multiplier for when the corn went in.
pub fn timing(planted_on: Option<Day>) -> f32 {
    match planted_on {
        None => 0.0,
        Some(d) => {
            let (_, m, dd) = d.date();
            if (m, dd) > LATE_AFTER {
                LATE_YIELD
            } else {
                1.0
            }
        }
    }
}

pub fn daily(world: &mut World) {
    let today = world.day;
    let (_, month, dom) = today.date();

    for f in 0..world.families.len() {
        let family = f as FamilyId;
        if !world.families[f].farms() || world.head_of(family).is_none() {
            continue;
        }
        // Fence rots; stock pushes through.
        let rails = world.families[f].stores.improvements.built
            & super::homestead::Improvement::RailFence.bit()
            != 0;
        let w = &mut world.families[f].stores.work;
        w.fences = (w.fences - if rails { 0.002 } else { 0.004 }).max(0.0);

        // Neighbors work their own places; the player works theirs by hand.
        let manual = family == 0 && !world.autopilot_player;
        if !manual {
            let task = due(today);
            let moon_wait = task == Task::Plant && waits_for_moon(world, family) && !today.waxing();
            let sunday = today.0 % 7 == 3;
            if !moon_wait && !sunday && world.rng.chance(0.8) {
                let hands = psyche::labor(world, family).clamp(0.2, 1.5);
                work(world, family, hands);
            }
        }

        strays(world, family);
    }

    match (month, dom) {
        // Planting: a family that ate its seed has to find some.
        (4, 21) => find_seed(world),
        // The planting window closes: whoever didn't plant, didn't.
        (6, 1) => {
            for f in &mut world.families {
                f.stores.work.plant_work = 0.0;
            }
        }
        // December: the field keeps a little; the rest is gone.
        (12, 1) => {
            for f in &mut world.families {
                let w = &mut f.stores.work;
                f.stores.food += w.standing * LEFT_IN_FIELD;
                w.standing = 0.0;
            }
        }
        // New year's work: reset the calendar.
        (3, 31) => {
            for f in &mut world.families {
                let w = &mut f.stores.work;
                w.plowed = 0.0;
                w.plant_work = 0.0;
                w.planted_on = None;
                w.butchered = false;
            }
        }
        _ => {}
    }
    if dom == 1 && matches!(month, 12 | 1 | 2 | 3) {
        winter_feed(world);
    }
}

/// Seed corn for families that ate theirs: from the store on the book, or
/// from a generous neighbor with some to spare — who is remembered for it.
fn find_seed(world: &mut World) {
    for f in 0..world.families.len() {
        let family = f as FamilyId;
        let hh = &world.families[f].stores;
        if !world.families[f].farms() || world.head_of(family).is_none() || hh.seed * 2 >= hh.acres
        {
            continue;
        }
        if family == 0 && !world.autopilot_player {
            continue; // the player buys their own
        }
        let want = hh.acres.saturating_sub(hh.seed);
        if !super::legacy::credit_cut(world, family) && hh.debt < 40 {
            let dollars = (want as f32 * world.market.price(Good::SeedCorn)).ceil() as i32;
            let got = market::buy_on_credit(world, family, Good::SeedCorn, dollars);
            if got > 0.0 {
                world.families[f].stores.debt += dollars;
                continue;
            }
        }
        let head = world.head_of(family).unwrap();
        let lender = world
            .families
            .iter()
            .filter(|o| o.id != family && o.farms() && o.stores.seed > o.stores.acres + 2)
            .filter_map(|o| world.head_of(o.id))
            .filter(|&h| world.npc(h).temperament.generosity > 0.5 && world.opinion(h, head) > -20)
            .max_by_key(|&h| world.opinion(h, head));
        if let Some(l) = lender {
            let lf = world.npc(l).family as usize;
            let spare =
                (world.families[lf].stores.seed - world.families[lf].stores.acres).min(want);
            world.families[lf].stores.seed -= spare;
            world.families[f].stores.seed += spare;
            world.adjust_opinion(head, l, 15);
        }
    }
}

/// Stock through bad fence. Most strays are found; some aren't, and the
/// owner decides who took it.
fn strays(world: &mut World, family: FamilyId) {
    let hh = &world.families[family as usize].stores;
    if hh.cattle == 0 {
        return;
    }
    // Someone riding the fence line finds most of them before they're gone.
    let p =
        0.004 * (1.0 - hh.work.fences) * hh.cattle as f32 * super::hands::herding(world, family);
    if !world.rng.chance(p) || !world.rng.chance(0.5) {
        return;
    }
    let Some(owner) = world.head_of(family) else {
        return;
    };
    world.families[family as usize].stores.cattle -= 1;
    world.emit_root(EventKind::Strayed { owner }, None);
}

/// Winter months eat hay; short hay costs stock.
fn winter_feed(world: &mut World) {
    for f in 0..world.families.len() {
        let hh = &mut world.families[f].stores;
        let head = (hh.cattle + hh.oxen) as f32;
        if head == 0.0 {
            continue;
        }
        let need = head * HAY_PER_HEAD / 4.0;
        if hh.work.hay >= need {
            hh.work.hay -= need;
            continue;
        }
        let short = 1.0 - hh.work.hay / need;
        hh.work.hay = 0.0;
        let lost = (hh.cattle as f32 * short * 0.3).round() as u32;
        if lost > 0 {
            hh.cattle -= lost.min(hh.cattle);
            let family = f as FamilyId;
            world.emit_root(
                EventKind::StockStarved {
                    family,
                    head: lost as u8,
                },
                None,
            );
        }
    }
}

/// What the player's fields need now, for the view.
pub fn status(world: &World) -> String {
    let w = &world.families[0].stores.work;
    let task = due(world.day);
    let mut s = format!("Season's work: {}", task.label());
    match task {
        Task::Plow => s.push_str(&format!(" ({:.0}/{PLOW_DAYS:.0} days)", w.plowed)),
        Task::Plant if w.planted_on.is_none() => {
            s.push_str(&format!(" ({:.0}/{PLANT_DAYS:.0} days)", w.plant_work))
        }
        Task::Plant => s = "Corn is in the ground".into(),
        Task::Pick => s.push_str(&format!(" ({:.0} days of food still standing)", w.standing)),
        _ => {}
    }
    s.push_str(&format!(
        "\nHay {:.1} t   Fences {:.0}%",
        w.hay,
        w.fences * 100.0
    ));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day_of(m: u32, d: u32) -> Day {
        (0..500)
            .map(Day)
            .find(|x| x.date().1 == m && x.date().2 == d)
            .unwrap()
    }

    #[test]
    fn the_calendar_asks_for_the_right_work() {
        assert_eq!(due(day_of(4, 5)), Task::Plow);
        assert_eq!(due(day_of(5, 10)), Task::Plant);
        assert_eq!(due(day_of(7, 20)), Task::Hay);
        assert_eq!(due(day_of(10, 1)), Task::Pick);
        assert_eq!(due(day_of(12, 5)), Task::Butcher);
        assert_eq!(due(day_of(2, 5)), Task::Fences);
    }

    #[test]
    fn late_corn_yields_less() {
        assert_eq!(timing(Some(day_of(5, 1))), 1.0);
        assert!(timing(Some(day_of(5, 28))) < 1.0);
        assert_eq!(timing(None), 0.0);
    }

    #[test]
    fn a_player_who_never_plants_has_nothing_standing() {
        let mut w = World::new(5);
        w.autopilot_player = false;
        w.families[0].stores.food = 5000.0; // don't starve for the test
        w.run_days(335); // to October 1856
        assert!(w.families[0].stores.work.planted_on.is_none());
        assert_eq!(w.families[0].stores.work.standing, 0.0);
        // The neighbors did.
        assert!((1..w.families.len()).any(|f| w.families[f].stores.work.planted_on.is_some()));
    }

    #[test]
    fn good_fence_keeps_stock_home() {
        let strays = |fences: f32| {
            (0..40)
                .map(|s| {
                    let mut w = World::new(s);
                    for f in &mut w.families {
                        f.stores.cattle = 6;
                    }
                    let mut n = 0;
                    for _ in 0..60 {
                        for f in &mut w.families {
                            f.stores.work.fences = fences;
                        }
                        w.advance_day();
                    }
                    n += w
                        .events
                        .iter()
                        .filter(|e| matches!(e.kind, EventKind::Strayed { .. }))
                        .count();
                    n
                })
                .sum::<usize>()
        };
        assert!(strays(0.0) > strays(1.0) * 3 + 5);
    }
}
