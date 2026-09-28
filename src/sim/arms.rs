//! Stockpiles, crafting and arms caches (PLAN Phase C).
//!
//! Every house has something to shoot with: an old musket or a fowling
//! piece, a bar of lead, a bag of balls. Free-State houses could send east
//! for "books": Sharps rifles shipped in crates stenciled BIBLES, which the
//! Missouri river towns learned to open. Lead gets cast into balls, balls and
//! powder get rolled into cartridges, timber gets split into rails. Rifles
//! go in the loft, under the floor, or into the ground; the sheriff's posse
//! comes looking after its musters, and what it finds, it takes.
//!
//! What each number does:
//! - `rifles` steady your aim (`action::sway`), stiffen your nerve at the
//!   gate (`action::nerve`), push harder in someone else's standoff
//!   (`action::pressure`), and get seized in searches.
//! - `guns` let you shoot at all; `balls` and `cartridges` are shots, one a
//!   pull (`fire`). No round, no ambush and no draw.
//! - `lead` casts balls; `rails` mend fence three times as fast (`farmwork::mend`).
//! - `hide` decides what a search finds, and whether you can reach the guns
//!   when riders come (buried ones you can't).

use super::calendar::Day;
use super::events::{EventKind, WorldEvent};
use super::life::Skill;
use super::market::Good;
use super::psyche::LifeStage;
use super::world::{Faction, FamilyId, World};

/// Cost of a Sharps and its cartridges, sent from the East.
pub const RIFLE_PRICE: i32 = 20;
/// An old shotgun at Dunmore's, before his markup on Free-State men.
pub const GUN_PRICE: i32 = 8;
/// Five pounds of bar lead.
pub const LEAD_PRICE: i32 = 1;
/// Days a posse rides after a muster is called before it searches houses.
const SEARCH_AFTER: u32 = 3;

/// Where the guns are kept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hide {
    /// Over the door, where a man keeps his gun.
    #[default]
    Open,
    /// Up in the loft under the bedding.
    Loft,
    /// Under a board in the floor.
    Floor,
    /// Wrapped in oilcloth in the timber. Safe, and no use at the gate.
    Buried,
}

impl Hide {
    pub const ALL: [Hide; 4] = [Hide::Open, Hide::Loft, Hide::Floor, Hide::Buried];

    /// Odds a searching posse turns them up.
    pub fn found(self) -> f32 {
        match self {
            Hide::Open => 0.9,
            Hide::Loft => 0.5,
            Hide::Floor => 0.25,
            Hide::Buried => 0.05,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Hide::Open => "over the door",
            Hide::Loft => "in the loft",
            Hide::Floor => "under the floor",
            Hide::Buried => "buried in the timber",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Armory {
    /// Sharps breechloaders. Steady, and seized on sight.
    pub rifles: u8,
    /// Old muskets and fowling pieces.
    pub guns: u8,
    /// Pounds of bar lead.
    pub lead: f32,
    /// Cast balls: one a shot from a muzzleloader.
    pub balls: u16,
    /// Paper cartridges: one a shot from anything, faster from a Sharps.
    pub cartridges: u16,
    /// Split rails, for mending fence.
    pub rails: u16,
    pub hide: Hide,
    /// Rifles ordered from the East, and the Wednesday they're due.
    pub on_order: Option<(Day, u8)>,
    /// One evening at the bench a day.
    pub crafted_on: Option<Day>,
}

impl Armory {
    pub fn rounds(&self) -> u16 {
        self.balls + self.cartridges
    }

    /// Something to shoot, something to shoot it with, and within reach.
    pub fn ready(&self) -> bool {
        self.rifles + self.guns > 0 && self.rounds() > 0 && self.hide != Hide::Buried
    }

    /// Guns the house can put in the doorway.
    pub fn at_hand(&self) -> u8 {
        if self.hide == Hide::Buried {
            0
        } else {
            self.rifles + self.guns
        }
    }
}

/// What the evening's work makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Craft {
    /// A pound of lead into the mold: sixteen balls, if you pour it right.
    Balls,
    /// Ten balls and a measure of powder rolled in paper.
    Cartridges,
    /// A load of timber split with maul and wedge.
    Rails,
}

impl Craft {
    pub const ALL: [Craft; 3] = [Craft::Balls, Craft::Cartridges, Craft::Rails];

    pub fn label(self) -> &'static str {
        match self {
            Craft::Balls => "cast balls",
            Craft::Cartridges => "roll cartridges",
            Craft::Rails => "split rails",
        }
    }

    /// How many good strokes the job takes (the minigame's count).
    pub fn strokes(self) -> u8 {
        match self {
            Craft::Balls => 4,
            Craft::Cartridges => 6,
            Craft::Rails => 5,
        }
    }

    /// Most this makes from one lot of stuff, with every stroke true.
    fn yield_full(self) -> f32 {
        match self {
            Craft::Balls => 16.0,
            Craft::Cartridges => 10.0,
            Craft::Rails => 12.0,
        }
    }
}

/// Is there stuff for this job, and an evening to do it?
pub fn can_craft(world: &World, c: Craft) -> bool {
    let h = &world.families[0].stores;
    let a = &h.arms;
    if !world.player_alive() || a.crafted_on == Some(world.day) {
        return false;
    }
    match c {
        Craft::Balls => a.lead >= 1.0,
        Craft::Cartridges => a.balls >= 10 && h.goods[Good::Powder.index()] >= 1.0,
        Craft::Rails => h.goods[Good::Timber.index()] >= 1.0,
    }
}

/// How wide the sweet spot is on the bench, 0..1 of the bar. Practice widens
/// it; carpentry for the maul, letters (patience with small work) for paper.
pub fn knack(world: &World, c: Craft) -> f32 {
    let s = match c {
        Craft::Rails => world.life.skill(Skill::Carpentry),
        Craft::Cartridges => world.life.skill(Skill::Letters),
        Craft::Balls => 0.5 * world.life.skill(Skill::Carpentry),
    };
    0.12 + 0.18 * s
}

/// The evening's work: `hits` good strokes out of `of`. Returns what it made.
pub fn craft(world: &mut World, c: Craft, hits: u8, of: u8) -> Option<u16> {
    if !can_craft(world, c) {
        return None;
    }
    let grade = hits as f32 / of.max(1) as f32;
    let made = (c.yield_full() * grade).round() as u16;
    let today = world.day;
    let h = &mut world.families[0].stores;
    h.arms.crafted_on = Some(today);
    match c {
        Craft::Balls => {
            h.arms.lead -= 1.0;
            h.arms.balls += made;
        }
        Craft::Cartridges => {
            h.arms.balls -= 10;
            h.goods[Good::Powder.index()] -= 1.0;
            h.arms.cartridges += made;
        }
        Craft::Rails => {
            h.goods[Good::Timber.index()] -= 1.0;
            h.arms.rails += made;
        }
    }
    world.life.learn(
        if c == Craft::Rails {
            Skill::Carpentry
        } else {
            Skill::Letters
        },
        0.5,
    );
    Some(made)
}

/// A shot. Cartridges first if there's a Sharps to put them in.
pub fn fire(world: &mut World, family: FamilyId) -> bool {
    let a = &mut world.families[family as usize].stores.arms;
    if a.cartridges > 0 && (a.rifles > 0 || a.balls == 0) {
        a.cartridges -= 1;
        true
    } else if a.balls > 0 {
        a.balls -= 1;
        true
    } else {
        false
    }
}

/// Put the guns somewhere.
pub fn hide(world: &mut World, family: FamilyId, place: Hide) {
    world.families[family as usize].stores.arms.hide = place;
}

/// Send east for rifles. They come in on a Wednesday two or three weeks on,
/// if the river's open.
pub fn order(world: &mut World, family: FamilyId, rifles: u8) -> bool {
    let cost = RIFLE_PRICE * rifles as i32;
    let h = &world.families[family as usize].stores;
    if h.cash < cost || h.arms.on_order.is_some() || rifles == 0 {
        return false;
    }
    world.families[family as usize].stores.cash -= cost;
    ship(world, family, rifles);
    true
}

/// A crate on its way, due the Wednesday after two weeks.
fn ship(world: &mut World, family: FamilyId, rifles: u8) {
    let today = world.day.0;
    let due = today + 14 + (7 - (today + 14) % 7) % 7;
    world.families[family as usize].stores.arms.on_order = Some((Day(due), rifles));
}

/// Dunmore sells an old gun. Free-State money buys it dearer.
pub fn gun_price(world: &World, family: FamilyId) -> i32 {
    if world.families[family as usize].faction == Faction::FreeState {
        GUN_PRICE * 3 / 2
    } else {
        GUN_PRICE
    }
}

pub fn buy_gun(world: &mut World, family: FamilyId) -> bool {
    let price = gun_price(world, family);
    let h = &mut world.families[family as usize].stores;
    if h.cash < price {
        return false;
    }
    h.cash -= price;
    h.arms.guns += 1;
    true
}

pub fn buy_lead(world: &mut World, family: FamilyId) -> bool {
    let h = &mut world.families[family as usize].stores;
    if h.cash < LEAD_PRICE {
        return false;
    }
    h.cash -= LEAD_PRICE;
    h.arms.lead += 5.0;
    true
}

/// Every house starts with something over the door. Settled by family, not
/// by dice, so the founding draws nothing from the county's luck.
pub fn founding(world: &mut World) {
    for f in 0..world.families.len() {
        let fam = &mut world.families[f];
        if fam.store {
            continue;
        }
        let a = &mut fam.stores.arms;
        a.guns = 1 + (fam.faction == Faction::ProSlavery && f % 2 == 0) as u8;
        a.balls = 10 + (f as u16 * 3) % 8;
        a.lead = 2.0;
    }
}

pub fn daily(world: &mut World) {
    deliveries(world);
    searches(world);
}

/// Wednesday: the Westport hack comes in, or the river towns open the crates.
fn deliveries(world: &mut World) {
    let today = world.day;
    for f in 0..world.families.len() {
        let Some((due, rifles)) = world.families[f].stores.arms.on_order else {
            continue;
        };
        if today < due {
            continue;
        }
        world.families[f].stores.arms.on_order = None;
        let family = f as FamilyId;
        let kind = if world.market.blockade {
            EventKind::Intercepted { family, rifles }
        } else {
            let a = &mut world.families[f].stores.arms;
            a.rifles += rifles;
            a.cartridges += 25 * rifles as u16;
            EventKind::ArmsArrived { family, rifles }
        };
        world.emit_root(kind, None);
    }
}

/// A few days after a pro-slavery muster is called, the posse goes house to
/// house among the Free-State claims.
fn searches(world: &mut World) {
    let Some(start) = world.day.0.checked_sub(SEARCH_AFTER) else {
        return;
    };
    let called = Day(start).date();
    if !super::law::MUSTERS
        .iter()
        .any(|m| m.pro_slavery && m.date == called)
    {
        return;
    }
    for f in 0..world.families.len() {
        let fam = &world.families[f];
        if fam.store || fam.faction != Faction::FreeState {
            continue;
        }
        if world.head_of(f as FamilyId).is_none() || !world.rng.chance(0.4) {
            continue;
        }
        let a = &fam.stores.arms;
        let found = a.rifles > 0 && world.rng.chance(a.hide.found());
        let seized = if found { a.rifles } else { 0 };
        if found {
            let a = &mut world.families[f].stores.arms;
            a.rifles = 0;
            a.cartridges /= 2;
        }
        world.emit_root(
            EventKind::Searched {
                family: f as FamilyId,
                seized,
            },
            None,
        );
    }
}

/// The Emigrant Aid men put rifles in the hands of Free-State houses with
/// the nerve to use them, at the Company's expense, in the hard months; the
/// other side has Missouri to draw on.
pub fn monthly(world: &mut World) {
    let (y, m, _) = world.day.date();
    let season = (y == 1855 && m == 12) || (y == 1856 && m <= 9);
    if !season {
        return;
    }
    for f in 1..world.families.len() {
        let fam = &world.families[f];
        if fam.store || fam.stores.arms.on_order.is_some() || fam.stores.arms.rifles > 1 {
            continue;
        }
        let Some(head) = world.head_of(f as FamilyId) else {
            continue;
        };
        let n = world.npc(head);
        if LifeStage::of(n.age) == LifeStage::Child {
            continue;
        }
        let t = n.temperament;
        match fam.faction {
            Faction::FreeState => {
                let p = 0.04 + 0.12 * t.loyalty + 0.08 * t.courage;
                if world.rng.chance(p) {
                    ship(world, f as FamilyId, 1);
                }
            }
            Faction::ProSlavery => {
                if world.rng.chance(0.05 + 0.1 * t.loyalty) {
                    let a = &mut world.families[f].stores.arms;
                    a.guns = (a.guns + 1).min(3);
                    a.balls += 10;
                }
            }
        }
    }
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::Searched { family, seized } if seized > 0 => {
            for n in world
                .npcs
                .iter_mut()
                .filter(|n| n.alive && n.family == family)
            {
                n.emotions.anger += 15.0;
            }
            world.add_grievance(Faction::FreeState, 2);
        }
        EventKind::Intercepted { .. } => {
            world.add_grievance(Faction::FreeState, 3);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::world::PLAYER;

    #[test]
    fn everyone_starts_with_something_over_the_door() {
        let w = World::new(3);
        assert!(w.families[0].stores.arms.ready());
        assert!(
            w.families
                .iter()
                .filter(|f| !f.store)
                .all(|f| f.stores.arms.guns >= 1)
        );
    }

    #[test]
    fn a_true_pour_makes_a_full_mold() {
        let mut w = World::new(3);
        let lead = w.families[0].stores.arms.lead;
        let before = w.families[0].stores.arms.balls;
        assert_eq!(craft(&mut w, Craft::Balls, 4, 4), Some(16));
        assert_eq!(w.families[0].stores.arms.balls, before + 16);
        assert_eq!(w.families[0].stores.arms.lead, lead - 1.0);
        // One evening at the bench.
        assert_eq!(craft(&mut w, Craft::Balls, 4, 4), None);
    }

    #[test]
    fn a_sloppy_pour_wastes_lead() {
        let mut w = World::new(3);
        assert_eq!(craft(&mut w, Craft::Balls, 1, 4), Some(4));
    }

    #[test]
    fn books_come_on_a_wednesday_unless_the_river_is_shut() {
        let mut w = World::new(3);
        w.families[0].stores.cash = 50;
        assert!(order(&mut w, 0, 1));
        assert_eq!(w.families[0].stores.cash, 30);
        let (due, _) = w.families[0].stores.arms.on_order.unwrap();
        assert!(due.0.is_multiple_of(7));
        w.run_days(due.0 - w.day.0);
        assert_eq!(w.families[0].stores.arms.rifles, 1);

        let mut w = World::new(3);
        w.families[0].stores.cash = 50;
        order(&mut w, 0, 1);
        let (due, _) = w.families[0].stores.arms.on_order.unwrap();
        w.day = due;
        w.market.blockade = true;
        deliveries(&mut w);
        assert_eq!(w.families[0].stores.arms.rifles, 0);
        assert!(
            w.events
                .iter()
                .any(|e| matches!(e.kind, EventKind::Intercepted { family: 0, .. }))
        );
    }

    #[test]
    fn buried_guns_are_safe_and_useless() {
        let mut w = World::new(3);
        hide(&mut w, 0, Hide::Buried);
        assert!(!w.families[0].stores.arms.ready());
        assert!(Hide::Buried.found() < Hide::Open.found());
    }

    #[test]
    fn every_shot_spends_a_round() {
        let mut w = World::new(3);
        let r = w.families[0].stores.arms.rounds();
        assert!(fire(&mut w, 0));
        assert_eq!(w.families[0].stores.arms.rounds(), r - 1);
        w.families[0].stores.arms.balls = 0;
        w.families[0].stores.arms.cartridges = 0;
        assert!(!fire(&mut w, 0));
        let _ = PLAYER;
    }

    #[test]
    fn jones_posse_goes_looking() {
        // Wakarusa: called 28 Nov 1855 (day 27); searches three days on.
        let mut w = World::new(4);
        for f in 1..w.families.len() {
            w.families[f].stores.arms.rifles = 2;
        }
        w.run_days(31);
        assert!(
            w.events
                .iter()
                .any(|e| matches!(e.kind, EventKind::Searched { .. }))
        );
    }
}
