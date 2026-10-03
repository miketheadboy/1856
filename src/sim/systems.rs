//! Systems. Each one reads events it cares about and emits new ones.
//! None of them call each other (§15.2). Adding a consequence = adding a system.

use super::attribution::{self, Rumor};
use super::character::{self, Archetype};
use super::economy;
use super::events::Cruelty;
use super::events::{EventId, EventKind, FireCause, Retaliation, Source, Suspect, WorldEvent};
use super::psyche::{self, LifeStage};
use super::world::{Faction, MemoryRef, NpcId, PLAYER, World, distance};

/// Grievance at which a faction starts sanctioning revenge.
const AUTHORIZE_AT: i32 = 60;
/// Opinion at or below which someone starts planning revenge, and at which
/// mutual hatred between two families becomes a feud.
const HATRED: i16 = -50;
/// Days after acting on a grudge before someone will plot again.
const REVENGE_COOLDOWN: u32 = 120;
/// Below this confidence a belief is a shrug, not an accusation.
pub const ACCUSATION_CONFIDENCE: u8 = 20;

pub fn dispatch(world: &mut World, ev: &WorldEvent) {
    fire_system(world, ev);
    death_system(world, ev);
    wound_death_system(world, ev);
    perception_system(world, ev);
    grief_system(world, ev);
    gossip_system(world, ev);
    belief_system(world, ev);
    faction_system(world, ev);
    opinion_system(world, ev);
    retaliation_system(world, ev);
    favor_system(world, ev);
    cruelty_system(world, ev);
    wound_system(world, ev);
    press_system(world, ev);
    super::ghosts::on_event(world, ev);
    super::reconcile::on_event(world, ev);
    super::family::on_event(world, ev);
    super::romance::on_event(world, ev);
    super::civic::on_event(world, ev);
    super::life::on_event(world, ev);
    super::mail::on_event(world, ev);
    super::law::on_event(world, ev);
    super::bees::on_event(world, ev);
    super::legacy::on_event(world, ev);
    super::land::on_event(world, ev);
    super::railroad::on_event(world, ev);
    super::market::on_event(world, ev);
    super::action::on_event(world, ev);
    super::arms::on_event(world, ev);
    super::hands::on_event(world, ev);
    super::warrant::on_event(world, ev);
    super::wardrobe::on_event(world, ev);
    super::sickness::on_event(world, ev);
    super::freight::on_event(world, ev);
}

/// A paper's version of a local event reaches its readers.
fn press_system(world: &mut World, ev: &WorldEvent) {
    if let EventKind::Headline {
        paper,
        about: Some(about),
        blamed: Some(blamed),
        ..
    } = ev.kind
    {
        super::history::read(world, ev.id, paper, about, blamed);
    }
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
    let farm = world.families[family as usize].farm;
    world.map.scorch(farm, day.0);
    let f = &mut world.families[family as usize];
    f.barn_standing = false;
    f.barn_burned_on = Some(day);
    for n in world
        .npcs
        .iter_mut()
        .filter(|n| n.family == family && n.alive)
    {
        n.emotions.fear += 15.0;
        n.emotions.anger += 10.0;
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
        let fuel = world.map.at(world.families[neighbor as usize].farm).fuel();
        if world.rng.chance(spread * fuel * (1.0 - d / 8.0))
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
    let (EventKind::Death { victim, .. } | EventKind::Perished { victim, .. }) = ev.kind else {
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
        psyche::feel(world, mourner, |e| e.grief += 60.0);
    }
}

/// Who learns about a harm right away, and what they conclude (§11).
/// The common law's year and a day: a man who dies of a wound within a year
/// and a day of taking it was killed by whoever gave it to him. A fever or a
/// wound gone bad takes a wounded man as a killing, linked to the shot.
/// Returns the event to emit and what caused it.
pub fn perished(
    world: &World,
    victim: NpcId,
    cause: super::events::Hardship,
) -> (EventKind, Option<EventId>) {
    use super::events::Hardship;
    use super::sickness::Disease;
    let wound_death = match cause {
        Hardship::Sickness(Disease::WoundFever) => true,
        Hardship::Fever => world.npc(victim).wounded,
        _ => false,
    };
    let dead = EventKind::Perished { victim, cause };
    if !wound_death {
        return (dead, None);
    }
    let since = world.day.0.saturating_sub(366);
    let shot = world
        .events
        .iter()
        .rev()
        .take_while(|e| e.day.0 >= since)
        .find_map(|e| match e.kind {
            EventKind::Wounded {
                victim: v,
                attacker,
            } if v == victim && attacker != victim => Some((e.id, attacker)),
            _ => None,
        });
    match shot {
        Some((wound, attacker)) => (
            EventKind::Death {
                victim,
                killer: Some(attacker),
            },
            Some(wound),
        ),
        None => (dead, None),
    }
}

/// The wound that killed him, if this death came late from a shooting: a
/// death caused by a wound to the same man, from the same hand. (A killing
/// *for* a wound, revenge on the shooter, has a different victim.)
pub fn fatal_wound(world: &World, ev: &WorldEvent) -> Option<EventId> {
    let EventKind::Death { victim, killer } = ev.kind else {
        return None;
    };
    let w = ev.caused_by?;
    match world.events[w as usize].kind {
        EventKind::Wounded {
            victim: v,
            attacker,
        } if v == victim && killer == Some(attacker) => Some(w),
        _ => None,
    }
}

/// He died in his bed, weeks after the shot. Nobody witnesses that: what
/// each person believed about the shooting becomes what they believe about
/// the killing. The shooter's own side, as often as not, says it was the
/// fever that took him and not the ball, and the county has two stories.
fn wound_death_system(world: &mut World, ev: &WorldEvent) {
    let Some(wound) = fatal_wound(world, ev) else {
        return;
    };
    let held: Vec<(NpcId, MemoryRef)> = world
        .living()
        .filter_map(|n| n.memory_of(wound).map(|m| (n.id, m.clone())))
        .collect();
    for (holder, m) in held {
        let own_side = matches!(m.believed, Suspect::Person(p)
            if world.npc(p).faction == world.npc(holder).faction);
        let (blamed, reason) = if own_side && world.rng.chance(0.6) {
            (Suspect::Nature, "a fever took him, not the ball")
        } else {
            (m.believed, "died of the wound")
        };
        world.emit_child(
            ev,
            EventKind::Belief {
                holder,
                about: ev.id,
                blamed,
                confidence: m.confidence,
                source: m.source,
                reason,
            },
        );
    }
}

fn perception_system(world: &mut World, ev: &WorldEvent) {
    // A death in bed from an old wound has no witnesses of its own.
    if fatal_wound(world, ev).is_some() {
        return;
    }
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
        EventKind::Theft { thief, victim, .. } => (victim, Some(thief), false),
        EventKind::Strayed { owner } => (owner, None, false),
        EventKind::Captured {
            at: Some(f),
            informer,
            ..
        } => match world.head_of(f) {
            Some(h) => (h, informer, false),
            None => return,
        },
        EventKind::Cruelty {
            actor,
            victim,
            act: Cruelty::KillStock | Cruelty::FoulWell | Cruelty::CutFence | Cruelty::SpoilHay,
        } => (victim, Some(actor), false),
        // You mostly see who shot you.
        EventKind::Wounded { victim, attacker } => (victim, Some(attacker), true),
        EventKind::ShotAt { shooter, target } => (target, Some(shooter), true),
        EventKind::Prowler {
            prowler, victim, ..
        } => (victim, Some(prowler), false),
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

    // Played out by hand: the minigame already knows who saw.
    let staged = world.action.staged_for(actor).map(|e| e.to_vec());
    for (observer, stakeholder) in observers {
        let eyewitness = staged.as_ref().map(|e| e.contains(&observer));
        if !stakeholder && eyewitness != Some(true) {
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
        // Stealth hides you; alertness catches you. Bushwhackers are ghosts.
        let hidden = actor.map_or(1.0, |a| {
            let n = world.npc(a);
            let ghost = if character::is(n, Archetype::Bushwhacker) {
                0.4
            } else {
                1.0
            };
            let clothes = n.outfit.bonus().stealth;
            (1.0 - 0.6 * (n.body.stealth + clothes).clamp(0.0, 1.0)) * ghost
        });
        let keen = 0.6 + 0.8 * world.npc(observer).body.alertness;
        // Timber hides a rider; open prairie shows him for miles (§7.4).
        // A full moon shows him too; a dark or clouded one hides him.
        let terrain = world.map.at(site).visibility() * (0.5 + 0.9 * world.night_light(ev.day));
        let saw = match eyewitness {
            Some(seen) => actor.filter(|_| seen),
            None => actor.filter(|_| world.rng.chance(sight * hidden * keen * terrain)),
        };
        // Seeing isn't knowing. By a thin moon, across a field, with a gun
        // going off, a witness matches the shape to a man they half expected.
        // Staged scenes already decided who saw the player's face.
        let saw = match (saw, eyewitness) {
            (Some(culprit), None) => Some(identify(
                world,
                observer,
                culprit,
                ev,
                stakeholder,
                is_death,
            )),
            (s, _) => s,
        };
        if !stakeholder {
            psyche::feel(world, observer, |e| e.fear += 10.0);
        }
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
                // No face, but maybe a red shirt in the lantern light: the
                // colors point at a side, and the observer picks the man of
                // that side they'd have suspected anyway (`wardrobe`).
                let colors = actor.and_then(|a| super::wardrobe::colors_of(world, a));
                let rumor = match colors {
                    Some(side) if world.rng.chance((1.5 * sight * keen * terrain).min(0.8)) => {
                        attribution::candidates(world, observer, ev.id, None)
                            .into_iter()
                            .filter(|c| {
                                matches!(c.suspect, Suspect::Person(p) if world.npc(p).faction == side)
                            })
                            .max_by(|a, b| a.score.total_cmp(&b.score))
                            .map(|c| attribution::Rumor {
                                suspect: c.suspect,
                                strength: 1.5,
                            })
                    }
                    _ => None,
                };
                let v = attribution::judge(world, observer, ev.id, rumor);
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

/// The odds a witness puts the right name to the shape they saw (Wells &
/// Loftus: light, distance, stress, a weapon, and whether the face was a
/// stranger's). Neighbors known by sight under a full moon are nearly always
/// right; a stranger on the other side, by starlight, is a coin toss.
pub fn identify_odds(world: &World, observer: NpcId, culprit: NpcId, ev: &WorldEvent) -> f32 {
    let o = world.npc(observer);
    let light = 0.7 + 0.3 * world.night_light(ev.day);
    let d = distance(world.farm_of(observer), world.farm_of(culprit));
    // A county of a few dozen souls: everyone is known by sight from the
    // store and the land office. Close neighbors best; the other side's men,
    // who don't come to your meetings, a little less.
    let known = if o.family == world.npc(culprit).family || d <= 6.0 {
        1.0
    } else if o.faction == world.npc(culprit).faction {
        0.93
    } else {
        0.87
    };
    let fright = 1.0 - 0.3 * (o.emotions.fear / 100.0);
    let keen = 0.8 + 0.2 * o.body.alertness;
    (light * known * fright * keen).clamp(0.15, 0.97)
}

/// Put a name to it. When the face is wrong it's rarely random: the witness
/// names the man of the same side they'd have suspected anyway, and is just
/// as sure (confidence doesn't track accuracy).
fn identify(
    world: &mut World,
    observer: NpcId,
    culprit: NpcId,
    ev: &WorldEvent,
    stakeholder: bool,
    violent: bool,
) -> NpcId {
    let mut p = identify_odds(world, observer, culprit, ev);
    if !stakeholder {
        // Across the section line, not in the yard.
        let d = distance(world.farm_of(observer), world.farm_of(culprit));
        p *= (1.0 - 0.02 * d).clamp(0.7, 1.0);
    }
    if violent {
        // Weapon focus: you watch the muzzle, not the face.
        p *= 0.9;
    }
    if world.rng.chance(p) {
        return culprit;
    }
    let side = world.npc(culprit).faction;
    attribution::candidates(world, observer, ev.id, None)
        .into_iter()
        .filter_map(|c| match c.suspect {
            Suspect::Person(x) if x != culprit && x != observer && world.npc(x).faction == side => {
                Some((x, c.score))
            }
            _ => None,
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
        .map_or(culprit, |c| c.0)
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
    if !world.npc(listener).alive {
        return;
    }
    // Someone who already has a view only changes it for a rumor that beats it.
    let prior = world
        .npc(listener)
        .memory_of(about)
        .map(|m| (m.believed, m.confidence));
    let trust = (world.opinion(listener, teller) as f32 + 100.0) / 200.0;
    let doubt = 1.0 - 0.5 * world.npc(listener).temperament.skepticism;
    let rumor = Rumor {
        suspect: blamed,
        // Testimonial injustice (Fricker): the same words weigh less from a
        // woman, a beggar, a newcomer or a jailbird (`standing`).
        strength: 45.0
            * trust
            * doubt
            * character::rumor_weight(world, teller)
            * super::standing::word(world, teller),
    };
    let v = attribution::judge(world, listener, about, Some(rumor));
    if let Some((believed, confidence)) = prior
        && (v.blamed == believed || v.confidence <= confidence)
    {
        return;
    }
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
    let about_kind = &world.events[about as usize].kind;
    let is_death = matches!(about_kind, EventKind::Death { .. });
    let is_theft = matches!(about_kind, EventKind::Theft { .. });
    let is_wound = matches!(about_kind, EventKind::Wounded { .. });
    let is_cruelty = matches!(about_kind, EventKind::Cruelty { .. });
    let is_fever = matches!(about_kind, EventKind::FellSick { .. });
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

    // Blaming a nation becomes a depredation claim against its annuity.
    if let Suspect::Nation(nid) = blamed
        && stake >= 1.0
        && confidence >= ACCUSATION_CONFIDENCE
        && matches!(source, Source::Victim | Source::Told(_))
    {
        let dollars = if is_theft { 16.0 } else { 12.0 };
        super::nations::depredation_claim(world, nid, holder, dollars);
    }

    if let Suspect::Person(target) = blamed
        && target != holder
        && confidence >= ACCUSATION_CONFIDENCE
    {
        let severity = if is_death {
            90.0
        } else if is_wound {
            70.0
        } else if is_cruelty {
            35.0
        } else if is_theft {
            30.0
        } else if is_fever {
            // A suspicion about a fever: it sours, it doesn't inflame.
            20.0
        } else {
            45.0
        };
        let delta = -(severity * stake * confidence as f32 / 100.0) as i16;
        let heat = severity * stake * confidence as f32 / 200.0;
        psyche::feel(world, holder, |e| e.anger += heat);
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
    let base = match world.events[about as usize].kind {
        EventKind::Death { .. } => 25.0,
        EventKind::Wounded { .. } => 15.0,
        EventKind::Cruelty { .. } => 8.0,
        EventKind::Theft { .. } => 6.0,
        _ => 12.0,
    };
    // The loyal take their side's wounds personally.
    let bump = (base * (0.5 + world.npc(holder).temperament.loyalty)) as i32;
    let before = world.grievance[faction.index()];
    world.add_grievance(faction, bump);
    let after = world.grievance[faction.index()];
    let crossed = [30, AUTHORIZE_AT, 100]
        .iter()
        .any(|t| before < *t && after >= *t);
    if crossed {
        let members: Vec<NpcId> = world
            .living()
            .filter(|n| n.faction == faction)
            .map(|n| n.id)
            .collect();
        for m in members {
            psyche::feel(world, m, |e| e.zeal += 12.0);
        }
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

    if world.opinion(target, holder) <= HATRED
        && !world.feud_between(hf, tf)
        && !world.truce_holds(hf, tf)
    {
        world.feuds.insert((hf.min(tf), hf.max(tf)));
        world.emit_child(ev, EventKind::FeudDeclared { a: hf, b: tf });
    }

    let h = world.npc(holder);
    let cooling = h
        .last_revenge
        .is_some_and(|d| ev.day.0 < d.0 + REVENGE_COOLDOWN);
    // Children don't ride at night, and the sick can't.
    let able = LifeStage::of(h.age) != LifeStage::Child && h.health >= 30;
    if holder == PLAYER
        || !h.alive
        || !able
        || h.plotting.is_some()
        || super::warrant::held(world, holder)
        || cooling
        || !world.npc(target).alive
    {
        return;
    }
    let mut p = 0.10 + 0.08 * h.violence.min(4) as f32;
    if faction_authorized(world, h.faction) {
        p += 0.2;
    }
    p += psyche::revenge_drive(world, holder);
    // A gunshot wound or a bad winter radicalizes by the economic path.
    if world.families[h.family as usize].stores.desperate() {
        p += 0.25;
    }
    let h = world.npc(holder);
    let zealot = character::is(h, Archetype::Zealot);
    let hothead = character::is(h, Archetype::Hothead);
    if zealot {
        p *= 1.5;
    }
    // Sworn on a grave.
    if world.ghosts.oath_against(holder, target) {
        p += 0.3;
    }
    // Knowing your own blood was in the wrong takes the fight out of you.
    if super::ghosts::ashamed(world, holder) {
        p *= 0.5;
    }
    // The humble can swallow a slight.
    p *= 1.0 - 0.4 * world.npc(holder).temperament.humility;
    // Faced down, or talked down, at this man's gate: not yet.
    if world.action.cowed(holder, target, ev.day) {
        p *= 0.15;
    }
    // Dragoons on the roads: people think twice.
    if world.pacified_until.is_some_and(|d| ev.day < d) {
        p *= 0.3;
    }
    if !world.rng.chance(p.clamp(0.02, 0.85)) {
        return;
    }
    let grieving = world
        .npc(holder)
        .memories
        .iter()
        .any(|m| m.weight == 255 && m.believed == Suspect::Person(target));
    let method = if (grieving && world.rng.chance(0.6))
        || (hothead && world.rng.chance(0.4))
        || (world.feud_between(hf, tf) && world.rng.chance(0.15))
    {
        Retaliation::Ambush
    } else {
        Retaliation::Arson
    };
    world.npc_mut(holder).plotting = Some(target);
    let mut delay = world.rng.range(3, 25);
    if hothead {
        delay /= 2;
    }
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
    caused_by: Option<super::events::EventId>,
) -> Option<Retaliation> {
    world.npc_mut(actor).plotting = None;
    if !world.npc(actor).alive || !world.npc(target).alive || super::warrant::held(world, actor) {
        return None;
    }
    // A man in the Lecompton jail, gone to the States or down the road
    // with a wagon isn't home to be shot; his barn still is. A man on the
    // road isn't here to ride on anybody.
    if method == Retaliation::Ambush
        && (super::warrant::held(world, target) || super::freight::away(world, target))
    {
        return None;
    }
    if super::freight::away(world, actor) {
        return None;
    }
    // Someone sat up with a rifle and called out at the fence.
    if super::hands::turn_back(world, actor, target, caused_by) {
        return None;
    }
    if let Some(why) = super::reconcile::mercy(world, actor, target, method) {
        world.emit_root(EventKind::Spared { actor, target, why }, caused_by);
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
        Retaliation::Ambush => {
            // Marksmanship and luck against the target's alertness.
            let (a, t) = (world.npc(actor), world.npc(target));
            let kill = if character::is(a, Archetype::Bushwhacker) {
                1.0
            } else {
                (0.2 + 0.5 * a.body.marksmanship - 0.3 * t.body.alertness) * a.hidden.luck
            };
            if world.rng.chance(kill.clamp(0.1, 1.0)) {
                EventKind::Death {
                    victim: target,
                    killer: Some(actor),
                }
            } else {
                EventKind::Wounded {
                    victim: target,
                    attacker: actor,
                }
            }
        }
    };
    world.emit_child(ev, kind);
}

fn wound_system(world: &mut World, ev: &WorldEvent) {
    let EventKind::Wounded { victim, .. } = ev.kind else {
        return;
    };
    // Where it went in: the chest is the biggest mark, the head the rarest.
    let roll = world.rng.unit();
    let limb = match roll {
        r if r < 0.08 => psyche::Limb::Head,
        r if r < 0.40 => psyche::Limb::Chest,
        r if r < 0.62 => psyche::Limb::GunArm,
        r if r < 0.75 => psyche::Limb::OffArm,
        _ => psyche::Limb::Leg,
    };
    let n = world.npc_mut(victim);
    n.wounded = true;
    n.hurt = Some(limb);
    let blow = if limb == psyche::Limb::Head { 60 } else { 45 };
    n.health = (n.health - blow).max(1);
    n.emotions.fear += 30.0;
    n.emotions.anger += 30.0;
}

/// Harm for its own sake. Slander becomes gossip with a lie in it.
fn cruelty_system(world: &mut World, ev: &WorldEvent) {
    let EventKind::Cruelty { actor, victim, act } = ev.kind else {
        return;
    };
    let family = world.npc(victim).family;
    match act {
        Cruelty::Slander { listener, about } => {
            world.emit_child(
                ev,
                EventKind::Gossip {
                    teller: actor,
                    listener,
                    about,
                    blamed: character::slander_target(victim),
                },
            );
        }
        Cruelty::KillStock => {
            let hh = &mut world.families[family as usize].stores;
            hh.cattle = hh.cattle.saturating_sub(1);
        }
        Cruelty::FoulWell => {
            let drinkers: Vec<NpcId> = world
                .living()
                .filter(|n| n.family == family)
                .map(|n| n.id)
                .collect();
            for d in drinkers {
                let n = world.npc_mut(d);
                n.health = (n.health - (25.0 * n.body.frailty()) as i32).max(1);
                n.emotions.fear += 20.0;
            }
        }
        Cruelty::CutFence => {
            world.families[family as usize].stores.work.fences = 0.0;
        }
        Cruelty::SpoilHay => {
            world.families[family as usize].stores.work.hay *= 0.3;
        }
        // The lice move in (`sickness`); the fear comes with the itch.
        Cruelty::FouledBlanket => {}
    }
}

/// The storekeeper's favor: sign the Law and Order petition, or be stripped.
/// Signing is forgiven by the ledger and not by your neighbors.
fn favor_system(world: &mut World, ev: &WorldEvent) {
    let EventKind::Favor {
        creditor,
        debtor,
        complied,
    } = ev.kind
    else {
        return;
    };
    economy::settle_favor(world, debtor, complied);
    let debtor_family = world.npc(debtor).family;
    if complied {
        // Signed against conscience: the mouth moves, the heart doesn't (ext. §16).
        let n = world.npc_mut(debtor);
        n.ideology.public = (n.ideology.public - 0.4).max(-1.0);
        let conscience = n.ideology.private.max(0.0);
        n.emotions.anger += 30.0 * conscience;
        let neighbors: Vec<NpcId> = world
            .living()
            .filter(|n| n.faction == Faction::FreeState && n.family != debtor_family)
            .filter(|n| n.id != PLAYER)
            .map(|n| n.id)
            .collect();
        for n in neighbors {
            if world.rng.chance(0.5) {
                world.emit_child(
                    ev,
                    EventKind::OpinionChange {
                        holder: n,
                        target: debtor,
                        delta: -15,
                        after: 0,
                    },
                );
            }
        }
    } else if debtor != PLAYER {
        psyche::feel(world, debtor, |e| e.zeal += 15.0);
        world.emit_child(
            ev,
            EventKind::OpinionChange {
                holder: debtor,
                target: creditor,
                delta: -45,
                after: 0,
            },
        );
    } else {
        world.adjust_opinion(creditor, debtor, -20);
    }
}

/// Daily: people with a fresh accusation tell someone (§14.2 gossip clock).
/// Allport & Postman's basic law of rumor: how much a thing is talked of
/// goes as its importance times its ambiguity. A killing nobody can pin on
/// anyone runs the county; a stray cow everyone agrees got through the fence
/// dies on the porch. Scaled so an ordinary grudge-fire is about 1.
pub fn talkability(world: &World, about: EventId) -> f32 {
    let importance = match world.events[about as usize].kind {
        EventKind::Death { .. } => 1.0,
        EventKind::Wounded { .. } | EventKind::ShotAt { .. } => 0.8,
        EventKind::Fire { .. } | EventKind::Captured { .. } => 0.7,
        EventKind::Cruelty { .. } | EventKind::Prowler { .. } => 0.55,
        EventKind::Theft { .. } => 0.45,
        _ => 0.25,
    };
    // Ambiguity: how split the county's minds are. One name on every tongue
    // is settled; five names is a story that keeps.
    let mut names: Vec<(Suspect, u32)> = Vec::new();
    for n in world.living() {
        if let Some(m) = n.memory_of(about) {
            match names.iter_mut().find(|x| x.0 == m.believed) {
                Some(x) => x.1 += 1,
                None => names.push((m.believed, 1)),
            }
        }
    }
    let total: u32 = names.iter().map(|x| x.1).sum();
    let top = names.iter().map(|x| x.1).max().unwrap_or(0);
    let ambiguity = if total == 0 {
        1.0
    } else {
        1.0 - top as f32 / total as f32
    };
    (importance * (0.5 + ambiguity) / 0.6).clamp(0.2, 2.5)
}

/// Frightened people talk (Rosnow): rumor is how a county handles dread.
/// 1.0 in a calm county, up to 1.6 when fear runs high.
pub fn county_fear(world: &World) -> f32 {
    let (sum, n) = world
        .living()
        .fold((0.0, 0.0), |(s, n), p| (s + p.emotions.fear, n + 1.0));
    let mean = if n > 0.0 { sum / n } else { 0.0 };
    1.0 + (mean / 50.0).min(0.6)
}

pub fn spread_gossip(world: &mut World) {
    let today = world.day.0;
    let mut tellings = Vec::new();
    for n in world.living() {
        if n.id == PLAYER || n.wounded {
            continue;
        }
        // Cowards keep what they saw to themselves.
        let coward = character::is(n, Archetype::Coward);
        let fresh = n
            .memories
            .iter()
            .filter(|m| today.saturating_sub(m.day.0) <= 20)
            .filter(|m| !(coward && m.source == Source::Witnessed))
            .filter(|m| matches!(m.believed, Suspect::Person(_)))
            .filter(|m| m.confidence >= ACCUSATION_CONFIDENCE)
            .max_by_key(|m| (m.weight, m.day));
        if let Some(m) = fresh {
            tellings.push((n.id, m.event, m.believed));
        }
    }
    let mut told_today = std::collections::HashSet::new();
    let dread = county_fear(world);
    for (teller, about, blamed) in tellings {
        let t = world.npc(teller);
        let mouth = 0.15 + 0.4 * t.temperament.sociability + t.emotions.zeal / 250.0;
        let talk = (mouth * talkability(world, about) * dread).min(0.95);
        if !world.rng.chance(talk) {
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
        let blamed = sharpen(world, teller, about, blamed).unwrap_or(blamed);
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

/// Bartlett's reconstructive memory, and Allport & Postman's sharpening:
/// each time a story is told it settles a little closer to what the teller
/// already thought of people. "Someone of the Holt crowd" becomes Cyrus
/// Holt, whom he never liked. The teller's own memory moves first (a Belief
/// of his own, so the books agree), then he passes on the new version. The
/// honest and the certain hold their stories; the zealous bend them.
fn sharpen(world: &mut World, teller: NpcId, about: EventId, blamed: Suspect) -> Option<Suspect> {
    let Suspect::Person(named) = blamed else {
        return None;
    };
    let t = world.npc(teller);
    let m = t.memory_of(about)?;
    if m.confidence >= 100 {
        return None;
    }
    let (confidence, source) = (m.confidence, m.source);
    let p = 0.12 * (1.0 - t.temperament.honesty) * (1.0 + t.emotions.zeal / 100.0);
    let family = t.family;
    if !world.rng.chance(p) {
        return None;
    }
    let side = world.npc(named).faction;
    let held = world.opinion(teller, named);
    let other = world
        .living()
        .filter(|n| n.id != named && n.id != teller && n.id != PLAYER && n.family != family)
        .filter(|n| n.faction == side && LifeStage::of(n.age) != LifeStage::Child)
        .map(|n| (n.id, world.opinion(teller, n.id)))
        .filter(|&(_, o)| o <= held - 20)
        .min_by_key(|&(id, o)| (o, id))?
        .0;
    world.emit_root(
        EventKind::Belief {
            holder: teller,
            about,
            blamed: Suspect::Person(other),
            confidence: confidence + 1,
            source,
            reason: "the story sharpened in the telling",
        },
        Some(about),
    );
    Some(Suspect::Person(other))
}
