//! Windows in the old blue-box style: a command list with a pointer, a toast
//! that says what happened, a thin bar across the top, a status screen on
//! Tab and the paper on N. Everything here pauses the clock while it's open.

use bevy::prelude::*;
use bleeding_kansas::sim::chronicle;
use bleeding_kansas::sim::world::Faction;

use crate::cmds::{self, MenuSpec};
use crate::panels::{self, plain};
use crate::{Clock, Fonts, Screen, Sim, TownId, text};

pub const WINDOW_TOP: Color = Color::srgb(0.12, 0.18, 0.46);
pub const WINDOW_BOTTOM: Color = Color::srgb(0.03, 0.05, 0.18);
pub const EDGE: Color = Color::srgb(0.90, 0.88, 0.80);
pub const LIGHT: Color = Color::srgb(0.95, 0.94, 0.90);
pub const GREY: Color = Color::srgb(0.55, 0.56, 0.62);
const NEWSPRINT: Color = Color::srgb(0.87, 0.83, 0.72);
const INK: Color = Color::srgb(0.10, 0.08, 0.06);

/// The open command window, if any.
#[derive(Resource, Default)]
pub struct Menu {
    pub spec: Option<MenuSpec>,
    pub cursor: usize,
    /// Set when the window needs redrawing.
    dirty: bool,
    /// Scenes put their window low and centered.
    pub scene: bool,
}

impl Menu {
    pub fn open(&mut self, spec: MenuSpec) {
        self.cursor = spec.items.iter().position(|i| i.enabled).unwrap_or(0);
        self.spec = Some(spec);
        self.dirty = true;
        self.scene = false;
    }

    pub fn open_scene(&mut self, spec: MenuSpec) {
        self.open(spec);
        self.scene = true;
    }

    pub fn close(&mut self) {
        self.spec = None;
        self.dirty = true;
        self.scene = false;
    }

    pub fn is_open(&self) -> bool {
        self.spec.is_some()
    }
}

#[derive(Resource, Default)]
pub struct Toast {
    pub text: String,
    pub left: f32,
}

impl Toast {
    pub fn say(&mut self, s: impl Into<String>) {
        self.text = s.into();
        self.left = 4.5;
    }
}

/// Full-screen overlays: the status screen and the paper.
#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub enum Overlay {
    #[default]
    None,
    Status(usize),
    Paper,
    /// A game played by hand: a standoff, an ambush.
    Action,
}

/// Anything open that should stop the clock and the feet.
pub fn blocking(menu: &Menu, overlay: &Overlay) -> bool {
    menu.is_open() || *overlay != Overlay::None
}

#[derive(Component)]
pub struct MenuRoot;
#[derive(Component)]
pub struct MenuTitle;
#[derive(Component)]
pub struct MenuBody;
#[derive(Component)]
pub struct MenuItems;
#[derive(Component)]
pub struct MenuRow(usize);
#[derive(Component)]
pub struct ToastBox;
#[derive(Component)]
pub struct ToastText;
#[derive(Component)]
pub enum Hud {
    Date,
    Purse,
    Place,
    Hint,
}
#[derive(Component)]
pub struct OverlayRoot;
#[derive(Component)]
pub struct Dusk;
#[derive(Component)]
pub struct OverlayTitle;
#[derive(Component)]
pub struct OverlayTabs;
#[derive(Component)]
pub struct OverlayBody;

/// The blue box: a gradient, a pale edge, rounded corners.
fn window_node(node: Node) -> impl Bundle {
    (
        Node {
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(7.0)),
            padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..node
        },
        BackgroundGradient::from(LinearGradient::to_bottom(vec![
            WINDOW_TOP.into(),
            WINDOW_BOTTOM.into(),
        ])),
        BorderColor::all(EDGE),
    )
}

pub fn setup(mut commands: Commands, fonts: Res<Fonts>) {
    let (display, body) = (&fonts.display, &fonts.body);

    // The top bar.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                height: Val::Px(30.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(4.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.03, 0.04, 0.72)),
            GlobalZIndex(10),
        ))
        .with_children(|bar| {
            bar.spawn((Text::new(""), text(display, 17.0, LIGHT), Hud::Date));
            bar.spawn((Text::new(""), text(body, 15.0, LIGHT), Hud::Place));
            bar.spawn((Text::new(""), text(body, 15.0, LIGHT), Hud::Purse));
        });
    // Night on foot: the close views darken with the day and the moon.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.09, 0.0)),
        GlobalZIndex(1),
        Dusk,
    ));
    commands.spawn((
        Text::new(""),
        text(body, 13.0, LIGHT.with_alpha(0.7)),
        GlobalZIndex(10),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            bottom: Val::Px(6.0),
            ..default()
        },
        Hud::Hint,
    ));

    // The command window.
    commands
        .spawn((
            window_node(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                top: Val::Px(48.0),
                width: Val::Px(430.0),
                display: Display::None,
                ..default()
            }),
            MenuRoot,
            GlobalZIndex(20),
        ))
        .with_children(|w| {
            w.spawn((Text::new(""), text(display, 21.0, LIGHT), MenuTitle));
            w.spawn((
                Text::new(""),
                text(body, 14.0, LIGHT.with_alpha(0.85)),
                Node {
                    margin: UiRect::bottom(Val::Px(6.0)),
                    ..default()
                },
                MenuBody,
            ));
            w.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..default()
                },
                MenuItems,
            ));
        });

    // The toast.
    commands
        .spawn((
            window_node(Node {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                top: Val::Px(44.0),
                max_width: Val::Px(460.0),
                display: Display::None,
                ..default()
            }),
            ToastBox,
            GlobalZIndex(25),
        ))
        .with_child((Text::new(""), text(body, 15.0, LIGHT), ToastText));

    // Status and paper overlays share one frame.
    commands
        .spawn((
            window_node(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(60.0),
                right: Val::Px(60.0),
                top: Val::Px(50.0),
                bottom: Val::Px(40.0),
                display: Display::None,
                ..default()
            }),
            OverlayRoot,
            GlobalZIndex(30),
        ))
        .with_children(|o| {
            o.spawn((Text::new(""), text(display, 26.0, LIGHT), OverlayTitle));
            o.spawn((Text::new(""), text(body, 14.0, GREY), OverlayTabs));
            o.spawn((Text::new(""), text(body, 15.0, LIGHT), OverlayBody));
        });
}

/// Keyboard and mouse for the open window.
pub fn drive_menu(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut menu: ResMut<Menu>,
    mut rows: Query<(&Interaction, &MenuRow), Changed<Interaction>>,
    mut sim: ResMut<Sim>,
    mut toast: ResMut<Toast>,
    mut overlay: ResMut<Overlay>,
    mut clock: ResMut<Clock>,
    mut next: ResMut<NextState<Screen>>,
    mut town: ResMut<TownId>,
    mut game: ResMut<crate::duel::Game>,
    mut raid: ResMut<crate::raid::Raid>,
    time: Res<Time>,
) {
    let Some(spec) = menu.spec.clone() else {
        return;
    };
    let n = spec.items.len();
    let mut chosen = None;
    for (i, row) in &mut rows {
        match i {
            Interaction::Hovered => {
                if menu.cursor != row.0 {
                    menu.cursor = row.0;
                    menu.dirty = true;
                }
            }
            Interaction::Pressed => chosen = Some(row.0),
            Interaction::None => {}
        }
    }
    let step = |c: usize, up: bool| {
        let mut c = c;
        for _ in 0..n {
            c = if up { (c + n - 1) % n } else { (c + 1) % n };
            if spec.items[c].enabled {
                break;
            }
        }
        c
    };
    if keys.any_just_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        menu.cursor = step(menu.cursor, true);
        menu.dirty = true;
    }
    if keys.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        menu.cursor = step(menu.cursor, false);
        menu.dirty = true;
    }
    if keys.any_just_pressed([KeyCode::Enter, KeyCode::KeyE, KeyCode::Space]) {
        chosen = Some(menu.cursor);
    }
    if keys.any_just_pressed([KeyCode::Escape, KeyCode::Backspace]) && !menu.scene {
        menu.close();
        return;
    }
    let Some(i) = chosen else {
        return;
    };
    let Some(it) = spec.items.get(i) else {
        return;
    };
    if !it.enabled {
        return;
    }
    // One press, one thing: the key that chose this doesn't also reach
    // whatever's standing behind the window.
    for k in [KeyCode::Enter, KeyCode::KeyE, KeyCode::Space] {
        keys.clear_just_pressed(k);
    }
    mouse.clear_just_pressed(MouseButton::Left);
    let out = cmds::run(&mut sim.0, it.cmd);
    menu.close();
    if let Some(t) = out.toast {
        toast.say(t);
    }
    if let Some(m) = out.next {
        menu.open(m);
    }
    if out.status {
        *overlay = Overlay::Status(0);
    }
    if out.rest {
        sim.0.advance_day();
        clock.timer.reset();
    }
    match out.play {
        Some(cmds::Play::Standoff(a)) => game.start(&sim.0, a, time.elapsed_secs_f64()),
        Some(cmds::Play::Ambush(t)) => {
            if let Some(plan) = bleeding_kansas::sim::action::ambush_plan(&sim.0, t) {
                game.ambush(&sim.0, plan, time.elapsed_secs_f64());
            }
        }
        Some(cmds::Play::Raid(t)) => {
            if let Some(plan) = bleeding_kansas::sim::action::raid_plan(&sim.0, t) {
                raid.plan = Some(plan);
                next.set(Screen::Raid);
            }
        }
        None => {}
    }
    if let Some((screen, t)) = out.goto {
        if let Some(t) = t {
            *town = t;
        }
        next.set(screen);
    }
}

pub fn draw_menu(
    mut commands: Commands,
    fonts: Res<Fonts>,
    mut menu: ResMut<Menu>,
    mut root: Query<&mut Node, With<MenuRoot>>,
    mut title: Query<&mut Text, (With<MenuTitle>, Without<MenuBody>)>,
    mut body: Query<&mut Text, (With<MenuBody>, Without<MenuTitle>)>,
    items: Query<Entity, With<MenuItems>>,
) {
    if !menu.dirty {
        return;
    }
    menu.dirty = false;
    let Ok(mut node) = root.single_mut() else {
        return;
    };
    let Some(spec) = &menu.spec else {
        node.display = Display::None;
        return;
    };
    node.display = Display::Flex;
    if menu.scene {
        node.left = Val::Px(380.0);
        node.top = Val::Auto;
        node.bottom = Val::Px(110.0);
        node.width = Val::Px(520.0);
    } else {
        node.left = Val::Px(24.0);
        node.top = Val::Px(48.0);
        node.bottom = Val::Auto;
        node.width = Val::Px(440.0);
    }
    if let Ok(mut t) = title.single_mut() {
        **t = plain(&spec.title);
    }
    if let Ok(mut t) = body.single_mut() {
        **t = plain(&spec.body);
    }
    let Ok(list) = items.single() else {
        return;
    };
    commands.entity(list).despawn_children();
    let cursor = menu.cursor;
    commands.entity(list).with_children(|l| {
        for (i, it) in spec.items.iter().enumerate() {
            let pointer = if i == cursor { "\u{25B6} " } else { "   " };
            let color = if !it.enabled {
                GREY.with_alpha(0.6)
            } else if i == cursor {
                LIGHT
            } else {
                LIGHT.with_alpha(0.8)
            };
            l.spawn((Button, Node::default(), MenuRow(i))).with_child((
                Text::new(format!("{pointer}{}", plain(&it.label))),
                text(&fonts.body, 16.0, color),
            ));
        }
    });
}

pub fn draw_toast(
    time: Res<Time>,
    mut toast: ResMut<Toast>,
    mut root: Query<&mut Node, With<ToastBox>>,
    mut text_q: Query<&mut Text, With<ToastText>>,
) {
    toast.left -= time.delta_secs();
    let Ok(mut node) = root.single_mut() else {
        return;
    };
    let show = toast.left > 0.0 && !toast.text.is_empty();
    node.display = if show { Display::Flex } else { Display::None };
    if show && let Ok(mut t) = text_q.single_mut() {
        **t = plain(&toast.text);
    }
}

pub fn draw_hud(
    sim: Res<Sim>,
    clock: Res<Clock>,
    state: Res<State<Screen>>,
    town: Res<TownId>,
    mut hud: Query<(&Hud, &mut Text)>,
) {
    let world = &sim.0;
    let w = world.weather_on(world.day);
    let sky = if w.blizzard {
        "blizzard"
    } else if w.storm {
        "a hard rain"
    } else if w.rain {
        "rain"
    } else {
        "clear"
    };
    let hh = &world.families[0].stores;
    for (h, mut t) in &mut hud {
        let s = match h {
            Hud::Date => format!(
                "{}   {}, {}{}",
                world.day,
                sky,
                world.day.moon_name(),
                if clock.paused { "   (paused)" } else { "" }
            ),
            Hud::Place => match state.get() {
                Screen::County => "Douglas County, K.T.".to_string(),
                Screen::Claim => "Your claim".to_string(),
                Screen::Town => town.name().to_string(),
                Screen::Raid => "Someone else's place, at night".to_string(),
            },
            Hud::Purse => {
                let people = world.living().filter(|n| n.family == 0).count().max(1) as f32;
                let fs = world.grievance[Faction::FreeState.index()];
                let ps = world.grievance[Faction::ProSlavery.index()];
                format!(
                    "${}  debt ${}   food {:.0} days   spirits {:.0}   tension {}/{}",
                    hh.cash,
                    hh.debt,
                    hh.food / people,
                    world.life.spirits,
                    fs,
                    ps
                )
            }
            Hud::Hint => match state.get() {
                Screen::County => {
                    "Click a neighbor to deal with them. Click your claim or a town to go there. Wheel zooms, drag pans. Tab: status  N: paper  Space: pause"
                }
                Screen::Raid => {
                    "WASD move   Shift: crawl   Hold E: do it   Stay out of the lantern light   Back to the road sign to leave"
                }
                _ => "WASD walk   E: use what's in front of you   M: county map   R: rest   Tab: status   N: paper   Space: pause",
            }
            .to_string(),
        };
        **t = plain(&s);
    }
}

pub fn dusk(
    sim: Res<Sim>,
    clock: Res<Clock>,
    state: Res<State<Screen>>,
    mut q: Query<&mut BackgroundColor, With<Dusk>>,
) {
    let a = if *state.get() == Screen::County {
        0.0
    } else {
        crate::scenery::darkness(clock.timer.fraction()) * (0.72 - 0.42 * sim.0.day.moonlight())
    };
    for mut c in &mut q {
        c.0 = Color::srgba(0.02, 0.03, 0.09, a);
    }
}

pub const TABS: [&str; 5] = ["Household", "Your life", "Market", "The county", "Nations"];

pub fn overlays(
    keys: Res<ButtonInput<KeyCode>>,
    mut overlay: ResMut<Overlay>,
    menu: Res<Menu>,
    sim: Res<Sim>,
    mut root: Query<(&mut Node, &mut BackgroundGradient, &mut BorderColor), With<OverlayRoot>>,
    mut texts: Query<
        (
            &mut Text,
            &mut TextColor,
            Option<&OverlayTitle>,
            Option<&OverlayTabs>,
            Option<&OverlayBody>,
        ),
        Or<(With<OverlayTitle>, With<OverlayTabs>, With<OverlayBody>)>,
    >,
) {
    if *overlay == Overlay::Action {
        if let Ok((mut node, ..)) = root.single_mut() {
            node.display = Display::None;
        }
        return;
    }
    if !menu.is_open() {
        if keys.just_pressed(KeyCode::Tab) {
            *overlay = match *overlay {
                Overlay::Status(_) => Overlay::None,
                _ => Overlay::Status(0),
            };
        }
        if keys.just_pressed(KeyCode::KeyN) {
            *overlay = if *overlay == Overlay::Paper {
                Overlay::None
            } else {
                Overlay::Paper
            };
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        *overlay = Overlay::None;
    }
    if let Overlay::Status(i) = *overlay {
        if keys.any_just_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
            *overlay = Overlay::Status((i + 1) % TABS.len());
        }
        if keys.any_just_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
            *overlay = Overlay::Status((i + TABS.len() - 1) % TABS.len());
        }
    }
    let Ok((mut node, mut grad, mut edge)) = root.single_mut() else {
        return;
    };
    let world = &sim.0;
    // The paper is newsprint and ink; everything else is the blue box.
    let paper = *overlay == Overlay::Paper;
    let (top, bottom, ink) = if paper {
        (NEWSPRINT, NEWSPRINT, INK)
    } else {
        (WINDOW_TOP, WINDOW_BOTTOM, LIGHT)
    };
    *grad = BackgroundGradient::from(LinearGradient::to_bottom(vec![top.into(), bottom.into()]));
    *edge = BorderColor::all(if paper { INK } else { EDGE });
    let (title, tabs, body) = match *overlay {
        Overlay::None | Overlay::Action => {
            node.display = Display::None;
            return;
        }
        Overlay::Status(i) => {
            let tabs = TABS
                .iter()
                .enumerate()
                .map(|(k, t)| {
                    if k == i {
                        format!("[ {t} ]")
                    } else {
                        t.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("     ");
            let body = match i {
                0 => panels::household(world),
                1 => panels::your_life(world),
                2 => panels::market_and_nations(world),
                3 => county_page(world),
                _ => panels::nations(world),
            };
            (
                "Status".to_string(),
                format!("{tabs}      (arrows to turn the page)"),
                body,
            )
        }
        Overlay::Paper => {
            let (name, place) = panels::masthead(world);
            let lines = chronicle::chronicle(world, false);
            let start = lines.len().saturating_sub(22);
            let body = lines[start..]
                .iter()
                .map(|l| panels::broadsheet(l))
                .collect::<Vec<_>>()
                .join("\n");
            (name.to_string(), format!("{place}  ~  {}", world.day), body)
        }
    };
    node.display = Display::Flex;
    for (mut t, mut color, title_m, tabs_m, _) in &mut texts {
        color.0 = if tabs_m.is_some() && !paper {
            GREY
        } else {
            ink
        };
        if title_m.is_some() {
            **t = plain(&title);
        } else if tabs_m.is_some() {
            **t = plain(&tabs);
        } else {
            **t = plain(&body);
        }
    }
}

fn county_page(world: &bleeding_kansas::sim::World) -> String {
    let fs = world.grievance[Faction::FreeState.index()];
    let ps = world.grievance[Faction::ProSlavery.index()];
    let feuds: Vec<String> = world
        .feuds
        .iter()
        .map(|&(a, b)| {
            format!(
                "the {} and the {}",
                world.families[a as usize].surname, world.families[b as usize].surname
            )
        })
        .collect();
    let mut s = format!(
        "Free-State grievance {} {}\nPro-Slavery grievance {} {}\n",
        panels::bar(fs as f32, 150.0),
        fs,
        panels::bar(ps as f32, 150.0),
        ps
    );
    if world.pacified_until.is_some_and(|d| world.day < d) {
        s.push_str("Federal dragoons patrol the roads.\n");
    }
    s.push_str(&format!(
        "\nFeuds: {}\n",
        if feuds.is_empty() {
            "none that anyone admits".to_string()
        } else {
            feuds.join("; ")
        }
    ));
    s.push_str(&format!(
        "\nLawrence lots go for ${:.0}. The land office has {} of the county's claims on file.\n",
        world.land.lot_price,
        world.civic.filed.iter().filter(|f| **f).count()
    ));
    s
}
