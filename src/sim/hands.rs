//! Hands and hired guns (PLAN Phase D). Who works your place, who sits up
//! with a rifle, and who you can pay to do worse.
//!
//! - **Tasks.** Grown kin and hired hands take a job each: the fields, the
//!   stock, the timber, or the watch. A watch at night turns riders back at
//!   the fence before they reach the gate.
//! - **Hired hands** come by the month from households that need the money.
//!   Wages go to their people. An unpaid hand walks, and a sour one talks:
//!   the other side learns where the guns are.
//! - **Companies.** The partisan companies of 1855–56 took pay, provisions
//!   and plunder. Rent one to sit on your place, or to ride on a neighbor's.
//!   They talk, and the other side's paper prints who paid.

use super::calendar::Day;
use super::events::{EventId, EventKind, FireCause, Source, Suspect, WorldEvent};
use super::history::Paper;
use super::psyche::LifeStage;
use super::world::{Faction, FamilyId, NpcId, PLAYER, World};

/// A month's wages and board for a farm hand in the Territory.
pub const WAGE: i32 = 12;
/// Hands a claim can keep busy.
pub const MAX_HANDS: usize = 2;
/// Turned back at the fence: they won't try again for a season.
const TURNED_DAYS: u32 = 60;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Task {
    /// Chores about the house; helps where it's needed.
    #[default]
    Home,
    /// Plow, plant, hay, pick: whatever the season wants.
    Fields,
    /// Rides the fence line; strays get found.
    Stock,
    /// Cuts timber on the creek bottoms.
    Woods,
    /// Sits up at night with a gun across the knees.
    Watch,
}

impl Task {
    pub const ALL: [Task; 5] = [
        Task::Home,
        Task::Fields,
        Task::Stock,
        Task::Woods,
        Task::Watch,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Task::Home => "about the house",
            Task::Fields => "in the fields",
            Task::Stock => "with the stock",
            Task::Woods => "cutting timber",
            Task::Watch => "on watch at night",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Hand {
    pub npc: NpcId,
    /// Dollars on the first of the month.
    pub wage: i32,
    /// 0 sour .. 1 yours. Paid wages and a fair name raise it; an empty
    /// purse and a hot county lower it. Below a third, he talks.
    pub loyalty: f32,
}

/// A partisan company that will ride for pay. Dates are when each was in
/// the county in force (approximate: all four were militias, and none kept
/// a ledger of what it was paid in cash, land, or plunder).
pub struct Company {
    pub name: &'static str,
    pub faction: Faction,
    pub from: (i32, u32, u32),
    pub until: (i32, u32, u32),
    pub men: u8,
    /// Dollars a man a night.
    pub per_man: i32,
    /// Chance each hire gets into the other side's paper.
    pub loose: f32,
}

pub const COMPANIES: &[Company] = &[
    Company {
        name: "the Kickapoo Rangers",
        faction: Faction::ProSlavery,
        from: (1855, 11, 1),
        until: (1856, 9, 15),
        men: 6,
        per_man: 1,
        loose: 0.6,
    },
    Company {
        name: "Buford's Southerners",
        faction: Faction::ProSlavery,
        from: (1856, 4, 25),
        until: (1856, 10, 1),
        men: 8,
        per_man: 2,
        loose: 0.4,
    },
    Company {
        name: "the Stubbs",
        faction: Faction::FreeState,
        from: (1855, 11, 1),
        until: (1857, 12, 31),
        men: 5,
        per_man: 1,
        loose: 0.3,
    },
    Company {
        name: "Lane's Army of the North",
        faction: Faction::FreeState,
        from: (1856, 8, 1),
        until: (1856, 10, 15),
        men: 10,
        per_man: 1,
        loose: 0.5,
    },
];

#[derive(Clone, Debug, Default)]
pub struct Hands {
    /// The player's household: who's doing what. Anyone absent is `Home`.
    pub tasks: Vec<(NpcId, Task)>,
    pub hired: Vec<Hand>,
    /// (company, until): hired guns sitting on your place.
    pub guard: Option<(u8, Day)>,
}

impl Hands {
    pub fn is_hired(&self, id: NpcId) -> bool {
        self.hired.iter().any(|h| h.npc == id)
    }
}

/// What someone in your household is doing.
pub fn task(world: &World, id: NpcId) -> Task {
    world
        .hands
        .tasks
        .iter()
        .find(|t| t.0 == id)
        .map(|t| t.1)
        .unwrap_or_default()
}

/// Who you can set to work: grown kin (not you), and hired hands.
pub fn workers(world: &World) -> Vec<NpcId> {
    world
        .living()
        .filter(|n| n.family == 0 && n.id != PLAYER && LifeStage::of(n.age) != LifeStage::Child)
        .map(|n| n.id)
        .chain(world.hands.hired.iter().map(|h| h.npc))
        .filter(|&id| !super::warrant::held(world, id))
        .collect()
}

pub fn assign(world: &mut World, id: NpcId, t: Task) -> bool {
    if !workers(world).contains(&id) {
        return false;
    }
    world.hands.tasks.retain(|x| x.0 != id);
    if t != Task::Home {
        world.hands.tasks.push((id, t));
    }
    true
}

/// A day's work out of one person: the same reckoning as a household's.
fn labor_of(world: &World, id: NpcId) -> f32 {
    let n = world.npc(id);
    let wound = if n.wounded { 0.3 } else { 1.0 };
    let grief = 1.0 - n.emotions.grief / 200.0;
    (0.5 + n.body.strength) * n.health as f32 / 100.0 * grief * wound / 2.0
}

// ---- hired hands ----------------------------------------------------------

/// Who'd take your wages: grown, not the head of their own house, and from
/// a household that could use the money. Best prospects first.
pub fn candidates(world: &World) -> Vec<NpcId> {
    if world.hands.hired.len() >= MAX_HANDS {
        return Vec::new();
    }
    let mut c: Vec<(NpcId, f32)> = world
        .living()
        .filter(|n| {
            n.family != 0
                && LifeStage::of(n.age) == LifeStage::Adult
                && !world.families[n.family as usize].store
                && world.head_of(n.family) != Some(n.id)
                && !world.hands.is_hired(n.id)
                && !super::warrant::held(world, n.id)
                && world.opinion(n.id, PLAYER) > -20
        })
        .map(|n| {
            let st = &world.families[n.family as usize].stores;
            let need = if st.desperate() { 0.5 } else { 0.0 }
                + (1.0 - st.cash as f32 / 50.0).clamp(0.0, 1.0) * 0.3;
            (n.id, need + world.opinion(n.id, PLAYER) as f32 / 100.0)
        })
        .filter(|&(_, w)| w > 0.2)
        .collect();
    c.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    c.into_iter().take(4).map(|(id, _)| id).collect()
}

pub fn hire(world: &mut World, id: NpcId) -> bool {
    if !candidates(world).contains(&id) {
        return false;
    }
    let hungry = world.families[world.npc(id).family as usize]
        .stores
        .desperate();
    let loyalty =
        (0.45 + world.opinion(id, PLAYER) as f32 / 200.0 + if hungry { 0.1 } else { 0.0 })
            .clamp(0.1, 0.9);
    world.hands.hired.push(Hand {
        npc: id,
        wage: WAGE,
        loyalty,
    });
    world.hands.tasks.push((id, Task::Fields));
    world.emit_root(
        EventKind::HandHired {
            hand: id,
            family: 0,
        },
        None,
    );
    world.run_cascades();
    true
}

pub fn let_go(world: &mut World, id: NpcId) -> bool {
    if !world.hands.is_hired(id) {
        return false;
    }
    quit(world, id, false);
    world.run_cascades();
    true
}

fn quit(world: &mut World, id: NpcId, unpaid: bool) {
    world.hands.hired.retain(|h| h.npc != id);
    world.hands.tasks.retain(|t| t.0 != id);
    world.emit_root(
        EventKind::HandQuit {
            hand: id,
            family: 0,
            unpaid,
        },
        None,
    );
}

// ---- the watch ------------------------------------------------------------

/// Who's sitting up tonight at a place, best eyes first. The player's
/// household by assignment; a neighbor's when they're in a feud and have a
/// grown son or brother to spare.
pub fn watchmen(world: &World, family: FamilyId) -> Vec<NpcId> {
    let mut w: Vec<NpcId> = if family == 0 {
        world
            .hands
            .tasks
            .iter()
            .filter(|t| t.1 == Task::Watch)
            .map(|t| t.0)
            .filter(|&id| world.npc(id).alive && !world.npc(id).wounded)
            .collect()
    } else if world.feuds.iter().any(|&(a, b)| a == family || b == family) {
        let head = world.head_of(family);
        world
            .living()
            .filter(|n| {
                n.family == family
                    && Some(n.id) != head
                    && LifeStage::of(n.age) == LifeStage::Adult
                    && !n.wounded
            })
            .map(|n| n.id)
            .collect()
    } else {
        Vec::new()
    };
    w.sort_by(|&a, &b| {
        world
            .npc(b)
            .body
            .alertness
            .total_cmp(&world.npc(a).body.alertness)
            .then(a.cmp(&b))
    });
    w
}

/// Hired guns on your place tonight.
pub fn guarded(world: &World) -> Option<&'static Company> {
    world
        .hands
        .guard
        .filter(|&(_, until)| world.day < until)
        .map(|(c, _)| &COMPANIES[c as usize])
}

/// Chance riders are seen and turned back at the fence. Alertness is the
/// whole of it; a company sitting on the place is nearly sure.
pub fn watch_odds(world: &World, family: FamilyId) -> f32 {
    if family == 0 && guarded(world).is_some() {
        return 0.9;
    }
    let miss: f32 = watchmen(world, family)
        .iter()
        .take(2)
        .map(|&id| 1.0 - (0.15 + 0.35 * world.npc(id).body.alertness))
        .product();
    1.0 - miss
}

/// Called when a plot comes due: is anyone awake to stop it? The careful
/// (stealth) slip past more often than the angry.
pub fn turn_back(
    world: &mut World,
    actor: NpcId,
    target: NpcId,
    caused_by: Option<EventId>,
) -> bool {
    let fam = world.npc(target).family;
    let odds = watch_odds(world, fam);
    if odds <= 0.0 {
        return false;
    }
    let slip = 0.5 * world.npc(actor).body.stealth;
    if !world.rng.chance(odds * (1.0 - slip)) {
        return false;
    }
    let watchman = if fam == 0 && guarded(world).is_some() {
        None
    } else {
        watchmen(world, fam).first().copied()
    };
    world.emit_root(
        EventKind::TurnedBack {
            rider: actor,
            target,
            watchman,
        },
        caused_by,
    );
    true
}

/// Strays per day, as a share of what they'd be with nobody riding fence.
pub fn herding(world: &World, family: FamilyId) -> f32 {
    if family == 0 && world.hands.tasks.iter().any(|t| t.1 == Task::Stock) {
        0.25
    } else {
        1.0
    }
}

/// Men at your back in a staredown who aren't kin.
pub fn at_back(world: &World) -> usize {
    world
        .hands
        .hired
        .iter()
        .filter(|h| h.loyalty > 0.6 && world.npc(h.npc).alive)
        .count()
        + if guarded(world).is_some() { 3 } else { 0 }
}

// ---- companies ------------------------------------------------------------

pub fn available(world: &World, c: usize) -> bool {
    let co = &COMPANIES[c];
    let today = world.day.date();
    co.from <= today && today <= co.until
}

/// Only a company of your own public side will ride for you.
pub fn will_ride(world: &World, c: usize) -> bool {
    available(world, c) && world.npc(PLAYER).faction == COMPANIES[c].faction
}

pub fn guard_price(c: usize, nights: u32) -> i32 {
    let co = &COMPANIES[c];
    co.men as i32 * co.per_man * nights as i32
}

pub fn raid_price(c: usize) -> i32 {
    guard_price(c, 3)
}

/// Who a company would ride on for you: heads of the other side's houses,
/// the ones you like least first.
pub fn raid_targets(world: &World, c: usize) -> Vec<NpcId> {
    let side = COMPANIES[c].faction;
    let mut t: Vec<NpcId> = (1..world.families.len() as FamilyId)
        .filter(|&f| {
            world.families[f as usize].faction != side && !world.families[f as usize].store
        })
        .filter_map(|f| world.head_of(f))
        .collect();
    t.sort_by_key(|&h| (world.opinion(PLAYER, h), h));
    t.truncate(8);
    t
}

pub fn hire_guard(world: &mut World, c: usize, nights: u32) -> bool {
    let cost = guard_price(c, nights);
    if !will_ride(world, c) || world.families[0].stores.cash < cost || guarded(world).is_some() {
        return false;
    }
    world.families[0].stores.cash -= cost;
    world.hands.guard = Some((c as u8, Day(world.day.0 + nights)));
    let printed = world.rng.chance(COMPANIES[c].loose);
    world.emit_root(
        EventKind::Hirelings {
            company: c as u8,
            payer: PLAYER,
            target: None,
            printed,
        },
        None,
    );
    world.run_cascades();
    true
}

/// Pay a company to ride on a neighbor tonight: the barn if there's one
/// standing, the stock if not. Nobody on that place sees your face; they
/// see strangers. Whether they learn who paid is up to the company's mouth.
pub fn hire_raid(world: &mut World, c: usize, target: NpcId) -> bool {
    let cost = raid_price(c);
    let fam = world.npc(target).family;
    if !will_ride(world, c)
        || world.families[0].stores.cash < cost
        || !raid_targets(world, c).contains(&target)
        || !super::action::night_free(world)
    {
        return false;
    }
    world.families[0].stores.cash -= cost;
    world.action.last_night_out = Some(world.day);
    let me = world.npc_mut(PLAYER);
    me.violence = me.violence.saturating_add(1);
    let printed = world.rng.chance(COMPANIES[c].loose);
    let hire = world.emit_root(
        EventKind::Hirelings {
            company: c as u8,
            payer: PLAYER,
            target: Some(target),
            printed,
        },
        None,
    );
    let act = if world.families[fam as usize].barn_standing {
        let id = super::action::unseen(
            world,
            PLAYER,
            EventKind::Fire {
                owner: target,
                cause: FireCause::Arson(PLAYER),
                spread_from: None,
            },
            Some(hire),
        );
        Some(id)
    } else if world.families[fam as usize].stores.cattle > 0 {
        let before = world.events.len();
        super::action::quietly(world, PLAYER, |w| {
            super::economy::steal_from(w, PLAYER, fam, target)
        });
        world.events[before..]
            .iter()
            .find(|e| matches!(e.kind, EventKind::Theft { .. }))
            .map(|e| e.id)
    } else {
        None
    };
    // The company drinks on it in Westport or Lawrence, and it's in print
    // by the week's end: the victim's side reads who paid.
    if printed && let Some(about) = act {
        let paper = match world.npc(target).faction {
            Faction::FreeState => Paper::HeraldOfFreedom,
            Faction::ProSlavery => Paper::SquatterSovereign,
        };
        world.schedule(
            5,
            EventKind::Headline {
                paper,
                about: Some(about),
                blamed: Some(Suspect::Person(PLAYER)),
                history: None,
            },
            hire,
        );
    }
    world.run_cascades();
    true
}

// ---- the tick -------------------------------------------------------------

pub fn daily(world: &mut World) {
    let today = world.day;
    // The dead, the jailed and the gone don't work.
    let gone: Vec<NpcId> = world
        .hands
        .hired
        .iter()
        .map(|h| h.npc)
        .filter(|&id| !world.npc(id).alive || world.npc(id).departed)
        .collect();
    for id in gone {
        world.hands.hired.retain(|h| h.npc != id);
    }
    let able = workers(world);
    world.hands.tasks.retain(|t| able.contains(&t.0));
    if world.hands.guard.is_some_and(|(_, until)| today >= until) {
        world.hands.guard = None;
    }
    // Neighbors' households work themselves (farmwork); on autopilot, so
    // does yours.
    if world.autopilot_player || !world.player_alive() {
        return;
    }
    let sunday = today.0 % 7 == 3;
    let jobs = world.hands.tasks.clone();
    for (id, t) in jobs {
        let work = labor_of(world, id);
        match t {
            Task::Fields if !sunday => {
                super::farmwork::work(world, 0, work);
            }
            Task::Woods if !sunday && world.rng.chance((0.8 * work).min(0.9)) => {
                super::homestead::cut_timber(world, 0);
            }
            _ => {}
        }
    }
}

/// Wages on the first. A hand of the other side sours as the county
/// heats; a sour hand talks in town about what's in your loft.
pub fn monthly(world: &mut World) {
    let hot = world.grievance;
    let mine = world.npc(PLAYER).faction;
    let hands = world.hands.hired.clone();
    for h in hands {
        let n = world.npc(h.npc);
        let (his, fam) = (n.faction, n.family);
        if world.families[0].stores.cash < h.wage {
            quit(world, h.npc, true);
            continue;
        }
        world.families[0].stores.cash -= h.wage;
        world.families[fam as usize].stores.cash += h.wage;
        let mut loyalty = h.loyalty + 0.05 + world.opinion(h.npc, PLAYER) as f32 / 1000.0;
        if his != mine && hot[his.index()] > 60 {
            loyalty -= 0.12;
        }
        let loyalty = loyalty.clamp(0.0, 1.0);
        if let Some(x) = world.hands.hired.iter_mut().find(|x| x.npc == h.npc) {
            x.loyalty = loyalty;
        }
        if loyalty < 0.34 && world.rng.chance(0.5) {
            world.emit_root(
                EventKind::HandTalked {
                    hand: h.npc,
                    family: 0,
                },
                None,
            );
        }
    }
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::TurnedBack { rider, target, .. } => {
            let n = world.npc_mut(rider);
            n.emotions.fear += 15.0;
            n.last_revenge = Some(ev.day);
            world
                .action
                .cowed
                .push((rider, target, Day(ev.day.0 + TURNED_DAYS)));
        }
        EventKind::HandHired { hand, .. } => {
            world.adjust_opinion(hand, PLAYER, 5);
            let fam = world.npc(hand).family;
            if let Some(head) = world.head_of(fam) {
                world.adjust_opinion(head, PLAYER, 5);
            }
        }
        EventKind::HandQuit { hand, unpaid, .. } => {
            world.adjust_opinion(hand, PLAYER, if unpaid { -20 } else { -5 });
            // Walked off owed wages: he says what he saw on his way out.
            if unpaid {
                world.emit_child(ev, EventKind::HandTalked { hand, family: 0 });
            }
        }
        EventKind::HandTalked { hand, family } => {
            // Only the other side's ears make the loft dangerous.
            if world.npc(hand).faction != world.families[family as usize].faction {
                world.families[family as usize].stores.arms.told = true;
            }
            let fam = world.npc(hand).family;
            if let Some(head) = world.head_of(fam) {
                world.adjust_opinion(head, PLAYER, -5);
            }
        }
        EventKind::Hirelings {
            company,
            payer,
            printed,
            target,
        } => {
            let side = COMPANIES[company as usize].faction;
            let other = match side {
                Faction::FreeState => Faction::ProSlavery,
                Faction::ProSlavery => Faction::FreeState,
            };
            // A guard is a provocation; a raid is war. Either way the other
            // side marks the man who paid, once it's in print.
            if printed {
                world.add_grievance(other, if target.is_some() { 5 } else { 2 });
                let readers: Vec<NpcId> = world
                    .living()
                    .filter(|n| n.faction == other)
                    .map(|n| n.id)
                    .collect();
                for r in readers {
                    world.adjust_opinion(r, payer, if target.is_some() { -10 } else { -4 });
                }
            }
        }
        _ => {}
    }
}

/// For the chronicle: the belief the victim's side reads.
pub fn print_source(company: u8) -> Source {
    match COMPANIES[company as usize].faction {
        Faction::FreeState => Source::Newspaper(Paper::SquatterSovereign),
        Faction::ProSlavery => Source::Newspaper(Paper::HeraldOfFreedom),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::events::Retaliation;

    fn world() -> World {
        let mut w = World::new(7);
        w.autopilot_player = false;
        w
    }

    fn kin(w: &mut World) -> NpcId {
        match workers(w).first() {
            Some(&k) => k,
            None => w.add_npc(0, "Abel", 24),
        }
    }

    #[test]
    fn kin_in_the_fields_plow() {
        let mut w = world();
        // Spring: plowing is due.
        while farm_task(&w) != super::super::farmwork::Task::Plow {
            w.day = Day(w.day.0 + 1);
        }
        let k = kin(&mut w);
        let before = w.families[0].stores.work.plowed;
        assert!(assign(&mut w, k, Task::Fields));
        daily(&mut w);
        daily(&mut w);
        assert!(w.families[0].stores.work.plowed > before);
    }

    fn farm_task(w: &World) -> super::super::farmwork::Task {
        super::super::farmwork::due(w.day)
    }

    #[test]
    fn a_watch_turns_riders_back() {
        let mut w = world();
        let k = kin(&mut w);
        w.npc_mut(k).body.alertness = 1.0;
        assign(&mut w, k, Task::Watch);
        let rider = (1..w.npcs.len() as NpcId)
            .find(|&i| w.npc(i).family != 0 && LifeStage::of(w.npc(i).age) == LifeStage::Adult)
            .unwrap();
        w.npc_mut(rider).body.stealth = 0.0;
        let mut back = 0;
        for _ in 0..200 {
            if turn_back(&mut w, rider, PLAYER, None) {
                back += 1;
            }
        }
        // Half, near enough, with one sharp pair of eyes.
        assert!((70..=130).contains(&back), "{back}");
        w.run_cascades();
        assert!(w.action.cowed(rider, PLAYER, w.day));
        let _ = Retaliation::Arson;
    }

    #[test]
    fn nobody_on_watch_means_no_warning() {
        let w = world();
        assert_eq!(watch_odds(&w, 0), 0.0);
    }

    #[test]
    fn an_unpaid_hand_walks_and_talks() {
        let mut w = world();
        let c = candidates(&w);
        assert!(!c.is_empty(), "somebody needs the money");
        assert!(hire(&mut w, c[0]));
        w.families[0].stores.arms.rifles = 1;
        w.families[0].stores.cash = 0;
        // Make him the other side so the talk lands.
        let other = match w.npc(PLAYER).faction {
            Faction::FreeState => Faction::ProSlavery,
            Faction::ProSlavery => Faction::FreeState,
        };
        w.npc_mut(c[0]).faction = other;
        monthly(&mut w);
        w.run_cascades();
        assert!(w.hands.hired.is_empty());
        assert!(w.families[0].stores.arms.told);
        // Moving the guns puts them back out of mind.
        super::super::arms::hide(&mut w, 0, super::super::arms::Hide::Buried);
        assert!(!w.families[0].stores.arms.told);
    }

    #[test]
    fn wages_go_to_the_hands_people() {
        let mut w = world();
        let c = candidates(&w)[0];
        hire(&mut w, c);
        let fam = w.npc(c).family as usize;
        w.families[0].stores.cash = 100;
        let theirs = w.families[fam].stores.cash;
        monthly(&mut w);
        assert_eq!(w.families[0].stores.cash, 100 - WAGE);
        assert_eq!(w.families[fam].stores.cash, theirs + WAGE);
    }

    #[test]
    fn a_hired_hand_works_for_you_not_his_people() {
        let mut w = world();
        let c = candidates(&w)[0];
        let fam = w.npc(c).family;
        let before = super::super::psyche::labor(&w, fam);
        let mine = super::super::psyche::labor(&w, 0);
        hire(&mut w, c);
        assert!(super::super::psyche::labor(&w, fam) < before);
        assert!(super::super::psyche::labor(&w, 0) > mine);
    }

    #[test]
    fn companies_ride_only_for_their_side_and_in_season() {
        let mut w = world();
        // November 1855: the Stubbs are about; Buford hasn't landed.
        let stubbs = COMPANIES
            .iter()
            .position(|c| c.name == "the Stubbs")
            .unwrap();
        let buford = COMPANIES
            .iter()
            .position(|c| c.name.starts_with("Buford"))
            .unwrap();
        assert!(available(&w, stubbs));
        assert!(!available(&w, buford));
        assert_eq!(w.npc(PLAYER).faction, Faction::FreeState);
        assert!(will_ride(&w, stubbs));
        w.families[0].stores.cash = 500;
        assert!(hire_guard(&mut w, stubbs, 3));
        assert!(guarded(&w).is_some());
        assert_eq!(watch_odds(&w, 0), 0.9);
        w.day = Day(w.day.0 + 3);
        daily(&mut w);
        assert!(guarded(&w).is_none());
    }

    #[test]
    fn a_hired_raid_burns_a_barn_nobody_saw_you_at() {
        let mut w = world();
        let stubbs = COMPANIES
            .iter()
            .position(|c| c.name == "the Stubbs")
            .unwrap();
        w.families[0].stores.cash = 500;
        let t = raid_targets(&w, stubbs)
            .into_iter()
            .find(|&t| w.families[w.npc(t).family as usize].barn_standing)
            .unwrap();
        assert!(hire_raid(&mut w, stubbs, t));
        let fire = w
            .events
            .iter()
            .find(|e| matches!(e.kind, EventKind::Fire { owner, .. } if owner == t))
            .expect("the barn went up");
        // Nobody witnessed you: no belief about it came from their own eyes.
        let fid = fire.id;
        assert!(!w.events.iter().any(|e| matches!(
            e.kind,
            EventKind::Belief { about, source: Source::Witnessed, blamed: Suspect::Person(PLAYER), .. } if about == fid
        )));
        // Once a night.
        assert!(!hire_raid(&mut w, stubbs, t));
    }
}
