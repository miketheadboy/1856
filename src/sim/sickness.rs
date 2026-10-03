//! Sickness (the illness pass). The frontier's killers weren't the border
//! war's rifles: they were ague in the creek bottoms every fall, cholera up
//! the river with the emigrants, the flux in August, typhus in a lousy cabin
//! in February, measles and the whooping cough through the schoolhouse, and
//! a wound gone bad. Nobody saw a germ. People saw a neighbor's child cough
//! at meeting, and the doctor bleed their husband, and believed what they
//! believed.
//!
//! - **Onset.** By season, ground and water (ague near the timber, cholera
//!   and typhoid where there's no well, or where someone fouled it), by
//!   cold and rags, by lice, and by contact: household first, then the
//!   school and the neighbors.
//! - **Course.** Days sick, a daily chance of dying that rises with youth,
//!   age, frailty and hunger, and health drained. Some leave you immune;
//!   ague and consumption stay for good.
//! - **Treatment.** Quinine truly works on ague. The doctor's calomel and
//!   the lancet mostly don't, and people pay for them anyway. Boneset,
//!   willow bark, slippery elm and spring greens help a little or a lot;
//!   boiled water and broth help the bowel fevers more than anything the
//!   doctor carries. Cowpox protects against smallpox.
//! - **Fouled bedding.** Lice carry typhus. A lousy blanket given as a
//!   kindness is a dark verb, read through attribution like a fouled well.

use super::calendar::{Day, Season};
use super::events::{Cruelty, EventKind, Hardship, WorldEvent};
use super::homestead::{self, Improvement};
use super::psyche::LifeStage;
use super::world::{FamilyId, NpcId, PLAYER, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Disease {
    /// Fever and ague: malaria, from the creek bottoms, July to October.
    Ague,
    Cholera,
    Typhoid,
    /// Dysentery: August, bad water, bad food.
    Flux,
    Measles,
    WhoopingCough,
    Diphtheria,
    Smallpox,
    /// Jail fever, ship fever: lice, cold, crowding.
    Typhus,
    /// The slow one.
    Consumption,
    /// Late winter on corn and salt pork.
    Scurvy,
    /// Pneumonia, from cold and rags.
    LungFever,
    /// A wound gone bad.
    WoundFever,
}

impl Disease {
    pub const ALL: [Disease; 13] = [
        Disease::Ague,
        Disease::Cholera,
        Disease::Typhoid,
        Disease::Flux,
        Disease::Measles,
        Disease::WhoopingCough,
        Disease::Diphtheria,
        Disease::Smallpox,
        Disease::Typhus,
        Disease::Consumption,
        Disease::Scurvy,
        Disease::LungFever,
        Disease::WoundFever,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Disease::Ague => "the ague",
            Disease::Cholera => "cholera",
            Disease::Typhoid => "typhoid",
            Disease::Flux => "the flux",
            Disease::Measles => "measles",
            Disease::WhoopingCough => "the whooping cough",
            Disease::Diphtheria => "diphtheria",
            Disease::Smallpox => "smallpox",
            Disease::Typhus => "typhus",
            Disease::Consumption => "consumption",
            Disease::Scurvy => "scurvy",
            Disease::LungFever => "lung fever",
            Disease::WoundFever => "a wound gone bad",
        }
    }

    /// (min days, max days, daily death risk for a grown person at the
    /// worst of it, extra risk for children, catching: leaves you immune).
    fn profile(self) -> (u32, u32, f32, f32, bool) {
        match self {
            Disease::Ague => (7, 21, 0.001, 1.5, false),
            Disease::Cholera => (3, 7, 0.10, 1.5, false),
            Disease::Typhoid => (14, 28, 0.012, 1.3, true),
            Disease::Flux => (5, 14, 0.006, 2.0, false),
            Disease::Measles => (8, 14, 0.002, 4.0, true),
            Disease::WhoopingCough => (20, 40, 0.0008, 4.0, true),
            Disease::Diphtheria => (6, 12, 0.012, 3.0, true),
            Disease::Smallpox => (14, 21, 0.018, 1.5, true),
            Disease::Typhus => (10, 18, 0.008, 1.0, true),
            Disease::Consumption => (900, 1500, 0.0003, 1.0, false),
            Disease::Scurvy => (20, 45, 0.0005, 1.0, false),
            Disease::LungFever => (7, 14, 0.012, 1.5, false),
            Disease::WoundFever => (6, 14, 0.02, 1.0, false),
        }
    }

    /// Passed person to person (and how readily, per day, in one house).
    fn contagion(self) -> f32 {
        match self {
            Disease::Measles => 0.12,
            Disease::WhoopingCough => 0.08,
            Disease::Diphtheria => 0.06,
            Disease::Smallpox => 0.07,
            Disease::Typhus => 0.03,
            Disease::Consumption => 0.0008,
            _ => 0.0,
        }
    }

    /// A fever of the bowels: boiled water and broth help.
    fn bowel(self) -> bool {
        matches!(self, Disease::Cholera | Disease::Typhoid | Disease::Flux)
    }

    /// A fever: willow bark and boneset help a little.
    fn fever(self) -> bool {
        !matches!(self, Disease::Scurvy | Disease::Consumption)
    }
}

/// The medicine chest, by household (`Household::medicine`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Remedy {
    /// Peruvian bark. The one thing that works on ague. Dear.
    Quinine,
    /// Mercury: the doctor's purge. Believed in; does harm.
    Calomel,
    /// Opium in whiskey: eases, doesn't cure.
    Laudanum,
    /// Bitter, for fevers. A little help.
    WillowBark,
    /// Boneset tea, "ague weed". Some help for fevers, more for ague.
    Boneset,
    /// Slippery elm, for the flux.
    SlipperyElm,
    /// Wild onions, lamb's quarters, potatoes from the cellar: cures scurvy.
    Greens,
}

pub const MEDS: usize = 7;

impl Remedy {
    pub const ALL: [Remedy; MEDS] = [
        Remedy::Quinine,
        Remedy::Calomel,
        Remedy::Laudanum,
        Remedy::WillowBark,
        Remedy::Boneset,
        Remedy::SlipperyElm,
        Remedy::Greens,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            Remedy::Quinine => "quinine",
            Remedy::Calomel => "calomel",
            Remedy::Laudanum => "laudanum",
            Remedy::WillowBark => "willow bark",
            Remedy::Boneset => "boneset",
            Remedy::SlipperyElm => "slippery elm",
            Remedy::Greens => "spring greens",
        }
    }

    /// Dollars a dose at Dunmore's; zero if it grows wild instead.
    pub fn price(self) -> i32 {
        match self {
            Remedy::Quinine => 2,
            Remedy::Calomel | Remedy::Laudanum => 1,
            _ => 0,
        }
    }

    /// What a dose does to the daily risk of dying of `d`, and whether it
    /// ends the case outright.
    fn effect(self, d: Disease) -> (f32, bool) {
        match (self, d) {
            (Remedy::Quinine, Disease::Ague) => (0.2, false),
            (Remedy::Quinine, _) if d.fever() => (0.9, false),
            // The heroic cure: it purges the patient along with the hope.
            (Remedy::Calomel, _) => (1.2, false),
            (Remedy::Laudanum, _) if d.bowel() => (0.9, false),
            (Remedy::WillowBark, _) if d.fever() => (0.85, false),
            (Remedy::Boneset, Disease::Ague) => (0.6, false),
            (Remedy::Boneset, _) if d.fever() => (0.85, false),
            (Remedy::SlipperyElm, Disease::Flux) => (0.7, false),
            (Remedy::Greens, Disease::Scurvy) => (0.0, true),
            _ => (1.0, false),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Case {
    pub who: NpcId,
    pub disease: Disease,
    pub since: Day,
    pub left: u32,
    /// Multiplier on the daily risk from what's been done for them.
    pub care: f32,
    /// Nursed with boiled water and broth.
    pub nursed: bool,
    /// Bled and purged by the doctor.
    pub doctored: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Sickness {
    pub cases: Vec<Case>,
    /// (who, what): had it and lived, or were vaccinated.
    pub immune: Vec<(NpcId, Disease)>,
    /// (who, what): carry it for life. Ague comes back every fall;
    /// consumption only goes one way.
    pub chronic: Vec<(NpcId, Disease)>,
    /// Households with lice in the bedding.
    pub lousy: Vec<FamilyId>,
    /// (household, until): someone fouled the well.
    pub fouled: Vec<(FamilyId, Day)>,
    /// Cholera up the river until this day.
    pub cholera_until: Option<Day>,
    /// Children kept home from school and meeting.
    pub quarantine: Vec<FamilyId>,
}

/// The Lawrence physician. John Doy practiced there from 1854 (approximate:
/// his practice is documented; his fees and methods here are period-typical).
pub const DOCTOR: &str = "Dr. Doy";
/// Dollars to bring him out to a claim.
pub const DOCTOR_FEE: i32 = 3;

impl Sickness {
    pub fn sick(&self, id: NpcId) -> Option<&Case> {
        self.cases.iter().find(|c| c.who == id)
    }

    fn has(&self, id: NpcId, d: Disease) -> bool {
        self.cases.iter().any(|c| c.who == id && c.disease == d)
    }

    fn immune_to(&self, id: NpcId, d: Disease) -> bool {
        self.immune.contains(&(id, d))
    }
}

/// Laid up: how much of a day's work a sick body gives (read by labor).
pub fn laid_up(world: &World, id: NpcId) -> f32 {
    match world.sickness.sick(id) {
        Some(c) if c.disease == Disease::Consumption => 0.7,
        Some(c) if c.disease == Disease::Ague => 0.5,
        Some(_) => 0.2,
        None => 1.0,
    }
}

/// Grown-ups had the childhood fevers back East. Hashed, not rolled.
pub fn founding(world: &mut World) {
    for n in world.npcs.iter() {
        let z = (world.seed ^ (n.id as u64).wrapping_mul(0x2545_F491_4F6C_DD1D)) % 100;
        if LifeStage::of(n.age) != LifeStage::Child {
            for d in [Disease::Measles, Disease::WhoopingCough] {
                if z < 85 {
                    world.sickness.immune.push((n.id, d));
                }
            }
            if z < 40 {
                world.sickness.immune.push((n.id, Disease::Smallpox));
            }
        }
    }
}

pub(crate) fn catch(world: &mut World, who: NpcId, d: Disease) -> bool {
    catch_from(world, who, d, None)
}

/// Caught from something that happened: the onset is that event's child in
/// the chronicle, so the web shows (a wagon home from the river, cholera).
pub(crate) fn catch_from(
    world: &mut World,
    who: NpcId,
    d: Disease,
    cause: Option<super::events::EventId>,
) -> bool {
    let n = world.npc(who);
    if !n.alive || world.sickness.has(who, d) || world.sickness.immune_to(who, d) {
        return false;
    }
    let (lo, hi, ..) = d.profile();
    let left = world.rng.range(lo, hi + 1);
    world.sickness.cases.push(Case {
        who,
        disease: d,
        since: world.day,
        left,
        care: 1.0,
        nursed: false,
        doctored: false,
    });
    world.emit_root(EventKind::FellSick { who, disease: d }, cause);
    true
}

/// Chance a teamster back from Westport brings the river's cholera home.
pub const RIVER_CHOLERA: f32 = 0.35;

fn members(world: &World, f: FamilyId) -> Vec<NpcId> {
    world
        .living()
        .filter(|n| n.family == f && !n.departed)
        .map(|n| n.id)
        .collect()
}

/// Close to the timber is close to the creek, and the mosquitoes.
fn bottoms(world: &World, f: FamilyId) -> bool {
    let farm = world.families[f as usize].farm;
    world
        .map
        .nearest_timber(farm)
        .is_some_and(|t| super::world::distance(t, farm) <= 3.0)
}

pub fn daily(world: &mut World) {
    let today = world.day;
    let month = today.month();
    world.sickness.fouled.retain(|f| today < f.1);
    if world.sickness.cholera_until.is_some_and(|d| today >= d) {
        world.sickness.cholera_until = None;
    }
    // Cholera comes up the Missouri with the boats in the hot months.
    if (6..=8).contains(&month)
        && world.sickness.cholera_until.is_none()
        && world.rng.chance(0.0025)
    {
        world.sickness.cholera_until = Some(Day(today.0 + 30));
        world.emit_root(
            EventKind::Epidemic {
                disease: Disease::Cholera,
            },
            None,
        );
    }
    onsets(world, month);
    spread(world);
    progress(world);
    world.run_cascades();
}

fn onsets(world: &mut World, month: u32) {
    let today = world.day;
    let winter = today.season() == Season::Winter;
    let cholera = world.sickness.cholera_until.is_some();
    for f in 0..world.families.len() as FamilyId {
        let people = members(world, f);
        if people.is_empty() {
            continue;
        }
        let well = homestead::has(world, f, Improvement::Well);
        let fouled = world.sickness.fouled.iter().any(|x| x.0 == f);
        let hungry = world.families[f as usize].stores.hungry_days > 0;
        let low = bottoms(world, f);
        let rags = people
            .iter()
            .map(|&p| world.npc(p).outfit.wear)
            .sum::<f32>()
            / people.len() as f32;
        // Lice come with rags, cold and a crowded cabin.
        if winter && !world.sickness.lousy.contains(&f) {
            let p = 0.002 * (1.0 + 3.0 * rags) * if hungry { 2.0 } else { 1.0 };
            if world.rng.chance(p) {
                world.sickness.lousy.push(f);
            }
        }
        // Lice go with warm weather, soap and a careful housewife.
        if world.sickness.lousy.contains(&f) && f != 0 {
            let careful = world.families[f as usize].stores.prudence;
            let summer = if winter { 0.0 } else { 0.03 };
            if world.rng.chance(0.01 + 0.03 * careful + summer) {
                world.sickness.lousy.retain(|&x| x != f);
            }
        }
        let lousy = world.sickness.lousy.contains(&f);
        for p in people {
            let n = world.npc(p);
            let warm = n.outfit.warmth();
            let mut risks: Vec<(Disease, f32)> = Vec::with_capacity(4);
            if (7..=10).contains(&month) {
                risks.push((Disease::Ague, if low { 0.004 } else { 0.0012 }));
            }
            if (7..=9).contains(&month) {
                // A creek gone to warm mud in a dry August is worst.
                let water = if well {
                    0.4
                } else if super::larder::water(world, f) == super::larder::Water::Creek
                    && super::larder::creek_low(world)
                {
                    1.6
                } else {
                    1.0
                };
                risks.push((
                    Disease::Flux,
                    0.0012 * water * if hungry { 2.0 } else { 1.0 },
                ));
            }
            if cholera {
                risks.push((Disease::Cholera, if well { 0.0008 } else { 0.003 }));
            }
            let typhoid = if fouled {
                0.03
            } else if (8..=10).contains(&month) && !well {
                0.0004
            } else {
                0.0
            };
            if typhoid > 0.0 {
                risks.push((Disease::Typhoid, typhoid));
            }
            if lousy {
                risks.push((Disease::Typhus, 0.0006));
            }
            // No potato, no turnip, no greens since the fall: the gums go.
            if (2..=5).contains(&month) && !super::larder::greens(world, f) {
                risks.push((Disease::Scurvy, if hungry { 0.0015 } else { 0.0005 }));
            }
            // Out in it in rags: the grown ones work outside, the children
            // are kept by the stove.
            if winter && warm < 0.3 {
                let indoors = if LifeStage::of(n.age) == LifeStage::Child {
                    0.4
                } else {
                    1.0
                };
                risks.push((Disease::LungFever, 0.0009 * (1.5 - warm) * indoors));
            }
            if n.age >= 20 {
                risks.push((Disease::Consumption, 0.00001));
            }
            // The childhood fevers find a first child now and then; after
            // that they travel on their own.
            if LifeStage::of(n.age) == LifeStage::Child && (month <= 5 || month >= 11) {
                risks.push((Disease::Measles, 0.0002));
                risks.push((Disease::WhoopingCough, 0.0002));
                risks.push((Disease::Diphtheria, 0.00012));
            }
            risks.push((Disease::Smallpox, 0.00002));
            for (d, r) in risks {
                if world.sickness.has(p, d) {
                    continue;
                }
                if world.rng.chance(r) {
                    catch(world, p, d);
                }
            }
            // Old ague comes back every fall.
            if (8..=10).contains(&month)
                && world.sickness.chronic.contains(&(p, Disease::Ague))
                && world.rng.chance(0.004)
            {
                catch(world, p, Disease::Ague);
            }
        }
    }
}

/// House to house: the sick one's household first, then a neighbor's
/// children at school or meeting, then lice in a borrowed blanket.
fn spread(world: &mut World) {
    let school = world.civic.built(super::civic::Project::Schoolhouse);
    let cases = world.sickness.cases.clone();
    let families = world.families.len() as u32;
    for c in cases {
        let rate = c.disease.contagion();
        if rate <= 0.0 {
            continue;
        }
        let f = world.npc(c.who).family;
        for k in members(world, f) {
            if k != c.who && world.rng.chance(rate) {
                catch(world, k, c.disease);
            }
        }
        if world.sickness.quarantine.contains(&f) {
            continue;
        }
        let out =
            if school { 0.04 } else { 0.02 } * world.npc(c.who).temperament.sociability.max(0.3);
        if world.rng.chance(out) {
            let other = world.rng.range(0, families) as FamilyId;
            if other != f && !world.sickness.quarantine.contains(&other) {
                let kids: Vec<NpcId> = members(world, other);
                if let Some(&k) = kids.first() {
                    let pick = kids
                        .iter()
                        .copied()
                        .find(|&k| LifeStage::of(world.npc(k).age) == LifeStage::Child)
                        .unwrap_or(k);
                    catch(world, pick, c.disease);
                    if c.disease == Disease::Typhus && !world.sickness.lousy.contains(&other) {
                        world.sickness.lousy.push(other);
                    }
                }
            }
        }
    }
}

fn progress(world: &mut World) {
    let mut i = 0;
    while i < world.sickness.cases.len() {
        let c = world.sickness.cases[i];
        let n = world.npc(c.who);
        if !n.alive {
            world.sickness.cases.remove(i);
            continue;
        }
        let (_, _, risk, child, immune) = c.disease.profile();
        let stage = LifeStage::of(n.age);
        let age = match stage {
            LifeStage::Child => 0.5 * child * if n.age < 3 { 1.8 } else { 1.0 },
            LifeStage::Elder => 1.8,
            LifeStage::Adult => 1.0,
        };
        let fam = &world.families[n.family as usize].stores;
        let hungry = if fam.hungry_days > 0 { 1.5 } else { 1.0 };
        let weak = if n.health < 50 { 1.5 } else { 1.0 };
        let nursed = if c.nursed && c.disease.bowel() {
            0.7
        } else {
            1.0
        };
        let p = risk * age * hungry * weak * nursed * c.care * n.body.frailty();
        let drain = if c.disease == Disease::Consumption {
            if world.day.0.is_multiple_of(12) { 1 } else { 0 }
        } else {
            1
        };
        world.npcs[c.who as usize].health -= drain;
        if world.rng.chance(p.min(0.5)) {
            world.sickness.cases.remove(i);
            let cause = Hardship::Sickness(c.disease);
            let (kind, why) = super::systems::perished(world, c.who, cause);
            world.emit_root(kind, why);
            continue;
        }
        if c.left <= 1 {
            world.sickness.cases.remove(i);
            if immune {
                world.sickness.immune.push((c.who, c.disease));
            }
            let lingers = match c.disease {
                Disease::Ague => world.rng.chance(0.5),
                Disease::Consumption => true,
                _ => false,
            };
            if lingers && !world.sickness.chronic.contains(&(c.who, c.disease)) {
                world.sickness.chronic.push((c.who, c.disease));
            }
            world.emit_root(
                EventKind::Recovered {
                    who: c.who,
                    disease: c.disease,
                },
                None,
            );
            continue;
        }
        world.sickness.cases[i].left -= 1;
        i += 1;
    }
}

// ---- care -----------------------------------------------------------------

/// Give a dose from the household's chest. Returns false if there's none,
/// or nobody sick in the house.
pub fn dose(world: &mut World, who: NpcId, r: Remedy) -> bool {
    let fam = world.npc(who).family as usize;
    let Some(i) = world.sickness.cases.iter().position(|c| c.who == who) else {
        return false;
    };
    let chest = &mut world.families[fam].stores.medicine;
    if chest[r.index()] == 0 {
        return false;
    }
    chest[r.index()] -= 1;
    let d = world.sickness.cases[i].disease;
    let (mult, cures) = r.effect(d);
    if cures {
        world.sickness.cases[i].left = 1;
    }
    let c = &mut world.sickness.cases[i];
    c.care = (c.care * mult).clamp(0.05, 2.0);
    if r == Remedy::Quinine && d == Disease::Ague {
        c.left = c.left.min(4);
    }
    true
}

/// Sit up with them: boiled water and broth, a cool cloth. Takes the day.
pub fn nurse(world: &mut World, who: NpcId) -> bool {
    if world.life.acted_on == Some(world.day) {
        return false;
    }
    let Some(c) = world.sickness.cases.iter_mut().find(|c| c.who == who) else {
        return false;
    };
    c.nursed = true;
    world.life.acted_on = Some(world.day);
    world.life.learn(super::life::Skill::Physic, 2.0);
    true
}

/// Bring the doctor out. He does what doctors did: quinine for ague (and it
/// works), cowpox for anyone who wants it, and for the rest the lancet and
/// calomel (which don't, and cost the same).
pub fn doctor(world: &mut World, family: FamilyId) -> bool {
    let cash = world.families[family as usize].stores.cash;
    if cash < DOCTOR_FEE {
        return false;
    }
    world.families[family as usize].stores.cash -= DOCTOR_FEE;
    let people = members(world, family);
    for &p in &people {
        if let Some(i) = world.sickness.cases.iter().position(|c| c.who == p) {
            let d = world.sickness.cases[i].disease;
            let c = &mut world.sickness.cases[i];
            match d {
                Disease::Ague => {
                    c.care *= 0.2;
                    c.left = c.left.min(4);
                }
                Disease::WoundFever => {
                    // Take the limb and the fever goes with it, mostly.
                    c.care *= 0.3;
                    c.left = c.left.min(3);
                    world.npcs[p as usize].scars |= super::psyche::Limb::Leg.bit();
                }
                _ => {
                    c.care *= 1.2;
                    c.doctored = true;
                    world.npcs[p as usize].health -= 5;
                }
            }
            c.care = c.care.clamp(0.05, 2.0);
        }
        if !world.sickness.immune_to(p, Disease::Smallpox) {
            world.sickness.immune.push((p, Disease::Smallpox));
        }
    }
    world.emit_root(EventKind::DoctorCalled { family }, None);
    world.run_cascades();
    true
}

/// Gather what grows: willow and elm bark all year, boneset in summer and
/// fall, greens in spring. What you know is what you find.
pub fn gather(world: &mut World) -> Vec<Remedy> {
    let month = world.day.month();
    let knack = world.life.skill(super::life::Skill::Physic);
    let mut found = Vec::new();
    let mut offer = vec![Remedy::WillowBark, Remedy::SlipperyElm];
    if (6..=10).contains(&month) {
        offer.push(Remedy::Boneset);
    }
    if (3..=6).contains(&month) {
        offer.push(Remedy::Greens);
    }
    for r in offer {
        if world.rng.chance(0.35 + 0.55 * knack) {
            world.families[0].stores.medicine[r.index()] += 1 + (2.0 * knack) as u8;
            found.push(r);
        }
    }
    world.life.learn(super::life::Skill::Physic, 2.0);
    found
}

pub fn buy(world: &mut World, r: Remedy) -> bool {
    let mut p = r.price();
    if p == 0 {
        return false;
    }
    // Everyone wants bark in ague season, and the river's shut in a blockade.
    if (8..=10).contains(&world.day.month()) && r == Remedy::Quinine {
        p += 1;
    }
    if world.market.blockade {
        p += 1;
    }
    if world.families[0].stores.cash < p {
        return false;
    }
    world.families[0].stores.cash -= p;
    world.families[0].stores.medicine[r.index()] += 1;
    true
}

/// Boil the bedding, burn what can't be saved. Takes the day.
pub fn delouse(world: &mut World) -> bool {
    if world.life.acted_on == Some(world.day) || !world.sickness.lousy.contains(&0) {
        return false;
    }
    world.life.acted_on = Some(world.day);
    world.sickness.lousy.retain(|&f| f != 0);
    true
}

/// Keep the children home from school and meeting, or let them go.
pub fn quarantine(world: &mut World, on: bool) {
    world.sickness.quarantine.retain(|&f| f != 0);
    if on {
        world.sickness.quarantine.push(0);
    }
}

/// Give a neighbor a blanket out of a lousy house. It looks like kindness;
/// a week on, the lice have moved in.
pub fn foul_blanket(world: &mut World, target: NpcId) -> bool {
    if !world.sickness.lousy.contains(&0) || world.life.acted_on == Some(world.day) {
        return false;
    }
    let fam = world.npc(target).family;
    if fam == 0 {
        return false;
    }
    world.life.acted_on = Some(world.day);
    world.adjust_opinion(target, PLAYER, 5);
    let gift = world.emit_root(
        EventKind::Gift {
            from: PLAYER,
            to: target,
        },
        None,
    );
    world.schedule(
        7,
        EventKind::Cruelty {
            actor: PLAYER,
            victim: target,
            act: Cruelty::FouledBlanket,
        },
        gift,
    );
    world.run_cascades();
    true
}

/// NPC households tend their own: the chest, and the doctor if they can pay
/// and trust him.
pub fn tend(world: &mut World) {
    let cases = world.sickness.cases.clone();
    for c in cases {
        let fam = world.npc(c.who).family;
        if fam == 0 && !world.autopilot_player {
            continue;
        }
        for r in Remedy::ALL {
            let (m, cures) = r.effect(c.disease);
            if (m < 1.0 || cures) && world.families[fam as usize].stores.medicine[r.index()] > 0 {
                dose(world, c.who, r);
                break;
            }
        }
        // Someone sits up with them: the family's own if anyone's on their
        // feet, and the neighbor women for a family whose name is good. The
        // widow alone and the jailbird's house wait longer (`standing`).
        if !c.nursed {
            let well = world.living().any(|n| {
                n.family == fam
                    && n.id != c.who
                    && world.sickness.sick(n.id).is_none()
                    && LifeStage::of(n.age) != LifeStage::Child
            });
            let name = world
                .head_of(fam)
                .map_or(0.5, |h| super::standing::word(world, h).min(1.0));
            let p = if well { 0.25 } else { 0.0 } + 0.2 * name;
            if world.rng.chance(p)
                && let Some(x) = world.sickness.cases.iter_mut().find(|x| x.who == c.who)
            {
                x.nursed = true;
            }
        }
        let t = world.npc(c.who).temperament;
        if c.left > 3
            && world.families[fam as usize].stores.cash >= DOCTOR_FEE + 10
            && world.rng.chance(0.03 * (1.0 - t.skepticism))
        {
            doctor(world, fam);
        }
    }
}

pub fn on_event(world: &mut World, ev: &WorldEvent) {
    // The dead are off the sick list the day they die, not the next time
    // the fever is walked round (the auditor caught them lingering).
    if let EventKind::Death { victim, .. } | EventKind::Perished { victim, .. } = ev.kind {
        world.sickness.cases.retain(|c| c.who != victim);
    }
    match ev.kind {
        EventKind::Cruelty {
            victim,
            act: Cruelty::FoulWell,
            ..
        } => {
            let f = world.npc(victim).family;
            world.sickness.fouled.push((f, Day(ev.day.0 + 40)));
        }
        EventKind::Cruelty {
            victim,
            act: Cruelty::FouledBlanket,
            ..
        } => {
            let f = world.npc(victim).family;
            if !world.sickness.lousy.contains(&f) {
                world.sickness.lousy.push(f);
            }
        }
        // The levee in a cholera summer: he slept by the boats, drank the
        // river, and came home with it. His house takes it from him.
        EventKind::WagonBack {
            teamster,
            route_west: true,
            ..
        } if world.sickness.cholera_until.is_some() => {
            if world.rng.chance(RIVER_CHOLERA) {
                catch_from(world, teamster, Disease::Cholera, Some(ev.id));
            }
        }
        // A wound gone bad: dirt in it, no clean cloth, and a hot week.
        EventKind::Wounded { victim, .. } => {
            let chest = world.npc(victim).hurt == Some(super::psyche::Limb::Chest);
            let p = if chest { 0.35 } else { 0.2 };
            if world.rng.chance(p) {
                catch(world, victim, Disease::WoundFever);
            }
        }
        EventKind::Perished {
            victim,
            cause: Hardship::Sickness(_),
        } => {
            // Burying one of their own frightens a family into keeping the
            // children home, if they're the careful kind.
            let f = world.npc(victim).family;
            if f != 0
                && world.families[f as usize].stores.prudence > 0.5
                && !world.sickness.quarantine.contains(&f)
            {
                world.sickness.quarantine.push(f);
            }
        }
        _ => {}
    }
}

/// Counts for the lab and the survey.
pub fn tally(world: &World) -> Vec<(Disease, usize, usize)> {
    Disease::ALL
        .iter()
        .map(|&d| {
            let sick = world
                .events
                .iter()
                .filter(|e| matches!(e.kind, EventKind::FellSick { disease, .. } if disease == d))
                .count();
            let dead = world
                .events
                .iter()
                .filter(|e| {
                    matches!(e.kind, EventKind::Perished { cause: Hardship::Sickness(x), .. } if x == d)
                })
                .count();
            (d, sick, dead)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        let mut w = World::new(9);
        w.autopilot_player = false;
        w
    }

    #[test]
    fn quinine_breaks_the_ague_and_calomel_does_not_help() {
        assert!(Remedy::Quinine.effect(Disease::Ague).0 < 0.5);
        assert!(Remedy::Calomel.effect(Disease::Cholera).0 > 1.0);
        assert!(Remedy::Greens.effect(Disease::Scurvy).1);
    }

    #[test]
    fn measles_goes_through_a_house() {
        let mut w = world();
        let fam = (1..w.families.len() as FamilyId)
            .find(|&f| {
                members(&w, f)
                    .iter()
                    .filter(|&&k| LifeStage::of(w.npc(k).age) == LifeStage::Child)
                    .count()
                    >= 1
                    && members(&w, f).len() >= 2
            })
            .unwrap();
        let ms = members(&w, fam);
        for &m in &ms {
            w.sickness.immune.retain(|x| x.0 != m);
        }
        catch(&mut w, ms[0], Disease::Measles);
        for _ in 0..14 {
            spread(&mut w);
        }
        let caught = ms.iter().filter(|&&m| {
            w.sickness.has(m, Disease::Measles) || w.sickness.immune_to(m, Disease::Measles)
        });
        assert!(caught.count() >= 2);
    }

    #[test]
    fn a_fouled_well_brings_typhoid() {
        let mut w = world();
        let f = 1;
        w.sickness.fouled.push((f, Day(w.day.0 + 40)));
        for _ in 0..40 {
            onsets(&mut w, 9);
        }
        assert!(
            members(&w, f)
                .iter()
                .any(|&m| w.sickness.has(m, Disease::Typhoid))
        );
    }

    #[test]
    fn the_doctor_vaccinates_and_bleeds() {
        let mut w = world();
        let me = PLAYER;
        catch(&mut w, me, Disease::Cholera);
        w.families[0].stores.cash = 10;
        let before = w.npc(me).health;
        assert!(doctor(&mut w, 0));
        assert!(w.sickness.immune_to(me, Disease::Smallpox));
        assert!(w.sickness.sick(me).unwrap().care > 1.0, "the lancet");
        assert!(w.npc(me).health < before);
    }

    #[test]
    fn greens_cure_scurvy() {
        let mut w = world();
        catch(&mut w, PLAYER, Disease::Scurvy);
        w.families[0].stores.medicine[Remedy::Greens.index()] = 1;
        assert!(dose(&mut w, PLAYER, Remedy::Greens));
        progress(&mut w);
        assert!(w.sickness.sick(PLAYER).is_none());
    }

    #[test]
    fn a_lousy_blanket_moves_the_lice() {
        let mut w = world();
        w.sickness.lousy.push(0);
        let t = w.head_of(1).unwrap();
        assert!(foul_blanket(&mut w, t));
        assert!(!w.sickness.lousy.contains(&1));
        for _ in 0..8 {
            w.advance_day();
        }
        assert!(w.sickness.lousy.contains(&1));
    }

    #[test]
    fn sickness_kills_some_and_not_all() {
        let (mut sick, mut dead) = (0, 0);
        for seed in 1..=6 {
            let mut w = World::new(seed);
            w.run_days(730);
            for (_, s, d) in tally(&w) {
                sick += s;
                dead += d;
            }
        }
        assert!(sick > 30, "{sick}");
        assert!(dead > 0 && dead * 4 < sick, "{dead}/{sick}");
    }
}
