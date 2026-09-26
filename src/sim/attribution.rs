//! The Attribution Engine (§11).
//!
//! NPCs never see causes, only effects. They score candidate causes, biased
//! mostly by who they already hate, then act on the answer with the same
//! force as if it were true.

use super::calendar::Season;
use super::events::Cruelty;
use super::events::{EventId, EventKind, Suspect};
use super::psyche::LifeStage;
use super::world::{NpcId, World, distance};

/// Softmax temperature. Lower = people are more certain of their prejudices.
/// Skeptics run hotter: less sure of anyone.
const TEMPERATURE: f32 = 6.0;
const SKEPTIC_TEMPERATURE: f32 = 8.0;

#[derive(Clone, Debug)]
pub struct Candidate {
    pub suspect: Suspect,
    pub score: f32,
    /// The biggest single reason, surfaced so the blame is legible (§11.5).
    pub reason: &'static str,
    /// Every term that went into the score, for debugging and `lab blame`.
    pub parts: Vec<(&'static str, f32)>,
}

#[derive(Clone, Debug)]
pub struct Verdict {
    pub blamed: Suspect,
    pub confidence: u8,
    pub reason: &'static str,
}

/// A rumor pushing toward a particular suspect, and how hard.
#[derive(Clone, Copy, Debug)]
pub struct Rumor {
    pub suspect: Suspect,
    pub strength: f32,
}

/// Who suffered the harm in `event`, if it is a harm.
pub fn victim_of(world: &World, event: EventId) -> Option<NpcId> {
    match world.events[event as usize].kind {
        EventKind::Fire { owner, .. } => Some(owner),
        EventKind::Death { victim, .. } => Some(victim),
        EventKind::Theft { victim, .. } => Some(victim),
        EventKind::Strayed { owner } => Some(owner),
        EventKind::Wounded { victim, .. } => Some(victim),
        EventKind::Cruelty {
            victim,
            act: Cruelty::KillStock | Cruelty::FoulWell,
            ..
        } => Some(victim),
        _ => None,
    }
}

pub fn candidates(
    world: &World,
    observer: NpcId,
    event: EventId,
    rumor: Option<Rumor>,
) -> Vec<Candidate> {
    let ev = &world.events[event as usize];
    let Some(victim) = victim_of(world, event) else {
        return Vec::new();
    };
    let is_fire = matches!(ev.kind, EventKind::Fire { .. });
    let is_theft = matches!(ev.kind, EventKind::Theft { .. });
    let site = world.farm_of(victim);
    let victim_family = world.npc(victim).family;
    let observer_family = world.npc(observer).family;
    let rumor_for = |s: Suspect| match rumor {
        Some(r) if r.suspect == s => r.strength,
        _ => 0.0,
    };

    let mut out = Vec::new();

    if is_fire {
        let weather = world.weather_on(ev.day);
        let nature = if weather.storm { 55.0 } else { -10.0 };
        out.push(Candidate {
            suspect: Suspect::Nature,
            score: nature + rumor_for(Suspect::Nature),
            reason: if weather.storm {
                "there was lightning that night"
            } else {
                "the sky did it"
            },
            parts: Vec::new(),
        });
        let accident = 12.0
            + match ev.day.season() {
                Season::Winter => 18.0,
                Season::Spring => 12.0,
                _ => 0.0,
            };
        out.push(Candidate {
            suspect: Suspect::Accident,
            score: accident + rumor_for(Suspect::Accident),
            reason: "a stray spark",
            parts: Vec::new(),
        });
    }

    match ev.kind {
        // A good fence makes the owner more suspicious, not less.
        EventKind::Strayed { .. } => {
            let fences = world.families[victim_family as usize].stores.work.fences;
            out.push(Candidate {
                suspect: Suspect::Accident,
                score: 20.0 + 40.0 * (1.0 - fences) + rumor_for(Suspect::Accident),
                reason: "it got through the fence",
                parts: Vec::new(),
            });
        }
        EventKind::Cruelty {
            act: Cruelty::KillStock,
            ..
        } => out.push(Candidate {
            suspect: Suspect::Nature,
            score: 25.0 + rumor_for(Suspect::Nature),
            reason: "wolves got it",
            parts: Vec::new(),
        }),
        EventKind::Cruelty {
            act: Cruelty::FoulWell,
            ..
        } => out.push(Candidate {
            suspect: Suspect::Accident,
            score: 25.0 + rumor_for(Suspect::Accident),
            reason: "bad water in a wet spring",
            parts: Vec::new(),
        }),
        _ => {}
    }

    // Lost stock: settlers reach for the nearest nation.
    let stock_loss = is_theft
        || matches!(
            ev.kind,
            EventKind::Cruelty {
                act: Cruelty::KillStock,
                ..
            }
        );
    if stock_loss {
        // Whichever nation the observer most suspects.
        let nid = super::nations::NationId::ALL
            .into_iter()
            .max_by(|a, b| {
                super::nations::suspicion(world, observer, *a, site)
                    .total_cmp(&super::nations::suspicion(world, observer, *b, site))
            })
            .unwrap_or(super::nations::NationId::Delaware);
        out.push(Candidate {
            suspect: Suspect::Nation(nid),
            score: super::nations::suspicion(world, observer, nid, site)
                + rumor_for(Suspect::Nation(nid)),
            reason: "they've been coming around hungry",
            parts: vec![(
                "prejudice",
                super::nations::suspicion(world, observer, nid, site),
            )],
        });
    }

    if is_theft {
        // Cows wander. Sometimes that's all it was.
        out.push(Candidate {
            suspect: Suspect::Accident,
            score: 20.0 + rumor_for(Suspect::Accident),
            reason: "it strayed off",
            parts: Vec::new(),
        });
    }

    for person in world.living() {
        let p = person.id;
        if p == observer
            || p == victim
            || person.family == observer_family
            || LifeStage::of(person.age) == LifeStage::Child
        {
            continue;
        }

        let hostility = (-world.opinion(observer, p)).max(0) as f32 * 0.9;
        let proximity = 20.0 * (1.0 - distance(world.farm_of(p), site) / 15.0).max(0.0);
        let mut motive = 0.0;
        // "Their side does this": judged by what people say out loud.
        let gap = (person.ideology.public - world.npc(victim).ideology.public).abs();
        motive += 10.0 * gap;
        if world.feud_between(person.family, victim_family) {
            motive += 20.0;
        }
        // Poverty makes you a suspect (§10): everyone knows who's been begging.
        let hunger = if is_theft
            && world.families[person.family as usize]
                .stores
                .visibly_hungry(ev.day)
        {
            30.0
        } else {
            0.0
        };
        let capability = 5.0;
        let priors = world
            .npc(observer)
            .memories
            .iter()
            .filter(|m| m.event != event && m.believed == Suspect::Person(p))
            .count()
            .min(3) as f32;
        let pattern = priors * 18.0;
        let alibi = if person.alibi == Some(ev.day) {
            50.0
        } else {
            0.0
        };
        let rumor_boost = rumor_for(Suspect::Person(p));

        let score =
            -25.0 + hostility + proximity + motive + hunger + capability + pattern + rumor_boost
                - alibi;

        let reasons = [
            (hostility, "bad blood between them"),
            (proximity, "was seen on the road that night"),
            (motive, "their side does this"),
            (pattern, "has done it before"),
            (rumor_boost, "everybody says so"),
            (hunger, "their family is starving"),
        ];
        let reason = reasons
            .iter()
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|r| r.1)
            .unwrap_or("a feeling");

        out.push(Candidate {
            suspect: Suspect::Person(p),
            score,
            reason,
            parts: vec![
                ("base", -25.0),
                ("hostility", hostility),
                ("proximity", proximity),
                ("motive", motive),
                ("hunger", hunger),
                ("capability", capability),
                ("pattern", pattern),
                ("rumor", rumor_boost),
                ("alibi", -alibi),
            ],
        });
    }

    out
}

/// Softmax over scores, returned in the same order as `candidates`.
pub fn beliefs(candidates: &[Candidate]) -> Vec<f32> {
    beliefs_at(candidates, TEMPERATURE)
}

pub fn beliefs_at(candidates: &[Candidate], temperature: f32) -> Vec<f32> {
    let max = candidates
        .iter()
        .map(|c| c.score)
        .fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = candidates
        .iter()
        .map(|c| ((c.score - max) / temperature).exp())
        .collect();
    let total: f32 = exps.iter().sum();
    exps.into_iter().map(|e| e / total).collect()
}

/// Run the full attribution for one observer and sample what they believe.
pub fn judge(world: &mut World, observer: NpcId, event: EventId, rumor: Option<Rumor>) -> Verdict {
    let cands = candidates(world, observer, event, rumor);
    if cands.is_empty() {
        return Verdict {
            blamed: Suspect::Accident,
            confidence: 0,
            reason: "no idea",
        };
    }
    let temperature =
        TEMPERATURE + SKEPTIC_TEMPERATURE * world.npc(observer).temperament.skepticism;
    let probs = beliefs_at(&cands, temperature);
    let roll = world.rng.unit();
    let mut acc = 0.0;
    let mut chosen = cands.len() - 1;
    for (i, p) in probs.iter().enumerate() {
        acc += p;
        if roll < acc {
            chosen = i;
            break;
        }
    }
    Verdict {
        blamed: cands[chosen].suspect,
        confidence: (probs[chosen] * 100.0).round().clamp(1.0, 100.0) as u8,
        reason: cands[chosen].reason,
    }
}
