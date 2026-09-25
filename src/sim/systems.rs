//! Systems. Each one reads events it cares about and emits new ones.
//! None of them call each other (§15.2). Adding a consequence = adding a system.

use super::attribution::{self, Rumor};
use super::events::{EventKind, FireCause, Retaliation, Source, Suspect, WorldEvent};
use super::world::{Faction, MemoryRef, NpcId, PLAYER, World, distance};

/// Grievance at which a faction starts sanctioning revenge.
const AUTHORIZE_AT: i32 = 60;
/// Opinion at or below which someone starts planning revenge, and at which
/// mutual hatred between two families becomes a feud.
const HATRED: i16 = -50;
/// Days after acting on a grudge before someone will plot again.
const REVENGE_COOLDOWN: u32 = 60;
/// Below this confidence a belief is a shrug, not an accusation.
pub const ACCUSATION_CONFIDENCE: u8 = 20;

pub fn dispatch(world: &mut World, ev: &WorldEvent) {
    fire_system(world, ev);
    death_system(world, ev);
    perception_system(world, ev);
    grief_system(world, ev);
    gossip_system(world, ev);
    belief_system(world, ev);
    faction_system(world, ev);
    opinion_system(world, ev);
    retaliation_system(world, ev);
}

fn is_stakeholder(world: &World, holder: NpcId, victim: NpcId) -> bool {
    world.npc(holder).family == world.npc(victim).family
}

/// §12: burns the barn, and maybe the neighbors' too.
fn fire_system(world: &mut World, ev: &WorldEvent) {
    let EventKind::Fire { owner, cause, .. } = ev.kind else {
        return;
    };
    let family = world.npc(owner).family;
    let day = ev.day;
    let f = &mut world.families[family as usize];
    f.barn_standing = false;
    f.barn_burned_on = Some(day);
    for n in world
        .npcs
        .iter_mut()
        .filter(|n| n.family == family && n.alive)
    {
        n.mood -= 15;
    }

    // Fire doesn't stop at property lines. Dry + windy is apocalyptic.
    let weather = world.weather_on(ev.day);
    let spread = weather.wind * (0.05 + 0.45 * world.dryness);
    let origin = world.families[family as usize].farm;
    let neighbors: Vec<_> = world
        .families
        .iter()
        .filter(|f| f.id != family && f.barn_standing)
        .map(|f| (f.id, distance(f.farm, origin)))
        .filter(|(_, d)| *d <= 7.0)
        .collect();
    for (neighbor, d) in neighbors {
        if world.rng.chance(spread * (1.0 - d / 8.0))
            && let Some(head) = world.head_of(neighbor)
        {
            world.emit_child(
                ev,
                EventKind::Fire {
                    owner: head,
                    cause,
                    spread_from: Some(owner),
                },
            );
        }
    }
}

fn death_system(world: &mut World, ev: &WorldEvent) {
    let EventKind::Death { victim, .. } = ev.kind else {
        return;
    };
    world.npc_mut(victim).alive = false;
    world.npc_mut(victim).plotting = None;
    let family = world.npc(victim).family;
    let kin: Vec<NpcId> = world
        .living()
        .filter(|n| n.family == family)
        .map(|n| n.id)
        .collect();
    for mourner in kin {
        world.emit_child(
            ev,
            EventKind::Grief {
                mourner,
                deceased: victim,
            },
        );
    }
}

fn grief_system(world: &mut World, ev: &WorldEvent) {
    if let EventKind::Grief { mourner, .. } = ev.kind {
        world.npc_mut(mourner).mood -= 35;
    }
}

/// Who learns about a harm right away, and what they conclude (§11).
fn perception_system(world: &mut World, ev: &WorldEvent) {
    let (victim, actor, is_death) = match ev.kind {
        EventKind::Fire { owner, cause, .. } => (
            owner,
            match cause {
                FireCause::Arson(p) => Some(p),
                _ => None,
            },
            false,
        ),
        EventKind::Death { victim, killer } => (victim, killer, true),
        _ => return,
    };
    let site = world.farm_of(victim);
    let victim_family = world.npc(victim).family;

    let mut observers: Vec<(NpcId, bool)> = Vec::new();
    for n in world.living() {
        if n.id == PLAYER || Some(n.id) == actor {
            continue;
        }
        if n.family == victim_family {
            observers.push((n.id, true));
        } else {
            let d = distance(world.farm_of(n.id), site);
            if d <= 8.0 {
                observers.push((n.id, false));
            }
        }
    }

    for (observer, stakeholder) in observers {
        if !stakeholder {
            let d = distance(world.farm_of(observer), site);
            if !world.rng.chance(0.45 * (1.0 - d / 9.0)) {
                continue;
            }
        }
        let sight = match (stakeholder, is_death) {
            (false, true) => 0.6,
            (false, false) => 0.25,
            (true, true) => 0.25,
            (true, false) => 0.12,
        };
        let saw = actor.filter(|_| world.rng.chance(sight));
        let belief = match saw {
            Some(culprit) => EventKind::Belief {
                holder: observer,
                about: ev.id,
                blamed: Suspect::Person(culprit),
                confidence: 90 + world.rng.range(0, 10) as u8,
                source: Source::Witnessed,
                reason: "saw it with their own eyes",
            },
            None => {
                let v = attribution::judge(world, observer, ev.id, None);
                EventKind::Belief {
                    holder: observer,
                    about: ev.id,
                    blamed: v.blamed,
                    confidence: v.confidence,
                    source: if stakeholder {
                        Source::Victim
                    } else {
                        Source::Bystander
                    },
                    reason: v.reason,
                }
            }
        };
        world.emit_child(ev, belief);
    }
}

/// Rumors mutate on retelling: the listener runs their own attribution,
/// nudged toward what they were told, and may land somewhere else entirely.
fn gossip_system(world: &mut World, ev: &WorldEvent) {
    let EventKind::Gossip {
        teller,
        listener,
        about,
        blamed,
    } = ev.kind
    else {
        return;
    };
    if !world.npc(listener).alive || world.npc(listener).memory_of(about).is_some() {
        return;
    }
    let trust = (world.opinion(listener, teller) as f32 + 100.0) / 200.0;
    let rumor = Rumor {
        suspect: blamed,
        strength: 45.0 * trust,
    };
    let v = attribution::judge(world, listener, about, Some(rumor));
    world.emit_child(
        ev,
        EventKind::Belief {
            holder: listener,
            about,
            blamed: v.blamed,
            confidence: v.confidence,
            source: Source::Told(teller),
            reason: v.reason,
        },
    );
}

/// Beliefs become memories, and memories become opinions.
fn belief_system(world: &mut World, ev: &WorldEvent) {
    let EventKind::Belief {
        holder,
        about,
        blamed,
        confidence,
        source,
        ..
    } = ev.kind
    else {
        return;
    };
    let Some(victim) = attribution::victim_of(world, about) else {
        return;
    };
    let is_death = matches!(world.events[about as usize].kind, EventKind::Death { .. });
    let stake = if is_stakeholder(world, holder, victim) {
        1.0
    } else {
        0.35
    };
    let weight = if is_death && stake >= 1.0 {
        255
    } else {
        (stake * if is_death { 180.0 } else { 120.0 }) as u8
    };
    let day = ev.day;
    world.npc_mut(holder).remember(MemoryRef {
        event: about,
        believed: blamed,
        confidence,
        source,
        day,
        weight,
    });

    if let Suspect::Person(target) = blamed
        && target != holder
        && confidence >= ACCUSATION_CONFIDENCE
    {
        let severity = if is_death { 90.0 } else { 45.0 };
        let delta = -(severity * stake * confidence as f32 / 100.0) as i16;
        if delta <= -3 {
            world.emit_child(
                ev,
                EventKind::OpinionChange {
                    holder,
                    target,
                    delta,
                    after: 0,
                },
            );
        }
    }
}

/// Factions keep score. Past a threshold, revenge is sanctioned.
fn faction_system(world: &mut World, ev: &WorldEvent) {
    let EventKind::Belief {
        holder,
        about,
        blamed: Suspect::Person(blamed),
        ..
    } = ev.kind
    else {
        return;
    };
    let Some(victim) = attribution::victim_of(world, about) else {
        return;
    };
    let faction = world.npc(holder).faction;
    if !is_stakeholder(world, holder, victim) || world.npc(blamed).faction == faction {
        return;
    }
    let bump = match world.events[about as usize].kind {
        EventKind::Death { .. } => 25,
        _ => 12,
    };
    let before = world.grievance[faction.index()];
    let after = before + bump;
    world.grievance[faction.index()] = after;
    let crossed = [30, AUTHORIZE_AT, 100]
        .iter()
        .any(|t| before < *t && after >= *t);
    if crossed {
        world.emit_child(
            ev,
            EventKind::FactionGrievance {
                faction,
                level: after,
                authorized: after >= AUTHORIZE_AT,
            },
        );
    }
}

pub fn faction_authorized(world: &World, faction: Faction) -> bool {
    world.grievance[faction.index()] >= AUTHORIZE_AT
}

/// Opinions move; hatred past a line turns into plans and feuds.
fn opinion_system(world: &mut World, ev: &WorldEvent) {
    let EventKind::OpinionChange {
        holder,
        target,
        delta,
        ..
    } = ev.kind
    else {
        return;
    };
    let now = world.adjust_opinion(holder, target, delta);
    if let EventKind::OpinionChange { after, .. } = &mut world.events[ev.id as usize].kind {
        *after = now;
    }
    if now > HATRED {
        return;
    }
    let (hf, tf) = (world.npc(holder).family, world.npc(target).family);
    if hf == tf {
        return;
    }

    if world.opinion(target, holder) <= HATRED && !world.feud_between(hf, tf) {
        world.feuds.insert((hf.min(tf), hf.max(tf)));
        world.emit_child(ev, EventKind::FeudDeclared { a: hf, b: tf });
    }

    let h = world.npc(holder);
    let cooling = h
        .last_revenge
        .is_some_and(|d| ev.day.0 < d.0 + REVENGE_COOLDOWN);
    if holder == PLAYER || !h.alive || h.plotting.is_some() || cooling || !world.npc(target).alive {
        return;
    }
    let mut p = 0.2 + 0.08 * h.violence.min(4) as f32;
    if faction_authorized(world, h.faction) {
        p += 0.2;
    }
    if h.mood < 30 {
        p += 0.1;
    }
    if !world.rng.chance(p.min(0.7)) {
        return;
    }
    let grieving = world
        .npc(holder)
        .memories
        .iter()
        .any(|m| m.weight == 255 && m.believed == Suspect::Person(target));
    let method = if grieving || (world.feud_between(hf, tf) && world.rng.chance(0.3)) {
        Retaliation::Ambush
    } else {
        Retaliation::Arson
    };
    world.npc_mut(holder).plotting = Some(target);
    let delay = world.rng.range(3, 25);
    world.schedule(
        delay,
        EventKind::Retaliation {
            actor: holder,
            target,
            method,
        },
        ev.id,
    );
}

/// A plot comes due. Decide whether it still happens, and how, before it is
/// announced: people give up, or find nothing left to burn and reach for a rifle.
pub fn resolve_plot(
    world: &mut World,
    actor: NpcId,
    target: NpcId,
    method: Retaliation,
) -> Option<Retaliation> {
    world.npc_mut(actor).plotting = None;
    if !world.npc(actor).alive || !world.npc(target).alive {
        return None;
    }
    let nothing_to_burn = !world.families[world.npc(target).family as usize].barn_standing;
    match method {
        Retaliation::Arson if nothing_to_burn => {
            world.rng.chance(0.25).then_some(Retaliation::Ambush)
        }
        m => Some(m),
    }
}

fn retaliation_system(world: &mut World, ev: &WorldEvent) {
    let EventKind::Retaliation {
        actor,
        target,
        method,
    } = ev.kind
    else {
        return;
    };
    // Moral momentum: the second time is easier.
    let day = ev.day;
    let a = world.npc_mut(actor);
    a.violence = a.violence.saturating_add(1);
    a.alibi = None;
    a.last_revenge = Some(day);
    let kind = match method {
        Retaliation::Arson => EventKind::Fire {
            owner: target,
            cause: FireCause::Arson(actor),
            spread_from: None,
        },
        Retaliation::Ambush => EventKind::Death {
            victim: target,
            killer: Some(actor),
        },
    };
    world.emit_child(ev, kind);
}

/// Daily: people with a fresh accusation tell someone (§14.2 gossip clock).
pub fn spread_gossip(world: &mut World) {
    let today = world.day.0;
    let mut tellings = Vec::new();
    for n in world.living() {
        if n.id == PLAYER {
            continue;
        }
        let fresh = n
            .memories
            .iter()
            .filter(|m| today.saturating_sub(m.day.0) <= 20)
            .filter(|m| matches!(m.believed, Suspect::Person(_)))
            .filter(|m| m.confidence >= ACCUSATION_CONFIDENCE)
            .max_by_key(|m| (m.weight, m.day));
        if let Some(m) = fresh {
            tellings.push((n.id, m.event, m.believed));
        }
    }
    let mut told_today = std::collections::HashSet::new();
    for (teller, about, blamed) in tellings {
        if !world.rng.chance(0.35) {
            continue;
        }
        let here = world.farm_of(teller);
        let faction = world.npc(teller).faction;
        let audience: Vec<NpcId> = world
            .living()
            .filter(|n| n.id != teller && n.id != PLAYER && n.memory_of(about).is_none())
            .filter(|n| !told_today.contains(&(n.id, about)))
            .filter(|n| n.faction == faction || distance(world.farm_of(n.id), here) <= 12.0)
            .map(|n| n.id)
            .collect();
        let Some(&listener) = world.rng.pick(&audience) else {
            continue;
        };
        told_today.insert((listener, about));
        world.emit_root(
            EventKind::Gossip {
                teller,
                listener,
                about,
                blamed,
            },
            Some(about),
        );
    }
}
