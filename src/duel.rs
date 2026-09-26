//! Face to face, and in the dark: the standoff at the gate and the ambush on
//! the road, played by hand. The sim sets the terms (his temper, his draw,
//! your nerve, the moon); your hands decide the rest, and the sim is told
//! what happened.
//!
//! - Talk him down: read what's driving him and answer it, three times,
//!   before the fuse burns.
//! - Face him down: hold his eye. Keep the needle in the calm.
//! - Draw: hands still until the moment. Early is a flinch; late is dead.
//!   Then put the ball where you mean it: high kills, low wounds.
//! - Ambush: riders cross your sights. Hold your breath (Shift) to steady.
//!   On a dark night you can't tell who's who; on a bright one, they can see
//!   you too.

use bevy::prelude::*;
use bleeding_kansas::sim::action::{self, AmbushPlan, DrawEnd, Mood, Shot, Turn};
use bleeding_kansas::sim::world::World;

use crate::scenery::Art;
use crate::ui::{EDGE, LIGHT, Overlay, Toast};
use crate::{Fonts, OXBLOOD, Sim, text};

const STAGE: Vec2 = Vec2::new(800.0, 380.0);
const STAGE_TOP: f32 = 150.0;
/// The man at the gate, in stage pixels.
const MAN: Rect = Rect {
    min: Vec2::new(520.0, 120.0),
    max: Vec2::new(610.0, 270.0),
};
const RIDER: Vec2 = Vec2::new(150.0, 131.0);
const ROAD_Y: f32 = 170.0;
const TALK_SECONDS: f32 = 7.0;
const STARE_SECONDS: f32 = 5.0;
const BREATH: f32 = 2.2;

/// How you mean to meet him.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Approach {
    Talk,
    Face,
    Draw,
}

#[derive(Default)]
pub enum Phase {
    #[default]
    Idle,
    Talk {
        round: u8,
        right: u8,
        seq: [Mood; 3],
        order: [Mood; 4],
        left: f32,
        last: Option<bool>,
    },
    Stare {
        t: f32,
        needle: f32,
        vel: f32,
        inside: f32,
        zone: f32,
        push: f32,
        kick: f32,
    },
    Draw {
        t: f32,
        cue: f32,
        theirs: f32,
    },
    Aim {
        t: f32,
        window: f32,
        sway: f32,
        reaction: f32,
        theirs: f32,
    },
    Ambush {
        plan: AmbushPlan,
        t: f32,
        breath: f32,
        /// The man you came for rides in front.
        target_leads: bool,
        sway: f32,
    },
    After {
        text: String,
        left: f32,
    },
}

#[derive(Resource, Default)]
pub struct Game {
    pub phase: Phase,
    /// View-side dice for sway and the moment of the draw. Never the sim's.
    seed: u64,
    /// Where the pointer is, in stage pixels.
    aim: Vec2,
    flash: f32,
    /// Holding your breath (ambush).
    holding: bool,
    /// Who's at the gate, kept after the sim has closed the standoff.
    cast: Option<(
        bleeding_kansas::sim::NpcId,
        Vec<bleeding_kansas::sim::NpcId>,
        bool,
    )>,
}

impl Game {
    pub fn active(&self) -> bool {
        !matches!(self.phase, Phase::Idle)
    }

    fn roll(&mut self) -> f32 {
        // xorshift: the view's own dice.
        let mut x = self.seed.max(1);
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.seed = x;
        (x >> 11) as f32 / (1u64 << 53) as f32
    }

    pub fn start(&mut self, world: &World, how: Approach, now: f64) {
        let Some(s) = world.action.standoff.clone() else {
            return;
        };
        self.seed = (now * 1e6) as u64 ^ 0x9E37_79B9_7F4A_7C15;
        self.cast = Some((s.actor, s.riders.clone(), s.yours));
        self.phase = match how {
            Approach::Talk => {
                let m = action::moods(world, s.actor);
                let order = self.shuffle();
                Phase::Talk {
                    round: 0,
                    right: 0,
                    seq: [m[0], m[1], m[0]],
                    order,
                    left: TALK_SECONDS,
                    last: None,
                }
            }
            Approach::Face => Phase::Stare {
                t: 0.0,
                needle: 0.0,
                vel: 0.0,
                inside: 0.0,
                zone: action::nerve(world) * 2.0,
                push: action::pressure(world, &s),
                kick: 0.0,
            },
            Approach::Draw => self.draw_phase(world),
        };
    }

    fn draw_phase(&mut self, world: &World) -> Phase {
        let theirs = world
            .action
            .standoff
            .as_ref()
            .map_or(0.5, |s| action::their_draw(world, s.actor));
        Phase::Draw {
            t: 0.0,
            cue: 1.4 + 2.2 * self.roll(),
            theirs,
        }
    }

    pub fn ambush(&mut self, world: &World, plan: AmbushPlan, now: f64) {
        self.seed = (now * 1e6) as u64 ^ 0xD1B5_4A32_D192_ED03;
        self.cast = None;
        let target_leads = plan.companion.is_none() || self.roll() < 0.5;
        self.phase = Phase::Ambush {
            plan,
            t: 0.0,
            breath: BREATH,
            target_leads,
            sway: action::sway(world),
        };
    }

    fn shuffle(&mut self) -> [Mood; 4] {
        let mut o = Mood::ALL;
        for i in (1..4).rev() {
            let j = (self.roll() * (i + 1) as f32) as usize;
            o.swap(i, j.min(i));
        }
        o
    }
}

fn after(text: impl Into<String>) -> Phase {
    Phase::After {
        text: text.into(),
        left: 3.2,
    }
}

fn tell(m: Mood, round: u8) -> &'static str {
    let odd = round % 2 == 1;
    match (m, odd) {
        (Mood::Grief, false) => "His hat's in his hands, turning. His eyes are red at the rims.",
        (Mood::Grief, true) => "He keeps looking past you, up the rise where the graves are.",
        (Mood::Rage, false) => "His jaw is working. The horse feels it and won't stand still.",
        (Mood::Rage, true) => "He spits, and says your name like it owes him money.",
        (Mood::Fear, false) => {
            "He keeps glancing at your windows, counting rifles that aren't there."
        }
        (Mood::Fear, true) => {
            "His voice is too loud for the distance. The rein shakes in his hand."
        }
        (Mood::Greed, false) => "He looks at your barn the way a man looks at a price.",
        (Mood::Greed, true) => "He mentions the winter, and your corn, and the winter again.",
    }
}

fn appeal(m: Mood) -> &'static str {
    match m {
        Mood::Grief => "\"I'm sorry for what you lost. I'd have stood at that grave with you.\"",
        Mood::Rage => "\"Then say it plain, in daylight, like a man. Not with a torch.\"",
        Mood::Fear => "\"Nobody in this house is coming for yours. You have my word.\"",
        Mood::Greed => "\"There's corn in my crib. Take a sack and call it square.\"",
    }
}

// ---- the frame ------------------------------------------------------------

/// Every piece of the frame, by role.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    Root,
    Title,
    Body,
    Stage,
    /// The ground under the horizon: a yard, or a road at night.
    Ground,
    Fig(usize),
    Name(usize),
    Bar,
    Zone,
    Needle,
    Fuse,
    Big,
    Reticle,
    Flash,
    Option(usize),
    OptionText(usize),
}

fn abs(left: f32, top: f32, w: f32, h: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(left),
        top: Val::Px(top),
        width: Val::Px(w),
        height: Val::Px(h),
        ..default()
    }
}

pub fn setup(mut commands: Commands, fonts: Res<Fonts>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.01, 0.02, 0.9)),
            GlobalZIndex(18),
            Act::Root,
        ))
        .with_children(|r| {
            r.spawn((
                Text::new(""),
                text(&fonts.display, 34.0, LIGHT),
                Node {
                    margin: UiRect::top(Val::Px(40.0)),
                    ..default()
                },
                Act::Title,
            ));
            r.spawn((
                Text::new(""),
                text(&fonts.body, 17.0, LIGHT.with_alpha(0.9)),
                TextLayout::new_with_justify(Justify::Center),
                Node {
                    max_width: Val::Px(820.0),
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
                Act::Body,
            ));
            // The stage: where the men stand and the sights wander.
            r.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(STAGE_TOP),
                    width: Val::Px(STAGE.x),
                    height: Val::Px(STAGE.y),
                    border: UiRect::all(Val::Px(1.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.07, 0.07, 0.08)),
                BorderColor::all(EDGE.with_alpha(0.3)),
                Act::Stage,
            ))
            .with_children(|s| {
                s.spawn((
                    abs(0.0, 250.0, STAGE.x, STAGE.y - 250.0),
                    BackgroundColor(Color::srgb(0.16, 0.13, 0.10)),
                    Act::Ground,
                ));
                for i in 0..3 {
                    s.spawn((
                        ImageNode::default(),
                        abs(0.0, 0.0, 10.0, 10.0),
                        UiTransform::default(),
                        Act::Fig(i),
                    ));
                    s.spawn((
                        Text::new(""),
                        text(&fonts.body, 14.0, LIGHT),
                        Node {
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        Act::Name(i),
                    ));
                }
                s.spawn((
                    abs(100.0, 330.0, 600.0, 18.0),
                    BackgroundColor(Color::srgb(0.25, 0.08, 0.06)),
                    Act::Bar,
                ))
                .with_children(|b| {
                    b.spawn((
                        abs(250.0, 0.0, 100.0, 18.0),
                        BackgroundColor(Color::srgb(0.30, 0.42, 0.28)),
                        Act::Zone,
                    ));
                    b.spawn((
                        abs(298.0, -6.0, 4.0, 30.0),
                        BackgroundColor(LIGHT),
                        Act::Needle,
                    ));
                });
                s.spawn((
                    abs(0.0, 0.0, STAGE.x, 6.0),
                    BackgroundColor(OXBLOOD),
                    Act::Fuse,
                ));
                s.spawn((
                    Text::new(""),
                    text(&fonts.display, 72.0, OXBLOOD),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(60.0),
                        top: Val::Px(40.0),
                        ..default()
                    },
                    Act::Big,
                ));
                s.spawn((
                    abs(0.0, 0.0, STAGE.x, STAGE.y),
                    BackgroundColor(Color::srgba(1.0, 0.9, 0.7, 0.0)),
                    Act::Flash,
                ));
                s.spawn((
                    Node {
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(18.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..abs(0.0, 0.0, 36.0, 36.0)
                    },
                    BorderColor::all(Color::srgb(0.95, 0.85, 0.55)),
                    Act::Reticle,
                ))
                .with_child((
                    Node {
                        width: Val::Px(4.0),
                        height: Val::Px(4.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.95, 0.85, 0.55)),
                ));
            });
            // The appeals, under the stage.
            r.spawn(Node {
                position_type: PositionType::Absolute,
                top: Val::Px(STAGE_TOP + STAGE.y + 14.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|o| {
                for i in 0..4 {
                    o.spawn((Button, Node::default(), Act::Option(i)))
                        .with_child((
                            Text::new(""),
                            text(&fonts.body, 17.0, LIGHT),
                            Act::OptionText(i),
                        ));
                }
            });
        });
}

/// Where the pointer is on the stage.
fn pointer(windows: &Query<&Window>) -> Option<Vec2> {
    let w = windows.single().ok()?;
    let c = w.cursor_position()?;
    let origin = Vec2::new((w.width() - STAGE.x) / 2.0, STAGE_TOP);
    Some(c - origin)
}

fn wander(t: f32, amp: f32) -> Vec2 {
    Vec2::new(
        (1.7 * t).sin() + 0.5 * (4.3 * t + 1.0).sin(),
        (1.3 * t).cos() + 0.4 * (3.1 * t).sin(),
    ) * amp
}

/// Where the ball lands on the man at the gate.
fn shot_at_man(p: Vec2) -> Shot {
    if !MAN.contains(p) {
        return Shot::Miss;
    }
    let h = MAN.height();
    let (x, y) = (p.x - MAN.min.x, p.y - MAN.min.y);
    let central = x > MAN.width() * 0.15 && x < MAN.width() * 0.85;
    if central && y < h * 0.45 {
        Shot::Kill
    } else {
        Shot::Wound
    }
}

/// Where a rider's man sits in his sprite.
fn shot_at_rider(p: Vec2, at: Vec2) -> Shot {
    let local = p - at;
    let man = Rect::new(52.0, 0.0, 80.0, 52.0);
    if !man.contains(local) {
        return Shot::Miss;
    }
    if local.y < 30.0 {
        Shot::Kill
    } else {
        Shot::Wound
    }
}

/// Hoofbeats before horses.
const LEAD_IN: f32 = 1.5;

fn rider_x(t: f32, pace: f32, behind: bool) -> f32 {
    let t = (t - LEAD_IN).max(0.0);
    let x = -RIDER.x + (STAGE.x + RIDER.x * 2.5) * (t / pace);
    if behind { x - 170.0 } else { x }
}

/// The hands: every frame of every game.
pub fn play(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    options: Query<(&Interaction, &Act), Changed<Interaction>>,
    mut game: ResMut<Game>,
    mut sim: ResMut<Sim>,
    mut overlay: ResMut<Overlay>,
    mut toast: ResMut<Toast>,
) {
    let dt = time.delta_secs();
    game.flash = (game.flash - dt * 3.0).max(0.0);
    if !game.active() {
        if *overlay == Overlay::Action {
            *overlay = Overlay::None;
        }
        return;
    }
    *overlay = Overlay::Action;
    if let Some(p) = pointer(&windows) {
        game.aim = p;
    }
    let fire = mouse.just_pressed(MouseButton::Left) || keys.just_pressed(KeyCode::Space);
    let world = &mut sim.0;
    let woman = world
        .action
        .standoff
        .as_ref()
        .is_some_and(|s| bleeding_kansas::sim::world::is_woman(world.name(s.actor)));
    let phase = std::mem::take(&mut game.phase);
    game.phase = match phase {
        Phase::Idle => Phase::Idle,
        Phase::Talk {
            round,
            mut right,
            seq,
            order,
            left,
            last,
        } => {
            let mut pick = [
                KeyCode::Digit1,
                KeyCode::Digit2,
                KeyCode::Digit3,
                KeyCode::Digit4,
            ]
            .iter()
            .position(|k| keys.just_pressed(*k));
            for (i, o) in &options {
                if let (Interaction::Pressed, Act::Option(k)) = (i, o) {
                    pick = Some(*k);
                }
            }
            let left = left - dt;
            if pick.is_some() || left <= 0.0 {
                let hit = pick.is_some_and(|i| order[i] == seq[round as usize]);
                right += hit as u8;
                if round + 1 >= 3 {
                    match action::talk(world, right, 3) {
                        Turn::Settled => after(format!(
                            "{right} of 3 landed. He looks at you a long moment, then turns the horse."
                        )),
                        Turn::Draw => {
                            toast.say("Words run out. His hand drops to his belt.");
                            game.draw_phase(world)
                        }
                    }
                } else {
                    let order = game.shuffle();
                    Phase::Talk {
                        round: round + 1,
                        right,
                        seq,
                        order,
                        left: TALK_SECONDS,
                        last: Some(hit),
                    }
                }
            } else {
                Phase::Talk {
                    round,
                    right,
                    seq,
                    order,
                    left,
                    last,
                }
            }
        }
        Phase::Stare {
            t,
            mut needle,
            mut vel,
            mut inside,
            zone,
            push,
            mut kick,
        } => {
            let t = t + dt;
            // He leans on you in shoves; you lean back.
            kick -= dt;
            if kick <= 0.0 {
                kick = 0.25 + 0.3 * game.roll();
                let dir = if game.roll() < 0.5 { -1.0 } else { 1.0 };
                vel += dir * push * (0.5 + game.roll()) * 0.9;
            }
            let mut input = 0.0;
            if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
                input -= 1.0;
            }
            if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
                input += 1.0;
            }
            vel += input * 4.2 * dt;
            vel *= 1.0 - 1.8 * dt;
            needle = (needle + vel * dt).clamp(-1.0, 1.0);
            if needle.abs() >= 1.0 {
                vel = -vel * 0.3;
            }
            if needle.abs() < zone {
                inside += dt;
            }
            if t >= STARE_SECONDS {
                let grade = inside / STARE_SECONDS;
                match action::face(world, grade) {
                    Turn::Settled => after(format!(
                        "You held {:.0}% of it. He blinks first. The horse backs, and then he's gone.",
                        grade * 100.0
                    )),
                    Turn::Draw => {
                        toast.say(format!(
                            "You held {:.0}% of it. Not enough. He calls it.",
                            grade * 100.0
                        ));
                        game.draw_phase(world)
                    }
                }
            } else {
                Phase::Stare {
                    t,
                    needle,
                    vel,
                    inside,
                    zone,
                    push,
                    kick,
                }
            }
        }
        Phase::Draw { t, cue, theirs } => {
            let t = t + dt;
            if fire && t < cue {
                action::draw(world, DrawEnd::Flinched);
                game.flash = 1.0;
                after("You moved first, and too soon. He didn't wait for a second chance.")
            } else if fire {
                let reaction = t - cue;
                Phase::Aim {
                    t: 0.0,
                    // What you beat him by, and a breath more.
                    window: (theirs - reaction).max(0.0) + 0.35,
                    sway: action::sway(world),
                    reaction,
                    theirs,
                }
            } else if t - cue > theirs {
                action::draw(world, DrawEnd::Slow);
                game.flash = 1.0;
                after(format!(
                    "His gun was out in {theirs:.2}s. Yours was still in the leather."
                ))
            } else {
                Phase::Draw { t, cue, theirs }
            }
        }
        Phase::Aim {
            t,
            window,
            sway,
            reaction,
            theirs,
        } => {
            let t = t + dt;
            if fire {
                let at = game.aim + wander(t * 2.2, 26.0 * sway);
                let shot = shot_at_man(at);
                action::draw(world, DrawEnd::Fired(shot));
                game.flash = 1.0;
                let clock =
                    format!("You cleared leather in {reaction:.2}s; his hand was {theirs:.2}s.");
                after(match shot {
                    Shot::Kill => {
                        format!("{clock} The ball took him high. He's down and not getting up.")
                    }
                    Shot::Wound => {
                        format!("{clock} The ball took him low. He's down, cursing, alive.")
                    }
                    Shot::Miss => format!("{clock} You missed. He didn't."),
                })
            } else if t > window {
                action::draw(world, DrawEnd::Slow);
                game.flash = 1.0;
                after("You had him, and you held too long.")
            } else {
                Phase::Aim {
                    t,
                    window,
                    sway,
                    reaction,
                    theirs,
                }
            }
        }
        Phase::Ambush {
            plan,
            t,
            mut breath,
            target_leads,
            sway,
        } => {
            let t = t + dt;
            let holding = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
            game.holding = holding;
            if holding && breath > 0.0 {
                breath -= dt;
            } else if !holding {
                breath = (breath + dt * 0.6).min(BREATH);
            }
            if fire {
                let amp = ambush_amp(sway, holding, breath);
                let at = game.aim + wander(t * 1.6, amp);
                let mut hit = None;
                let riders = [
                    (plan.target, !target_leads),
                    (plan.companion.unwrap_or(plan.target), target_leads),
                ];
                for (who, behind) in riders.iter().take(1 + plan.companion.is_some() as usize) {
                    let pos = Vec2::new(rider_x(t, plan.pace, *behind), ROAD_Y);
                    let s = shot_at_rider(at, pos);
                    if s != Shot::Miss {
                        hit = Some((*who, s));
                    }
                }
                let fired = hit.unwrap_or((plan.target, Shot::Miss));
                action::ambush(world, &plan, Some(fired));
                game.flash = 1.0;
                let name = world.name(fired.0).to_string();
                after(match fired.1 {
                    Shot::Kill if fired.0 == plan.target => {
                        format!("The flash, the horse screaming, and {name} in the road.")
                    }
                    Shot::Kill => format!(
                        "The flash, and a man in the road. It's {name}. Not the one you came for."
                    ),
                    Shot::Wound => format!("{name} is hit and hanging on, spurring for home."),
                    Shot::Miss => "The ball went into the dark. So did they, fast, and now they know someone's out here.".to_string(),
                })
            } else if rider_x(t, plan.pace, true) > STAGE.x + 20.0 {
                action::ambush(world, &plan, None);
                after("They ride on by. The road's quiet again. So are you.")
            } else {
                Phase::Ambush {
                    plan,
                    t,
                    breath,
                    target_leads,
                    sway,
                }
            }
        }
        Phase::After { text, left } => {
            let left = left - dt;
            if left <= 0.0 || (left < 2.6 && (fire || keys.just_pressed(KeyCode::Enter))) {
                toast.say(text);
                toast.text = crate::panels::gendered(&toast.text, woman);
                Phase::Idle
            } else {
                Phase::After { text, left }
            }
        }
    };
    if !game.active() {
        *overlay = Overlay::None;
    }
}

fn ambush_amp(sway: f32, holding: bool, breath: f32) -> f32 {
    let base = 34.0 * sway;
    if holding && breath > 0.0 {
        base * 0.3
    } else if breath <= 0.05 {
        base * 1.7
    } else {
        base
    }
}

fn tint(world: &World, id: bleeding_kansas::sim::NpcId) -> Color {
    match world.npc(id).faction {
        bleeding_kansas::sim::Faction::FreeState => Color::srgb(0.93, 0.89, 0.80),
        bleeding_kansas::sim::Faction::ProSlavery => Color::srgb(0.95, 0.78, 0.70),
    }
}

/// Paint whatever game is on.
pub fn draw(
    game: Res<Game>,
    sim: Res<Sim>,
    art: Res<Art>,
    windows: Query<&Window>,
    mut parts: Query<(
        &Act,
        &mut Node,
        Option<&mut ImageNode>,
        Option<&mut Text>,
        Option<&mut BackgroundColor>,
        Option<&mut UiTransform>,
    )>,
) {
    if !game.active() {
        for (a, mut node, ..) in &mut parts {
            if *a == Act::Root {
                node.display = Display::None;
            }
        }
        return;
    }
    let world = &sim.0;
    let actor = game.cast.as_ref().map(|c| c.0);
    let riders: Vec<_> = game.cast.as_ref().map(|c| c.1.clone()).unwrap_or_default();
    let place = match &game.cast {
        Some((a, _, true)) => format!(
            "At the {} door",
            world.families[world.npc(*a).family as usize].surname
        ),
        _ => "At your gate".to_string(),
    };

    let (mut title, mut body, mut big) = (String::new(), String::new(), String::new());
    let mut show_bar = false;
    let mut show_reticle = false;
    let mut fuse = 0.0;
    let mut opts: Vec<String> = Vec::new();
    // (image, stage position, size, tint, label, down)
    let mut people: Vec<(Handle<Image>, Vec2, Vec2, Color, String, bool)> = Vec::new();
    let mut sky = Color::srgb(0.10, 0.11, 0.15);
    let mut ground = Color::srgb(0.17, 0.14, 0.10);
    let mut horizon = 250.0;
    let mut needle_x = 0.0;
    let mut zone_w = 0.0;
    let mut sights = game.aim;

    let gate_men = |shake: f32| {
        let mut v = Vec::new();
        if let Some(a) = actor {
            let n = world.npc(a);
            let down = !n.alive || (n.wounded && matches!(game.phase, Phase::After { .. }));
            v.push((
                art.gunman.clone(),
                if down {
                    MAN.min + Vec2::new(40.0, 70.0)
                } else {
                    MAN.min + Vec2::new(shake, 0.0)
                },
                MAN.size(),
                if down {
                    Color::srgb(0.55, 0.25, 0.2)
                } else {
                    tint(world, a)
                },
                world.name(a).to_string(),
                down,
            ));
        }
        for (i, r) in riders.iter().enumerate() {
            v.push((
                art.gunman.clone(),
                Vec2::new(if i == 0 { 400.0 } else { 660.0 }, 150.0),
                Vec2::new(72.0, 120.0),
                tint(world, *r).with_alpha(0.8),
                world.name(*r).to_string(),
                false,
            ));
        }
        v
    };

    match &game.phase {
        Phase::Idle => {}
        Phase::Talk {
            round,
            seq,
            order,
            left,
            last,
            ..
        } => {
            title = format!("{place}: talk him down");
            body = format!(
                "{}{}\nRead him. Answer what's driving him. (1-4, or click)",
                match last {
                    Some(true) => "That landed. ",
                    Some(false) => "That missed. ",
                    None => "",
                },
                tell(seq[*round as usize], *round)
            );
            fuse = left / TALK_SECONDS;
            opts = order
                .iter()
                .enumerate()
                .map(|(i, m)| format!("{}  {}", i + 1, appeal(*m)))
                .collect();
            people = gate_men(0.0);
            big = format!("{}/3", round + 1);
        }
        Phase::Stare {
            t, needle, zone, ..
        } => {
            title = format!("{place}: face him down");
            body = "Hold his eye. A / D to steady yourself; keep the needle in the green.".into();
            show_bar = true;
            needle_x = *needle;
            zone_w = *zone;
            fuse = 1.0 - t / STARE_SECONDS;
            people = gate_men(needle * 14.0);
        }
        Phase::Draw { t, cue, .. } => {
            title = format!("{place}: draw");
            body =
                "Hands still. Wait for it. Space or click the instant it comes; not before.".into();
            if t >= cue {
                big = "DRAW".into();
            }
            people = gate_men(0.0);
        }
        Phase::Aim {
            t, window, sway, ..
        } => {
            title = format!("{place}: fire");
            body = "Put it where you mean it. High kills. Low wounds.".into();
            show_reticle = true;
            fuse = (1.0 - t / window).max(0.0);
            sights = game.aim + wander(*t * 2.2, 26.0 * sway);
            people = gate_men(0.0);
        }
        Phase::Ambush {
            plan,
            t,
            breath,
            target_leads,
            sway,
        } => {
            let seen_faces = plan.moon > 0.55;
            title = "Lying in wait".into();
            body = format!(
                "{} Lead them. Shift to hold your breath. Click to fire, once.",
                match (plan.companion.is_some(), seen_faces) {
                    (true, true) => {
                        "Two riders, and the moon's good enough to know which is which."
                    }
                    (true, false) => "Two riders in the dark. One of them is the man you came for.",
                    (false, true) => "One rider, clear in the moonlight. Clear both ways.",
                    (false, false) => "One rider, a shape against a darker shape.",
                }
            );
            let b = 0.10 + 0.85 * plan.moon;
            sky = Color::srgb(0.02 + 0.07 * b, 0.03 + 0.08 * b, 0.06 + 0.12 * b);
            ground = Color::srgb(0.04 + 0.08 * b, 0.04 + 0.07 * b, 0.03 + 0.05 * b);
            horizon = ROAD_Y + RIDER.y - 12.0;
            fuse = breath / BREATH;
            show_reticle = true;
            sights = game.aim + wander(*t * 1.6, ambush_amp(*sway, game.holding, *breath));
            let mut add = |who, behind| {
                people.push((
                    art.rider.clone(),
                    Vec2::new(rider_x(*t, plan.pace, behind), ROAD_Y),
                    RIDER,
                    Color::srgb(b, b, b * 0.95),
                    if seen_faces {
                        world.name(who).to_string()
                    } else {
                        String::new()
                    },
                    false,
                ));
            };
            add(plan.target, !*target_leads);
            if let Some(c) = plan.companion {
                add(c, *target_leads);
            }
        }
        Phase::After { text, .. } => {
            title = "After".into();
            body = text.clone();
            people = gate_men(0.0);
        }
    }

    let woman = actor.is_some_and(|a| bleeding_kansas::sim::world::is_woman(world.name(a)));
    let g = |s: String| crate::panels::gendered(&s, woman);
    let (title, body, opts) = (
        g(title),
        g(body),
        opts.into_iter().map(g).collect::<Vec<_>>(),
    );
    let win_w = windows.single().map(|w| w.width()).unwrap_or(1280.0);
    for (a, mut node, img, text, bg, ui_t) in &mut parts {
        let set_text = |s: &str| {
            if let Some(mut t) = text
                && **t != *s
            {
                **t = crate::panels::plain(s);
            }
        };
        match *a {
            Act::Root => node.display = Display::Flex,
            Act::Title => set_text(&title),
            Act::Body => set_text(&body),
            Act::Big => set_text(&big),
            Act::OptionText(i) => set_text(opts.get(i).map_or("", |s| s.as_str())),
            Act::Option(_) => {}
            Act::Stage => {
                node.left = Val::Px((win_w - STAGE.x) / 2.0);
                if let Some(mut bg) = bg {
                    bg.0 = sky;
                }
            }
            Act::Ground => {
                node.top = Val::Px(horizon);
                node.height = Val::Px(STAGE.y - horizon);
                if let Some(mut bg) = bg {
                    bg.0 = ground;
                }
            }
            Act::Flash => {
                if let Some(mut bg) = bg {
                    bg.0 = Color::srgba(1.0, 0.92, 0.7, 0.8 * game.flash);
                }
            }
            Act::Fig(i) => match (people.get(i), img) {
                (Some((h, at, size, tint, _, down)), Some(mut img)) => {
                    node.display = Display::Flex;
                    // The fallen lie down.
                    if let Some(mut u) = ui_t {
                        u.rotation = if *down {
                            Rot2::degrees(-90.0)
                        } else {
                            Rot2::IDENTITY
                        };
                    }
                    img.image = h.clone();
                    img.color = *tint;
                    node.left = Val::Px(at.x);
                    node.top = Val::Px(at.y);
                    node.width = Val::Px(size.x);
                    node.height = Val::Px(size.y);
                }
                _ => node.display = Display::None,
            },
            Act::Name(i) => match people.get(i) {
                Some((_, at, size, _, label, _)) if !label.is_empty() => {
                    node.display = Display::Flex;
                    node.left = Val::Px(at.x + size.x * 0.25);
                    node.top = Val::Px(at.y - 22.0);
                    set_text(label);
                }
                _ => node.display = Display::None,
            },
            Act::Bar => {
                node.display = if show_bar {
                    Display::Flex
                } else {
                    Display::None
                }
            }
            Act::Zone => {
                node.width = Val::Px(zone_w * 300.0 * 2.0);
                node.left = Val::Px(300.0 - zone_w * 300.0);
            }
            Act::Needle => node.left = Val::Px(298.0 + needle_x * 300.0),
            Act::Fuse => node.width = Val::Px(STAGE.x * fuse.clamp(0.0, 1.0)),
            Act::Reticle => {
                node.display = if show_reticle {
                    Display::Flex
                } else {
                    Display::None
                };
                node.left = Val::Px(sights.x - 18.0);
                node.top = Val::Px(sights.y - 18.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_kills_low_wounds_wide_misses() {
        let c = MAN.center().x;
        assert_eq!(shot_at_man(Vec2::new(c, MAN.min.y + 20.0)), Shot::Kill);
        assert_eq!(shot_at_man(Vec2::new(c, MAN.max.y - 10.0)), Shot::Wound);
        assert_eq!(
            shot_at_man(Vec2::new(MAN.min.x - 5.0, MAN.center().y)),
            Shot::Miss
        );
    }

    #[test]
    fn a_rider_is_hit_in_the_man_not_the_horse() {
        let at = Vec2::new(100.0, ROAD_Y);
        assert_eq!(shot_at_rider(at + Vec2::new(66.0, 10.0), at), Shot::Kill);
        assert_eq!(shot_at_rider(at + Vec2::new(66.0, 45.0), at), Shot::Wound);
        // The horse takes it.
        assert_eq!(shot_at_rider(at + Vec2::new(60.0, 90.0), at), Shot::Miss);
    }

    #[test]
    fn riders_come_after_the_hoofbeats() {
        assert!(rider_x(0.5, 5.0, false) < 0.0);
        assert!(rider_x(LEAD_IN + 5.0, 5.0, true) > STAGE.x);
    }

    #[test]
    fn she_is_she() {
        assert_eq!(
            crate::panels::gendered("He spits, and says his piece. Talk him down.", true),
            "She spits, and says her piece. Talk her down."
        );
        assert_eq!(crate::panels::gendered("Heather", true), "Heather");
    }
}
