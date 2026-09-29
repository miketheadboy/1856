use super::attribution::{self, Rumor};
use super::chronicle;
use super::events::{EventKind, FireCause, MAX_CASCADE_DEPTH, Suspect};
use super::world::{NpcId, PLAYER, World};

fn first_of_family(w: &World, family: u32) -> NpcId {
    w.head_of(family).unwrap()
}

#[test]
fn same_seed_same_history() {
    let mut a = World::new(42);
    let mut b = World::new(42);
    a.run_days(200);
    b.run_days(200);
    assert_eq!(a.events.len(), b.events.len());
    assert_eq!(
        chronicle::chronicle(&a, true),
        chronicle::chronicle(&b, true)
    );
}

#[test]
fn different_seeds_diverge() {
    let mut a = World::new(1);
    let mut b = World::new(2);
    a.run_days(200);
    b.run_days(200);
    assert_ne!(
        chronicle::chronicle(&a, true),
        chronicle::chronicle(&b, true)
    );
}

#[test]
fn relationships_are_sparse_and_default_on_miss() {
    let w = World::new(7);
    let (a, b) = (first_of_family(&w, 1), first_of_family(&w, 2));
    assert!(
        w.relationship_count() <= 14,
        "only seeded grudges are stored"
    );
    let expected = if w.npc(a).faction == w.npc(b).faction {
        10
    } else {
        -10
    };
    // Either a seeded grudge or the faction default.
    assert!(w.opinion(a, b) == expected || w.opinion(a, b) <= -35);
}

#[test]
fn cascade_depth_is_capped() {
    let mut w = World::new(3);
    let victim = first_of_family(&w, 1);
    w.player_kill(victim);
    assert!(
        w.events
            .iter()
            .all(|e| e.cascade_depth <= MAX_CASCADE_DEPTH)
    );
}

#[test]
fn killing_someone_makes_their_family_grieve() {
    let mut w = World::new(5);
    let family = w.npc(first_of_family(&w, 3)).family;
    let victim = first_of_family(&w, 3);
    w.player_kill(victim);
    assert!(!w.npc(victim).alive);
    let mourners = w
        .events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Grief { deceased, .. } if deceased == victim))
        .count();
    let kin = w.living().filter(|n| n.family == family).count();
    assert_eq!(mourners, kin);
}

#[test]
fn prior_hostility_dominates_attribution() {
    let mut w = World::new(11);
    let owner = first_of_family(&w, 1);
    // Find two outsiders; make the owner hate one of them.
    let outsiders: Vec<NpcId> = w
        .living()
        .filter(|n| n.id != PLAYER && n.family != w.npc(owner).family && n.age >= 16)
        .map(|n| n.id)
        .take(2)
        .collect();
    let (hated, neutral) = (outsiders[0], outsiders[1]);
    w.set_opinion(owner, hated, -90);
    w.set_opinion(owner, neutral, 0);
    let fire = w.emit_root(
        EventKind::Fire {
            owner,
            cause: FireCause::Hearth,
            spread_from: None,
        },
        None,
    );
    let cands = attribution::candidates(&w, owner, fire, None);
    let score = |s: Suspect| cands.iter().find(|c| c.suspect == s).unwrap().score;
    assert!(score(Suspect::Person(hated)) > score(Suspect::Person(neutral)) + 40.0);
}

#[test]
fn an_alibi_protects_you() {
    let mut w = World::new(13);
    let owner = first_of_family(&w, 2);
    w.set_opinion(owner, PLAYER, -80);
    let fire = w.emit_root(
        EventKind::Fire {
            owner,
            cause: FireCause::Hearth,
            spread_from: None,
        },
        None,
    );
    let without = attribution::candidates(&w, owner, fire, None);
    w.player_go_to_tavern();
    let with = attribution::candidates(&w, owner, fire, None);
    let player = |c: &Vec<attribution::Candidate>| {
        c.iter()
            .find(|c| c.suspect == Suspect::Person(PLAYER))
            .unwrap()
            .score
    };
    assert!(player(&with) < player(&without) - 40.0);
}

#[test]
fn rumors_push_belief() {
    let mut w = World::new(17);
    let owner = first_of_family(&w, 4);
    let fire = w.emit_root(
        EventKind::Fire {
            owner,
            cause: FireCause::Hearth,
            spread_from: None,
        },
        None,
    );
    let rumor = Rumor {
        suspect: Suspect::Person(PLAYER),
        strength: 40.0,
    };
    let base = attribution::candidates(&w, owner, fire, None);
    let pushed = attribution::candidates(&w, owner, fire, Some(rumor));
    let p = |c: &[attribution::Candidate]| {
        let probs = attribution::beliefs(c);
        c.iter()
            .zip(probs)
            .find(|(c, _)| c.suspect == Suspect::Person(PLAYER))
            .unwrap()
            .1
    };
    assert!(p(&pushed) > p(&base));
}

#[test]
fn memory_is_capped_but_sticky_survives() {
    use super::events::Source;
    use super::world::{MEMORY_CAPACITY, MemoryRef};
    let mut w = World::new(19);
    let npc = w.npc_mut(1);
    npc.remember(MemoryRef {
        event: 9999,
        believed: Suspect::Person(PLAYER),
        confidence: 90,
        source: Source::Victim,
        day: super::Day(0),
        weight: 255,
    });
    for i in 0..100 {
        npc.remember(MemoryRef {
            event: i,
            believed: Suspect::Accident,
            confidence: 50,
            source: Source::Bystander,
            day: super::Day(i),
            weight: 10,
        });
    }
    assert_eq!(npc.memories.len(), npc.memory_capacity());
    assert!(npc.memory_capacity() <= MEMORY_CAPACITY);
    assert!(npc.memory_of(9999).is_some());
}

/// Phase 1 gate (§24): can the sim generate a war that nobody started?
#[test]
fn some_seed_produces_a_war_nobody_started() {
    let found = (1..=40).any(|seed| {
        let mut w = World::new(seed);
        w.run_days(365);
        !chronicle::wars_nobody_started(&w).is_empty()
    });
    assert!(
        found,
        "no seed in 1..=40 produced a feud rooted in a natural fire"
    );
}

/// The player can corner corn. The county notices: compared with the same
/// winter where they didn't, their name is worse.
#[test]
fn cornering_corn_spikes_the_price_and_breeds_resentment() {
    use super::market::Good;
    let run = |corner: bool| {
        let mut w = World::new(21);
        w.winter_severity = 1.6;
        w.autopilot_player = false;
        w.run_days(60); // into the cold
        let before = w.market.price(Good::Corn);
        if corner {
            w.families[0].stores.cash = 5000;
            w.player_buy(Good::Corn, 10_000.0);
            assert!(w.market.stock(Good::Corn) < 1.0, "bought the store out");
            w.run_days(1);
            assert!(
                w.market.price(Good::Corn) > before * 2.0,
                "scarcity moves the price"
            );
        } else {
            w.run_days(1);
        }
        w.run_days(30);
        super::psyche::reputation(&w, PLAYER)
    };
    let (hoarder, neighbor) = (run(true), run(false));
    assert!(
        hoarder < neighbor,
        "the hoarder's name goes bad in a hungry winter: {hoarder} vs {neighbor}"
    );
}

/// Phase 2 gate (§24): does a bad winter change the political map six months later?
/// Politics here: thefts, storekeeper favors, and accusations over thefts by
/// August 1856. Accusations over fires are left out: they are dominated by
/// whichever feud cascades a seed happens to roll, not by the winter.
#[test]
fn a_harsh_winter_sours_the_summer() {
    use super::events::Suspect;
    let politics = |severity: f32| -> usize {
        (1..=24)
            .map(|seed| {
                let mut w = World::with_winter(seed, Some(severity));
                w.run_days(290);
                w.events
                    .iter()
                    .map(|e| match e.kind {
                        EventKind::Theft { .. } | EventKind::Favor { .. } => 20,
                        EventKind::Belief {
                            blamed: Suspect::Person(_),
                            confidence,
                            about,
                            ..
                        } if confidence >= 20
                            && matches!(w.events[about as usize].kind, EventKind::Theft { .. }) =>
                        {
                            1
                        }
                        _ => 0,
                    })
                    .sum::<usize>()
            })
            .sum()
    };
    let (mild, harsh) = (politics(0.5), politics(1.6));
    assert!(
        harsh as f32 > mild as f32 * 1.1,
        "mild {mild}, harsh {harsh}"
    );
}
/// HashSet and HashMap iterate in a different order every process; a
/// feud set walked in that order once made two runs of one seed part ways
/// at a child's grave. Two long runs must match exactly.
#[test]
fn same_seed_same_history_over_years() {
    for seed in [2, 8] {
        let run = || {
            let mut w = World::new(seed);
            w.run_days(500);
            (w.events.len(), w.rng.unit())
        };
        assert_eq!(run(), run(), "seed {seed}");
    }
}

// ---- the books against each other (see `audit`) --------------------------

/// Every system keeps its own books; every day of two years across seeds,
/// they must agree: no dead man on the sick list or the payroll, no paper
/// nobody swore to, no corpse in the jail, no coat on a head.
#[test]
fn the_books_agree_every_day() {
    use super::audit;
    for seed in [1, 4, 9, 15] {
        let mut w = World::new(seed);
        for _ in 0..500 {
            w.advance_day();
            let breaches = audit::check(&w);
            assert!(
                breaches.is_empty(),
                "seed {seed}, day {}:\n{}",
                w.day.0,
                breaches
                    .iter()
                    .map(|b| b.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }
}

/// The auditor caught Thomas Pike riding out to shoot Cassius Bell three
/// days after Bell went to the Lecompton jail. A jailed man isn't home.
#[test]
fn a_jailed_man_is_not_home_to_be_shot() {
    use super::events::Retaliation;
    use super::warrant::Held;
    let mut w = World::new(1);
    let target = first_of_family(&w, 2);
    let actor = first_of_family(&w, 3);
    w.warrants
        .held
        .push((target, super::Day(w.day.0 + 30), Held::Jail));
    let out = super::systems::resolve_plot(&mut w, actor, target, Retaliation::Ambush, None);
    w.run_cascades();
    assert!(out.is_none());
    assert!(w.npc(target).alive);
}

/// ...and then the justice tried the corpse and gave it thirty days.
#[test]
fn no_court_tries_a_corpse_and_his_papers_close_the_day_he_dies() {
    use super::warrant::State;
    let mut w = World::new(1);
    w.run_days(3);
    let accused = first_of_family(&w, 2);
    let about = w.emit_root(
        EventKind::Fire {
            owner: first_of_family(&w, 3),
            cause: FireCause::Arson(accused),
            spread_from: None,
        },
        None,
    );
    w.run_cascades();
    let i = super::warrant::write(&mut w, accused, about, 20);
    w.warrants.trials.push((accused, i, w.day));
    w.player_kill(accused);
    assert_eq!(w.warrants.list[i].state, State::Dead, "closed the same day");
    w.advance_day();
    assert!(
        !w.events
            .iter()
            .any(|e| matches!(e.kind, EventKind::Tried { accused: a, .. } if a == accused)),
        "tried a dead man"
    );
    assert!(!w.warrants.held.iter().any(|h| h.0 == accused));
}

/// A dead child's fever doesn't outlive the child by a day.
#[test]
fn the_dead_leave_the_sick_list_the_day_they_die() {
    use super::sickness::Disease;
    let mut w = World::new(2);
    let who = first_of_family(&w, 4);
    super::sickness::catch(&mut w, who, Disease::Flux);
    assert!(w.sickness.cases.iter().any(|c| c.who == who));
    w.player_kill(who);
    assert!(!w.sickness.cases.iter().any(|c| c.who == who));
}

/// Gossip against attribution, across seeds. Three claims of the design:
/// a rumor moves blame (a telling that never lands is a dead system) but
/// doesn't dictate it (listeners weigh their own grudges, §11); what's
/// passed mouth to mouth is wrong far more often than what was seen; and
/// talk crosses the faction line, but less than it stays home.
#[test]
fn gossip_against_attribution() {
    use super::events::Source;
    let (mut told, mut carried) = (0, 0);
    let (mut seen, mut seen_right, mut heard, mut heard_right) = (0, 0, 0, 0);
    let (mut tellings, mut across) = (0, 0);
    for seed in [1, 3, 5, 7] {
        let mut w = World::new(seed);
        w.run_days(400);
        for e in &w.events {
            match e.kind {
                EventKind::Gossip {
                    teller, listener, ..
                } => {
                    tellings += 1;
                    if w.npc(teller).faction != w.npc(listener).faction {
                        across += 1;
                    }
                }
                EventKind::Belief {
                    about,
                    blamed,
                    source,
                    ..
                } => {
                    if let Some(p) = e.parent
                        && let EventKind::Gossip { blamed: said, .. } = w.events[p as usize].kind
                    {
                        told += 1;
                        carried += (said == blamed) as u32;
                    }
                    let truth = match w.events[about as usize].kind {
                        EventKind::Fire { cause, .. } => cause.truth(),
                        EventKind::Death {
                            killer: Some(k), ..
                        } => Suspect::Person(k),
                        _ => continue,
                    };
                    match source {
                        Source::Witnessed => {
                            seen += 1;
                            seen_right += (truth == blamed) as u32;
                        }
                        Source::Told(_) => {
                            heard += 1;
                            heard_right += (truth == blamed) as u32;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
    let rate = |a: u32, n: u32| a as f32 / n.max(1) as f32;
    let carry = rate(carried, told);
    assert!(told > 200, "gossip barely lands: {told} beliefs");
    assert!(
        (0.2..0.8).contains(&carry),
        "a rumor should sway, not dictate: {carried}/{told}"
    );
    assert!(heard > 100 && seen > 0, "{heard} heard, {seen} seen");
    assert!(
        rate(seen_right, seen) > rate(heard_right, heard) + 0.3,
        "eyes {seen_right}/{seen} vs ears {heard_right}/{heard}"
    );
    let cross = rate(across, tellings);
    assert!(
        (0.1..0.5).contains(&cross),
        "talk across the line: {across}/{tellings}"
    );
}

/// Eyewitnesses (Wells & Loftus): at night, afraid, with a gun going off,
/// a witness is often wrong, and no less sure for it. When wrong, the name
/// is a man of the same side they'd have suspected anyway.
#[test]
fn witnesses_are_sure_and_often_wrong() {
    use super::events::Source;
    let (mut seen, mut right, mut wrong_sure) = (0u32, 0u32, 0u32);
    let mut same_side = 0u32;
    for seed in 1..=8 {
        let mut w = World::new(seed);
        w.run_days(450);
        for e in &w.events {
            let EventKind::Belief {
                about,
                blamed,
                confidence,
                source: Source::Witnessed,
                reason: "saw it with their own eyes",
                ..
            } = e.kind
            else {
                continue;
            };
            let truth = match w.events[about as usize].kind {
                EventKind::Fire {
                    cause: FireCause::Arson(a),
                    ..
                } => a,
                EventKind::Death {
                    killer: Some(k), ..
                } => k,
                EventKind::Wounded { attacker, .. } => attacker,
                EventKind::ShotAt { shooter, .. } => shooter,
                EventKind::Theft { thief, .. } => thief,
                _ => continue,
            };
            seen += 1;
            if blamed == Suspect::Person(truth) {
                right += 1;
            } else if let Suspect::Person(x) = blamed {
                wrong_sure += (confidence >= 85) as u32;
                same_side += (w.npc(x).faction == w.npc(truth).faction) as u32;
            }
        }
    }
    let wrong = seen - right;
    assert!(seen >= 20, "{seen} witnesses");
    let rate = right as f32 / seen as f32;
    assert!((0.4..0.9).contains(&rate), "right {right}/{seen}");
    assert_eq!(wrong_sure, wrong, "the wrong are as sure as the right");
    assert_eq!(same_side, wrong, "a wrong face is one of the same side");
}

/// Allport & Postman: rumor goes as importance times ambiguity. A killing
/// the county can't agree on outruns a theft everyone pins on one man.
#[test]
fn rumor_runs_on_importance_and_ambiguity() {
    use super::events::Source;
    use super::systems::talkability;
    use super::world::MemoryRef;
    let mut w = World::new(6);
    let (a, b) = (first_of_family(&w, 1), first_of_family(&w, 2));
    let (c, d) = (first_of_family(&w, 3), first_of_family(&w, 4));
    let killing = w.emit_root(
        EventKind::Death {
            victim: a,
            killer: Some(b),
        },
        None,
    );
    let theft = w.emit_root(
        EventKind::Theft {
            thief: c,
            victim: d,
            loot: super::events::Loot::Grain,
        },
        None,
    );
    let people: Vec<NpcId> = w.living().map(|n| n.id).take(12).collect();
    let today = w.day;
    for (i, &p) in people.iter().enumerate() {
        let split = [b, c, d][i % 3];
        for (event, who) in [(killing, split), (theft, c)] {
            w.npc_mut(p).remember(MemoryRef {
                event,
                believed: Suspect::Person(who),
                confidence: 70,
                source: Source::Told(a),
                day: today,
                weight: 100,
            });
        }
    }
    let (k, t) = (talkability(&w, killing), talkability(&w, theft));
    assert!(k > 2.5 * t, "a disputed killing {k} vs a settled theft {t}");
}

/// Rosnow: frightened people talk.
#[test]
fn fear_loosens_tongues() {
    use super::systems::county_fear;
    let mut w = World::new(6);
    let calm = county_fear(&w);
    for n in &mut w.npcs {
        n.emotions.fear = 70.0;
    }
    assert!(county_fear(&w) > calm + 0.4);
}

/// Stories sharpen in the telling, toward the teller's old grudges, but
/// most tellings carry what the teller believed. Each sharpening moves the
/// teller's own memory first, so the auditor sees an honest record.
#[test]
fn stories_sharpen_but_mostly_hold() {
    let (mut sharpened, mut tellings) = (0, 0);
    for seed in [2, 4, 6, 8] {
        let mut w = World::new(seed);
        w.run_days(450);
        for e in &w.events {
            match e.kind {
                EventKind::Gossip { .. } => tellings += 1,
                EventKind::Belief {
                    reason: "the story sharpened in the telling",
                    ..
                } => sharpened += 1,
                _ => {}
            }
        }
        assert!(super::audit::check(&w).is_empty(), "seed {seed}");
    }
    assert!(sharpened > 0, "no story ever sharpened");
    assert!(
        (sharpened as f32) < 0.2 * tellings as f32,
        "{sharpened} of {tellings} tellings sharpened"
    );
}

// ---- the lies the county tells on purpose ---------------------------------

/// Year and a day: a wounded man who dies of the fever in it was killed by
/// the man who shot him. Those who blamed someone for the shooting carry it
/// to the killing; his own side may say it was the fever.
#[test]
fn a_wound_that_kills_later_is_a_killing() {
    use super::events::{Hardship, Source};
    use super::sickness::Disease;
    use super::world::MemoryRef;
    let mut w = World::new(4);
    let victim = first_of_family(&w, 2);
    let shooter = first_of_family(&w, 3);
    let wound = w.emit_root(
        EventKind::Wounded {
            victim,
            attacker: shooter,
        },
        None,
    );
    w.run_cascades();
    let kin = w
        .living()
        .find(|n| n.family == w.npc(victim).family && n.id != victim)
        .unwrap()
        .id;
    let today = w.day;
    w.npc_mut(kin).remember(MemoryRef {
        event: wound,
        believed: Suspect::Person(shooter),
        confidence: 90,
        source: Source::Victim,
        day: today,
        weight: 200,
    });
    w.run_days(10);
    let (kind, why) = super::systems::perished(&w, victim, Hardship::Sickness(Disease::WoundFever));
    assert_eq!(
        kind,
        EventKind::Death {
            victim,
            killer: Some(shooter)
        }
    );
    assert_eq!(why, Some(wound));
    let death = w.emit_root(kind, why);
    w.run_cascades();
    assert!(!w.npc(victim).alive);
    assert_eq!(
        w.npc(kin).memory_of(death).map(|m| m.believed),
        Some(Suspect::Person(shooter)),
        "his people know who killed him"
    );
    assert!(
        !w.events.iter().any(|e| matches!(e.kind,
            EventKind::Belief { about, reason: "saw it with their own eyes", .. } if about == death)),
        "nobody watched him die of it"
    );
    // Hunger isn't a wound's doing.
    let other = first_of_family(&w, 4);
    let (k, _) = super::systems::perished(&w, other, Hardship::Hunger);
    assert!(matches!(k, EventKind::Perished { .. }));
}

/// A dead man on the roll: kept on purpose, with an event for it; never
/// among the men who ride.
#[test]
fn the_dead_are_carried_on_the_rolls_but_never_ride() {
    let mut padded = 0;
    for seed in 1..=12 {
        let mut w = World::new(seed);
        for _ in 0..730 {
            w.advance_day();
            if let Some((_, _, men)) = &w.law.muster {
                assert!(men.iter().all(|&m| w.npc(m).alive), "a dead man rode");
            }
        }
        for e in &w.events {
            if let EventKind::RollPadded { name, .. } = e.kind {
                padded += 1;
                assert!(
                    w.death_of(name).is_some_and(|d| d < e.id),
                    "padded a living man"
                );
            }
        }
    }
    assert!(padded > 0, "no captain ever padded a roll");
}

/// A newcomer with a wife back in the States marries here; the letter comes;
/// the marriage is undone and the bride goes home to her people.
#[test]
fn a_letter_from_the_states_undoes_a_bigamous_marriage() {
    use super::romance::married_to;
    let mut w = World::new(9);
    let liar = w.add_npc(2, "Silas", 30);
    w.hearts.back_east.push(liar);
    let bride = w
        .living()
        .find(|n| {
            n.family != 2
                && n.family != 0
                && super::world::is_woman(&n.name)
                && (18..40).contains(&n.age)
                && married_to(&w, n.id).is_none()
        })
        .map(|n| n.id);
    let bride = match bride {
        Some(b) => b,
        None => w.add_npc(4, "Ruth", 22),
    };
    let home = w.npc(bride).family;
    w.emit_root(EventKind::Marriage { a: liar, b: bride }, None);
    w.run_cascades();
    assert_eq!(married_to(&w, liar), Some(bride));
    assert_eq!(w.hearts.bigamous.len(), 1);
    assert!(super::audit::check(&w).is_empty());
    w.emit_root(
        EventKind::Bigamy {
            bigamist: liar,
            spouse: bride,
        },
        None,
    );
    w.run_cascades();
    assert_eq!(married_to(&w, liar), None);
    assert_eq!(w.npc(bride).family, home, "she went home");
    assert!(w.opinion(bride, liar) < -50);
    assert!(w.hearts.bigamous.is_empty());
    assert!(super::audit::check(&w).is_empty());
}
