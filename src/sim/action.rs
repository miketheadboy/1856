//! Action, and how to avoid it (PLAN Phase B). The sim sets the terms; the
//! player's hands decide the rest.
//!
//! - **Standoffs.** A plot against you that comes due while you're home is
//!   parked at the gate instead of resolved. You talk them down, face them
//!   down, draw, or back down. Or you ride to them and start it yourself.
//! - **Raids.** A neighbor's place at night: what's there to hit, who's
//!   awake, whether there's a dog, how bright the moon is.
//! - **Ambush.** A man on the road after dark, maybe not alone.
//!
//! When the player plays an act out, the minigame knows who saw: that list
//! is *staged* here and the perception system uses it instead of the dice.
//! Nobody who didn't see gets told the truth; they run attribution as always,
//! and can blame whoever they already hated.

use super::calendar::Day;
use super::character::{self, Archetype};
use super::events::{Cruelty, EventId, EventKind, FireCause, Retaliation};
use super::homestead::{self, Improvement};
use super::life::Skill;
use super::psyche::LifeStage;
use super::world::{FamilyId, NpcId, PLAYER, World};

/// Faced down or talked down: they won't come again until this day.
const COWED_DAYS: u32 = 150;
const SETTLED_DAYS: u32 = 90;

#[derive(Clone, Debug, Default)]
pub struct Action {
    /// Someone at the gate (or you at theirs), waiting on your answer.
    pub standoff: Option<Standoff>,
    /// Who saw the act being played out: (the actor, the eyes).
    staged: Option<(NpcId, Vec<NpcId>)>,
    /// (who, of whom, until): men who won't ride on someone for a while.
    pub cowed: Vec<(NpcId, NpcId, Day)>,
    /// One night out a night.
    pub last_night_out: Option<Day>,
}

impl Action {
    /// Is `who` holding off on `whom`? Read by the revenge system.
    pub fn cowed(&self, who: NpcId, whom: NpcId, today: Day) -> bool {
        self.cowed
            .iter()
            .any(|&(a, b, until)| a == who && b == whom && today < until)
    }

    /// The staged witnesses for an act by `actor`, if the act was played out.
    pub fn staged_for(&self, actor: Option<NpcId>) -> Option<&[NpcId]> {
        match (&self.staged, actor) {
            (Some((a, eyes)), Some(b)) if *a == b => Some(eyes),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Standoff {
    pub actor: NpcId,
    pub method: Retaliation,
    pub caused_by: Option<EventId>,
    pub day: Day,
    /// You rode to them.
    pub yours: bool,
    /// Kin who came along. They see everything.
    pub riders: Vec<NpcId>,
    /// A warrant is being served, on you or by you (index into the list).
    pub serving: Option<usize>,
}

/// What's driving the man in front of you. The talk-down reads it off him.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    Grief,
    Rage,
    Fear,
    Greed,
}

impl Mood {
    pub const ALL: [Mood; 4] = [Mood::Grief, Mood::Rage, Mood::Fear, Mood::Greed];
}

/// Where a talk or a stare goes next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turn {
    Settled,
    Draw,
}

/// Where your ball went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shot {
    Kill,
    Wound,
    Miss,
}

/// How the draw went from your side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawEnd {
    Fired(Shot),
    /// Moved before the moment. They don't wait for a second one.
    Flinched,
    /// They were quicker.
    Slow,
}

/// How a standoff ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    TalkedDown,
    FacedDown,
    Shot { killed: bool },
    Beaten,
    BackedDown,
}

// ---- standoffs ------------------------------------------------------------

/// Hold a plot at the gate for the player to answer. False if it should just
/// happen (you're away, it's autopilot, somebody's already there).
pub fn park(
    world: &mut World,
    actor: NpcId,
    method: Retaliation,
    caused_by: Option<EventId>,
) -> bool {
    if world.autopilot_player
        || !world.player_alive()
        || world.action.standoff.is_some()
        || super::family::away(world).is_some()
    {
        return false;
    }
    let fam = world.npc(actor).family;
    let kin: Vec<NpcId> = world
        .living()
        .filter(|n| n.family == fam && n.id != actor && LifeStage::of(n.age) == LifeStage::Adult)
        .map(|n| n.id)
        .collect();
    let mut riders = Vec::new();
    for k in kin {
        if riders.len() < 2 && world.rng.chance(0.4) {
            riders.push(k);
        }
    }
    world.npc_mut(actor).alibi = None;
    world.action.standoff = Some(Standoff {
        actor,
        method,
        caused_by,
        day: world.day,
        yours: false,
        riders,
        serving: None,
    });
    world.emit_root(
        EventKind::RidersAtGate {
            actor,
            target: PLAYER,
            method,
        },
        caused_by,
    );
    world.run_cascades();
    true
}

/// Ride to someone's door and have it out. Takes the day.
pub fn confront(world: &mut World, t: NpcId) -> bool {
    let n = world.npc(t);
    if !world.player_alive()
        || t == PLAYER
        || !n.alive
        || n.family == 0
        || LifeStage::of(n.age) == LifeStage::Child
        || world.action.standoff.is_some()
        || world.life.acted_on == Some(world.day)
        || super::family::away(world).is_some()
    {
        return false;
    }
    world.life.acted_on = Some(world.day);
    world.action.standoff = Some(Standoff {
        actor: t,
        method: Retaliation::Ambush,
        caused_by: None,
        day: world.day,
        yours: true,
        riders: Vec::new(),
        serving: None,
    });
    true
}

/// A posse or a hunter at your gate with a paper on you.
pub(crate) fn serve_at_gate(world: &mut World, leader: NpcId, riders: Vec<NpcId>, paper: usize) {
    world.npc_mut(leader).alibi = None;
    world.action.standoff = Some(Standoff {
        actor: leader,
        method: Retaliation::Ambush,
        caused_by: None,
        day: world.day,
        yours: false,
        riders,
        serving: Some(paper),
    });
}

/// An act with known eyes: they see who did it; everyone else guesses.
pub(crate) fn witnessed(
    world: &mut World,
    actor: NpcId,
    eyes: Vec<NpcId>,
    kind: EventKind,
    caused_by: Option<EventId>,
) -> EventId {
    world.action.staged = Some((actor, eyes));
    let id = world.emit_root(kind, caused_by);
    world.run_cascades();
    world.action.staged = None;
    id
}

/// The same, done serving a warrant: no paper gets written on it.
pub(crate) fn witnessed_lawful(
    world: &mut World,
    actor: NpcId,
    eyes: Vec<NpcId>,
    kind: EventKind,
    caused_by: Option<EventId>,
) -> EventId {
    world.action.staged = Some((actor, eyes));
    let id = world.emit_root(kind, caused_by);
    world.warrants.lawful.push(id);
    world.run_cascades();
    world.action.staged = None;
    id
}

/// Done by strangers in the dark on someone's pay: nobody saw a face.
pub(crate) fn unseen(
    world: &mut World,
    actor: NpcId,
    kind: EventKind,
    caused_by: Option<EventId>,
) -> EventId {
    witnessed(world, actor, Vec::new(), kind, caused_by)
}

/// Run something that emits acts by `actor` with nobody's eyes on them.
pub(crate) fn quietly(world: &mut World, actor: NpcId, f: impl FnOnce(&mut World)) {
    world.action.staged = Some((actor, Vec::new()));
    f(world);
    world.run_cascades();
    world.action.staged = None;
}

/// The two things most on his mind, strongest first.
pub fn moods(world: &World, actor: NpcId) -> [Mood; 2] {
    let n = world.npc(actor);
    let e = &n.emotions;
    let mut s = [
        (Mood::Grief, e.grief),
        (Mood::Rage, e.anger + 30.0 * n.temperament.temper),
        (Mood::Fear, e.fear + 20.0 * (1.0 - n.temperament.courage)),
        (Mood::Greed, 40.0 * (1.0 - n.temperament.honesty)),
    ];
    s.sort_by(|a, b| b.1.total_cmp(&a.1));
    [s[0].0, s[1].0]
}

/// Seconds from the moment to his gun going off. Human hands are about a
/// quarter second; a bushwhacker is about that too.
pub fn their_draw(world: &World, actor: NpcId) -> f32 {
    let n = world.npc(actor);
    let mut t = 0.66 - 0.30 * n.body.marksmanship - 0.10 * n.temperament.courage;
    if character::is(n, Archetype::Bushwhacker) {
        t -= 0.08;
    }
    if n.wounded {
        t += 0.15;
    }
    // Liquor slows everybody but the man who thinks it doesn't.
    t.clamp(0.30, 0.75)
}

/// How much your sights wander, 0 steady .. 1 all over. Marksmanship
/// steadies; whiskey today doesn't.
pub fn sway(world: &World) -> f32 {
    let me = world.npc(PLAYER);
    let drunk = if world.life.drinks.last() == Some(&world.day) {
        0.35
    } else {
        0.0
    };
    let hurt = if me.wounded { 0.25 } else { 0.0 };
    // A Sharps has sights worth the name.
    let sharps = if world.families[0].stores.arms.rifles > 0 {
        0.8
    } else {
        1.0
    };
    ((0.85 - 0.6 * me.body.marksmanship + drunk + hurt) * sharps).clamp(0.12, 1.2)
}

/// Half-width of the calm zone in a staredown, 0..0.5 of the bar. Courage,
/// grown men at your back, powder in the house.
pub fn nerve(world: &World) -> f32 {
    let me = world.npc(PLAYER);
    let men = world
        .living()
        .filter(|n| n.family == 0 && n.id != PLAYER && LifeStage::of(n.age) == LifeStage::Adult)
        .count()
        .min(2) as f32
        + super::hands::at_back(world).min(3) as f32;
    // Guns you can reach, and enough rounds to make them more than furniture.
    let a = &world.families[0].stores.arms;
    let guns = a.at_hand().min(3) as f32;
    let loaded = if a.rounds() >= 10 { 0.02 } else { 0.0 };
    0.08 + 0.10 * me.temperament.courage + 0.03 * men + 0.02 * guns + loaded
}

/// How hard he pushes back in a staredown, 0.5 .. 2.
pub fn pressure(world: &World, s: &Standoff) -> f32 {
    let n = world.npc(s.actor);
    // A Sharps across the saddle leans harder than a fowling piece.
    let rifles = world.families[n.family as usize].stores.arms.rifles.min(2) as f32;
    (0.6 + n.emotions.anger / 100.0
        + 0.5 * n.temperament.courage
        + 0.25 * s.riders.len() as f32
        + 0.15 * rifles)
        .clamp(0.5, 2.2)
}

/// You made your appeals; `right` of them landed. Oratory carries the rest.
pub fn talk(world: &mut World, right: u8, of: u8) -> Turn {
    let Some(s) = world.action.standoff.clone() else {
        return Turn::Settled;
    };
    let frac = right as f32 / of.max(1) as f32;
    let p = 0.04 + 0.86 * frac * frac + 0.25 * world.life.skill(Skill::Oratory)
        - 0.1 * world.npc(s.actor).temperament.temper;
    if world.rng.chance(p.clamp(0.02, 0.97)) {
        finish(world, End::TalkedDown);
        Turn::Settled
    } else {
        Turn::Draw
    }
}

/// You held your nerve for `grade` of the staredown.
pub fn face(world: &mut World, grade: f32) -> Turn {
    let Some(s) = world.action.standoff.clone() else {
        return Turn::Settled;
    };
    let need = 0.55 + 0.25 * (world.npc(s.actor).emotions.anger / 100.0).min(1.0);
    if grade >= need {
        finish(world, End::FacedDown);
        Turn::Settled
    } else {
        Turn::Draw
    }
}

/// Something loaded and within reach.
pub fn armed(world: &World) -> bool {
    world.families[0].stores.arms.ready()
}

pub fn draw(world: &mut World, how: DrawEnd) {
    // An empty gun, or one buried in the timber, clicks.
    let how = match how {
        DrawEnd::Fired(_) if !armed(world) || !super::arms::fire(world, 0) => DrawEnd::Slow,
        h => h,
    };
    let end = match how {
        DrawEnd::Fired(Shot::Kill) => End::Shot { killed: true },
        DrawEnd::Fired(Shot::Wound) => End::Shot { killed: false },
        _ => End::Beaten,
    };
    finish(world, end);
}

pub fn back_down(world: &mut World) {
    finish(world, End::BackedDown);
}

fn finish(world: &mut World, end: End) {
    let Some(s) = world.action.standoff.take() else {
        return;
    };
    let actor = s.actor;
    if s.serving.is_none() {
        world.npc_mut(actor).plotting = None;
    }
    let id = world.emit_root(
        EventKind::Standoff {
            other: actor,
            end,
            yours: s.yours,
            law: s.serving.is_some(),
        },
        s.caused_by,
    );
    // Who was there: at your gate, your household and his riders; at his
    // door, his household.
    let mut eyes: Vec<NpcId> = if s.yours {
        let fam = world.npc(actor).family;
        world
            .living()
            .filter(|n| n.family == fam)
            .map(|n| n.id)
            .collect()
    } else {
        world
            .living()
            .filter(|n| n.family == 0 && n.id != PLAYER)
            .map(|n| n.id)
            .chain(s.riders.iter().copied())
            .collect()
    };
    let mut lawful = false;
    let consequence = match s.serving {
        Some(paper) => {
            if matches!(end, End::Shot { .. }) {
                world.npc_mut(PLAYER).violence = world.npc(PLAYER).violence.saturating_add(1);
                eyes.push(actor);
            }
            super::warrant::settle(world, paper, s.yours, actor, end).map(|(who, kind, law)| {
                lawful = law;
                (who, kind)
            })
        }
        None => match end {
            End::Shot { killed } => {
                world.npc_mut(PLAYER).violence = world.npc(PLAYER).violence.saturating_add(1);
                eyes.push(actor);
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
                ))
            }
            End::Beaten => {
                let deadly = if s.method == Retaliation::Ambush {
                    0.35
                } else {
                    0.15
                };
                let today = world.day;
                let a = world.npc_mut(actor);
                a.violence = a.violence.saturating_add(1);
                a.last_revenge = Some(today);
                Some((
                    actor,
                    if world.rng.chance(deadly) {
                        EventKind::Death {
                            victim: PLAYER,
                            killer: Some(actor),
                        }
                    } else {
                        EventKind::Wounded {
                            victim: PLAYER,
                            attacker: actor,
                        }
                    },
                ))
            }
            End::BackedDown if !s.yours => {
                let today = world.day;
                world.npc_mut(actor).last_revenge = Some(today);
                match s.method {
                    Retaliation::Arson if world.families[0].barn_standing => Some((
                        actor,
                        EventKind::Fire {
                            owner: PLAYER,
                            cause: FireCause::Arson(actor),
                            spread_from: None,
                        },
                    )),
                    // Nothing to burn, or they came with rifles: they take a cow
                    // and your pride and ride off.
                    _ => {
                        world.action.staged = Some((actor, eyes.clone()));
                        super::economy::steal_from(world, actor, 0, PLAYER);
                        world.run_cascades();
                        world.action.staged = None;
                        None
                    }
                }
            }
            _ => None,
        },
    };
    if let Some((who, kind)) = consequence {
        if lawful {
            witnessed_lawful(world, who, eyes, kind, Some(id));
        } else {
            witnessed(world, who, eyes, kind, Some(id));
        }
    } else {
        world.run_cascades();
    }
}

/// What a standoff does to the people in it.
pub fn on_event(world: &mut World, ev: &super::events::WorldEvent) {
    let EventKind::Standoff {
        other, end, yours, ..
    } = ev.kind
    else {
        return;
    };
    let today = ev.day;
    match end {
        End::TalkedDown => {
            let n = world.npc_mut(other);
            n.emotions.anger = (n.emotions.anger - 40.0).max(0.0);
            world.adjust_opinion(other, PLAYER, 35);
            if yours {
                world.adjust_opinion(PLAYER, other, 15);
            }
            world
                .action
                .cowed
                .push((other, PLAYER, Day(today.0 + SETTLED_DAYS)));
            world.life.spirits = (world.life.spirits + 6.0).min(100.0);
        }
        End::FacedDown => {
            world.npc_mut(other).emotions.fear += 40.0;
            // Nobody likes the man who made them blink.
            world.adjust_opinion(other, PLAYER, -10);
            world
                .action
                .cowed
                .push((other, PLAYER, Day(today.0 + COWED_DAYS)));
            world.life.spirits = (world.life.spirits + 4.0).min(100.0);
        }
        End::Beaten => {
            world.life.spirits = (world.life.spirits - 10.0).max(0.0);
        }
        End::BackedDown => {
            world.life.spirits = (world.life.spirits - 6.0).max(0.0);
            if yours {
                world.adjust_opinion(other, PLAYER, -10);
            }
        }
        End::Shot { .. } => {}
    }
}

/// A standoff nobody answered by morning: they did what they came to do.
/// Stale cowings fall away.
pub fn daily(world: &mut World) {
    if world
        .action
        .standoff
        .as_ref()
        .is_some_and(|s| s.day < world.day)
    {
        back_down(world);
    }
    let today = world.day;
    world.action.cowed.retain(|c| today < c.2);
}

// ---- raids ----------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Objective {
    /// Put the barn to the torch.
    Burn,
    /// A carcass down the well.
    Foul,
    /// Shoot a cow and leave it. Loud.
    Shoot,
    /// Drive a cow off. Slow going with a cow.
    Drive,
    /// Pull the fence rails.
    Cut,
    /// Wet the haystack.
    Wet,
    /// Crouch under the window and listen.
    Listen,
}

impl Objective {
    pub fn label(self) -> &'static str {
        match self {
            Objective::Burn => "burn the barn",
            Objective::Foul => "foul the well",
            Objective::Shoot => "shoot a cow",
            Objective::Drive => "drive off a cow",
            Objective::Cut => "pull the rails",
            Objective::Wet => "wet the hay",
            Objective::Listen => "listen at the window",
        }
    }

    /// Seconds of work, standing still where they might look.
    pub fn seconds(self) -> f32 {
        match self {
            Objective::Burn => 2.5,
            Objective::Foul => 2.0,
            Objective::Shoot => 0.6,
            Objective::Drive => 1.5,
            Objective::Cut => 1.8,
            Objective::Wet => 1.8,
            Objective::Listen => 5.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Watcher {
    pub id: NpcId,
    /// How far and how sharp they see, 0..1.
    pub alertness: f32,
    /// Sitting up with a lantern. The rest are asleep until something wakes them.
    pub awake: bool,
}

#[derive(Clone, Debug)]
pub struct RaidPlan {
    pub target: NpcId,
    pub family: FamilyId,
    pub surname: &'static str,
    /// 0 dark .. 1 full and clear.
    pub moon: f32,
    pub dog: bool,
    pub watchers: Vec<Watcher>,
    pub objectives: Vec<Objective>,
    pub barn: bool,
    pub well: bool,
    pub cattle: u8,
    pub hay: bool,
    pub fences: f32,
    /// Your stealth, 0..1: how small you are in the grass.
    pub stealth: f32,
}

/// What their place looks like tonight. Reads the world; rolls nothing.
pub fn raid_plan(world: &World, target: NpcId) -> Option<RaidPlan> {
    let n = world.npc(target);
    if !world.player_alive() || !n.alive || n.family == 0 {
        return None;
    }
    let fam = n.family;
    let f = &world.families[fam as usize];
    let st = &f.stores;
    let watchers: Vec<Watcher> = world
        .living()
        .filter(|m| m.family == fam && LifeStage::of(m.age) != LifeStage::Child)
        .map(|m| Watcher {
            id: m.id,
            alertness: m.body.alertness,
            // The frightened sit up; so do people waiting on you in particular.
            awake: m.emotions.fear > 45.0
                || world.opinion(m.id, PLAYER) < -40
                || super::hands::watchmen(world, fam).first() == Some(&m.id),
        })
        .collect();
    let mut objectives = Vec::new();
    if f.barn_standing {
        objectives.push(Objective::Burn);
    }
    let well = homestead::has(world, fam, Improvement::Well);
    if well {
        objectives.push(Objective::Foul);
    }
    if st.cattle > 0 {
        objectives.push(Objective::Shoot);
        objectives.push(Objective::Drive);
    }
    if st.work.fences > 0.2 {
        objectives.push(Objective::Cut);
    }
    let hay = st.work.hay > 0.5;
    if hay {
        objectives.push(Objective::Wet);
    }
    objectives.push(Objective::Listen);
    Some(RaidPlan {
        target,
        family: fam,
        surname: f.surname,
        moon: world.night_light(world.day),
        // Most places with stock keep a dog; which ones is theirs to know.
        dog: st.cattle > 0 && (fam * 7 + 3) % 5 < 3,
        watchers,
        objectives,
        barn: f.barn_standing,
        well,
        cattle: st.cattle.min(6) as u8,
        hay,
        fences: st.work.fences,
        stealth: world.npc(PLAYER).body.stealth,
    })
}

/// Can you go out tonight?
pub fn night_free(world: &World) -> bool {
    world.player_alive()
        && world.action.last_night_out != Some(world.day)
        && super::family::away(world).is_none()
}

/// Back from their place. `done` is what you did; `seen_by` is who saw you.
pub fn raid(
    world: &mut World,
    target: NpcId,
    done: Option<Objective>,
    seen_by: Vec<NpcId>,
) -> bool {
    if !night_free(world) || !world.npc(target).alive {
        return false;
    }
    world.action.last_night_out = Some(world.day);
    world.npc_mut(PLAYER).alibi = None;
    let seen = !seen_by.is_empty();
    world.action.staged = Some((PLAYER, seen_by.clone()));
    let sabotage = |w: &mut World, act| {
        super::intrigue::sabotage(w, target, act);
    };
    match done {
        Some(Objective::Burn) => {
            world.player_burn(target);
        }
        Some(Objective::Foul) => sabotage(world, Cruelty::FoulWell),
        Some(Objective::Shoot) => sabotage(world, Cruelty::KillStock),
        Some(Objective::Cut) => sabotage(world, Cruelty::CutFence),
        Some(Objective::Wet) => sabotage(world, Cruelty::SpoilHay),
        Some(Objective::Drive) => {
            world.player_steal(target);
        }
        Some(Objective::Listen) => {
            super::intrigue::learn(world, target);
            if seen {
                prowled(world, target, seen_by.len());
            }
        }
        None => {
            if seen {
                prowled(world, target, seen_by.len());
            }
        }
    }
    world.run_cascades();
    world.action.staged = None;
    true
}

fn prowled(world: &mut World, target: NpcId, seen: usize) {
    world.emit_root(
        EventKind::Prowler {
            prowler: PLAYER,
            victim: target,
            seen: seen as u8,
        },
        None,
    );
}

// ---- ambush ---------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct AmbushPlan {
    pub target: NpcId,
    pub moon: f32,
    /// Someone riding with him tonight.
    pub companion: Option<NpcId>,
    /// Seconds to cross your sights.
    pub pace: f32,
}

pub fn ambush_plan(world: &World, target: NpcId) -> Option<AmbushPlan> {
    let n = world.npc(target);
    if !world.player_alive() || !n.alive || n.family == 0 {
        return None;
    }
    let fam = n.family;
    // Some nights a man rides home with his brother. You don't know which.
    let companion = world
        .living()
        .filter(|m| m.family == fam && m.id != target && LifeStage::of(m.age) == LifeStage::Adult)
        .map(|m| m.id)
        .find(|&m| (m + world.day.0).is_multiple_of(3));
    Some(AmbushPlan {
        target,
        moon: world.night_light(world.day),
        companion,
        pace: 7.0 - 2.5 * n.body.alertness,
    })
}

/// You fired at `hit` (and the ball found who it found), or you missed.
/// `None` for `fired` means you let him ride by.
pub fn ambush(world: &mut World, plan: &AmbushPlan, fired: Option<(NpcId, Shot)>) -> bool {
    if !night_free(world) {
        return false;
    }
    world.action.last_night_out = Some(world.day);
    let Some((hit, shot)) = fired else {
        return true;
    };
    super::arms::fire(world, 0);
    world.npc_mut(PLAYER).alibi = None;
    world.npc_mut(PLAYER).violence = world.npc(PLAYER).violence.saturating_add(1);
    // A bright moon shows the muzzle flash and the man behind it.
    let moon = plan.moon;
    let mut eyes = Vec::new();
    let riders = [Some(plan.target), plan.companion];
    for r in riders.into_iter().flatten() {
        let dead = r == hit && shot == Shot::Kill;
        let keen = 0.3 + 0.7 * world.npc(r).body.alertness;
        if !dead && world.rng.chance((moon * 1.2 * keen).min(0.95)) {
            eyes.push(r);
        }
    }
    world.action.staged = Some((PLAYER, eyes));
    let kind = match shot {
        Shot::Kill => EventKind::Death {
            victim: hit,
            killer: Some(PLAYER),
        },
        Shot::Wound => EventKind::Wounded {
            victim: hit,
            attacker: PLAYER,
        },
        Shot::Miss => EventKind::ShotAt {
            shooter: PLAYER,
            target: hit,
        },
    };
    let id = world.emit_root(kind, None);
    world.run_cascades();
    world.action.staged = None;
    // A man you missed shoots back at the flash.
    if shot != Shot::Kill {
        let back = if shot == Shot::Miss { hit } else { plan.target };
        let n = world.npc(back);
        if n.alive && !n.wounded && world.rng.chance(0.45 * n.body.marksmanship) {
            world.emit_root(
                EventKind::Wounded {
                    victim: PLAYER,
                    attacker: back,
                },
                Some(id),
            );
            world.run_cascades();
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at_gate(seed: u64) -> (World, NpcId) {
        let mut w = World::new(seed);
        w.autopilot_player = false;
        let t = w.head_of(3).unwrap();
        assert!(park(&mut w, t, Retaliation::Arson, None));
        (w, t)
    }

    #[test]
    fn plots_park_at_the_gate_when_youre_home() {
        let (w, t) = at_gate(4);
        let s = w.action.standoff.as_ref().unwrap();
        assert_eq!(s.actor, t);
        assert!(
            w.events
                .iter()
                .any(|e| matches!(e.kind, EventKind::RidersAtGate { actor, .. } if actor == t))
        );
    }

    #[test]
    fn autopilot_never_parks() {
        let mut w = World::new(4);
        let t = w.head_of(3).unwrap();
        assert!(!park(&mut w, t, Retaliation::Arson, None));
    }

    #[test]
    fn unanswered_they_burn_it_and_your_family_saw() {
        let (mut w, t) = at_gate(4);
        w.advance_day();
        assert!(w.action.standoff.is_none());
        assert!(!w.families[0].barn_standing);
        // The household watched it happen: a true belief, not a guess.
        let kin = w
            .living()
            .find(|n| n.family == 0 && n.id != PLAYER)
            .unwrap()
            .id;
        assert!(w.events.iter().any(|e| matches!(
            e.kind,
            EventKind::Belief { holder, blamed: super::super::Suspect::Person(b), source: super::super::events::Source::Witnessed, .. }
                if holder == kin && b == t
        )));
    }

    #[test]
    fn facing_them_down_keeps_them_off() {
        let (mut w, t) = at_gate(5);
        assert_eq!(face(&mut w, 1.0), Turn::Settled);
        assert!(w.action.cowed(t, PLAYER, w.day));
        assert!(w.families[0].barn_standing);
    }

    #[test]
    fn a_weak_stare_goes_to_guns() {
        let (mut w, _) = at_gate(5);
        assert_eq!(face(&mut w, 0.1), Turn::Draw);
        assert!(w.action.standoff.is_some());
    }

    #[test]
    fn the_right_words_usually_work() {
        let settled = (0..30)
            .filter(|&s| {
                let (mut w, _) = at_gate(s);
                talk(&mut w, 3, 3) == Turn::Settled
            })
            .count();
        let wrong = (0..30)
            .filter(|&s| {
                let (mut w, _) = at_gate(s);
                talk(&mut w, 0, 3) == Turn::Settled
            })
            .count();
        assert!(settled > 20 && wrong < 8, "{settled} vs {wrong}");
    }

    #[test]
    fn winning_the_draw_kills_him_and_his_riders_saw() {
        let (mut w, t) = at_gate(6);
        draw(&mut w, DrawEnd::Fired(Shot::Kill));
        assert!(!w.npc(t).alive);
        assert!(w.events.iter().any(|e| matches!(
            e.kind,
            EventKind::Death { victim, killer: Some(PLAYER) } if victim == t
        )));
    }

    #[test]
    fn flinching_gets_you_shot() {
        let (mut w, _) = at_gate(6);
        draw(&mut w, DrawEnd::Flinched);
        let me = w.npc(PLAYER);
        assert!(!me.alive || me.wounded);
    }

    #[test]
    fn an_unseen_raid_leaves_only_guesses() {
        let mut w = World::new(7);
        let t = w.head_of(2).unwrap();
        assert!(raid(&mut w, t, Some(Objective::Wet), Vec::new()));
        assert!(!w.events.iter().any(|e| matches!(
            e.kind,
            EventKind::Belief {
                source: super::super::events::Source::Witnessed,
                blamed: super::super::Suspect::Person(PLAYER),
                ..
            }
        )));
        // One night out a night.
        assert!(!raid(&mut w, t, None, Vec::new()));
    }

    #[test]
    fn a_seen_raid_names_you() {
        let mut w = World::new(7);
        let t = w.head_of(2).unwrap();
        assert!(raid(&mut w, t, Some(Objective::Cut), vec![t]));
        assert!(w.events.iter().any(|e| matches!(
            e.kind,
            EventKind::Belief { holder, source: super::super::events::Source::Witnessed, blamed: super::super::Suspect::Person(PLAYER), .. }
                if holder == t
        )));
    }

    #[test]
    fn raid_plans_read_the_place() {
        let w = World::new(8);
        let t = w.head_of(4).unwrap();
        let p = raid_plan(&w, t).unwrap();
        assert!(p.objectives.contains(&Objective::Listen));
        assert_eq!(p.barn, p.objectives.contains(&Objective::Burn));
        assert!(!p.watchers.is_empty());
        assert!(raid_plan(&w, PLAYER).is_none());
    }

    #[test]
    fn an_ambush_kill_is_a_death_by_your_hand() {
        let mut w = World::new(9);
        let t = w.head_of(5).unwrap();
        let plan = ambush_plan(&w, t).unwrap();
        assert!(ambush(&mut w, &plan, Some((t, Shot::Kill))));
        assert!(!w.npc(t).alive);
    }

    #[test]
    fn a_miss_is_still_an_attack() {
        let mut w = World::new(9);
        let t = w.head_of(5).unwrap();
        let plan = ambush_plan(&w, t).unwrap();
        assert!(ambush(&mut w, &plan, Some((t, Shot::Miss))));
        assert!(
            w.events
                .iter()
                .any(|e| matches!(e.kind, EventKind::ShotAt { target, .. } if target == t))
        );
        assert!(w.npc(t).alive);
    }

    #[test]
    fn an_empty_gun_clicks() {
        let (mut w, t) = at_gate(6);
        w.families[0].stores.arms.balls = 0;
        w.families[0].stores.arms.cartridges = 0;
        draw(&mut w, DrawEnd::Fired(Shot::Kill));
        assert!(w.npc(t).alive);
        let me = w.npc(PLAYER);
        assert!(!me.alive || me.wounded);
    }

    #[test]
    fn a_tipsy_hand_wanders() {
        let mut w = World::new(10);
        let sober = sway(&w);
        let d = w.day;
        w.life.drinks.push(d);
        assert!(sway(&w) > sober);
    }
}
