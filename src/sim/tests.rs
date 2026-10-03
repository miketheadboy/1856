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

/// The river shut is a coffee famine at Dunmore's: the store runs down on
/// what the wagons don't bring, and the price climbs (`market`, `larder`).
#[test]
fn a_shut_river_is_a_coffee_famine() {
    use super::market::Good;
    let price_after = |shut: bool| {
        let mut w = World::new(12);
        for _ in 0..70 {
            if shut {
                w.market.freight_factor = 0.3;
            }
            w.advance_day();
        }
        (w.market.price(Good::Coffee), w.market.stock(Good::Coffee))
    };
    let (open_price, open_stock) = price_after(false);
    let (shut_price, shut_stock) = price_after(true);
    assert!(
        shut_stock < open_stock,
        "stock {shut_stock} vs {open_stock}"
    );
    assert!(
        shut_price > open_price,
        "price {shut_price} vs {open_price}"
    );
}

/// Both of them left someone back east: one letter undoes the marriage, and
/// neither lie outlives it.
#[test]
fn when_both_are_bigamists_one_letter_undoes_it() {
    let mut w = World::new(9);
    let a = w.add_npc(2, "Jacob", 30);
    let b = w.add_npc(4, "Temperance", 25);
    w.hearts.back_east.extend([a, b]);
    w.emit_root(EventKind::Marriage { a, b }, None);
    w.run_cascades();
    assert_eq!(w.hearts.bigamous.len(), 2);
    w.emit_root(
        EventKind::Bigamy {
            bigamist: a,
            spouse: b,
        },
        None,
    );
    w.run_cascades();
    assert!(w.hearts.bigamous.is_empty());
    assert!(super::audit::check(&w).is_empty());
}

// Webs: one system's event moving another's books, through the bus.

fn wagon_house(w: &World, side: super::world::Faction) -> u32 {
    (1..w.families.len() as u32)
        .find(|&f| {
            let fam = &w.families[f as usize];
            fam.faction == side && fam.farms() && w.head_of(f).is_some()
        })
        .unwrap()
}

fn parent_kind(w: &World, e: &super::events::WorldEvent) -> Option<EventKind> {
    e.parent
        .or(e.caused_by)
        .map(|p| w.events[p as usize].kind.clone())
}

#[test]
fn a_seeker_hid_in_the_loft_rides_north_in_the_wagon() {
    use super::freight::{self, Route};
    use super::railroad::{Seeker, Status};
    use super::world::Faction;
    for (route, freed) in [(Route::LaneTrail, true), (Route::Westport, false)] {
        let mut w = World::new(9);
        let f = wagon_house(&w, Faction::FreeState);
        w.families[f as usize].stores.oxen = 2;
        w.families[f as usize].stores.cash = 40;
        w.railroad.seekers.push(Seeker {
            name: "Sam",
            from: "Platte County",
            arrived: w.day,
            status: Status::Hidden(f),
            passed: 0,
            wary: 0.5,
            tried: Vec::new(),
            noticed_by: Vec::new(),
        });
        w.railroad
            .pursuit
            .push((0, super::calendar::Day(w.day.0 + 2), 3));
        assert!(freight::set_out(&mut w, f, route, 40));
        w.run_cascades();
        assert_eq!(w.railroad.seekers[0].status == Status::Free, freed);
        let rode = w.events.iter().any(|e| {
            matches!(e.kind, EventKind::Freedom { seeker: 0 })
                && matches!(parent_kind(&w, e), Some(EventKind::WagonOut { .. }))
        });
        assert_eq!(rode, freed, "{route:?}");
        assert_eq!(w.railroad.pursuit.is_empty(), freed);
    }
}

#[test]
fn the_river_sends_cholera_home_with_the_wagon() {
    use super::sickness::Disease;
    let mut w = World::new(9);
    let men: Vec<NpcId> = (1..w.families.len() as u32)
        .filter_map(|f| w.head_of(f))
        .collect();
    // Not in a clean summer.
    for &t in &men {
        w.emit_root(
            EventKind::WagonBack {
                teamster: t,
                route_west: true,
                tenths_of_a_ton: 5,
            },
            None,
        );
    }
    w.run_cascades();
    assert!(
        w.sickness
            .cases
            .iter()
            .all(|c| c.disease != Disease::Cholera)
    );
    w.sickness.cholera_until = Some(super::calendar::Day(w.day.0 + 30));
    for &t in men.iter().cycle().take(men.len() * 4) {
        w.emit_root(
            EventKind::WagonBack {
                teamster: t,
                route_west: true,
                tenths_of_a_ton: 5,
            },
            None,
        );
    }
    w.run_cascades();
    let caught: Vec<_> = w
        .events
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                EventKind::FellSick {
                    disease: Disease::Cholera,
                    ..
                }
            )
        })
        .collect();
    assert!(!caught.is_empty(), "nobody of {} caught it", men.len());
    assert!(caught.iter().all(|e| matches!(
        parent_kind(&w, e),
        Some(EventKind::WagonBack {
            route_west: true,
            ..
        })
    )));
    assert!(super::audit::check(&w).is_empty());
}

#[test]
fn the_lane_trail_brings_the_crate_the_river_towns_would_open() {
    use super::world::Faction;
    let mut w = World::new(9);
    let f = wagon_house(&w, Faction::FreeState);
    let t = w.head_of(f).unwrap();
    w.market.blockade = true;
    w.families[f as usize].stores.arms.on_order = Some((super::calendar::Day(w.day.0 + 30), 2));
    let before = w.families[f as usize].stores.arms.rifles;
    w.emit_root(
        EventKind::WagonBack {
            teamster: t,
            route_west: false,
            tenths_of_a_ton: 5,
        },
        None,
    );
    w.run_cascades();
    let a = &w.families[f as usize].stores.arms;
    assert!(a.on_order.is_none());
    assert!(a.rifles >= before + 2);
    assert!(
        w.events
            .iter()
            .any(|e| matches!(e.kind, EventKind::ArmsArrived { family, .. } if family == f))
    );
}

#[test]
fn a_stopped_teamster_swears_it_was_the_neighbor_he_hates() {
    use super::world::Faction;
    let mut w = World::new(9);
    let f = wagon_house(&w, Faction::FreeState);
    let t = w.head_of(f).unwrap();
    let enemy = w
        .living()
        .find(|n| {
            n.faction == Faction::ProSlavery
                && n.family != 0
                && n.age >= 16
                && !super::world::is_woman(&n.name)
        })
        .unwrap()
        .id;
    w.adjust_opinion(t, enemy, -90);
    let before: Vec<i16> = (0..w.npcs.len() as NpcId)
        .map(|x| w.opinion(t, x))
        .collect();
    let stop = w.emit_root(
        EventKind::WagonStopped {
            teamster: t,
            seized: 30,
        },
        None,
    );
    w.run_cascades();
    let m = w.npc(t).memory_of(stop).expect("he remembers the road");
    assert_eq!(m.believed, Suspect::Person(enemy));
    assert!(before[enemy as usize] < 0);
    assert_eq!(attribution::victim_of(&w, stop), Some(t));
}

#[test]
fn short_weight_sticks_to_a_name() {
    use super::world::Faction;
    let mut w = World::new(9);
    let f = wagon_house(&w, Faction::ProSlavery);
    let t = w.head_of(f).unwrap();
    let n = w.head_of(wagon_house(&w, Faction::FreeState)).unwrap();
    let word = super::standing::word(&w, t);
    w.emit_root(
        EventKind::ShortWeight {
            teamster: t,
            noticed_by: n,
        },
        None,
    );
    w.run_cascades();
    assert!(super::freight::short_weight(&w, t));
    assert!(super::standing::word(&w, t) < word);
    assert!(
        super::standing::parts(&w, t)
            .iter()
            .any(|p| p.name == "gave short weight")
    );
    w.day = super::calendar::Day(w.day.0 + super::freight::SHORT_MEMORY);
    assert!(!super::freight::short_weight(&w, t));
}

#[test]
fn friends_who_sit_up_with_the_dead_catch_it() {
    use super::events::Hardship;
    use super::sickness::{self, Disease};
    let mut w = World::new(9);
    let dead = w.head_of(3).unwrap();
    let home = w.families[3].farm;
    let friends: Vec<NpcId> = w
        .living()
        .filter(|n| n.family != 3 && n.age >= 16 && !n.departed)
        .filter(|n| super::world::distance(w.farm_of(n.id), home) <= 10.0)
        .map(|n| n.id)
        .collect();
    assert!(friends.len() >= 2);
    for &f in &friends {
        w.adjust_opinion(f, dead, 80);
    }
    // One careful house keeps away.
    let careful = w.npc(friends[0]).family;
    w.sickness.quarantine.push(careful);
    let mourner = w.head_of(3);
    let before = mourner.map(|m| w.opinion(m, friends[0]));
    w.npc_mut(dead).alive = false;
    w.emit_root(
        EventKind::Perished {
            victim: dead,
            cause: Hardship::Sickness(Disease::Smallpox),
        },
        None,
    );
    w.run_cascades();
    let wake = w
        .events
        .iter()
        .find(|e| matches!(e.kind, EventKind::Wake { .. }))
        .expect("a burying");
    let EventKind::Wake {
        came, stayed_away, ..
    } = wake.kind
    else {
        unreachable!()
    };
    assert!(came >= 1 && stayed_away >= 1);
    if let (Some(m), Some(b)) = (mourner, before)
        && m != friends[0]
        && w.npc(m).alive
    {
        assert!(
            w.opinion(m, friends[0]) < b,
            "the bereaved remember who kept away"
        );
    }
    // Nobody from the careful house caught it at the wake.
    assert!(
        w.sickness
            .cases
            .iter()
            .all(|c| w.npc(c.who).family != careful)
    );
    assert!(sickness::wake_risk(Disease::Smallpox) > 0.0);
    assert_eq!(sickness::wake_risk(Disease::Scurvy), 0.0);
}

#[test]
fn a_fever_in_the_house_finds_a_poisoner() {
    use super::sickness::Disease;
    let mut w = World::new(9);
    let f = 2u32;
    let who = w.head_of(f).unwrap();
    let id = w.emit_root(
        EventKind::FellSick {
            who,
            disease: Disease::Typhoid,
        },
        None,
    );
    w.run_cascades();
    let belief = w.events.iter().find(|e| {
        matches!(e.kind, EventKind::Belief { about, .. } if about == id) && e.parent == Some(id)
    });
    let Some(b) = belief else {
        panic!("the house wants a cause")
    };
    let EventKind::Belief { holder, .. } = b.kind else {
        unreachable!()
    };
    assert_eq!(w.npc(holder).family, f);
    // The second case in the house doesn't start a new hunt.
    let n = w.events.len();
    w.emit_root(
        EventKind::FellSick {
            who,
            disease: Disease::Cholera,
        },
        None,
    );
    w.run_cascades();
    assert!(
        !w.events[n..]
            .iter()
            .any(|e| matches!(e.kind, EventKind::Belief { .. }) && e.parent == Some(n as u32))
    );
    // Measles is God's will, not a neighbor's.
    assert!(!super::sickness::suspected(Disease::Measles));
}

#[test]
fn a_bank_note_in_the_mail_looks_like_blood_money() {
    use super::mail::Letter;
    let mut w = World::new(9);
    let f = 4u32;
    w.emit_root(
        EventKind::Letter {
            family: f,
            letter: Letter::Money(15),
            reader: None,
        },
        None,
    );
    w.run_cascades();
    assert!(attribution::came_into_money(&w, w.day).contains(&f));
    let harbor = (1..w.families.len() as u32)
        .find(|&h| h != f && w.head_of(h).is_some())
        .unwrap();
    let cap = w.emit_root(
        EventKind::Captured {
            seeker: 0,
            at: Some(harbor),
            informer: None,
        },
        None,
    );
    let observer = w.head_of(harbor).unwrap();
    let suspect = w.head_of(f).unwrap();
    let cands = attribution::candidates(&w, observer, cap, None);
    let c = cands
        .iter()
        .find(|c| c.suspect == Suspect::Person(suspect))
        .unwrap();
    assert!(
        c.parts
            .iter()
            .any(|&(k, v)| k == "money" && v == attribution::FLUSH)
    );
    // A fortnight on, the money is spent and forgotten.
    w.day = super::calendar::Day(w.day.0 + 20);
    assert!(!attribution::came_into_money(&w, w.day).contains(&f));
}

#[test]
fn debt_thins_a_free_state_conscience() {
    use super::railroad::{Answer, answer_for};
    use super::world::Faction;
    let sold = |pressed: bool| {
        let mut n = 0;
        for seed in 0..200 {
            let mut w = World::new(seed);
            let f = wagon_house(&w, Faction::FreeState);
            let h = w.head_of(f).unwrap();
            w.npc_mut(h).ideology.private = 0.1;
            w.npc_mut(h).temperament.honesty = 0.2;
            if pressed {
                w.families[f as usize].stores.debt = super::economy::CREDIT_LIMIT;
            }
            if answer_for(&mut w, f) == Answer::Betray {
                n += 1;
            }
        }
        n
    };
    let (easy, owing) = (sold(false), sold(true));
    assert_eq!(easy, 0, "a free-state man out of debt doesn't sell");
    assert!(owing >= 10, "owing Dunmore, {owing}/200 sold");
}

#[test]
fn the_dead_swear_nothing() {
    // Silas Ashby, seed 9: shot the same tick he watched his wife killed,
    // his witness belief landed after his death and swore an oath.
    let mut w = World::new(9);
    let (a, b, c) = (
        w.head_of(2).unwrap(),
        w.head_of(3).unwrap(),
        w.head_of(4).unwrap(),
    );
    let kin = w
        .living()
        .find(|n| n.family == 2 && n.id != a)
        .map(|n| n.id)
        .unwrap();
    let wife = w.emit_root(
        EventKind::Death {
            victim: kin,
            killer: Some(b),
        },
        None,
    );
    w.emit_root(
        EventKind::Death {
            victim: a,
            killer: Some(c),
        },
        None,
    );
    w.emit_root(
        EventKind::Belief {
            holder: a,
            about: wife,
            blamed: Suspect::Person(b),
            confidence: 100,
            source: super::events::Source::Witnessed,
            reason: "saw it with their own eyes",
        },
        None,
    );
    w.run_cascades();
    assert!(w.ghosts.oaths.iter().all(|o| o.holder != a));
    assert!(
        super::audit::check(&w)
            .iter()
            .all(|br| br.rule != "oath-dead")
    );
}

#[test]
fn every_event_belongs_to_a_system() {
    // `lab matrix` reads the county as a graph of systems; an event nobody
    // owns is a hole in it.
    for seed in [3, 15] {
        let mut w = World::new(seed);
        w.run_days(730);
        for e in &w.events {
            assert_ne!(
                super::debug::system_of(&e.kind),
                "?",
                "{} has no owner",
                super::debug::kind_name(&e.kind)
            );
        }
    }
}

#[test]
fn a_wedding_across_the_line_marks_them_both() {
    use super::marks::{self, Mark};
    use super::world::Faction;
    let mut w = World::new(9);
    let a = w
        .living()
        .find(|n| n.faction == Faction::FreeState && n.age >= 18)
        .unwrap()
        .id;
    let b = w
        .living()
        .find(|n| n.faction == Faction::ProSlavery && n.age >= 18 && n.family != 0)
        .unwrap()
        .id;
    w.hearts
        .couples
        .retain(|&(x, y)| x != a && y != a && x != b && y != b);
    w.emit_root(EventKind::Marriage { a, b }, None);
    w.run_cascades();
    if super::romance::married_to(&w, a) == Some(b) {
        assert!(marks::has(&w, a, Mark::MarriedAcross));
        assert!(marks::has(&w, b, Mark::MarriedAcross));
        assert!(super::standing::repute(&w, a) < 1.0);
    }
}

#[test]
fn the_river_road_is_safer_for_a_man_who_knows_it() {
    use super::freight::{Route, stop_odds};
    use super::marks::Mark;
    use super::world::Faction;
    let mut w = World::new(9);
    let f = wagon_house(&w, Faction::FreeState);
    let t = w.head_of(f).unwrap();
    w.market.blockade = true;
    let before = stop_odds(&w, t, Route::Westport);
    w.emit_root(
        EventKind::MarkEarned {
            who: t,
            mark: Mark::KnowsTheRiver,
        },
        None,
    );
    w.run_cascades();
    assert!((stop_odds(&w, t, Route::Westport) - before * 0.5).abs() < 1e-6);
}

#[test]
fn the_convicted_mans_people_turn_on_those_who_swore() {
    let mut w = World::new(9);
    let accused = w.head_of(2).unwrap();
    let kin = w
        .living()
        .find(|n| n.family == 2 && n.id != accused)
        .map(|n| n.id);
    let witness = w.head_of(3).unwrap();
    let victim = w.head_of(4).unwrap();
    let shot = w.emit_root(
        EventKind::Wounded {
            victim,
            attacker: accused,
        },
        None,
    );
    let today = w.day;
    w.npc_mut(witness).remember(super::world::MemoryRef {
        event: shot,
        believed: Suspect::Person(accused),
        confidence: 90,
        source: super::events::Source::Witnessed,
        day: today,
        weight: 100,
    });
    w.emit_root(
        EventKind::Tried {
            accused,
            judge: victim,
            convicted: true,
            days: 10,
            fine: 0,
        },
        Some(shot),
    );
    w.run_cascades();
    if let Some(k) = kin {
        assert!(w.opinion(k, witness) < 0, "the kin know who swore");
    }
    assert!(
        w.is_scheduled(|e| matches!(*e,
        EventKind::OpinionChange { holder, target, .. } if holder == accused && target == witness))
    );
}

#[test]
fn a_muster_fight_is_the_cause_of_its_wounds() {
    for seed in 1..=30 {
        let mut w = World::new(seed);
        w.run_days(400);
        for e in &w.events {
            if let EventKind::Wounded { .. } = e.kind
                && let Some(p) = e.parent
                && matches!(w.events[p as usize].kind, EventKind::Muster { .. })
            {
                let EventKind::Muster { wounded, .. } = w.events[p as usize].kind else {
                    unreachable!()
                };
                assert!(wounded >= 1);
                return;
            }
        }
    }
    panic!("no muster fight wounded anyone in 30 seeds");
}
