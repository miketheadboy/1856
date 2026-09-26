//! The player's days (§13 player life). Somewhere between the Sims, Stardew
//! Valley and Jones in the Fast Lane: one thing a day, skills that grow by
//! doing, spirits that need tending, goals nobody hands you. You can live in
//! the small loops — chores, the creek, the woods — until the county pulls
//! you in. It always does.
//!
//! Every activity feeds something outside itself: fish feed the household,
//! visits move opinions and carry rumors, sermons and baptisms move piety
//! (and piety moves revenge), speeches move faction grievance, and being off
//! somewhere is an absence your kin will remember (`family`).

use super::calendar::{Day, Season};
use super::civic::{self, Project};
use super::events::{EventKind, Suspect, WorldEvent};
use super::family::{self, Errand};
use super::institutions::Venue;
use super::market::{self, Good};
use super::psyche::{self, LifeStage};
use super::romance;
use super::world::{Faction, NpcId, PLAYER, World};

pub const SKILLS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skill {
    /// Garden, hens and the cow: food from chores.
    Farming,
    /// Catfish from the Wakarusa.
    Fishing,
    /// Barn raisings and county projects go faster.
    Carpentry,
    /// Speeches, brokering, courting.
    Oratory,
    /// Sermons draw a flock; baptisms need it.
    Scripture,
    /// Better prices at Dunmore's.
    Trade,
    /// Forms at the land office; the papers.
    Letters,
    /// A head for whiskey: less harm per glass.
    Drink,
}

impl Skill {
    pub const ALL: [Skill; SKILLS] = [
        Skill::Farming,
        Skill::Fishing,
        Skill::Carpentry,
        Skill::Oratory,
        Skill::Scripture,
        Skill::Trade,
        Skill::Letters,
        Skill::Drink,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            Skill::Farming => "farming",
            Skill::Fishing => "fishing",
            Skill::Carpentry => "carpentry",
            Skill::Oratory => "oratory",
            Skill::Scripture => "scripture",
            Skill::Trade => "trade",
            Skill::Letters => "letters",
            Skill::Drink => "drink",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Chores,
    Fish,
    Roam,
    Visit(NpcId),
    Court(NpcId),
    Drink,
    /// Sunday: preach at meeting. Other days: study.
    Preach,
    Baptize(NpcId),
    Speech {
        calm: bool,
    },
    Build(Project),
    FileClaim,
    Study,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Find {
    Nothing,
    BeeTree,
    Nuts,
    Neighbor(NpcId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pastime {
    Chores,
    Fished,
    Roamed(Find),
    Drank,
    Studied,
    Visited(NpcId),
    Courted(NpcId),
    Built(Project),
}

#[derive(Clone, Debug)]
pub struct Life {
    pub xp: [f32; SKILLS],
    /// 0 despair .. 100 content. Low spirits slow learning and want whiskey.
    pub spirits: f32,
    pub acted_on: Option<Day>,
    pub drinks: Vec<Day>,
    pub sermons: u32,
    pub baptized: Vec<NpcId>,
    pub roamed: u32,
    pub built: u32,
}

impl Default for Life {
    fn default() -> Self {
        Life {
            xp: [0.0; SKILLS],
            spirits: 55.0,
            acted_on: None,
            drinks: Vec::new(),
            sermons: 0,
            baptized: Vec::new(),
            roamed: 0,
            built: 0,
        }
    }
}

impl Life {
    /// 0..1, diminishing returns: 200 xp (a season of steady practice) is halfway.
    pub fn skill(&self, s: Skill) -> f32 {
        let x = self.xp[s.index()];
        x / (x + 200.0)
    }

    fn learn(&mut self, s: Skill, amount: f32) {
        let mood = if self.spirits < 20.0 { 0.5 } else { 1.0 };
        self.xp[s.index()] += amount * mood;
    }

    /// Good days lift less the better you already feel; bad days sink less
    /// the lower you already are.
    fn cheer(&mut self, delta: f32) {
        let room = if delta > 0.0 {
            (100.0 - self.spirits) / 50.0
        } else {
            self.spirits / 50.0
        };
        self.spirits = (self.spirits + delta * room).clamp(0.0, 100.0);
    }

    /// Drinks in the last `days`.
    pub fn drinking(&self, today: Day, days: u32) -> usize {
        self.drinks
            .iter()
            .filter(|d| today.0.saturating_sub(d.0) < days)
            .count()
    }
}

/// Jones-style goals: money, name, spirits, learning. 0..1 each.
pub fn goals(world: &World) -> [(&'static str, f32); 4] {
    let cash = world.families[0].stores.cash as f32;
    let standing = psyche::reputation(world, PLAYER);
    let learning: f32 = Skill::ALL.iter().map(|&s| world.life.skill(s)).sum();
    [
        ("wealth", (cash / 300.0).clamp(0.0, 1.0)),
        ("standing", ((standing + 10.0) / 40.0).clamp(0.0, 1.0)),
        ("spirits", (world.life.spirits / 75.0).clamp(0.0, 1.0)),
        ("learning", (learning / 2.0).clamp(0.0, 1.0)),
    ]
}

/// What the county would call you, if it had to. Paths emerge from what you
/// did, not from a menu.
pub fn paths(world: &World) -> Vec<&'static str> {
    let l = &world.life;
    let today = world.day;
    let mut p = Vec::new();
    if l.sermons >= 8 && l.baptized.len() >= 3 {
        p.push("exalted preacher");
    } else if l.sermons >= 3 {
        p.push("lay preacher");
    }
    if l.drinking(today, 30) >= 10 {
        p.push("town souse");
    }
    if l.skill(Skill::Oratory) >= 0.5 {
        p.push("silver tongue");
    }
    if world.npc(PLAYER).violence >= 2 {
        p.push("ruffian");
    }
    if world.families[0].stores.cash >= 500 {
        p.push("tycoon");
    }
    if l.skill(Skill::Carpentry) + l.skill(Skill::Trade) >= 1.0 {
        p.push("master of industry");
    }
    if l.roamed >= 30 && world.player_away.is_none() {
        p.push("vagabond");
    }
    if psyche::reputation(world, PLAYER) >= 25.0 {
        p.push("big shot");
    }
    if l.built >= 20 {
        p.push("pillar of the community");
    }
    if !world
        .hearts
        .affairs
        .iter()
        .all(|&(a, b, _)| a != PLAYER && b != PLAYER)
    {
        p.push("rake");
    }
    p
}

/// One thing a day. Returns false if you've already acted, are away, or
/// can't do it.
pub fn act(world: &mut World, what: Activity) -> bool {
    let today = world.day;
    if !world.player_alive()
        || world.life.acted_on == Some(today)
        || family::away(world).is_some()
        || world.npc(PLAYER).wounded
    {
        return false;
    }
    let done = match what {
        Activity::Chores => chores(world),
        Activity::Fish => fish(world),
        Activity::Roam => roam(world),
        Activity::Visit(n) => visit(world, n),
        Activity::Court(n) => court(world, n),
        Activity::Drink => drink(world),
        Activity::Preach => preach(world),
        Activity::Baptize(n) => baptize(world, n),
        Activity::Speech { calm } => speech(world, calm),
        Activity::Build(p) => {
            let hands = 1.0 + 2.0 * world.life.skill(Skill::Carpentry);
            civic::work(world, PLAYER, p, hands);
            world.life.learn(Skill::Carpentry, 2.0);
            world.life.built += 1;
            world.life.cheer(3.0);
            pastime(world, Pastime::Built(p), hands as u16);
            true
        }
        Activity::FileClaim => {
            let letters = world.life.skill(Skill::Letters);
            let ok = civic::file_claim(world, 0, letters);
            if ok {
                family::leave_for(world, Errand::Town, 2);
                world.life.learn(Skill::Letters, 1.0);
            }
            ok
        }
        Activity::Study => {
            world.life.learn(Skill::Letters, 2.0);
            world.life.learn(Skill::Scripture, 1.0);
            let school = if world.civic.built(Project::Schoolhouse) {
                0.02
            } else {
                0.01
            };
            let t = &mut world.npc_mut(PLAYER).temperament;
            t.literacy = (t.literacy + school).min(1.0);
            pastime(world, Pastime::Studied, 1);
            true
        }
    };
    if done {
        world.life.acted_on = Some(today);
        world.run_cascades();
    }
    done
}

fn pastime(world: &mut World, what: Pastime, amount: u16) {
    world.emit_root(EventKind::Pastime { what, amount }, None);
}

fn chores(world: &mut World) -> bool {
    let farming = world.life.skill(Skill::Farming);
    let food = 1.0 + 4.0 * farming;
    world.families[0].stores.food += food;
    world.life.learn(Skill::Farming, 2.0);
    // Drudgery, unless you've come to love it.
    world.life.cheer(-1.0 + 4.0 * farming);
    pastime(world, Pastime::Chores, food as u16);
    true
}

fn fish(world: &mut World) -> bool {
    let skill = world.life.skill(Skill::Fishing);
    let season = match world.day.season() {
        Season::Winter => 0.3,
        Season::Spring => 1.2,
        Season::Summer => 1.0,
        Season::Autumn => 0.9,
    };
    // Old men swear they bite best in the dark of the moon.
    let moon = 0.8 + 0.4 * (1.0 - world.day.moonlight());
    let luck = world.npc(PLAYER).hidden.luck;
    let noise = world.rng.unit();
    let fish = ((1.0 + 8.0 * skill) * season * moon * luck * (0.4 + noise)).round();
    world.families[0].stores.food += 2.0 * fish;
    world.life.learn(Skill::Fishing, 2.0);
    world.life.cheer(6.0);
    family::leave_for(world, Errand::Fishing, 1);
    pastime(world, Pastime::Fished, fish as u16);
    true
}

fn roam(world: &mut World) -> bool {
    world.life.roamed += 1;
    world.life.cheer(8.0);
    let month = world.day.month();
    let warm = (4..=10).contains(&month);
    let find = if (9..=10).contains(&month) && world.rng.chance(0.4) {
        world.families[0].stores.food += 6.0;
        Find::Nuts
    } else if warm && world.rng.chance(0.12) {
        // Honey keeps; half to the table, half to sell.
        world.families[0].stores.food += 5.0;
        world.families[0].stores.cash += 2;
        Find::BeeTree
    } else if world.rng.chance(0.35) {
        let others: Vec<NpcId> = world
            .living()
            .filter(|n| n.family != 0 && LifeStage::of(n.age) != LifeStage::Child)
            .map(|n| n.id)
            .collect();
        if others.is_empty() {
            Find::Nothing
        } else {
            let who = others[world.rng.range(0, others.len() as u32) as usize];
            world.adjust_opinion(who, PLAYER, 3);
            world.adjust_opinion(PLAYER, who, 3);
            Find::Neighbor(who)
        }
    } else {
        Find::Nothing
    };
    family::leave_for(world, Errand::Fishing, 1);
    pastime(world, Pastime::Roamed(find), 1);
    true
}

/// Sit on someone's porch. They like you a little more and tell you who they
/// blame for things — which is not the same as who did it.
fn visit(world: &mut World, who: NpcId) -> bool {
    if who == PLAYER || !world.npc(who).alive {
        return false;
    }
    let talk = world.life.skill(Skill::Oratory);
    let social = world.npc(who).temperament.sociability;
    let warm = (2.0 + 4.0 * social + 6.0 * talk) as i16;
    world.adjust_opinion(who, PLAYER, warm);
    world.life.learn(Skill::Oratory, 1.0);
    world.life.cheer(3.0);
    let told = world
        .npc(who)
        .memories
        .iter()
        .filter(|m| {
            m.confidence >= 30 && matches!(m.believed, Suspect::Person(_) | Suspect::Nation(_))
        })
        .max_by_key(|m| m.confidence)
        .map(|m| (m.event, m.believed));
    pastime(world, Pastime::Visited(who), warm as u16);
    if let Some((about, blamed)) = told {
        world.emit_root(
            EventKind::Gossip {
                teller: who,
                listener: PLAYER,
                about,
                blamed,
            },
            None,
        );
    }
    true
}

fn court(world: &mut World, who: NpcId) -> bool {
    if who == PLAYER
        || !world.npc(who).alive
        || LifeStage::of(world.npc(who).age) == LifeStage::Child
    {
        return false;
    }
    let bright = if world.life.spirits > 70.0 { 0.1 } else { 0.0 };
    let oratory = world.life.skill(Skill::Oratory) + bright;
    romance::court(world, PLAYER, who, oratory);
    world.life.learn(Skill::Oratory, 1.0);
    world.life.cheer(5.0);
    family::leave_for(world, Errand::Courting, 1);
    pastime(world, Pastime::Courted(who), 1);
    // If either of you is married, someone may see.
    let mine = romance::married_to(world, PLAYER);
    let theirs = romance::married_to(world, who);
    if (mine.is_some() || theirs.is_some())
        && world.rng.chance(romance::exposure(world, PLAYER, who))
    {
        let wronged = mine.or(theirs).unwrap_or(who);
        world.emit_root(
            EventKind::Scandal {
                a: PLAYER,
                b: who,
                wronged,
            },
            None,
        );
    }
    true
}

fn drink(world: &mut World) -> bool {
    let today = world.day;
    let head = world.life.skill(Skill::Drink);
    let price = market::bid_price(world, Good::Whiskey) * 0.1;
    let hh = &mut world.families[0].stores;
    if hh.cash > 0 {
        hh.cash -= (price.ceil() as i32).max(1).min(hh.cash);
    }
    world.life.drinks.push(today);
    world
        .life
        .drinks
        .retain(|d| today.0.saturating_sub(d.0) < 60);
    world.life.learn(Skill::Drink, 2.0);
    world.life.cheer(10.0);
    let p = world.npc_mut(PLAYER);
    p.health = (p.health - (4.0 * (1.0 - head)) as i32).max(1);
    // A souse's kin see it every time.
    if world.life.drinking(today, 14) >= 6 {
        world.life.cheer(-14.0);
        let kin: Vec<NpcId> = world
            .living()
            .filter(|n| n.family == 0 && n.id != PLAYER)
            .map(|n| n.id)
            .collect();
        for k in kin {
            world.adjust_opinion(k, PLAYER, -4);
        }
    }
    family::leave_for(world, Errand::Drinking, 1);
    pastime(world, Pastime::Drank, 1);
    // The groggery talks: overhear somebody's version of something.
    if world.institutions.open[Venue::JackOfHearts.index()] {
        let fresh: Vec<(NpcId, u32, Suspect)> = world
            .living()
            .filter(|n| n.id != PLAYER)
            .filter_map(|n| {
                n.memories
                    .iter()
                    .filter(|m| today.0.saturating_sub(m.day.0) <= 30 && m.confidence >= 30)
                    .max_by_key(|m| m.confidence)
                    .map(|m| (n.id, m.event, m.believed))
            })
            .collect();
        if !fresh.is_empty() {
            let (teller, about, blamed) = fresh[world.rng.range(0, fresh.len() as u32) as usize];
            world.emit_root(
                EventKind::Gossip {
                    teller,
                    listener: PLAYER,
                    about,
                    blamed,
                },
                None,
            );
        }
    }
    true
}

fn preach(world: &mut World) -> bool {
    let sunday = world.day.0 % 7 == 3;
    world
        .life
        .learn(Skill::Scripture, if sunday { 3.0 } else { 1.5 });
    if !sunday || !world.institutions.open[Venue::PlymouthChurch.index()] {
        pastime(world, Pastime::Studied, 1);
        return true;
    }
    let scripture = world.life.skill(Skill::Scripture);
    let flock: Vec<NpcId> = world
        .living()
        .filter(|n| n.id != PLAYER && n.temperament.piety > 0.35)
        .map(|n| n.id)
        .collect();
    let mut heard = 0u8;
    for n in flock {
        if !world.rng.chance(0.3 + 0.5 * scripture) {
            continue;
        }
        heard += 1;
        // A good sermon wins a hearing, not devotion.
        if world.opinion(n, PLAYER) < 40 {
            world.adjust_opinion(n, PLAYER, (1.0 + 5.0 * scripture) as i16);
        }
        let t = &mut world.npc_mut(n).temperament;
        t.piety = (t.piety + 0.01).min(1.0);
    }
    world.life.sermons += 1;
    world.life.cheer(4.0);
    world.emit_root(
        EventKind::Sermon {
            preacher: PLAYER,
            flock: heard,
        },
        None,
    );
    true
}

/// Down to the Wakarusa. Faith and humility up, grief washed down, and in
/// the cold months a fever is a real risk.
fn baptize(world: &mut World, who: NpcId) -> bool {
    if who == PLAYER
        || !world.npc(who).alive
        || world.life.skill(Skill::Scripture) < 0.25
        || world.opinion(who, PLAYER) < 10
        || world.life.baptized.contains(&who)
    {
        return false;
    }
    let cold = matches!(world.day.month(), 11 | 12 | 1 | 2 | 3);
    world.emit_root(
        EventKind::Baptism {
            preacher: PLAYER,
            convert: who,
            cold,
        },
        None,
    );
    world.life.baptized.push(who);
    world.life.learn(Skill::Scripture, 3.0);
    world.life.cheer(8.0);
    true
}

/// On a barrel at the Free State Hotel. A calming speech cools both sides a
/// little; a hot one fires your own and marks you with the other.
fn speech(world: &mut World, calm: bool) -> bool {
    let oratory = world.life.skill(Skill::Oratory);
    let lyceum = if world.civic.built(Project::Lyceum) {
        1.5
    } else {
        1.0
    };
    let crowd: Vec<NpcId> = world
        .living()
        .filter(|n| n.id != PLAYER && LifeStage::of(n.age) != LifeStage::Child)
        .map(|n| n.id)
        .collect();
    let mut heard = 0u8;
    for n in crowd {
        let p = (0.15 + 0.4 * world.npc(n).temperament.sociability) * lyceum;
        if !world.rng.chance(p.min(0.95)) {
            continue;
        }
        heard += 1;
        let ours = world.npc(n).faction == world.npc(PLAYER).faction;
        let delta = match (calm, ours) {
            (true, _) => 2.0 + 4.0 * oratory,
            (false, true) => 3.0 + 6.0 * oratory,
            (false, false) => -(4.0 + 8.0 * oratory),
        };
        world.adjust_opinion(n, PLAYER, delta as i16);
    }
    let push = (3.0 + 12.0 * oratory) as i32;
    let own = world.npc(PLAYER).faction;
    if calm {
        for f in [Faction::FreeState, Faction::ProSlavery] {
            world.add_grievance(f, -push);
        }
    } else {
        world.add_grievance(own, push);
    }
    world.life.learn(Skill::Oratory, 3.0);
    world.life.cheer(4.0);
    family::leave_for(world, Errand::Town, 1);
    world.emit_root(
        EventKind::Speech {
            speaker: PLAYER,
            calm,
            heard,
        },
        None,
    );
    true
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    match ev.kind {
        EventKind::Baptism { convert, cold, .. } => {
            let n = world.npc_mut(convert);
            n.temperament.piety = (n.temperament.piety + 0.15).min(1.0);
            n.temperament.humility = (n.temperament.humility + 0.1).min(1.0);
            n.emotions.grief *= 0.5;
            if cold {
                n.health = (n.health - (12.0 * n.body.frailty()) as i32).max(1);
            }
            let family = world.npc(convert).family;
            world.adjust_opinion(convert, PLAYER, 15);
            let kin: Vec<NpcId> = world
                .living()
                .filter(|k| k.family == family && k.id != convert)
                .map(|k| k.id)
                .collect();
            for k in kin {
                world.adjust_opinion(k, PLAYER, 5);
            }
        }
        // Home news, good and bad.
        EventKind::Death { victim, .. } | EventKind::Perished { victim, .. }
            if world.npc(victim).family == 0 && victim != PLAYER =>
        {
            world.life.cheer(-30.0)
        }
        EventKind::Fire { owner, .. } if world.npc(owner).family == 0 => world.life.cheer(-15.0),
        EventKind::Scandal { a, b, .. } if a == PLAYER || b == PLAYER => world.life.cheer(-20.0),
        EventKind::Marriage { a, b } if a == PLAYER || b == PLAYER => world.life.cheer(25.0),
        _ => {}
    }
}

pub fn daily(world: &mut World) {
    // Spirits drift home; a good marriage lifts them, hunger drags them.
    let l = &mut world.life;
    l.spirits += (50.0 - l.spirits) * 0.05;
    let hungry = world.families[0].stores.hungry_days > 0;
    let happy_home =
        romance::married_to(world, PLAYER).is_some_and(|s| world.opinion(s, PLAYER) >= 30);
    let l = &mut world.life;
    if hungry {
        l.cheer(-2.0);
    }
    if happy_home {
        l.cheer(0.3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> World {
        let mut w = World::new(10);
        w.autopilot_player = false;
        w
    }

    #[test]
    fn one_thing_a_day() {
        let mut w = fresh();
        assert!(act(&mut w, Activity::Chores));
        assert!(!act(&mut w, Activity::Chores));
        w.run_days(1);
        assert!(act(&mut w, Activity::Chores));
    }

    #[test]
    fn practice_makes_a_farmer() {
        let mut w = fresh();
        let before = w.life.skill(Skill::Farming);
        for _ in 0..20 {
            act(&mut w, Activity::Chores);
            w.run_days(1);
        }
        assert!(w.life.skill(Skill::Farming) > before + 0.1);
    }

    #[test]
    fn fishing_feeds_and_takes_you_away() {
        let mut w = World::new(10);
        w.run_days(180); // spring, the sim keeping house till now
        w.autopilot_player = false;
        w.npc_mut(PLAYER).wounded = false;
        w.npc_mut(PLAYER).alive = true;
        let food = w.families[0].stores.food;
        assert!(act(&mut w, Activity::Fish));
        assert!(family::away(&w).is_some());
        assert!(w.families[0].stores.food >= food);
    }

    #[test]
    fn a_calming_speech_cools_the_county() {
        let mut w = fresh();
        w.grievance = [80, 80];
        act(&mut w, Activity::Speech { calm: true });
        assert!(w.grievance[0] < 80 && w.grievance[1] < 80);
    }

    #[test]
    fn baptism_needs_scripture_and_softens_the_convert() {
        let mut w = fresh();
        let convert = w.head_of(2).unwrap();
        w.set_opinion(convert, PLAYER, 40);
        assert!(!act(&mut w, Activity::Baptize(convert)), "no scripture yet");
        w.life.xp[Skill::Scripture.index()] = 100.0;
        let humility = w.npc(convert).temperament.humility;
        assert!(act(&mut w, Activity::Baptize(convert)));
        assert!(w.npc(convert).temperament.humility > humility);
    }

    #[test]
    fn the_souse_loses_his_family() {
        let mut w = fresh();
        let spouse = romance::married_to(&w, PLAYER).unwrap();
        let before = w.opinion(spouse, PLAYER);
        for _ in 0..12 {
            act(&mut w, Activity::Drink);
            w.run_days(1);
        }
        assert!(paths(&w).contains(&"town souse") || w.life.drinking(w.day, 30) >= 10);
        assert!(w.opinion(spouse, PLAYER) < before);
    }
}
