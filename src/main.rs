use bevy::prelude::*;

const GRID_SIZE: i32 = 30;
const TILE_SIZE: f32 = 24.0;
const NPC_COUNT: usize = 10;

#[derive(Component)]
struct Npc {
    name: &'static str,
    mood: i32,
    opinion_of_player: i32,
    family: Option<Entity>,
    alive: bool,
}

#[derive(Component)]
struct InspectionLabel;

#[derive(Component)]
struct LogLabel;

#[derive(Component)]
struct KillButton;

#[derive(Resource, Default)]
struct Selection(Option<Entity>);

#[derive(Resource, Default)]
struct WorldLog {
    entries: Vec<String>,
}

#[derive(Resource)]
struct WanderTimer(Timer);

#[derive(Message)]
struct NpcKilled {
    victim: Entity,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bleeding Kansas".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_message::<NpcKilled>()
        .init_resource::<Selection>()
        .init_resource::<WorldLog>()
        .insert_resource(WanderTimer(Timer::from_seconds(0.8, TimerMode::Repeating)))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                wander_npcs,
                select_npc,
                update_inspection_panel,
                kill_selected_npc,
                process_npc_death,
                update_log,
            )
                .chain(),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut log: ResMut<WorldLog>,
) {
    commands.spawn(Camera2d);

    let grid_width = GRID_SIZE as f32 * TILE_SIZE;
    let grid_height = GRID_SIZE as f32 * TILE_SIZE;
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(grid_width, grid_height))),
        MeshMaterial2d(materials.add(Color::srgb(0.12, 0.18, 0.12))),
        Transform::from_xyz(-180.0, 0.0, 0.0),
    ));

    let names = [
        "Jonas Webb",
        "Martha Bell",
        "Cyrus Mallory",
        "Elias Brown",
        "Ruth Carter",
        "Samuel Pike",
        "Clara Finch",
        "Thomas Reed",
        "Ada Mercer",
        "William Holt",
    ];

    let mut npc_entities = Vec::with_capacity(NPC_COUNT);
    for (index, name) in names.iter().enumerate() {
        let position = Vec3::new(
            -180.0 + (index as f32 % 5.0) * 92.0,
            -180.0 + (index as f32 / 5.0).floor() * 120.0,
            1.0,
        );
        let entity = commands
            .spawn((
                Sprite {
                    color: Color::srgb(0.82, 0.67, 0.38),
                    custom_size: Some(Vec2::splat(18.0)),
                    ..default()
                },
                Transform::from_translation(position),
                Npc {
                    name,
                    mood: 50 + (index as i32 % 4) * 10,
                    opinion_of_player: 10 - index as i32 * 2,
                    family: None,
                    alive: true,
                },
            ))
            .id();
        npc_entities.push(entity);
    }

    for (index, entity) in npc_entities.iter().enumerate() {
        let family = if index < 4 {
            Some(npc_entities[(index + 1) % 4])
        } else {
            None
        };
        commands.entity(*entity).insert(Npc {
            name: names[index],
            mood: 50 + (index as i32 % 4) * 10,
            opinion_of_player: 10 - index as i32 * 2,
            family,
            alive: true,
        });
    }

    log.entries.push(
        "[WORLD] Winter 1855. Click a neighbor to inspect them; select KILL to test the cascade."
            .into(),
    );

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(24.0),
            top: Val::Px(24.0),
            width: Val::Px(360.0),
            height: Val::Percent(92.0),
            padding: UiRect::all(Val::Px(16.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.05, 0.04, 0.94)),
    ))
    .with_children(|panel| {
        panel.spawn((
            Text::new("BLEEDING KANSAS"),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgb(0.88, 0.72, 0.42)),
        ));
        panel.spawn((
            Text::new("Select a neighbor"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            InspectionText,
        ));
        panel.spawn((
            Button,
            Node {
                width: Val::Px(120.0),
                height: Val::Px(36.0),
                padding: UiRect::all(Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.35, 0.12, 0.10)),
            KillButton,
        ))
        .with_child((
            Text::new("KILL"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
        panel.spawn((
            Text::new("CASCADE LOG"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.72, 0.72, 0.62)),
        ));
        panel.spawn((
            Text::new(""),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(Color::srgb(0.68, 0.74, 0.68)),
            InspectionText,
            LogText,
        ));
    });
}

fn wander_npcs(
    time: Res<Time>,
    mut timer: ResMut<WanderTimer>,
    mut query: Query<(&Npc, &mut Transform)>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    for (npc, mut transform) in &mut query {
        if npc.alive {
            let direction = Vec3::new(
                (transform.translation.y * 0.013).sin() * 7.0,
                (transform.translation.x * 0.017).cos() * 7.0,
                0.0,
            );
            transform.translation += direction;
            transform.translation.x = transform.translation.x.clamp(-520.0, 140.0);
            transform.translation.y = transform.translation.y.clamp(-300.0, 300.0);
        }
    }
}

fn select_npc(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    npcs: Query<(Entity, &GlobalTransform, &Npc)>,
    mut selection: ResMut<Selection>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, cursor) else {
        return;
    };

    selection.0 = npcs
        .iter()
        .filter(|(_, _, npc)| npc.alive)
        .find(|(_, transform, _)| transform.translation().truncate().distance(world_position) < 18.0)
        .map(|(entity, _, _)| entity);
}

fn update_inspection_panel(
    selection: Res<Selection>,
    npcs: Query<&Npc>,
    mut text: Query<&mut Text, (With<InspectionText>, Without<LogText>)>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let Some(entity) = selection.0 else {
        **text = "Select a neighbor".into();
        return;
    };
    let Ok(npc) = npcs.get(entity) else {
        **text = "That neighbor is gone".into();
        return;
    };
    **text = format!(
        "{}\nMood: {}\nOpinion of you: {}",
        npc.name, npc.mood, npc.opinion_of_player
    );
}

fn kill_selected_npc(
    interactions: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<KillButton>)>,
    selection: Res<Selection>,
    npcs: Query<&Npc>,
    mut events: MessageWriter<NpcKilled>,
) {
    for (interaction, mut color) in interactions {
        if *interaction == Interaction::Pressed {
            *color = BackgroundColor(Color::srgb(0.55, 0.16, 0.10));
            if let Some(victim) = selection.0 {
                if npcs.get(victim).is_ok_and(|npc| npc.alive) {
                    events.write(NpcKilled { victim });
                }
            }
        }
    }
}

fn process_npc_death(
    mut events: MessageReader<NpcKilled>,
    mut npcs: Query<(Entity, &mut Npc, &mut Sprite)>,
    mut log: ResMut<WorldLog>,
    selection: Res<Selection>,
) {
    for event in events.read() {
        let Ok((_, mut victim, mut sprite)) = npcs.get_mut(event.victim) else {
            continue;
        };
        if !victim.alive {
            continue;
        }
        victim.alive = false;
        sprite.color = Color::srgb(0.18, 0.18, 0.18);
        let victim_name = victim.name;
        let family = victim.family;
        log.entries.push(format!("[EVENT] {} was killed.", victim_name));
        drop(victim);
        drop(sprite);

        if let Some(family) = family {
            if let Ok((_, _, mut relative_sprite)) = npcs.get_mut(family) {
                relative_sprite.color = Color::srgb(0.88, 0.34, 0.22);
            }
            if let Ok((_, mut relative, _)) = npcs.get_mut(family) {
                relative.opinion_of_player -= 40;
                relative.mood -= 30;
                log.entries.push(format!(
                    "[GRIEF] {} grieves {}. Opinion drops to {}.",
                    relative.name, victim_name, relative.opinion_of_player
                ));
                log.entries.push(format!(
                    "[GOSSIP] {} tells the settlement what you did.",
                    relative.name
                ));
            }
        }
        if selection.0 == Some(event.victim) {
            log.entries.push("[WORLD] The empty place at the table is noticed.".into());
        }
    }
}

fn update_log(log: Res<WorldLog>, mut text: Query<&mut Text, With<LogText>>) {
    if !log.is_changed() {
        return;
    }
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    **text = log
        .entries
        .iter()
        .rev()
        .take(12)
        .rev()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
}
