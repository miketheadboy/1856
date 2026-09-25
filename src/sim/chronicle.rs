//! Reading the machine: the chronicle (§18.3) and the cascade debug tree (§15.4).

use std::collections::HashMap;

use super::events::{EventId, EventKind, FireCause, Retaliation, Source, Suspect, WorldEvent};
use super::world::{NpcId, PLAYER, World};

fn who(world: &World, id: NpcId) -> String {
    if id == PLAYER {
        "you".into()
    } else {
        world.name(id).to_string()
    }
}

pub fn suspect_label(world: &World, s: Suspect) -> String {
    match s {
        Suspect::Nature => "lightning".into(),
        Suspect::Accident => "an accident".into(),
        Suspect::Person(p) => who(world, p),
    }
}

fn cause_label(world: &World, cause: FireCause) -> String {
    match cause {
        FireCause::Lightning => "lightning".into(),
        FireCause::Hearth => "an unattended hearth".into(),
        FireCause::EscapedBurn => "a controlled burn that got away".into(),
        FireCause::Arson(p) => format!("arson by {}", who(world, p)),
    }
}

fn surname(world: &World, id: NpcId) -> &'static str {
    world.family_of(id).surname
}

fn barn(world: &World, owner: NpcId) -> String {
    if world.npc(owner).family == 0 {
        "Your barn".into()
    } else {
        format!("The {} barn", surname(world, owner))
    }
}

/// One-line debug description of any event. `omniscient` reveals true causes.
pub fn debug_line(world: &World, ev: &WorldEvent, omniscient: bool) -> String {
    match ev.kind {
        EventKind::Fire {
            owner,
            cause,
            spread_from,
        } => {
            let spread = spread_from
                .map(|s| format!(" (spread from the {} place)", surname(world, s)))
                .unwrap_or_default();
            let truth = if omniscient || cause == FireCause::Arson(PLAYER) {
                format!(" [truth: {}]", cause_label(world, cause))
            } else {
                String::new()
            };
            format!("[FIRE] {} burned{}{}", barn(world, owner), spread, truth)
        }
        EventKind::Death { victim, killer } => {
            let by = match killer {
                Some(k) if omniscient || k == PLAYER => format!(" by {}", who(world, k)),
                _ => String::new(),
            };
            if victim == PLAYER {
                format!("[DEATH] You were killed{}", by)
            } else {
                format!("[DEATH] {} was killed{}", who(world, victim), by)
            }
        }
        EventKind::Grief { mourner, deceased } => {
            format!(
                "[GRIEF] {} mourns {}",
                who(world, mourner),
                who(world, deceased)
            )
        }
        EventKind::Belief {
            holder,
            blamed,
            confidence,
            source,
            reason,
            ..
        } => {
            let how = match source {
                Source::Witnessed => "saw".to_string(),
                Source::Victim => "blames".into(),
                Source::Bystander => "suspects".into(),
                Source::Told(t) => format!("heard from {} and blames", who(world, t)),
            };
            format!(
                "[ATTRIBUTION] {} {} {} ({}%) — {}",
                who(world, holder),
                how,
                suspect_label(world, blamed),
                confidence,
                reason
            )
        }
        EventKind::OpinionChange {
            holder,
            target,
            delta,
            after,
        } => format!(
            "[OPINION] {} → {}: {:+} (now {})",
            who(world, holder),
            who(world, target),
            delta,
            after
        ),
        EventKind::Gossip {
            teller,
            listener,
            blamed,
            ..
        } => format!(
            "[GOSSIP] {} tells {} it was {}",
            who(world, teller),
            who(world, listener),
            suspect_label(world, blamed)
        ),
        EventKind::FactionGrievance {
            faction,
            level,
            authorized,
        } => format!(
            "[FACTION] {} grievance at {}{}",
            faction.label(),
            level,
            if authorized {
                " — retaliation authorized"
            } else {
                ""
            }
        ),
        EventKind::FeudDeclared { a, b } => format!(
            "[FEUD] {} and {} are at war",
            family_label(world, a),
            family_label(world, b)
        ),
        EventKind::Retaliation {
            actor,
            target,
            method,
        } => format!(
            "[REVENGE] {} goes after {} ({})",
            who(world, actor),
            who(world, target),
            match method {
                Retaliation::Arson => "with a torch",
                Retaliation::Ambush => "with a rifle",
            }
        ),
    }
}

fn family_label(world: &World, family: u32) -> String {
    if family == 0 {
        "your family".into()
    } else {
        let name = world.families[family as usize].surname;
        let plural = if ["s", "x", "ch", "sh"].iter().any(|e| name.ends_with(e)) {
            "es"
        } else {
            "s"
        };
        format!("the {}{}", name, plural)
    }
}

/// Is this event worth a line in the chronicle, or is it plumbing?
/// Without `omniscient`, only what the player could plausibly hear about.
pub fn is_notable(world: &World, ev: &WorldEvent, omniscient: bool) -> bool {
    match ev.kind {
        EventKind::Gossip { .. } | EventKind::Grief { .. } => false,
        EventKind::Retaliation { .. } => omniscient,
        EventKind::OpinionChange { delta, .. } => delta <= -40,
        EventKind::Belief {
            holder,
            source,
            confidence,
            ..
        } => {
            let victim = super::attribution::victim_of(world, ev_about(ev));
            (source == Source::Witnessed
                || (source == Source::Victim
                    && confidence >= super::systems::ACCUSATION_CONFIDENCE))
                && victim.is_some_and(|v| world.npc(v).family == world.npc(holder).family)
        }
        _ => true,
    }
}

fn ev_about(ev: &WorldEvent) -> EventId {
    match ev.kind {
        EventKind::Belief { about, .. } | EventKind::Gossip { about, .. } => about,
        _ => ev.id,
    }
}

/// The chronicle: notable events, dated. Omniscient mode knows the truth.
pub fn chronicle(world: &World, omniscient: bool) -> Vec<String> {
    world
        .events
        .iter()
        .filter(|ev| is_notable(world, ev, omniscient))
        .map(|ev| format!("{}  {}", ev.day, debug_line(world, ev, omniscient)))
        .collect()
}

fn children_index(world: &World) -> HashMap<EventId, Vec<EventId>> {
    let mut children: HashMap<EventId, Vec<EventId>> = HashMap::new();
    for ev in &world.events {
        if let Some(origin) = ev.origin() {
            children.entry(origin).or_default().push(ev.id);
        }
    }
    children
}

/// The cascade tree (§15.4) rooted at `root`, following both same-tick
/// children and later consequences. Gossip hops are flattened, and beliefs
/// that led nowhere are counted instead of listed.
pub fn cascade_tree(world: &World, root: EventId, max_lines: usize) -> String {
    let children = children_index(world);
    let mut out = Vec::new();
    let mut quiet = 0usize;
    walk(
        world, &children, root, 0, "", &mut out, &mut quiet, max_lines,
    );
    if out.len() >= max_lines {
        out.push("  … (truncated)".into());
    }
    if quiet > 0 {
        out.push(format!(
            "  + {} other beliefs formed that led nowhere (yet)",
            quiet
        ));
    }
    out.join("\n")
}

#[allow(clippy::too_many_arguments)]
fn walk(
    world: &World,
    children: &HashMap<EventId, Vec<EventId>>,
    id: EventId,
    depth: usize,
    prefix: &str,
    out: &mut Vec<String>,
    quiet: &mut usize,
    max_lines: usize,
) {
    if out.len() >= max_lines {
        return;
    }
    let ev = &world.events[id as usize];
    let kids = children.get(&id).cloned().unwrap_or_default();

    if matches!(ev.kind, EventKind::Gossip { .. }) {
        for k in kids {
            walk(world, children, k, depth, prefix, out, quiet, max_lines);
        }
        return;
    }
    if matches!(ev.kind, EventKind::Belief { .. }) && kids.is_empty() && depth > 0 {
        *quiet += 1;
        return;
    }

    let later = ev.parent.is_none() && depth > 0;
    out.push(format!(
        "{}{}{}",
        prefix,
        if later {
            format!("⟶ {}: ", ev.day)
        } else {
            String::new()
        },
        debug_line(world, ev, true)
    ));
    let child_prefix = format!("{}  ", prefix);
    for k in kids {
        walk(
            world,
            children,
            k,
            depth + 1,
            &child_prefix,
            out,
            quiet,
            max_lines,
        );
    }
}

/// Walk back from an event to the root of everything that led to it.
pub fn ancestry(world: &World, id: EventId) -> Vec<EventId> {
    let mut chain = vec![id];
    let mut cur = id;
    while let Some(origin) = world.events[cur as usize].origin() {
        chain.push(origin);
        cur = origin;
    }
    chain
}

/// Feuds whose whole lineage traces back to a fire no human set (§24 Phase 1 gate).
pub fn wars_nobody_started(world: &World) -> Vec<(EventId, EventId)> {
    world
        .events
        .iter()
        .filter(|ev| matches!(ev.kind, EventKind::FeudDeclared { .. }))
        .filter_map(|ev| {
            let root = *ancestry(world, ev.id).last()?;
            match world.events[root as usize].kind {
                EventKind::Fire { cause, .. } if !matches!(cause, FireCause::Arson(_)) => {
                    Some((ev.id, root))
                }
                _ => None,
            }
        })
        .collect()
}

/// The feud's lineage with gossip and plumbing stripped out, oldest first.
pub fn lineage(world: &World, id: EventId) -> Vec<String> {
    let mut chain = ancestry(world, id);
    chain.reverse();
    chain
        .into_iter()
        .map(|e| &world.events[e as usize])
        .filter(|ev| !matches!(ev.kind, EventKind::Gossip { .. }))
        .map(|ev| format!("{}  {}", ev.day, debug_line(world, ev, true)))
        .collect()
}
