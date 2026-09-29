//! Warrants and bounties (PLAN Phase D). The law in Douglas County was a
//! justice appointed by the bogus legislature and a sheriff who rode with
//! Missourians. It saw what everyone else saw: effects, and who people
//! swore did them.
//!
//! - **Complaints.** When enough people who believe a man did a violent
//!   thing (their eyes, their loss, or a neighbor's word; not the paper)
//!   would swear to it, the justice writes a warrant. Belief, not proof: the
//!   wrong man hangs as easily as the right one. The justice listens harder
//!   to his own side, and is slow to paper his own.
//! - **Serving.** A posse of the justice's men, or a hunter after the
//!   bounty, rides out. A man comes in, runs, or fights. Shooting the law
//!   makes a new crime and a bigger price.
//! - **You.** Wanted, you can give yourself up, light out for the States,
//!   lie low in the timber, or meet them at the gate. Or take a paper
//!   yourself: the bounty is paid on delivery, half for a dead man.
//!
//! Warrants are for violence only. There is no paper here on a freedom
//! seeker; that darkness lives at the railroad's door, not in a career.

use super::calendar::Day;
use super::events::{EventId, EventKind, FireCause, Source, Suspect, WorldEvent};
use super::psyche::LifeStage;
use super::world::{NpcId, PLAYER, World, is_woman};

/// A paper nobody serves in four months is dead.
pub const LAPSE_DAYS: u32 = 120;
/// Gone to the States: long enough for the trail to go cold.
pub const RUN_DAYS: u32 = 45;
/// Sworn complaints needed before the justice writes.
const SWORN: f32 = 1.8;
/// Belief strong enough to swear to.
const SWEAR_CONFIDENCE: u8 = 60;
/// Days between writing a paper and riders going out.
const FIRST_TRY: u32 = 5;
/// Lying low in the timber: how long it lasts, and the odds they find you.
const LOW_DAYS: u32 = 7;
const FOUND_LYING_LOW: f32 = 0.35;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Open,
    Served,
    Lapsed,
    /// The man died before he could be served.
    Dead,
}

#[derive(Clone, Debug)]
pub struct Warrant {
    pub accused: NpcId,
    /// The violent act sworn to.
    pub about: EventId,
    pub issued: Day,
    /// Dollars, paid by the Territory on delivery.
    pub bounty: u16,
    /// Who took the paper for the money. None: the sheriff's posse.
    pub hunter: Option<NpcId>,
    /// The next day riders go out.
    pub next_try: Day,
    /// Times they've come back without him. Each time, more men.
    pub tries: u8,
    pub state: State,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held {
    /// In the Lecompton jail.
    Jail,
    /// Gone to the States.
    Fled,
}

#[derive(Clone, Debug, Default)]
pub struct Warrants {
    pub list: Vec<Warrant>,
    /// (accused, act, weight sworn so far, who swore, day of the act).
    complaints: Vec<(NpcId, EventId, f32, Vec<NpcId>, Day)>,
    /// (who, until, why): out of the county's reach for now.
    pub held: Vec<(NpcId, Day, Held)>,
    /// Blood spilled serving a paper. The justice doesn't write on his own
    /// deputies.
    pub lawful: Vec<EventId>,
    /// (accused, warrant, day): trials waiting on the justice.
    pub(crate) trials: Vec<(NpcId, usize, Day)>,
    /// (warrant, who rides with the posse besides the leader).
    pub posse_riders: Vec<(usize, NpcId)>,
    /// You're in the timber until this day.
    pub lying_low: Option<Day>,
    /// Everyone the justice ever convicted: a record follows a man
    /// (`standing`).
    pub record: Vec<NpcId>,
}

/// In jail, or gone. Doesn't plot, vote, muster, or work.
pub fn held(world: &World, id: NpcId) -> bool {
    world
        .warrants
        .held
        .iter()
        .any(|&(who, until, _)| who == id && world.day < until)
}

/// Open papers on a man.
pub fn wanted(world: &World, id: NpcId) -> Vec<usize> {
    world
        .warrants
        .list
        .iter()
        .enumerate()
        .filter(|(_, w)| w.accused == id && w.state == State::Open)
        .map(|(i, _)| i)
        .collect()
}

/// What's on a man's head, all papers together.
pub fn price_on(world: &World, id: NpcId) -> u16 {
    wanted(world, id)
        .iter()
        .map(|&i| world.warrants.list[i].bounty)
        .sum()
}

/// Papers anyone could take at the justice's: open, not on you or yours.
pub fn open_papers(world: &World) -> Vec<usize> {
    world
        .warrants
        .list
        .iter()
        .enumerate()
        .filter(|(_, w)| {
            w.state == State::Open
                && w.accused != PLAYER
                && world.npc(w.accused).family != 0
                && world.npc(w.accused).alive
        })
        .map(|(i, _)| i)
        .collect()
}

fn is_violent(world: &World, about: EventId) -> Option<u16> {
    match world.events[about as usize].kind {
        EventKind::Death {
            killer: Some(_), ..
        } => Some(50),
        EventKind::Wounded { .. } => Some(20),
        EventKind::ShotAt { .. } => Some(15),
        EventKind::Fire {
            cause: FireCause::Arson(_),
            ..
        }
        | EventKind::Fire {
            cause: FireCause::Lightning | FireCause::Hearth | FireCause::EscapedBurn,
            ..
        } => Some(15),
        _ => None,
    }
}

fn justice(world: &World) -> Option<NpcId> {
    world.law.justice.filter(|&j| world.npc(j).alive)
}

/// A belief, sworn to. Enough of them and the justice writes.
fn complain(world: &mut World, ev: &WorldEvent) {
    let EventKind::Belief {
        holder,
        about,
        blamed: Suspect::Person(accused),
        confidence,
        source,
        ..
    } = ev.kind
    else {
        return;
    };
    let Some(judge) = justice(world) else {
        return;
    };
    let Some(base) = is_violent(world, about) else {
        return;
    };
    // An eyewitness is worth an oath; a loss is worth most of one; a
    // neighbor's say-so, a little. The paper is worth nothing in court.
    let oath = match source {
        Source::Witnessed => 1.0,
        Source::Victim => 0.6,
        Source::Told(_) => 0.3,
        _ => 0.0,
    };
    let swears = oath > 0.0;
    let (h, a) = (world.npc(holder), world.npc(accused));
    let (h_side, a_side) = (h.faction, a.faction);
    if !swears
        || confidence < SWEAR_CONFIDENCE
        || accused == holder
        || accused == judge
        || h.family == a.family
        || !a.alive
        || LifeStage::of(h.age) == LifeStage::Child
        || world.warrants.lawful.contains(&about)
        || world
            .warrants
            .list
            .iter()
            .any(|w| w.accused == accused && w.about == about)
    {
        return;
    }
    let jf = world.npc(judge).faction;
    let h_family = h.family;
    // The bogus courts hear their own side at full weight.
    let mut weight = oath * if h_side == jf { 1.0 } else { 0.5 };
    // A woman's oath, a beggar's, a jailbird's: sworn, and weighed light.
    weight *= super::standing::word(world, holder).min(1.0);
    // Against a man of no account it takes fewer oaths.
    let needed = SWORN * super::standing::repute(world, accused).clamp(0.5, 1.0);
    if a_side == jf {
        weight *= 0.5;
    }
    // One household is one voice, however many of them swear.
    let mut family_sworn = 0.0;
    let today = ev.day;
    let c = &mut world.warrants.complaints;
    let i = match c.iter().position(|x| x.0 == accused && x.1 == about) {
        Some(i) => i,
        None => {
            c.push((accused, about, 0.0, Vec::new(), today));
            c.len() - 1
        }
    };
    if c[i].3.contains(&holder) {
        return;
    }
    for &x in &c[i].3 {
        if world.npcs[x as usize].family == h_family {
            family_sworn += 1.0;
        }
    }
    let c = &mut world.warrants.complaints;
    if family_sworn > 0.0 {
        weight *= 0.25;
    }
    c[i].2 += weight;
    c[i].3.push(holder);
    if c[i].2 < needed {
        return;
    }
    c.remove(i);
    let mut bounty = base;
    if a_side != jf {
        bounty = bounty * 3 / 2;
    }
    world.warrants.list.push(Warrant {
        accused,
        about,
        issued: today,
        bounty,
        hunter: None,
        next_try: Day(today.0 + FIRST_TRY),
        tries: 0,
        state: State::Open,
    });
    world.emit_child(
        ev,
        EventKind::Warrant {
            accused,
            about,
            bounty,
        },
    );
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::Belief { .. } => complain(world, ev),
        // A dead man's papers close and his cell empties the day he dies;
        // otherwise the wanted list shows a corpse for two weeks.
        EventKind::Death { victim, .. } | EventKind::Perished { victim, .. } => {
            for w in &mut world.warrants.list {
                if w.accused == victim && w.state == State::Open {
                    w.state = State::Dead;
                }
            }
            world
                .warrants
                .held
                .retain(|h| h.0 != victim || h.2 == Held::Fled);
        }
        EventKind::Warrant { accused, .. } => {
            let n = world.npc_mut(accused);
            n.emotions.fear += 25.0;
            // The accused's people think the law is the other side's.
            let fam = world.npc(accused).family;
            if let Some(judge) = justice(world) {
                let kin: Vec<NpcId> = world
                    .living()
                    .filter(|n| n.family == fam)
                    .map(|n| n.id)
                    .collect();
                for k in kin {
                    world.adjust_opinion(k, judge, -10);
                }
                if world.npc(accused).faction != world.npc(judge).faction {
                    world.add_grievance(world.npc(accused).faction, 2);
                }
            }
        }
        EventKind::Arrested { accused, by } => {
            let fam = world.npc(accused).family;
            let kin: Vec<NpcId> = world
                .living()
                .filter(|n| n.family == fam && n.id != accused)
                .map(|n| n.id)
                .collect();
            for k in kin {
                world.adjust_opinion(k, by, -15);
            }
            world.adjust_opinion(accused, by, -25);
        }
        EventKind::BountyPaid {
            hunter,
            accused,
            dollars,
            ..
        } => {
            let fam = world.npc(hunter).family as usize;
            world.families[fam].stores.cash += dollars as i32;
            // Blood money. His people don't forget who collected it.
            let theirs = world.npc(accused).family;
            let kin: Vec<NpcId> = world
                .living()
                .filter(|n| n.family == theirs)
                .map(|n| n.id)
                .collect();
            for k in kin {
                world.adjust_opinion(k, hunter, -20);
            }
        }
        EventKind::Tried {
            accused,
            convicted,
            fine,
            ..
        } => {
            let fam = world.npc(accused).family as usize;
            world.families[fam].stores.cash -= fine as i32;
            if world.families[fam].stores.cash < 0 {
                let short = -world.families[fam].stores.cash;
                world.families[fam].stores.cash = 0;
                // A man who can't pay the costs works them off in the jail,
                // a dollar a day; the store takes the rest on his name.
                let worked = if convicted { short.min(30) } else { 0 };
                world.families[fam].stores.debt += short - worked;
                if let Some(h) = world.warrants.held.iter_mut().find(|h| h.0 == accused) {
                    h.1 = Day(h.1.0 + worked as u32);
                }
            }
            if convicted
                && let Some(judge) = justice(world)
                && world.npc(accused).faction != world.npc(judge).faction
            {
                world.add_grievance(world.npc(accused).faction, 3);
            }
        }
        _ => {}
    }
}

// ---- serving --------------------------------------------------------------

/// Who rides for the bounty: a greedy man with a steady hand, not kin to the
/// accused, and not his friend.
fn hunter_for(world: &World, accused: NpcId) -> Option<NpcId> {
    let fam = world.npc(accused).family;
    world
        .living()
        .filter(|n| {
            n.id != PLAYER
                && n.id != accused
                && n.family != fam
                && super::law::eligible(world, n.id)
                && !held(world, n.id)
                && n.temperament.honesty < 0.45
                && n.body.marksmanship > 0.45
                && world.opinion(n.id, accused) < 20
        })
        .map(|n| {
            let greed = 1.0 - n.temperament.honesty;
            (
                n.id,
                greed + n.body.marksmanship + 0.1 * n.violence.min(3) as f32,
            )
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|(id, _)| id)
}

/// The sheriff's posse: the justice's side, best shots first, more men each
/// time they come back empty.
fn posse_for(world: &World, w: &Warrant) -> Vec<NpcId> {
    let Some(judge) = justice(world) else {
        return Vec::new();
    };
    let side = world.npc(judge).faction;
    let fam = world.npc(w.accused).family;
    let mut men: Vec<NpcId> = world
        .living()
        .filter(|n| {
            n.id != PLAYER
                && n.faction == side
                && n.family != fam
                && super::law::eligible(world, n.id)
                && !held(world, n.id)
                && !is_woman(&n.name)
        })
        .map(|n| n.id)
        .collect();
    men.sort_by(|&a, &b| {
        world
            .npc(b)
            .body
            .marksmanship
            .total_cmp(&world.npc(a).body.marksmanship)
            .then(a.cmp(&b))
    });
    men.truncate(3 + w.tries.min(3) as usize);
    men
}

pub fn daily(world: &mut World) {
    let today = world.day;
    world.warrants.held.retain(|h| today < h.1);
    world.warrants.complaints.retain(|c| today.0 < c.4.0 + 60);
    if world.warrants.lying_low.is_some_and(|d| today >= d) {
        world.warrants.lying_low = None;
    }
    // Trials first: the justice sits in the morning.
    let due: Vec<(NpcId, usize, Day)> = world
        .warrants
        .trials
        .iter()
        .copied()
        .filter(|t| t.2 <= today)
        .collect();
    world.warrants.trials.retain(|t| t.2 > today);
    for (accused, i, _) in due {
        try_case(world, accused, i, 0.0);
    }
    for i in 0..world.warrants.list.len() {
        let w = &world.warrants.list[i];
        if w.state != State::Open || w.next_try > today {
            continue;
        }
        if !world.npc(w.accused).alive {
            world.warrants.list[i].state = State::Dead;
            continue;
        }
        if today.0 >= w.issued.0 + LAPSE_DAYS {
            world.warrants.list[i].state = State::Lapsed;
            continue;
        }
        if held(world, w.accused) {
            world.warrants.list[i].next_try = Day(today.0 + 14);
            continue;
        }
        serve(world, i);
    }
    world.run_cascades();
}

/// Riders go out on paper `i`.
fn serve(world: &mut World, i: usize) {
    let today = world.day;
    let w = world.warrants.list[i].clone();
    let Some(judge) = justice(world) else {
        return;
    };
    // His own side's men he's slow to paper; he's slower still to send
    // anybody out after them.
    let own = world.npc(w.accused).faction == world.npc(judge).faction;
    world.warrants.list[i].next_try = Day(today.0 + 21);
    world.warrants.list[i].tries = w.tries.saturating_add(1);
    if own && !world.rng.chance(0.3) {
        return;
    }
    // You took this paper: it waits on you.
    if w.hunter == Some(PLAYER) {
        return;
    }
    let hunter = if w.bounty >= 30 {
        hunter_for(world, w.accused)
    } else {
        None
    };
    let (leader, riders) = match hunter {
        Some(h) => (h, Vec::new()),
        None => {
            let mut men = posse_for(world, &w);
            if men.is_empty() {
                return;
            }
            let lead = men.remove(0);
            (lead, men)
        }
    };
    world.warrants.list[i].hunter = hunter;
    let mut riders = riders;
    for &(wi, r) in &world.warrants.posse_riders {
        if wi == i && !riders.contains(&r) {
            riders.push(r);
        }
    }
    if w.accused == PLAYER && !world.autopilot_player {
        at_your_gate(world, i, leader, riders);
        return;
    }
    resolve(world, i, leader, riders);
}

/// They've come for you.
fn at_your_gate(world: &mut World, i: usize, leader: NpcId, riders: Vec<NpcId>) {
    let away = super::family::away(world).is_some();
    let low = world.warrants.lying_low.is_some();
    let found = !away && (!low || world.rng.chance(FOUND_LYING_LOW));
    if !found || world.action.standoff.is_some() {
        world.emit_root(
            EventKind::PosseOut {
                accused: PLAYER,
                leader,
                men: riders.len() as u8 + 1,
                found: false,
            },
            None,
        );
        // They turn the place over looking, and your people take it.
        let kin: Vec<NpcId> = world
            .living()
            .filter(|n| n.family == 0 && n.id != PLAYER)
            .map(|n| n.id)
            .collect();
        for k in kin {
            world.adjust_opinion(k, PLAYER, -3);
            world.adjust_opinion(k, leader, -10);
        }
        return;
    }
    world.emit_root(
        EventKind::PosseOut {
            accused: PLAYER,
            leader,
            men: riders.len() as u8 + 1,
            found: true,
        },
        None,
    );
    super::action::serve_at_gate(world, leader, riders, i);
}

/// An NPC answers the riders: comes in, runs, or fights.
fn resolve(world: &mut World, i: usize, leader: NpcId, riders: Vec<NpcId>) {
    let w = world.warrants.list[i].clone();
    let accused = w.accused;
    let a = world.npc(accused);
    let (t, fear, violence) = (a.temperament, a.emotions.fear, a.violence);
    let flee = (fear / 200.0 + 0.3 * (1.0 - t.courage)).clamp(0.05, 0.6);
    let fight = (0.2 * t.courage + 0.2 * t.temper + 0.06 * violence.min(4) as f32).clamp(0.03, 0.5);
    let men = riders.len() as u8 + 1;
    world.emit_root(
        EventKind::PosseOut {
            accused,
            leader,
            men,
            found: true,
        },
        Some(w.about),
    );
    if world.rng.chance(flee) {
        let days = world.rng.range(45, 120) as u16;
        world
            .warrants
            .held
            .push((accused, Day(world.day.0 + days as u32), Held::Fled));
        world.warrants.list[i].next_try = Day(world.day.0 + days as u32 + 7);
        world.emit_root(EventKind::Fled { accused, days }, Some(w.about));
        return;
    }
    if !world.rng.chance(fight) {
        take_in(world, i, leader);
        return;
    }
    // A fight in the yard. His people stand with him.
    let fam = world.npc(accused).family;
    let kin = world
        .living()
        .filter(|n| n.family == fam && n.id != accused && LifeStage::of(n.age) == LifeStage::Adult)
        .count() as f32;
    let posse: Vec<NpcId> = std::iter::once(leader)
        .chain(riders.iter().copied())
        .collect();
    let theirs: f32 = posse.iter().map(|&p| world.npc(p).body.marksmanship).sum();
    let his = world.npc(accused).body.marksmanship + 0.5 * t.courage + 0.3 * kin;
    let mut eyes: Vec<NpcId> = world
        .living()
        .filter(|n| n.family == fam)
        .map(|n| n.id)
        .collect();
    eyes.extend(posse.iter().copied());
    if world.rng.chance(his / (his + 0.8 * theirs)) {
        // He drove them off, and put one of them down doing it.
        let hit = posse[world.rng.range(0, posse.len() as u32) as usize];
        let kind = if world.rng.chance(0.2) {
            EventKind::Death {
                victim: hit,
                killer: Some(accused),
            }
        } else {
            EventKind::Wounded {
                victim: hit,
                attacker: accused,
            }
        };
        let n = world.npc_mut(accused);
        n.violence = n.violence.saturating_add(1);
        super::action::witnessed(world, accused, eyes, kind, Some(w.about));
        // Resisting: the price goes up.
        let b = &mut world.warrants.list[i].bounty;
        *b = b.saturating_mul(2).min(500);
    } else {
        let dead = world.rng.chance(0.25);
        let kind = if dead {
            EventKind::Death {
                victim: accused,
                killer: Some(leader),
            }
        } else {
            EventKind::Wounded {
                victim: accused,
                attacker: leader,
            }
        };
        super::action::witnessed_lawful(world, leader, eyes, kind, Some(w.about));
        if dead {
            finish_dead(world, i, leader);
        } else {
            take_in(world, i, leader);
        }
    }
}

/// He comes in (or is carried): arrested, the bounty paid, a trial set.
fn take_in(world: &mut World, i: usize, by: NpcId) {
    let w = world.warrants.list[i].clone();
    world.warrants.list[i].state = State::Served;
    world.emit_root(
        EventKind::Arrested {
            accused: w.accused,
            by,
        },
        Some(w.about),
    );
    if let Some(h) = w.hunter {
        world.emit_root(
            EventKind::BountyPaid {
                hunter: h,
                accused: w.accused,
                dollars: w.bounty,
                dead: false,
            },
            None,
        );
    } else {
        pay_posse(world, i, w.bounty);
    }
    world
        .warrants
        .trials
        .push((w.accused, i, Day(world.day.0 + 5)));
    // Held till the trial.
    world
        .warrants
        .held
        .push((w.accused, Day(world.day.0 + 5), Held::Jail));
    if w.accused == PLAYER {
        super::family::leave_for(world, super::family::Errand::Jail, 5);
    }
}

fn finish_dead(world: &mut World, i: usize, by: NpcId) {
    let w = world.warrants.list[i].clone();
    world.warrants.list[i].state = State::Served;
    let hunter = w.hunter.unwrap_or(by);
    world.emit_root(
        EventKind::BountyPaid {
            hunter,
            accused: w.accused,
            dollars: w.bounty / 2,
            dead: true,
        },
        None,
    );
}

/// You rode with the posse: a share, if it came in.
fn pay_posse(world: &mut World, i: usize, bounty: u16) {
    let riders: Vec<NpcId> = world
        .warrants
        .posse_riders
        .iter()
        .filter(|r| r.0 == i)
        .map(|r| r.1)
        .collect();
    world.warrants.posse_riders.retain(|r| r.0 != i);
    if riders.contains(&PLAYER) {
        let share = bounty / (riders.len() as u16 + 3);
        let accused = world.warrants.list[i].accused;
        world.emit_root(
            EventKind::BountyPaid {
                hunter: PLAYER,
                accused,
                dollars: share.max(1),
                dead: false,
            },
            None,
        );
    }
}

/// Before the justice. Sworn belief decides it; so does which side you're on.
/// `plea`: coming in on your own shades it your way.
fn try_case(world: &mut World, accused: NpcId, i: usize, plea: f32) {
    let Some(judge) = justice(world) else {
        return;
    };
    // No court tries a corpse.
    if !world.npc(accused).alive {
        return;
    }
    let w = world.warrants.list[i].clone();
    let other = world.npc(accused).faction != world.npc(judge).faction;
    let mut p: f32 = if other { 0.75 } else { 0.3 };
    if accused == PLAYER {
        // A good tongue before a bad court.
        p -= 0.25 * world.life.skill(super::life::Skill::Oratory);
    }
    p -= plea;
    // Nobody to speak to his character.
    p += 0.3 * (1.0 - super::standing::repute(world, accused)).max(0.0);
    let convicted = world.rng.chance(p.clamp(0.05, 0.95));
    let killing = matches!(world.events[w.about as usize].kind, EventKind::Death { .. });
    let (days, fine) = if convicted {
        (if killing { 90 } else { 30 }, w.bounty / 2)
    } else {
        (0, 0)
    };
    world.emit_root(
        EventKind::Tried {
            accused,
            judge,
            convicted,
            days,
            fine,
        },
        Some(w.about),
    );
    if convicted {
        if !world.warrants.record.contains(&accused) {
            world.warrants.record.push(accused);
        }
        let until = Day(world.day.0 + days as u32);
        world.warrants.held.retain(|h| h.0 != accused);
        world.warrants.held.push((accused, until, Held::Jail));
        if accused == PLAYER {
            super::family::leave_for(world, super::family::Errand::Jail, days as u32);
        }
    }
}

// ---- the player -----------------------------------------------------------

/// Write a paper on someone for an act, now. For the view's BK_PLAY hook
/// and the lab; the county writes its own through `complain`.
pub fn write(world: &mut World, accused: NpcId, about: EventId, bounty: u16) -> usize {
    world.warrants.list.push(Warrant {
        accused,
        about,
        issued: world.day,
        bounty,
        hunter: None,
        next_try: world.day,
        tries: 0,
        state: State::Open,
    });
    world.emit_root(
        EventKind::Warrant {
            accused,
            about,
            bounty,
        },
        None,
    );
    world.run_cascades();
    world.warrants.list.len() - 1
}

/// Riders out on paper `i` today, whatever the schedule said.
pub fn serve_now(world: &mut World, i: usize) {
    world.warrants.list[i].next_try = world.day;
    serve(world, i);
    world.run_cascades();
}

/// Walk into Lecompton and give yourself up. Tried the same day.
pub fn surrender(world: &mut World) -> bool {
    let papers = wanted(world, PLAYER);
    let Some(&i) = papers.first() else {
        return false;
    };
    if justice(world).is_none() || world.life.acted_on == Some(world.day) {
        return false;
    }
    world.life.acted_on = Some(world.day);
    for &p in &papers {
        world.warrants.list[p].state = State::Served;
    }
    let judge = justice(world).unwrap_or(PLAYER);
    world.emit_root(
        EventKind::Arrested {
            accused: PLAYER,
            by: judge,
        },
        None,
    );
    try_case(world, PLAYER, i, 0.1);
    world.run_cascades();
    true
}

/// Light out for the States until it blows over. The farm goes on without
/// you, badly.
pub fn run(world: &mut World) -> bool {
    if wanted(world, PLAYER).is_empty() || super::family::away(world).is_some() {
        return false;
    }
    super::family::leave(world, super::family::Errand::Fled);
    let days = RUN_DAYS as u16;
    world.emit_root(
        EventKind::Fled {
            accused: PLAYER,
            days,
        },
        None,
    );
    world.run_cascades();
    true
}

/// A week in the timber. The riders may miss you; the work will.
pub fn lie_low(world: &mut World) -> bool {
    if wanted(world, PLAYER).is_empty() || world.life.acted_on == Some(world.day) {
        return false;
    }
    world.life.acted_on = Some(world.day);
    world.warrants.lying_low = Some(Day(world.day.0 + LOW_DAYS));
    true
}

/// Take a paper at the justice's. Someone else may already be after him.
pub fn take(world: &mut World, i: usize) -> bool {
    if !open_papers(world).contains(&i) || world.warrants.list[i].hunter == Some(PLAYER) {
        return false;
    }
    world.warrants.list[i].hunter = Some(PLAYER);
    true
}

/// Ride along with the sheriff's men on paper `i`: a share if he comes in.
pub fn ride_with(world: &mut World, i: usize) -> bool {
    if !open_papers(world).contains(&i)
        || world.warrants.list[i].hunter == Some(PLAYER)
        || world.life.acted_on == Some(world.day)
        || world
            .warrants
            .posse_riders
            .iter()
            .any(|r| r.0 == i && r.1 == PLAYER)
    {
        return false;
    }
    world.life.acted_on = Some(world.day);
    world.warrants.posse_riders.push((i, PLAYER));
    // Out tomorrow, whenever they meant to go.
    world.warrants.list[i].next_try = Day(world.day.0 + 1);
    super::family::leave(world, super::family::Errand::Posse);
    let accused = world.warrants.list[i].accused;
    let fam = world.npc(accused).family;
    let kin: Vec<NpcId> = world
        .living()
        .filter(|n| n.family == fam)
        .map(|n| n.id)
        .collect();
    for k in kin {
        world.adjust_opinion(k, PLAYER, -15);
    }
    true
}

/// Go after the man on a paper you took. He may be gone when you get there.
pub fn ride_after(world: &mut World, i: usize) -> bool {
    let Some(w) = world.warrants.list.get(i).cloned() else {
        return false;
    };
    if w.state != State::Open || w.hunter != Some(PLAYER) || held(world, w.accused) {
        return false;
    }
    let a = world.npc(w.accused);
    let flee = (a.emotions.fear / 250.0 + 0.2 * (1.0 - a.temperament.courage)).clamp(0.03, 0.4);
    if !super::action::confront(world, w.accused) {
        return false;
    }
    if world.rng.chance(flee) {
        world.action.standoff = None;
        let days = world.rng.range(45, 120) as u16;
        world
            .warrants
            .held
            .push((w.accused, Day(world.day.0 + days as u32), Held::Fled));
        world.emit_root(
            EventKind::Fled {
                accused: w.accused,
                days,
            },
            Some(w.about),
        );
        world.run_cascades();
        return true;
    }
    if let Some(s) = world.action.standoff.as_mut() {
        s.serving = Some(i);
    }
    true
}

/// How a standoff over paper `i` settles, from the law's side. Returns the
/// violence it comes to, if any: (who did it, what, lawful).
pub(crate) fn settle(
    world: &mut World,
    i: usize,
    yours: bool,
    actor: NpcId,
    end: super::action::End,
) -> Option<(NpcId, EventKind, bool)> {
    use super::action::End;
    match (yours, end) {
        // You went for him and he came in.
        (true, End::TalkedDown | End::FacedDown) => {
            take_in(world, i, PLAYER);
            None
        }
        (true, End::Shot { killed }) => {
            if killed {
                finish_dead(world, i, PLAYER);
            } else {
                take_in(world, i, PLAYER);
            }
            Some((
                PLAYER,
                if killed {
                    EventKind::Death {
                        victim: actor,
                        killer: Some(PLAYER),
                    }
                } else {
                    EventKind::Wounded {
                        victim: actor,
                        attacker: PLAYER,
                    }
                },
                true,
            ))
        }
        // He was quicker. That's a new crime of his own.
        (true, End::Beaten) => Some((
            actor,
            EventKind::Wounded {
                victim: PLAYER,
                attacker: actor,
            },
            false,
        )),
        (true, End::BackedDown) => None,
        // They came for you and you sent them off: they'll be back with more.
        (false, End::TalkedDown | End::FacedDown) => {
            let w = &mut world.warrants.list[i];
            w.next_try = Day(world.day.0 + 21);
            None
        }
        (false, End::BackedDown) => {
            take_in(world, i, actor);
            None
        }
        // You shot the law. Now there's a price worth riding for.
        (false, End::Shot { killed }) => {
            let w = &mut world.warrants.list[i];
            w.bounty = w.bounty.saturating_mul(2).min(500);
            w.next_try = Day(world.day.0 + 10);
            Some((
                PLAYER,
                if killed {
                    EventKind::Death {
                        victim: actor,
                        killer: Some(PLAYER),
                    }
                } else {
                    EventKind::Wounded {
                        victim: actor,
                        attacker: PLAYER,
                    }
                },
                false,
            ))
        }
        // Beaten at your own gate: they carry you in.
        (false, End::Beaten) => {
            take_in(world, i, actor);
            Some((
                actor,
                EventKind::Wounded {
                    victim: PLAYER,
                    attacker: actor,
                },
                true,
            ))
        }
    }
}

/// The law's year at a glance, for the lab and the survey. `wrong_man`:
/// papers written on someone who didn't do it (the chronicle never says).
#[derive(Clone, Copy, Debug, Default)]
pub struct Ledger {
    pub papers: usize,
    pub wrong_man: usize,
    pub arrested: usize,
    pub convicted: usize,
    pub fled: usize,
    pub lapsed: usize,
    /// Killed or wounded by a posse or hunter serving a paper.
    pub shot_by_law: usize,
    /// Posse men shot by the man they came for.
    pub law_shot: usize,
    pub bounties_paid: u32,
}

/// Who really did it, for the lens only.
pub fn truth(world: &World, about: EventId) -> Option<NpcId> {
    match world.events[about as usize].kind {
        EventKind::Death { killer, .. } => killer,
        EventKind::Wounded { attacker, .. } => Some(attacker),
        EventKind::ShotAt { shooter, .. } => Some(shooter),
        EventKind::Fire {
            cause: FireCause::Arson(a),
            ..
        } => Some(a),
        _ => None,
    }
}

pub fn ledger(world: &World) -> Ledger {
    let mut l = Ledger::default();
    for w in &world.warrants.list {
        l.papers += 1;
        if truth(world, w.about) != Some(w.accused) {
            l.wrong_man += 1;
        }
        if w.state == State::Lapsed {
            l.lapsed += 1;
        }
    }
    for e in &world.events {
        match e.kind {
            EventKind::Arrested { .. } => l.arrested += 1,
            EventKind::Tried {
                convicted: true, ..
            } => l.convicted += 1,
            EventKind::Fled { .. } => l.fled += 1,
            EventKind::BountyPaid { dollars, .. } => l.bounties_paid += dollars as u32,
            EventKind::Death { .. } | EventKind::Wounded { .. }
                if world.warrants.lawful.contains(&e.id) =>
            {
                l.shot_by_law += 1
            }
            _ => {}
        }
    }
    // A man shooting back at riders who came with a paper.
    l.law_shot = world
        .events
        .iter()
        .filter(|e| {
            matches!(e.kind, EventKind::Death { .. } | EventKind::Wounded { .. })
                && e.caused_by
                    .is_some_and(|c| world.warrants.list.iter().any(|w| w.about == c))
                && !world.warrants.lawful.contains(&e.id)
        })
        .count();
    l
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::action::{self, End};
    use crate::sim::world::Faction;

    fn world() -> World {
        let mut w = World::new(11);
        w.autopilot_player = false;
        w
    }

    /// A pro-slavery justice, and someone of each side to swear and be sworn at.
    fn cast(w: &World) -> (NpcId, NpcId, NpcId) {
        let judge = w.law.justice.unwrap();
        let pick = |f: Faction, not: &[NpcId]| {
            (1..w.npcs.len() as NpcId)
                .find(|&i| {
                    let n = w.npc(i);
                    n.family != 0
                        && n.faction == f
                        && LifeStage::of(n.age) == LifeStage::Adult
                        && !not.contains(&i)
                        && !not.iter().any(|&x| w.npc(x).family == n.family)
                        && !w.families[n.family as usize].store
                })
                .unwrap()
        };
        let fs = pick(Faction::FreeState, &[judge]);
        let ps = pick(Faction::ProSlavery, &[judge, fs]);
        (judge, fs, ps)
    }

    /// Grown people of a side, one from each household.
    fn one_a_house(w: &World, side: Faction, not: &[u32]) -> Vec<NpcId> {
        let mut seen = not.to_vec();
        let mut out = Vec::new();
        for n in w.living() {
            // Men: these tests are about sides, not standing (see
            // `standing` for whose oath weighs less; `swear` gives them
            // their letters).
            if n.faction == side
                && LifeStage::of(n.age) == LifeStage::Adult
                && Some(n.id) != w.law.justice
                && !seen.contains(&n.family)
                && !crate::sim::world::is_woman(&n.name)
            {
                seen.push(n.family);
                out.push(n.id);
            }
        }
        out
    }

    fn a_killing(w: &mut World, victim: NpcId, killer: NpcId) -> EventId {
        w.emit_root(
            EventKind::Death {
                victim,
                killer: Some(killer),
            },
            None,
        )
    }

    fn swear(w: &mut World, holder: NpcId, about: EventId, blamed: NpcId) {
        w.npc_mut(holder).temperament.literacy = 0.9;
        let f = w.npc(holder).family as usize;
        w.families[f].stores.debt = 0;
        w.families[f].stores.hungry_days = 0;
        w.families[f].stores.begged_on = None;
        w.emit_root(
            EventKind::Belief {
                holder,
                about,
                blamed: Suspect::Person(blamed),
                confidence: 95,
                source: Source::Witnessed,
                reason: "test",
            },
            None,
        );
        w.run_cascades();
    }

    #[test]
    fn the_wrong_man_gets_the_paper() {
        let mut w = world();
        let (_, fs, ps) = cast(&w);
        // Pro-slavery killed; the justice's side swears it was the Free-Stater.
        let victim = (1..w.npcs.len() as NpcId)
            .find(|&i| {
                w.npc(i).faction == Faction::ProSlavery
                    && i != ps
                    && w.npc(i).family != w.npc(ps).family
                    && Some(i) != w.law.justice
            })
            .unwrap();
        let id = a_killing(&mut w, victim, ps);
        w.run_cascades();
        w.warrants.list.clear();
        let swearers = one_a_house(
            &w,
            Faction::ProSlavery,
            &[w.npc(fs).family, w.npc(ps).family],
        );
        for &s in &swearers[..2] {
            swear(&mut w, s, id, fs);
        }
        assert_eq!(wanted(&w, fs).len(), 1, "belief wrote the paper");
        assert!(wanted(&w, ps).is_empty(), "the man who did it walks");
        assert_eq!(
            w.warrants.list[0].bounty, 75,
            "half again for the other side"
        );
    }

    #[test]
    fn the_justice_is_slow_on_his_own() {
        let mut w = world();
        let (_, fs, ps) = cast(&w);
        let id = a_killing(&mut w, fs, ps);
        w.run_cascades();
        w.warrants.list.clear();
        let fs_men = one_a_house(
            &w,
            Faction::FreeState,
            &[w.npc(ps).family, w.npc(fs).family],
        );
        // Free-State eyewitnesses against a pro-slavery man: a quarter each.
        // It takes eight households to move him; the other way, two.
        for &s in fs_men.iter().take(5) {
            swear(&mut w, s, id, ps);
        }
        assert!(wanted(&w, ps).is_empty());
    }

    #[test]
    fn the_paper_is_for_violence_only() {
        let mut w = world();
        let (_, fs, ps) = cast(&w);
        let theft = w.emit_root(
            EventKind::Theft {
                thief: fs,
                victim: ps,
                loot: crate::sim::events::Loot::Cow,
            },
            None,
        );
        w.run_cascades();
        w.warrants.list.clear();
        let swearers: Vec<NpcId> = w
            .living()
            .filter(|n| n.faction == Faction::ProSlavery && n.family != w.npc(fs).family)
            .map(|n| n.id)
            .take(3)
            .collect();
        for s in swearers {
            swear(&mut w, s, theft, fs);
        }
        assert!(wanted(&w, fs).is_empty());
    }

    fn paper_on(w: &mut World, accused: NpcId) -> usize {
        let other = (1..w.npcs.len() as NpcId)
            .find(|&i| w.npc(i).family != w.npc(accused).family && i != accused)
            .unwrap();
        let about = a_killing(w, other, accused);
        w.warrants.list.push(Warrant {
            accused,
            about,
            issued: w.day,
            bounty: 60,
            hunter: None,
            next_try: w.day,
            tries: 0,
            state: State::Open,
        });
        w.warrants.list.len() - 1
    }

    #[test]
    fn they_come_to_your_gate() {
        let mut w = world();
        let i = paper_on(&mut w, PLAYER);
        daily(&mut w);
        let s = w.action.standoff.as_ref().expect("riders at the gate");
        assert_eq!(s.serving, Some(i));
        // Stand aside and they take you in; the trial's set.
        action::back_down(&mut w);
        assert!(w.warrants.list[i].state == State::Served);
        assert!(held(&w, PLAYER));
        assert!(crate::sim::family::away(&w).is_some());
        for _ in 0..6 {
            w.advance_day();
        }
        assert!(w.events.iter().any(|e| matches!(
            e.kind,
            EventKind::Tried {
                accused: PLAYER,
                ..
            }
        )));
    }

    #[test]
    fn gone_to_the_states_they_find_nobody() {
        let mut w = world();
        let i = paper_on(&mut w, PLAYER);
        assert!(run(&mut w));
        daily(&mut w);
        assert!(w.action.standoff.is_none());
        assert!(w.events.iter().any(|e| matches!(
            e.kind,
            EventKind::PosseOut {
                accused: PLAYER,
                found: false,
                ..
            }
        )));
        assert_eq!(w.warrants.list[i].state, State::Open);
    }

    #[test]
    fn papers_lapse() {
        let mut w = world();
        let i = paper_on(&mut w, PLAYER);
        w.warrants.list[i].next_try = Day(w.day.0 + LAPSE_DAYS + 1);
        w.day = Day(w.day.0 + LAPSE_DAYS + 1);
        daily(&mut w);
        assert_eq!(w.warrants.list[i].state, State::Lapsed);
    }

    #[test]
    fn a_bounty_on_delivery() {
        let mut w = world();
        let (_, fs, _) = cast(&w);
        let i = paper_on(&mut w, fs);
        assert!(take(&mut w, i));
        // Keep him home and brave so he's there to face.
        w.npc_mut(fs).emotions.fear = 0.0;
        w.npc_mut(fs).temperament.courage = 1.0;
        assert!(ride_after(&mut w, i));
        assert_eq!(w.action.standoff.as_ref().unwrap().serving, Some(i));
        let cash = w.families[0].stores.cash;
        let _ = action::face(&mut w, 1.0);
        assert_eq!(w.warrants.list[i].state, State::Served);
        assert_eq!(w.families[0].stores.cash, cash + 60);
        assert!(
            w.events
                .iter()
                .any(|e| matches!(e.kind, EventKind::Arrested { by: PLAYER, .. }))
        );
    }

    #[test]
    fn a_lawful_killing_writes_no_paper() {
        let mut w = world();
        let (_, fs, _) = cast(&w);
        let i = paper_on(&mut w, fs);
        take(&mut w, i);
        w.npc_mut(fs).emotions.fear = 0.0;
        w.npc_mut(fs).temperament.courage = 1.0;
        let a = &mut w.families[0].stores.arms;
        a.guns = 1;
        a.balls = 5;
        a.hide = crate::sim::arms::Hide::Open;
        assert!(ride_after(&mut w, i));
        let cash = w.families[0].stores.cash;
        action::draw(&mut w, action::DrawEnd::Fired(action::Shot::Kill));
        assert!(w.events.iter().any(|e| matches!(
            e.kind,
            EventKind::Death { victim, killer: Some(PLAYER) } if victim == fs
        )));
        assert!(wanted(&w, PLAYER).is_empty(), "no paper on the law's man");
        // Half for a dead man.
        assert_eq!(w.families[0].stores.cash, cash + 30);
        let _ = End::TalkedDown;
    }

    #[test]
    fn jailed_men_dont_plot() {
        let mut w = world();
        let (_, fs, _) = cast(&w);
        w.warrants.held.push((fs, Day(w.day.0 + 30), Held::Jail));
        assert!(held(&w, fs));
        assert!(!crate::sim::law::eligible(&w, fs));
        w.day = Day(w.day.0 + 31);
        daily(&mut w);
        assert!(!held(&w, fs));
    }
}
