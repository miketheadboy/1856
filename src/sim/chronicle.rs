//! Reading the machine: the chronicle (§18.3) and the cascade debug tree (§15.4).

use std::collections::HashMap;

use super::events::{
    Cruelty, Desperate, EventId, EventKind, FireCause, Hardship, Loot, Retaliation, Source,
    Suspect, WorldEvent,
};
use super::world::{NpcId, PLAYER, World};

fn who(world: &World, id: NpcId) -> String {
    if id == PLAYER {
        "you".into()
    } else {
        world.name(id).to_string()
    }
}

fn whose(world: &World, id: NpcId) -> String {
    if id == PLAYER {
        "your".into()
    } else {
        format!("{}'s", world.name(id))
    }
}

pub fn suspect_label(world: &World, s: Suspect) -> String {
    match s {
        Suspect::Nature => "lightning".into(),
        Suspect::Accident => "an accident".into(),
        Suspect::Person(p) => who(world, p),
        Suspect::Nation(n) => format!("the {}", n.name()),
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

/// The Finches, the Pikes.
pub fn plural(surname: &str) -> String {
    if surname.ends_with("ch") || surname.ends_with('s') || surname.ends_with("sh") {
        format!("{surname}es")
    } else {
        format!("{surname}s")
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
            about,
            blamed,
            confidence,
            source,
            reason,
        } => {
            let how = match source {
                Source::Witnessed => "saw".to_string(),
                Source::Victim => "blames".into(),
                Source::Bystander => "suspects".into(),
                Source::Told(t) if !world.npc(t).alive => {
                    format!("heard from the ghost of {} and blames", who(world, t))
                }
                Source::Told(t) => format!("heard from {} and blames", who(world, t)),
                Source::Newspaper(p) => format!("read in the {} and blames", p.name()),
            };
            let stray = blamed == Suspect::Accident
                && matches!(world.events[about as usize].kind, EventKind::Theft { .. });
            format!(
                "[ATTRIBUTION] {} {} {} ({}%) — {}",
                who(world, holder),
                how,
                if stray {
                    "nobody".to_string()
                } else {
                    suspect_label(world, blamed)
                },
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
        EventKind::Perished { victim, cause } => {
            let how = match cause {
                Hardship::Hunger => "starved",
                Hardship::Cold => "froze",
                Hardship::Fever => "died of a fever",
            };
            if victim == PLAYER {
                format!("[DEATH] You {}", how)
            } else {
                format!("[DEATH] {} {}", who(world, victim), how)
            }
        }
        EventKind::Theft {
            thief,
            victim,
            loot,
        } => {
            let what = match loot {
                Loot::Cow => "A cow was taken",
                Loot::Grain => "A sack of grain was taken",
            };
            let place = if world.npc(victim).family == 0 {
                "your place".to_string()
            } else {
                format!("the {} place", world.family_of(victim).surname)
            };
            let truth = if omniscient || thief == PLAYER {
                format!(" [truth: {}]", who(world, thief))
            } else {
                String::new()
            };
            format!("[THEFT] {} from {}{}", what, place, truth)
        }
        EventKind::Desperation { family, act } => {
            let fam = capitalize(&family_label(world, family));
            let line = match act {
                Desperate::EatSeed { bushels } => {
                    format!(
                        "{} ate {} bushel{} of their seed corn",
                        fam,
                        bushels,
                        if bushels == 1 { "" } else { "s" }
                    )
                }
                Desperate::Slaughter => format!("{} butchered breeding stock", fam),
                Desperate::SlaughterOx => format!("{} butchered a plow ox", fam),
                Desperate::Borrow { lender, dollars } => {
                    format!(
                        "{} bought ${} of food on credit from {}",
                        fam,
                        dollars,
                        who(world, lender)
                    )
                }
                Desperate::Beg { neighbor, granted } => format!(
                    "{} begged {} for food{}",
                    fam,
                    who(world, neighbor),
                    if granted {
                        " and were fed"
                    } else {
                        " and were turned away"
                    }
                ),
            };
            format!("[HUNGER] {}", line)
        }
        EventKind::Wounded { victim, attacker } => {
            let by = if omniscient || attacker == PLAYER {
                format!(" [truth: {}]", who(world, attacker))
            } else {
                String::new()
            };
            format!(
                "[WOUNDED] {} was shot from the brush and lived{}",
                capitalize(&who(world, victim)),
                by
            )
        }
        EventKind::Cruelty { actor, victim, act } => {
            let truth = if omniscient {
                format!(" [truth: {}]", who(world, actor))
            } else {
                String::new()
            };
            let place = if world.npc(victim).family == 0 {
                "your place".to_string()
            } else {
                format!("the {} place", world.family_of(victim).surname)
            };
            match act {
                Cruelty::Slander { listener, .. } => format!(
                    "[SLANDER] Someone whispered to {} about {}{}",
                    who(world, listener),
                    who(world, victim),
                    truth
                ),
                Cruelty::KillStock => {
                    format!("[CRUELTY] A cow was found shot at {}{}", place, truth)
                }
                Cruelty::FoulWell => format!("[CRUELTY] The well at {} was fouled{}", place, truth),
                Cruelty::CutFence => {
                    format!("[CRUELTY] The fence at {} was down in the morning{}", place, truth)
                }
                Cruelty::SpoilHay => format!("[CRUELTY] The hay at {} was rotting{}", place, truth),
            }
        }
        EventKind::Trespass { family, nation } => format!(
            "[LAND] {} cut timber on {} land",
            capitalize(&family_label(world, family)),
            nation.name()
        ),
        EventKind::Annuity {
            nation,
            paid,
            docked,
        } => {
            if docked > 0 {
                format!(
                    "[TREATY] The {} annuity paid: ${}, less ${} docked for settler claims",
                    nation.name(),
                    paid,
                    docked
                )
            } else {
                format!("[TREATY] The {} annuity paid: ${}", nation.name(), paid)
            }
        }
        EventKind::Complaint { nation, acted } => format!(
            "[TREATY] The {} protest trespass to their agent{}",
            nation.name(),
            if acted {
                ". Squatters are ordered off"
            } else {
                ". Nothing is done"
            }
        ),
        EventKind::Plundered { nation } => format!(
            "[LAND] Armed riders crossed {} land and took stock and corn",
            nation.name()
        ),
        EventKind::Visit { nation, host, fed } => format!(
            "[VISIT] A hungry {} family came to {} door{}",
            nation.name(),
            whose(world, host),
            if fed {
                " and was fed"
            } else {
                " and was sent away"
            }
        ),
        EventKind::Adopted { person, nation } => format!(
            "[KIN] {} was taken in by the {}",
            capitalize(&who(world, person)),
            nation.name()
        ),
        EventKind::KawHunt { good } => {
            if good {
                "[BUFFALO] The Kaw are back from the fall hunt with meat and robes".to_string()
            } else {
                "[BUFFALO] The Kaw fall hunt found the herds thin. A hungry winter ahead"
                    .to_string()
            }
        }
        EventKind::BuffaloHunt { hunter, animals } => {
            if animals == 0 {
                format!(
                    "[BUFFALO] {} came back from the range with nothing",
                    capitalize(&who(world, hunter))
                )
            } else {
                format!(
                    "[BUFFALO] {} came back from the range with {} buffalo",
                    capitalize(&who(world, hunter)),
                    animals
                )
            }
        }
        EventKind::Brawl { venue, a, b } => format!(
            "[BRAWL] {} and {} came to blows at {}",
            capitalize(&who(world, a)),
            who(world, b),
            venue.name()
        ),
        EventKind::Blackmail {
            victim,
            extorter,
            paid,
        } => {
            if paid {
                format!(
                    "[SECRET] {} paid {} to keep quiet about Westport",
                    capitalize(&who(world, victim)),
                    who(world, extorter)
                )
            } else {
                format!(
                    "[SCANDAL] Word got around about what {} did in Westport",
                    who(world, victim)
                )
            }
        }
        EventKind::OathSworn {
            holder,
            target,
            over,
        } => format!(
            "[OATH] {}, {}, swore on {} grave to see {} dead",
            capitalize(&who(world, holder)),
            world.npc(holder).age,
            whose(world, over),
            who(world, target)
        ),
        EventKind::OathInherited { heir, target } => format!(
            "[OATH] The oath against {} passed to {}",
            who(world, target),
            who(world, heir)
        ),
        EventKind::OathWakes { holder, target } => format!(
            "[OATH] {} is {} now, and has not forgotten {}",
            capitalize(&who(world, holder)),
            world.npc(holder).age,
            who(world, target)
        ),
        EventKind::ShameCarried { holder, over } => format!(
            "[SHAME] {}, {}, buried {} and said nothing against the man who did it",
            capitalize(&who(world, holder)),
            world.npc(holder).age,
            who(world, over)
        ),
        EventKind::ShameWakes { holder, over, turn } => match turn {
            super::ghosts::ShameTurn::Atonement => format!(
                "[SHAME] {} is grown now, and tries to make right what {} did",
                capitalize(&who(world, holder)),
                who(world, over)
            ),
            super::ghosts::ShameTurn::Ruin => format!(
                "[SHAME] {} is grown now, and drinks to forget what {} did",
                capitalize(&who(world, holder)),
                who(world, over)
            ),
        },
        EventKind::Spared { actor, target, why } => format!(
            "[MERCY] {} had {} in reach and let it go ({})",
            capitalize(&who(world, actor)),
            who(world, target),
            match why {
                super::reconcile::Mercy::Children => "there were children at the window",
                super::reconcile::Mercy::Faith => "thou shalt not",
                super::reconcile::Mercy::Weariness => "too tired of it",
            }
        ),
        EventKind::BarnRaising {
            owner,
            hands,
            rival,
        } => format!(
            "[RAISING] {} neighbors raised the {} barn in a day{}",
            hands,
            surname(world, owner),
            rival
                .map(|r| format!(", and {} was among them", who(world, r)))
                .unwrap_or_default()
        ),
        EventKind::Apology { actor, to } => format!(
            "[APOLOGY] {} went to {} and owned what was done",
            capitalize(&who(world, actor)),
            who(world, to)
        ),
        EventKind::Condolence { visitor, mourner } => format!(
            "[CONDOLENCE] {} came, hat in hand, to the burying at the {} place",
            capitalize(&who(world, visitor)),
            surname(world, mourner)
        ),
        EventKind::FeudEnded { a, b, how } => format!(
            "[PEACE] The {} and {} feud is over ({})",
            world.families[a as usize].surname,
            world.families[b as usize].surname,
            match how {
                super::reconcile::Peace::Exhaustion => "nobody could remember why. It's all over now",
                super::reconcile::Peace::CommonEnemy => "they hate someone else more",
                super::reconcile::Peace::Calamity => "trouble did it",
                super::reconcile::Peace::Brokered => "someone talked them down",
                super::reconcile::Peace::Marriage => "a wedding made them kin",
            }
        ),
        EventKind::WhereWereYou { kin, errand, .. } => format!(
            "[HOME] {} wants to know why you were {} when it happened",
            capitalize(&who(world, kin)),
            errand.label()
        ),
        EventKind::Pastime { what, amount } => {
            use super::life::{Find, Pastime};
            match what {
                Pastime::Chores(task) => format!("[FARM] A day of {}", task.label()),
                Pastime::Fished => match amount {
                    0 => "[CREEK] Nothing biting on the Wakarusa".into(),
                    n => format!("[CREEK] You took {n} catfish out of the Wakarusa"),
                },
                Pastime::Roamed(Find::Nothing) => {
                    "[ROAM] You walked the timber and came home".into()
                }
                Pastime::Roamed(Find::BeeTree) => {
                    "[ROAM] You found a bee tree and cut it out".into()
                }
                Pastime::Roamed(Find::Nuts) => {
                    "[ROAM] Pecans and wild plums down by the creek".into()
                }
                Pastime::Roamed(Find::Neighbor(n)) => {
                    format!("[ROAM] You fell in with {} on the road", who(world, n))
                }
                Pastime::Drank => "[GROGGERY] You drank at the Jack of Hearts".into(),
                Pastime::Studied => "[STUDY] You sat up with the Book and the papers".into(),
                Pastime::Visited(n) => format!("[VISIT] You sat a spell with {}", who(world, n)),
                Pastime::Courted(n) => format!("[COURTING] You called on {}", who(world, n)),
                Pastime::Built(p) => format!("[WORK] You gave a day to the {}", p.label()),
                Pastime::Trapped { beaver } => match (amount, beaver) {
                    (0, 0) => "[TRAPLINE] You set your traps along the creek".into(),
                    (m, 0) => format!("[TRAPLINE] {m} muskrat on the line"),
                    (m, _) => format!("[TRAPLINE] {m} muskrat and a beaver"),
                },
                Pastime::Camped(sight) => {
                    use super::life::Sight;
                    match sight {
                        Sight::Stars => "[NIGHT] You slept out under the stars".into(),
                        Sight::Wisp => {
                            "[NIGHT] A light moved on the bottoms, and moved away when you did"
                                .into()
                        }
                        Sight::Spirit(s) => {
                            format!("[NIGHT] You saw {} where it happened", world.name(s))
                        }
                        Sight::Riders { rider, toward } => format!(
                            "[NIGHT] {} riding toward the {} place, late",
                            rider.map_or("Someone".to_string(), |r| world.name(r).to_string()),
                            world.families[toward as usize].surname
                        ),
                    }
                }
                Pastime::WroteHome => "[LETTER] You wrote home".into(),
                Pastime::Gathered(bee) => format!("[NEIGHBORS] You went to the {}", bee.label()),
            }
        }
        EventKind::Sermon { preacher, flock } => format!(
            "[SERMON] {} preached to {} at the Sunday meeting",
            capitalize(&who(world, preacher)),
            flock
        ),
        EventKind::Baptism {
            preacher,
            convert,
            cold,
        } => format!(
            "[BAPTISM] {} put {} under in the Wakarusa{}",
            capitalize(&who(world, preacher)),
            who(world, convert),
            if cold { ", through the ice" } else { "" }
        ),
        EventKind::Speech {
            speaker,
            calm,
            heard,
        } => format!(
            "[SPEECH] {} spoke to {} in Lawrence, {}",
            capitalize(&who(world, speaker)),
            heard,
            if calm {
                "for patience"
            } else {
                "and the room caught fire"
            }
        ),
        EventKind::Affair { a, b } => format!(
            "[SECRET] {} and {} are carrying on",
            capitalize(&who(world, a)),
            who(world, b)
        ),
        EventKind::Scandal { a, b, wronged } => format!(
            "[SCANDAL] {} and {} were seen together, tangled up. {} knows",
            capitalize(&who(world, a)),
            who(world, b),
            capitalize(&who(world, wronged))
        ),
        EventKind::Marriage { a, b } => format!(
            "[WEDDING] {} and {} were married",
            capitalize(&who(world, a)),
            who(world, b)
        ),
        EventKind::Built { project } => {
            format!(
                "[COUNTY] The {} is finished, raised by neighbors",
                project.label()
            )
        }
        EventKind::ClaimFiled { family, delayed } => format!(
            "[LAND OFFICE] The {} claim {} at Lecompton",
            world.families[family as usize].surname,
            if delayed {
                "papers were \"mislaid\""
            } else {
                "was filed"
            }
        ),
        EventKind::ClaimJumped {
            family,
            lost,
            neighbor,
        } => format!(
            "[CLAIM] {} jumped the {} claim{}",
            neighbor.map_or("A stranger".to_string(), |n| capitalize(&who(world, n))),
            world.families[family as usize].surname,
            if lost == 0 {
                " and was run off".to_string()
            } else {
                format!(", taking {lost} acres")
            }
        ),
        EventKind::Festival { holiday, crowd } => {
            format!("[{}] {} turned out", holiday.label().to_uppercase(), crowd)
        }
        EventKind::Strayed { owner } => format!(
            "[STOCK] A cow is gone from the {} place",
            surname(world, owner)
        ),
        EventKind::StockStarved { family, head } => format!(
            "[STOCK] The {} hay ran out; {} head died on the prairie",
            world.families[family as usize].surname, head
        ),
        EventKind::Letter {
            family,
            letter,
            reader,
        } => {
            use super::mail::Letter;
            let what = match letter {
                Letter::Money(d) => format!("${d} from home"),
                Letter::BadNews => "a death back home".into(),
                Letter::KinComing => "kin coming out in the spring".into(),
                Letter::Homesick => "asking when they'll come home".into(),
            };
            let read = reader
                .map(|r| format!(", read to them by {}", who(world, r)))
                .unwrap_or_default();
            if family == 0 {
                format!("[MAIL] A letter for you: {what}{read}")
            } else {
                format!(
                    "[MAIL] A letter for the {}: {what}{read}",
                    plural(world.families[family as usize].surname)
                )
            }
        }
        EventKind::KinArrived { family, newcomer } => match newcomer {
            Some(n) => format!(
                "[ARRIVAL] {} came out from the States to the {} place",
                world.name(n),
                world.families[family as usize].surname
            ),
            None => "[ARRIVAL] Kin were expected, and didn't come".into(),
        },
        EventKind::Lawsuit {
            plaintiff,
            defendant,
            judge,
            acres,
            won,
        } => format!(
            "[COURT] {} sued {} over {} acres before Justice {}, and {}",
            capitalize(&who(world, plaintiff)),
            who(world, defendant),
            acres,
            world.name(judge),
            if won { "won" } else { "lost" }
        ),
        EventKind::Election {
            index,
            free_state,
            pro_slavery,
            missourians,
        } => {
            let e = &super::law::ELECTIONS[index as usize];
            let mo = if missourians > 0 {
                format!(", {missourians} of them Missourians")
            } else {
                String::new()
            };
            let fraud = if e.fraud_rejected {
                ". The Governor threw out the fraudulent returns"
            } else {
                ""
            };
            format!(
                "[ELECTION] The vote on {}: {} Free-State, {} Pro-Slavery{}{}",
                e.name, free_state, pro_slavery, mo, fraud
            )
        }
        EventKind::VoteSold { seller } => format!(
            "[ELECTION] Word is {} sold a vote at the store",
            who(world, seller)
        ),
        EventKind::Muster {
            index,
            joined,
            dodged,
            wounded,
            ..
        } => format!(
            "[MUSTER] {}: {} rode out, {} stayed home{}",
            super::law::MUSTERS[index as usize].name,
            joined,
            dodged,
            if wounded > 0 {
                format!(", {wounded} came back shot")
            } else {
                String::new()
            }
        ),
        EventKind::BeeHeld { bee, host, crowd } => format!(
            "[NEIGHBORS] {} came to the {} {}",
            crowd,
            world.families[host as usize].surname,
            bee.label()
        ),
        EventKind::ChurchSplit => {
            "[MEETING] The union meeting has split, North and South. Old pew-mates pass without a word"
                .into()
        }
        EventKind::Notice {
            paper,
            about,
            praise,
        } => {
            let what = debug_line(world, &world.events[about as usize], false);
            let what = what.split_once("] ").map_or(what.clone(), |(_, r)| r.to_string());
            format!(
                "[PRESS] {} {} you: {}",
                paper.name(),
                if praise { "praises" } else { "damns" },
                what
            )
        }
        EventKind::Legacy {
            child,
            path,
            rebelled,
        } => format!(
            "[FAMILY] {} is near grown: {} the {}",
            world.name(child),
            if rebelled {
                "nothing like"
            } else {
                "the very image of"
            },
            path
        ),
        EventKind::Born { child, mother } => format!(
            "[BIRTH] {} was born to {}",
            world.name(child),
            who(world, mother)
        ),
        EventKind::ClaimBought {
            family,
            seller,
            price,
        } => format!(
            "[LAND] {} sold you the {} claim for ${} and took the family back to the States",
            capitalize(&who(world, seller)),
            world.families[family as usize].surname,
            price
        ),
        EventKind::SeekerAtDoor { seeker, family } => {
            let s = &world.railroad.seekers[seeker as usize];
            format!(
                "[NIGHT] A knock at {} after dark: {}, from {}",
                if family == 0 {
                    "your door".to_string()
                } else {
                    format!("the {} door", world.families[family as usize].surname)
                },
                s.name,
                s.from
            )
        }
        EventKind::Sheltered { seeker, family: 0 } => format!(
            "[RAILROAD] You hid {} in the loft: shelter from the storm",
            world.railroad.name(seeker)
        ),
        EventKind::Sheltered { seeker, family } => format!(
            "[RAILROAD] The {} hid {} in the loft",
            plural(world.families[family as usize].surname),
            world.railroad.name(seeker)
        ),
        EventKind::TurnedAway { seeker, family } => format!(
            "[RAILROAD] {} turned {} away",
            if family == 0 {
                "You".to_string()
            } else {
                format!("The {}", plural(world.families[family as usize].surname))
            },
            world.railroad.name(seeker)
        ),
        EventKind::Pursuers { seeker } => format!(
            "[PURSUIT] Men from {} rode through asking after {}",
            world.railroad.seekers[seeker as usize].from,
            world.railroad.name(seeker)
        ),
        EventKind::Captured {
            seeker,
            at,
            informer,
        } => {
            let place = at
                .map(|f| format!(" at the {} place", world.families[f as usize].surname))
                .unwrap_or_default();
            let told = match informer {
                Some(i) if omniscient || i == PLAYER => format!(" [told: {}]", who(world, i)),
                _ => String::new(),
            };
            format!(
                "[PURSUIT] {} was taken{} and carried back to Missouri{}",
                world.railroad.name(seeker),
                place,
                told
            )
        }
        EventKind::Freedom { seeker } => format!(
            "[RAILROAD] Word came back: {} reached free soil",
            world.railroad.name(seeker)
        ),
        EventKind::Charged {
            accused,
            convicted,
            fine,
        } => format!(
            "[COURT] {} charged with harboring a fugitive: {}",
            capitalize(&who(world, accused)),
            if convicted {
                format!("convicted, fined ${fine}")
            } else {
                "let go".to_string()
            }
        ),
        EventKind::Cornered { buyer, good, units } => format!(
            "[MARKET] Dunmore says {} bought him out: {} {} of {}",
            who(world, buyer),
            units,
            good.unit(),
            good.label()
        ),
        EventKind::Improved { family, what } => format!(
            "[CLAIM] {} put up a {}",
            if family == 0 {
                "You".to_string()
            } else {
                format!("The {}", plural(world.families[family as usize].surname))
            },
            what.label()
        ),
        EventKind::StandCut { family, .. } => format!(
            "[TIMBER] The stand nearest the {} place is stumps now",
            world.families[family as usize].surname
        ),
        EventKind::SpiritSeen { witness, spirit } => format!(
            "[SPIRIT] {} saw {} standing in the dark where it happened",
            capitalize(&who(world, witness)),
            if spirit == PLAYER {
                "you".to_string()
            } else {
                world.name(spirit).to_string()
            }
        ),
        EventKind::SpiritRests { spirit } => {
            if spirit == PLAYER {
                "[SPIRIT] You rest easy now".to_string()
            } else {
                format!("[SPIRIT] {} rests easy now", world.name(spirit))
            }
        }
        EventKind::History { index } => {
            format!("[NEWS] {}", super::history::TIMELINE[index].title)
        }
        EventKind::Headline {
            paper,
            about,
            blamed,
            history,
        } => {
            let free = paper.faction() == super::world::Faction::FreeState;
            let text = if let Some(i) = history {
                let m = &super::history::TIMELINE[i];
                (if free { m.free_state } else { m.pro_slavery }).to_string()
            } else if let Some(about) = about {
                let what = debug_line(world, &world.events[about as usize], false);
                let what = what
                    .split_once("] ")
                    .map(|(_, r)| r)
                    .unwrap_or(&what)
                    .to_string();
                let villain = match blamed {
                    Some(Suspect::Person(p)) => who(world, p),
                    _ if free => "border ruffians".to_string(),
                    _ => "abolitionists".to_string(),
                };
                format!("{}. The work of {}.", what, villain)
            } else {
                String::new()
            };
            format!("[PRESS] {}: \"{}\"", paper.name(), text)
        }
        EventKind::PriceMove {
            good,
            cents,
            rising,
        } => format!(
            "[MARKET] {} {} to ${}.{:02} a {} at Dunmore's",
            capitalize(good.label()),
            if rising { "up" } else { "down" },
            cents / 100,
            cents % 100,
            good.unit()
        ),
        EventKind::Harvest {
            family,
            planted,
            food,
        } => {
            let fam = capitalize(&family_label(world, family));
            if planted == 0 {
                format!("[HARVEST] {} had nothing in the ground", fam)
            } else {
                format!(
                    "[HARVEST] {} brought in {} acres: about {} days of food",
                    fam, planted, food
                )
            }
        }
        EventKind::Favor {
            creditor,
            debtor,
            complied,
        } => {
            if complied {
                format!(
                    "[DEBT] {} signed {}'s Law and Order petition to settle up",
                    capitalize(&who(world, debtor)),
                    who(world, creditor)
                )
            } else {
                format!(
                    "[DEBT] {} refused {}'s petition. The store seized their stock",
                    capitalize(&who(world, debtor)),
                    who(world, creditor)
                )
            }
        }
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

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().chain(c).collect(),
        None => String::new(),
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
        EventKind::Retaliation { .. } | EventKind::Spared { .. } => omniscient,
        EventKind::SeekerAtDoor { family, .. }
        | EventKind::Sheltered { family, .. }
        | EventKind::TurnedAway { family, .. } => omniscient || family == 0,
        EventKind::Affair { a, b } => omniscient || a == PLAYER || b == PLAYER,
        EventKind::Blackmail {
            paid: true,
            extorter,
            victim,
        } => omniscient || extorter == PLAYER || victim == PLAYER,
        EventKind::Cruelty {
            act: Cruelty::Slander { .. },
            ..
        } => omniscient,
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
