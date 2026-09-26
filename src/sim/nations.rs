//! The eastern Kansas nations: sovereign peoples on treaty lands, pressed by
//! squatters and timber thieves, caught between two factions fighting over
//! land that had been theirs.
//!
//! Historically (Miner & Unrau, *The End of Indian Kansas*) none of them made
//! war on settlers in this period. They were pragmatic and legalistic: they
//! protested through their agents and leaned on treaty rights, stayed out of
//! the slavery fight, and lost property to both sides. The mechanics follow
//! that: pressure, complaints, trade, trust, and settler prejudice that pins
//! lost livestock on them.
//!
//! The plains nations (Cheyenne, Kiowa, Comanche, Arapaho, Pawnee, Sioux) are
//! off this map; they enter through trail safety and the news (`history`).

use super::events::{EventKind, Suspect};
use super::market::Good;
use super::psyche::LifeStage;
use super::world::{Faction, NpcId, World, distance};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NationId {
    Delaware,
    Shawnee,
    Kaw,
}

impl NationId {
    pub const ALL: [NationId; 3] = [NationId::Delaware, NationId::Shawnee, NationId::Kaw];

    pub fn name(self) -> &'static str {
        match self {
            NationId::Delaware => "Delaware",
            NationId::Shawnee => "Shawnee",
            NationId::Kaw => "Kaw",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug)]
pub struct Nation {
    pub id: NationId,
    /// Where settler contact happens, in county tiles (the reserve line, or the
    /// road toward it).
    pub border: (i32, i32),
    pub population: u32,
    /// 0 starving .. 1 secure.
    pub food_security: f32,
    /// Standing timber on the reserve, in wagon loads.
    pub timber: f32,
    /// 0..100: squatters, cut timber, trespass. Drives complaints and cessions.
    pub land_pressure: f32,
    /// Trust toward each settler faction, -100..100 (indexed by `Faction`).
    pub trust: [f32; 2],
    /// How hostile settlers are toward this nation, 0..100. Prejudice with a memory.
    pub settler_hostility: f32,
    /// Dollars paid under treaty, and the months they come.
    pub annuity: f32,
    pub annuity_months: &'static [u32],
    /// Dollars docked from the next annuity to pay settler depredation claims.
    pub docked: f32,
    /// 0..1: how settled-farming the nation's economy is. Farmers sell corn;
    /// hunters sell hides and suffer when the game goes.
    pub farming: f32,
    /// Shawnee: mission leaders (tied to the proslavery Johnson) vs
    /// traditionalists. Tilts trust toward the Pro-Slavery side.
    pub mission_tilt: f32,
    pub complaints: u32,
    /// 0..1: how recently they've been seen around the county (visits, trade).
    pub presence: f32,
}

pub fn founding() -> Vec<Nation> {
    vec![
        Nation {
            id: NationId::Delaware,
            border: (15, 0),
            population: 900,
            food_security: 0.7,
            timber: 400.0,
            land_pressure: 20.0,
            trust: [0.0, 5.0],
            settler_hostility: 10.0,
            annuity: 12_000.0,
            annuity_months: &[4, 10],
            docked: 0.0,
            farming: 0.8,
            mission_tilt: 0.0,
            complaints: 0,
            presence: 0.6,
        },
        Nation {
            id: NationId::Shawnee,
            border: (29, 15),
            population: 900,
            food_security: 0.7,
            timber: 150.0,
            land_pressure: 30.0,
            trust: [5.0, 0.0],
            settler_hostility: 10.0,
            annuity: 8_000.0,
            annuity_months: &[5, 11],
            docked: 0.0,
            farming: 0.8,
            mission_tilt: 0.4,
            complaints: 0,
            presence: 0.5,
        },
        Nation {
            id: NationId::Kaw,
            // Council Grove, some eighty miles up the Santa Fe road: off the map.
            border: (-160, 28),
            population: 1300,
            food_security: 0.45,
            timber: 200.0,
            land_pressure: 25.0,
            trust: [-10.0, -10.0],
            settler_hostility: 30.0,
            annuity: 3_500.0,
            annuity_months: &[6],
            docked: 0.0,
            farming: 0.25,
            mission_tilt: 0.0,
            complaints: 0,
            presence: 0.1,
        },
    ]
}

/// The nation nearest a farm: the one its settlers deal with and trespass on.
pub fn nearest(world: &World, farm: (i32, i32)) -> NationId {
    world
        .nations
        .iter()
        .min_by(|a, b| distance(a.border, farm).total_cmp(&distance(b.border, farm)))
        .map(|n| n.id)
        .unwrap_or(NationId::Delaware)
}

pub fn nation(world: &World, id: NationId) -> &Nation {
    &world.nations[id.index()]
}

fn nation_mut(world: &mut World, id: NationId) -> &mut Nation {
    &mut world.nations[id.index()]
}

/// A cash-poor family cuts its barn timber on treaty land: faster than
/// hauling from its own creek, and a trespass the nation will remember.
pub fn cut_reserve_timber(world: &mut World, family: u32) -> bool {
    let farm = world.families[family as usize].farm;
    let id = nearest(world, farm);
    let Some(head) = world.head_of(family) else {
        return false;
    };
    let faction = world.npc(head).faction;
    let n = nation_mut(world, id);
    if n.timber < 4.0 || distance(n.border, farm) > 16.0 {
        return false;
    }
    n.timber -= 4.0;
    n.land_pressure = (n.land_pressure + 3.0).min(100.0);
    n.trust[faction.index()] -= 6.0;
    world.families[family as usize].stores.goods[Good::Timber.index()] += 4.0;
    world.emit_root(EventKind::Trespass { family, nation: id }, None);
    true
}

/// Monthly: food, annuities, complaints, trade, and the war's spillover.
pub fn monthly(world: &mut World) {
    let month = world.day.month();
    let tension = (world.grievance[0] + world.grievance[1]) as f32;

    for i in 0..world.nations.len() {
        let id = world.nations[i].id;

        // Winter is lean; hunters' winters leaner.
        let n = &mut world.nations[i];
        let lean = match month {
            12 | 1 | 2 => 0.05 * (1.0 - n.farming),
            3 => 0.03,
            8..=10 => -0.08 * n.farming - 0.02,
            _ => -0.01,
        };
        n.food_security = (n.food_security - lean).clamp(0.05, 1.0);

        // Treaty money arrives, minus whatever settlers claimed was stolen.
        if n.annuity_months.contains(&month) {
            let paid = (n.annuity / n.annuity_months.len() as f32 - n.docked).max(0.0);
            let docked = n.docked;
            n.docked = 0.0;
            n.food_security = (n.food_security + paid / n.annuity * 0.4).min(1.0);
            // Traders and whiskey sellers descend on payment day.
            world.market.goods[Good::Whiskey.index()].stock -=
                10.0_f32.min(world.market.stock(Good::Whiskey));
            world.market.goods[Good::Powder.index()].stock -=
                5.0_f32.min(world.market.stock(Good::Powder));
            world.emit_root(
                EventKind::Annuity {
                    nation: id,
                    paid: paid as u32,
                    docked: docked as u32,
                },
                None,
            );
        }

        // Farming nations sell corn to settlers they trust.
        let n = &world.nations[i];
        let willing = (n.trust[0] + n.trust[1]) / 2.0 > -30.0;
        if willing && n.farming > 0.5 && n.food_security > 0.5 {
            let corn = 40.0 * n.farming * n.food_security;
            world.market.goods[Good::Corn.index()].stock += corn;
        }
        // Hunting nations bring hides.
        if willing && n.farming < 0.5 {
            world.market.goods[Good::Hides.index()].stock += 15.0 * world.market.trail_safety;
        }

        // Pressure becomes a complaint to the agent. Washington rarely acts.
        let n = &world.nations[i];
        if n.land_pressure >= 40.0 && world.rng.chance(n.land_pressure / 150.0) {
            let acted = world.rng.chance(0.15);
            let n = &mut world.nations[i];
            n.complaints += 1;
            if acted {
                n.land_pressure = (n.land_pressure - 15.0).max(0.0);
            }
            world.emit_root(EventKind::Complaint { nation: id, acted }, None);
        }

        // When the county is at war, riders cross reserve land and take what
        // they want. Both sides.
        if tension >= 80.0 && world.rng.chance((tension - 60.0) / 200.0) {
            let n = &mut world.nations[i];
            n.food_security = (n.food_security - 0.1).max(0.05);
            n.trust[0] -= 5.0;
            n.trust[1] -= 5.0;
            world.emit_root(EventKind::Plundered { nation: id }, None);
        }

        // Timber regrows on the scale of decades (§7.3): barely at all.
        let n = &mut world.nations[i];
        n.timber += 0.2;
        n.settler_hostility = (n.settler_hostility * 0.97).max(5.0);
        if n.id == NationId::Kaw {
            n.presence *= 0.7;
        }
    }
    visits(world);
    adoptions(world);
}

/// Hungry Kaw families come to settler doors. What happens there is remembered
/// on both sides.
fn visits(world: &mut World) {
    let kaw = nation(world, NationId::Kaw);
    if kaw.food_security > 0.4 {
        return;
    }
    let hosts: Vec<NpcId> = world
        .families
        .iter()
        .filter(|f| f.farms())
        .filter_map(|f| world.head_of(f.id))
        .collect();
    let Some(&host) = world.rng.pick(&hosts) else {
        return;
    };
    let n = world.npc(host);
    let (faction, family, generosity) = (n.faction, n.family, n.temperament.generosity);
    let fed = world.rng.chance(0.2 + 0.6 * generosity);
    if fed {
        let hh = &mut world.families[family as usize].stores;
        hh.food = (hh.food - 20.0).max(0.0);
    }
    let k = nation_mut(world, NationId::Kaw);
    k.presence = (k.presence + 0.3).min(1.0);
    if fed {
        k.trust[faction.index()] += 4.0;
        k.food_security = (k.food_security + 0.02).min(1.0);
    } else {
        k.trust[faction.index()] -= 3.0;
        k.settler_hostility += 3.0;
    }
    world.emit_root(
        EventKind::Visit {
            nation: NationId::Kaw,
            host,
            fed,
        },
        None,
    );
}

/// A settler's blame for lost stock lands on a nation. The claim goes to the
/// agent, who docks it from the next annuity. Hunger follows, then more visits,
/// then more blame.
pub fn depredation_claim(world: &mut World, id: NationId, accuser: NpcId, dollars: f32) {
    let faction = world.npc(accuser).faction;
    let n = nation_mut(world, id);
    n.docked += dollars;
    n.settler_hostility = (n.settler_hostility + 6.0).min(100.0);
    n.trust[faction.index()] -= 4.0;
}

/// How strongly an observer would suspect a nation of a livestock loss.
/// Pure prejudice, sharpened when the nation is hungry and visible.
pub fn suspicion(world: &World, observer: NpcId, id: NationId, farm: (i32, i32)) -> f32 {
    let n = nation(world, id);
    let skeptic = world.npc(observer).temperament.skepticism;
    let near = (1.0 - distance(n.border, farm) / 30.0).max(n.presence);
    let hunger = (0.6 - n.food_security).max(0.0) * 40.0;
    (n.settler_hostility * 0.9 + hunger * 1.5) * near * (1.0 - 0.6 * skeptic) - 5.0
}

/// Orphans and the destitute can be taken in by council, if the nation trusts
/// their people and the settler community has failed them. Kinship by
/// adoption and marriage was a documented practice (Shawnee, Delaware,
/// Wyandot); here it runs through council acceptance, not captivity.
fn adoptions(world: &mut World) {
    let candidates: Vec<(NpcId, Faction, (i32, i32))> = world
        .living()
        .filter(|n| n.id != super::world::PLAYER)
        .filter(|n| {
            let kin_adults = world
                .living()
                .filter(|k| k.family == n.family && LifeStage::of(k.age) != LifeStage::Child)
                .count();
            let orphan = LifeStage::of(n.age) == LifeStage::Child && kin_adults == 0;
            let destitute = world.families[n.family as usize].stores.hungry_days > 20;
            orphan || destitute
        })
        .map(|n| (n.id, n.faction, world.farm_of(n.id)))
        .collect();
    for (id, faction, farm) in candidates {
        let nid = nearest(world, farm);
        let n = nation(world, nid);
        if n.trust[faction.index()] > 10.0 && world.rng.chance(0.3) {
            let p = world.npc_mut(id);
            p.adopted_by = Some(nid);
            p.plotting = None;
            world.emit_root(
                EventKind::Adopted {
                    person: id,
                    nation: nid,
                },
                None,
            );
        }
    }
}

/// The blamed nation, if a belief is about one.
pub fn blamed_nation(s: Suspect) -> Option<NationId> {
    match s {
        Suspect::Nation(n) => Some(n),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timber_cutting_raises_pressure_and_costs_trust() {
        let mut w = World::new(3);
        let family = 1;
        let id = nearest(&w, w.families[family as usize].farm);
        w.nations[id.index()].border = w.families[family as usize].farm;
        let before = nation(&w, id).clone();
        assert!(cut_reserve_timber(&mut w, family));
        let after = nation(&w, id);
        assert!(after.land_pressure > before.land_pressure);
        assert!(after.timber < before.timber);
        let f = w.families[family as usize].faction.index();
        assert!(after.trust[f] < before.trust[f]);
    }

    #[test]
    fn depredation_claims_come_out_of_the_annuity() {
        let mut w = World::new(3);
        depredation_claim(&mut w, NationId::Kaw, 1, 30.0);
        assert_eq!(nation(&w, NationId::Kaw).docked, 30.0);
        // Run to the June payment: the claim is deducted and cleared.
        w.run_days(250);
        assert_eq!(nation(&w, NationId::Kaw).docked, 0.0);
        let docked = w.events.iter().any(|e| {
            matches!(e.kind, EventKind::Annuity { nation: NationId::Kaw, docked, .. } if docked >= 30)
        });
        assert!(docked);
    }

    #[test]
    fn a_hungry_nation_draws_more_suspicion() {
        let mut w = World::new(3);
        let farm = nation(&w, NationId::Kaw).border;
        let fed = suspicion(&w, 1, NationId::Kaw, farm);
        w.nations[NationId::Kaw.index()].food_security = 0.1;
        assert!(suspicion(&w, 1, NationId::Kaw, farm) > fed);
    }
}
