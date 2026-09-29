//! The inside of a person: body, emotions, temperament, and the gap between
//! what they say and what they believe (ext. §16, §22).
//!
//! Every number here is read by some other system. If it isn't a variable
//! that changes an outcome, it doesn't exist.

use super::calendar::Season;
use super::world::{Faction, NpcId, World, distance};

/// Emotions spread like disease (ext. §22): toward your family's mood, and a
/// little toward your neighbors'.
const FAMILY_PULL: f32 = 0.06;
const NEIGHBOR_PULL: f32 = 0.02;
const NEIGHBOR_RANGE: f32 = 8.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Emotions {
    /// Makes people cautious: less revenge, more giving in.
    pub fear: f32,
    /// Makes people act: revenge comes sooner.
    pub anger: f32,
    /// Weighs on everything; long-lived.
    pub grief: f32,
    /// Hardens ideology and makes compromise costly.
    pub zeal: f32,
}

impl Emotions {
    pub(crate) fn clamp(&mut self) {
        for v in [
            &mut self.fear,
            &mut self.anger,
            &mut self.grief,
            &mut self.zeal,
        ] {
            *v = v.clamp(0.0, 100.0);
        }
    }
}

/// Fixed at birth. Shapes how emotions turn into action. All 0..1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Temperament {
    /// Quick to violence: revenge, theft.
    pub temper: f32,
    /// Hard to scare: fear holds them back less; resists the storekeeper.
    pub courage: f32,
    /// Grudges thaw faster; Sunday church is an alibi; less theft.
    pub piety: f32,
    /// Feeds a beggar at the door.
    pub generosity: f32,
    /// Won't steal unless it's that or a dead child.
    pub honesty: f32,
    /// Talks: spreads gossip, gets seen in town (alibis).
    pub sociability: f32,
    /// Discounts rumor; less certain of who's to blame.
    pub skepticism: f32,
    /// Faction loyalty: takes its grievances personally, won't sign.
    pub loyalty: f32,
    /// Owns up: apologizes for harm done, spares a man at his door, and
    /// takes a slight without needing blood for it.
    pub humility: f32,
    /// Reads the papers, and believes what they print.
    pub literacy: f32,
}

/// Body and hands. All 0..1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Body {
    /// Field labor (harvest) and how much you can carry off.
    pub strength: f32,
    /// How slowly hunger, cold and fever wear you down.
    pub hardiness: f32,
    /// Whether an ambush kills; whether a winter hunt brings anything home.
    pub marksmanship: f32,
    /// Whether anyone sees you do it.
    pub stealth: f32,
    /// Whether you notice who did it; whether you see the ambush coming.
    pub alertness: f32,
    /// How much you remember before small things fall away (§14.4).
    pub recall: f32,
}

impl Body {
    pub fn roll(rng: &mut super::rng::SimRng, age: u8) -> Self {
        let grown = match LifeStage::of(age) {
            LifeStage::Child => 0.35,
            LifeStage::Adult => 1.0,
            LifeStage::Elder => 0.6,
        };
        Self {
            strength: rng.unit() * grown,
            hardiness: rng.unit(),
            marksmanship: rng.unit() * grown,
            stealth: rng.unit(),
            alertness: rng.unit(),
            recall: rng.unit(),
        }
    }

    /// Health lost per point of hardship, scaled by constitution. Survivors
    /// (hardy and strong) waste slower still.
    pub fn frailty(&self) -> f32 {
        let survivor = if self.hardiness > 0.75 && self.strength > 0.6 {
            0.6
        } else {
            1.0
        };
        (1.4 - 0.8 * self.hardiness) * survivor
    }
}

impl Temperament {
    pub fn roll(rng: &mut super::rng::SimRng) -> Self {
        Self {
            temper: rng.unit(),
            courage: rng.unit(),
            piety: rng.unit(),
            generosity: rng.unit(),
            honesty: rng.unit(),
            sociability: rng.unit(),
            skepticism: rng.unit(),
            loyalty: rng.unit(),
            // New Englanders came with schooling; Missourians less often.
            literacy: rng.unit(),
            humility: rng.unit(),
        }
    }
}

/// -1 pro-slavery .. +1 free-state. Public is what you say and sign;
/// private is what you believe. They drift apart under pressure (ext. §16).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ideology {
    pub public: f32,
    pub private: f32,
}

impl Ideology {
    pub fn for_faction(faction: Faction, conviction: f32) -> Self {
        let sign = match faction {
            Faction::ProSlavery => -1.0,
            Faction::FreeState => 1.0,
        };
        Self {
            public: sign * conviction,
            private: sign * conviction,
        }
    }

    /// How far someone's mouth is from their heart.
    pub fn hypocrisy(&self) -> f32 {
        (self.public - self.private).abs()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifeStage {
    Child,
    Adult,
    Elder,
}

impl LifeStage {
    pub fn of(age: u8) -> Self {
        match age {
            0..=15 => LifeStage::Child,
            16..=54 => LifeStage::Adult,
            _ => LifeStage::Elder,
        }
    }

    /// Health lost per day without food.
    pub fn starvation_rate(self) -> i32 {
        match self {
            LifeStage::Child => 6,
            LifeStage::Adult => 3,
            LifeStage::Elder => 5,
        }
    }
}

/// A one-word read on someone, for the UI and the chronicle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    Dead,
    Wounded,
    Starving,
    Hungry,
    Grieving,
    Enraged,
    Afraid,
    Zealous,
    Content,
}

impl Condition {
    pub fn label(self) -> &'static str {
        match self {
            Condition::Dead => "dead",
            Condition::Wounded => "wounded",
            Condition::Starving => "starving",
            Condition::Hungry => "hungry",
            Condition::Grieving => "grieving",
            Condition::Enraged => "enraged",
            Condition::Afraid => "afraid",
            Condition::Zealous => "zealous",
            Condition::Content => "content",
        }
    }
}

pub fn condition(world: &World, id: NpcId) -> Condition {
    let n = world.npc(id);
    let hh = &world.families[n.family as usize].stores;
    let e = n.emotions;
    if !n.alive {
        Condition::Dead
    } else if n.wounded {
        Condition::Wounded
    } else if hh.hungry_days > 0 && n.health < 50 {
        Condition::Starving
    } else if hh.hungry_days > 0 {
        Condition::Hungry
    } else if e.grief > 40.0 {
        Condition::Grieving
    } else if e.anger > 60.0 {
        Condition::Enraged
    } else if e.fear > 50.0 {
        Condition::Afraid
    } else if e.zeal > 60.0 {
        Condition::Zealous
    } else {
        Condition::Content
    }
}

/// How much more (or less) likely this person is to act on a grudge.
pub fn revenge_drive(world: &World, id: NpcId) -> f32 {
    let n = world.npc(id);
    let e = n.emotions;
    let nerve = 1.0 - n.temperament.courage;
    e.anger / 200.0 + e.grief / 300.0 + n.temperament.temper * 0.15 - e.fear * nerve / 250.0
}

/// How likely this person is to give in to pressure (sign the petition).
pub fn pliability(world: &World, id: NpcId) -> f32 {
    let n = world.npc(id);
    let e = n.emotions;
    e.fear / 200.0 - e.zeal / 200.0 - n.temperament.courage * 0.1 - n.temperament.loyalty * 0.2
}

/// Whether this person would take what isn't theirs, this once.
pub fn would_steal(world: &World, id: NpcId) -> f32 {
    let n = world.npc(id);
    let t = n.temperament;
    ((1.2 - t.honesty) * (1.0 - 0.5 * t.piety) * (0.5 + t.temper)).clamp(0.0, 1.0)
}

/// Average fitness of a family's working hands, 0..1+: drives the harvest.
/// Where a ball went in. A healing wound and an old one both show: the
/// gun arm spoils the aim and the draw (`action`), a leg slows the walk and
/// the work and the crawl (`labor`, raids), the chest turns bad more often
/// (`sickness`), and a head wound costs memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limb {
    Head,
    Chest,
    GunArm,
    OffArm,
    Leg,
}

impl Limb {
    pub const ALL: [Limb; 5] = [
        Limb::Head,
        Limb::Chest,
        Limb::GunArm,
        Limb::OffArm,
        Limb::Leg,
    ];

    pub fn bit(self) -> u8 {
        1 << (self as u8)
    }

    pub fn label(self) -> &'static str {
        match self {
            Limb::Head => "the head",
            Limb::Chest => "the chest",
            Limb::GunArm => "the gun arm",
            Limb::OffArm => "the other arm",
            Limb::Leg => "the leg",
        }
    }
}

pub fn labor(world: &World, family: u32) -> f32 {
    // Hired out, a man works someone else's ground; in jail, nobody's.
    let hands: Vec<f32> = world
        .living()
        .filter(|n| {
            let mine = n.family == family && !world.hands.is_hired(n.id);
            let hired = family == 0 && world.hands.is_hired(n.id);
            (mine || hired)
                && LifeStage::of(n.age) != LifeStage::Child
                && !super::warrant::held(world, n.id)
        })
        .map(|n| {
            let grief = 1.0 - n.emotions.grief / 200.0;
            let wound = if n.wounded { 0.3 } else { 1.0 };
            let lame = if n.scars & Limb::Leg.bit() != 0 {
                0.85
            } else {
                1.0
            };
            let sick = super::sickness::laid_up(world, n.id);
            let survivor = if super::character::is(n, super::character::Archetype::Survivor) {
                1.3
            } else {
                1.0
            };
            (0.5 + n.body.strength) * n.health as f32 / 100.0
                * grief
                * wound
                * survivor
                * lame
                * sick
        })
        .collect();
    if hands.is_empty() {
        0.2
    } else {
        hands.iter().sum::<f32>() / 2.0
    }
}

/// Derived: how the county sees someone, -100..100 (mean opinion of the living).
pub fn reputation(world: &World, id: NpcId) -> f32 {
    let others: Vec<i16> = world
        .living()
        .filter(|n| n.id != id)
        .map(|n| world.opinion(n.id, id))
        .collect();
    if others.is_empty() {
        0.0
    } else {
        others.iter().map(|&o| o as f32).sum::<f32>() / others.len() as f32
    }
}

pub fn feel(world: &mut World, id: NpcId, f: impl FnOnce(&mut Emotions)) {
    let n = world.npc_mut(id);
    f(&mut n.emotions);
    n.emotions.clamp();
}

/// Daily: bodies heal or waste, emotions decay and spread.
pub fn daily(world: &mut World) {
    let today = world.day;
    let blizzard = world.weather_on(today).blizzard;
    let winter = today.season() == Season::Winter;

    // Bodies.
    for i in 0..world.npcs.len() {
        if !world.npcs[i].alive {
            continue;
        }
        let family = world.npcs[i].family as usize;
        let hungry = world.families[family].stores.hungry_days > 0;
        let exposed = blizzard && !world.families[family].barn_standing;
        let stage = LifeStage::of(world.npcs[i].age);
        let n = &mut world.npcs[i];
        let frailty = n.body.frailty();
        if hungry {
            n.health -= (stage.starvation_rate() as f32 * frailty).round() as i32;
            // Hunger frightens, and it angers: someone must be to blame.
            n.emotions.fear += 1.5;
            n.emotions.anger += 1.5;
        } else if n.health < 100 {
            n.health += 1;
        }
        // A buffalo coat against a blizzard; a calico dress against January.
        let warmth = n.outfit.warmth();
        if exposed {
            n.health -= (3.0 * frailty * (1.2 - warmth).clamp(0.3, 1.2)).round() as i32;
        } else if winter && warmth < 0.25 && today.0.is_multiple_of(3) {
            n.health -= 1;
        }
        if n.wounded && n.health >= 80 {
            n.wounded = false;
            // Some wounds heal crooked: a stiff arm, a limp, a gap in memory.
            if let Some(limb) = n.hurt.take()
                && (today.0 + n.id) % 10 < 3
            {
                n.scars |= limb.bit();
                if limb == Limb::Head {
                    n.body.recall = (n.body.recall - 0.15).max(0.1);
                }
            }
        }
        if winter && stage != LifeStage::Adult && world.rng.chance(0.002) {
            // Winter fevers find the young and the old.
            let hit = 15.0 * world.npcs[i].body.frailty();
            world.npcs[i].health -= hit as i32;
        }
        world.npcs[i].health = world.npcs[i].health.clamp(0, 100);
    }

    // Emotions: decay, then contagion.
    type Row = (NpcId, u32, (i32, i32), Emotions, bool);
    let snapshot: Vec<Row> = world
        .npcs
        .iter()
        .map(|n| (n.id, n.family, world.farm_of(n.id), n.emotions, n.alive))
        .collect();
    for (id, family, farm, _, alive) in &snapshot {
        if !alive {
            continue;
        }
        let mut kin = (Emotions::default(), 0.0);
        let mut near = (Emotions::default(), 0.0);
        for (oid, ofam, ofarm, oe, oalive) in &snapshot {
            if oid == id || !oalive {
                continue;
            }
            let bucket = if ofam == family {
                &mut kin
            } else if distance(*farm, *ofarm) <= NEIGHBOR_RANGE {
                &mut near
            } else {
                continue;
            };
            bucket.0.fear += oe.fear;
            bucket.0.anger += oe.anger;
            bucket.0.grief += oe.grief;
            bucket.0.zeal += oe.zeal;
            bucket.1 += 1.0;
        }
        let n = world.npc_mut(*id);
        let e = &mut n.emotions;
        e.fear *= 0.97;
        e.anger *= 0.98;
        e.grief *= 0.995;
        e.zeal *= 0.99;
        for (bucket, pull) in [(kin, FAMILY_PULL), (near, NEIGHBOR_PULL)] {
            if bucket.1 > 0.0 {
                let avg = |x: f32| x / bucket.1;
                e.fear += (avg(bucket.0.fear) - e.fear) * pull;
                e.anger += (avg(bucket.0.anger) - e.anger) * pull;
                e.grief += (avg(bucket.0.grief) - e.grief) * pull * 0.5;
                e.zeal += (avg(bucket.0.zeal) - e.zeal) * pull;
            }
        }
        e.clamp();
        // Zeal drags your public stance toward your side; so does grief with a culprit.
        let sign = n.ideology.private.signum();
        let pull = (n.ideology.private - n.ideology.public) * 0.002;
        n.ideology.public =
            (n.ideology.public + sign * n.emotions.zeal / 20000.0 + pull).clamp(-1.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn children_starve_first() {
        assert!(LifeStage::Child.starvation_rate() > LifeStage::Adult.starvation_rate());
        assert_eq!(LifeStage::of(10), LifeStage::Child);
        assert_eq!(LifeStage::of(30), LifeStage::Adult);
        assert_eq!(LifeStage::of(60), LifeStage::Elder);
    }

    #[test]
    fn hardiness_reduces_frailty() {
        let tough = Body {
            hardiness: 1.0,
            ..Default::default()
        };
        let weak = Body::default();
        assert!(tough.frailty() < weak.frailty());
    }

    #[test]
    fn anger_drives_revenge_and_fear_restrains_it() {
        let mut w = World::new(1);
        let base = revenge_drive(&w, 1);
        feel(&mut w, 1, |e| e.anger = 90.0);
        assert!(revenge_drive(&w, 1) > base);
        let angry = revenge_drive(&w, 1);
        w.npc_mut(1).temperament.courage = 0.0;
        feel(&mut w, 1, |e| e.fear = 90.0);
        assert!(revenge_drive(&w, 1) < angry);
    }

    #[test]
    fn emotions_spread_through_a_family() {
        let mut w = World::new(1);
        let kin: Vec<_> = w.living().filter(|n| n.family == 1).map(|n| n.id).collect();
        feel(&mut w, kin[0], |e| e.grief = 100.0);
        let before = w.npc(kin[1]).emotions.grief;
        daily(&mut w);
        assert!(w.npc(kin[1]).emotions.grief > before);
    }

    #[test]
    fn signing_against_conscience_is_hypocrisy() {
        let mut i = Ideology::for_faction(Faction::FreeState, 0.8);
        assert_eq!(i.hypocrisy(), 0.0);
        i.public -= 0.4;
        assert!(i.hypocrisy() > 0.3);
    }
}
