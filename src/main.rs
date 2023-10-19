use std::f32::consts::PI;

use bevy::{
    audio::{PlaybackMode, Volume, VolumeLevel},
    prelude::*,
    sprite::MaterialMesh2dBundle,
};
use bevy_xpbd_2d::prelude::*;

const DROP_LINE: f32 = 3.0;
const SIZE_COUNT: usize = 11;

const BOX_WIDTH: f32 = 4.4;
const BOX_HEIGHT: f32 = 5.0;
const TOP_OFFSET: f32 = -0.8;

const DEATH_LINE: f32 = 2.0;
const OVERTOP_TIMER: f32 = 1.5;

const MAX_RADIUS: f32 = (BOX_WIDTH / 2.2) / 2.0;
const MIN_RADIUS: f32 = 0.15;

const LINEAR_DAMPING: f32 = 3.0;
const ANGULAR_DAMPING: f32 = 0.7;
const FRICTION: f32 = 0.7;
const RESTITUTION: f32 = 0.5;

const G: f32 = 70.0;

const BALL_ORDER: &'static [&'static str] = &[
    "sweet",
    "spider",
    "bat",
    "apple",
    "candy_apple",
    "ghost",
    "vampire",
    "mummy",
    "frankenstein",
    "skull",
    "pumpkin",
];
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_linear()),
            PhysicsPlugins::default(),
        ))
        .insert_resource(Gravity(Vec2::NEG_Y * G))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (
                    fake_ball_follow_mouse,
                    cursor_to_world,
                    release_ball,
                    tick_next_ball,
                    merge_on_collision,
                    enter_splash,
                    update_score,
                    update_next_up,
                    check_over_top,
                    enter_gameover,
                )
                    .run_if(in_state(GameState::Running)),
                enter_running.run_if(in_state(GameState::Splash)),
                enter_running.run_if(in_state(GameState::GameOver)),
                toggle_bgm,
                toggle_sfx,
                do_kill_me,
            ),
        )
        .add_systems(OnEnter(GameState::Splash), build_splash)
        .add_systems(OnExit(GameState::Splash), despawn_with::<SplashTag>)
        .add_systems(
            OnEnter(GameState::Running),
            (build_running, add_walls, set_next_size),
        )
        .add_systems(OnExit(GameState::Running), despawn_with::<RunningTag>)
        .add_systems(OnEnter(GameState::GameOver), build_gameover)
        .add_systems(OnExit(GameState::GameOver), despawn_with::<GameOverTag>)
        .add_systems(OnEnter(NextBallState::Pick), set_next_size)
        .add_systems(OnEnter(AppState::Running), resume)
        .add_systems(OnExit(AppState::Running), pause)
        .init_resource::<CursorWorldPos>()
        .init_resource::<NextBallSize>()
        .init_resource::<NextNextBallSize>()
        .init_resource::<Score>()
        .init_resource::<Multiplier>()
        .insert_resource(NextBallTimer(Timer::from_seconds(0.5, TimerMode::Once)))
        .add_state::<AppState>()
        .add_state::<GameState>()
        .add_state::<NextBallState>()
        .run();
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
enum AppState {
    Paused,
    #[default]
    Running,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
enum GameState {
    #[default]
    Splash,
    Running,
    GameOver,
}

#[derive(Component)]
struct SplashTag;

#[derive(Component)]
struct RunningTag;

#[derive(Component)]
struct GameOverTag;

fn enter_running(keys: Res<Input<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::Space) {
        next_state.0 = Some(GameState::Running)
    }
}

fn enter_splash(keys: Res<Input<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.0 = Some(GameState::Splash)
    }
}

fn enter_gameover(keys: Res<Input<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::G) {
        next_state.0 = Some(GameState::GameOver)
    }
}

#[derive(Component)]
struct MusicTag;

#[derive(Resource)]
struct MusicToggle(bool);
fn toggle_bgm(
    keys: Res<Input<KeyCode>>,
    bgm_q: Query<&mut AudioSink, With<MusicTag>>,
    mut toggle: ResMut<MusicToggle>,
) {
    if keys.just_pressed(KeyCode::M) {
        toggle.0 = !toggle.0;
        if let Ok(sink) = bgm_q.get_single() {
            sink.toggle();
        }
    }
}

#[derive(Resource)]
struct SoundToggle(bool);

fn toggle_sfx(keys: Res<Input<KeyCode>>, mut toggle: ResMut<SoundToggle>) {
    if keys.just_pressed(KeyCode::S) {
        toggle.0 = !toggle.0;
    }
}

#[derive(Resource)]
struct BallSizes(Vec<(f32, Handle<Mesh>, Handle<ColorMaterial>, Handle<Image>)>);

#[derive(Resource)]
struct AudioHandles {
    drop: Handle<AudioSource>,
    merge: Handle<AudioSource>,
    game_over: Handle<AudioSource>,
}

#[derive(Component)]
struct BallSize(usize);

fn despawn_with<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    for e in q.iter() {
        commands.entity(e).despawn_recursive();
    }
}

fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
    v0 + t * (v1 - v0)
}

//fn ease_in_cubic(t: f32) -> f32 {
//    t * t * t
//}
//
//fn ease_in_circ(t: f32) -> f32 {
//    1.0 - (1.0 - t.powi(2)).sqrt()
//}

fn ease_in_sine(t: f32) -> f32 {
    1.0 - ((t * PI) / 2.0).cos()
}

#[derive(Component)]
struct ScoreTag;

fn build_splash(mut commands: Commands, font: Res<CustomFont>) {
    let style = TextStyle {
        font_size: 30.0,
        font: font.0.clone_weak(),
        ..default()
    };

    commands
        .spawn(
            TextBundle::from_sections([
                TextSection {
                    value: "Pumpkin Game!\n\n\n\n".into(),
                    style: TextStyle {
                        font_size: 40.0,
                        font: font.0.clone_weak(),
                        ..default()
                    },
                },
                TextSection {
                    value: "[Space] - Start\n[Esc] - Quit\n[Mouse] - Aim\n[Click] - Drop\n[S] - Toggle SFX\n[M] - Toggle BGM".into(),
                    style,
                },
            ])
            .with_style(Style {
                position_type: PositionType::Absolute,
                top: Val::Px(50.0),
                left: Val::Px(150.0),
                ..default()
            })
            .with_text_alignment(TextAlignment::Center),
        )
        .insert(SplashTag);
}

fn build_running(
    mut score: ResMut<Score>,
    mut commands: Commands,
    ball_sizes: Res<BallSizes>,
    next_ball_size: Res<NextBallSize>,
    font: Res<CustomFont>,
    bgm_q: Query<&AudioSink, With<MusicTag>>,
    bgm_toggle: Res<MusicToggle>,
) {
    score.0 = 0;

    if let Ok(sink) = bgm_q.get_single() {
        if bgm_toggle.0 {
            sink.play();
        }
    }

    commands
        .spawn(
            TextBundle::from_section(
                "Score: 0",
                TextStyle {
                    font_size: 30.0,
                    font: font.0.clone_weak(),
                    ..default()
                },
            )
            .with_style(Style {
                position_type: PositionType::Absolute,
                left: Val::Px(5.0),
                top: Val::Px(5.0),
                ..default()
            }),
        )
        .insert((ScoreTag, RunningTag));

    commands
        .spawn(
            TextBundle::from_section(
                "Next:",
                TextStyle {
                    font_size: 30.0,
                    font: font.0.clone_weak(),
                    ..default()
                },
            )
            .with_style(Style {
                position_type: PositionType::Absolute,
                right: Val::Px(5.0),
                top: Val::Px(5.0),
                ..default()
            }),
        )
        .insert(RunningTag);

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(50.0),
                height: Val::Px(50.0),
                position_type: PositionType::Absolute,
                right: Val::Px(15.0),
                top: Val::Px(45.0),
                ..default()
            },
            background_color: Color::WHITE.into(),
            ..default()
        })
        .insert(UiImage::new(ball_sizes.0[next_ball_size.0].3.clone_weak()))
        .insert((RunningTag, NextUpTag));
}

#[derive(Component)]
struct NextUpTag;

fn update_next_up(
    next_q: Query<Entity, With<NextUpTag>>,
    next_next_ball_size: Res<NextNextBallSize>,
    ball_sizes: Res<BallSizes>,
    mut commands: Commands,
) {
    if !next_next_ball_size.is_changed() {
        return;
    }

    if let Ok(entity) = next_q.get_single() {
        commands.entity(entity).insert(UiImage::new(
            ball_sizes.0[next_next_ball_size.0].3.clone_weak(),
        ));
    }
}

fn update_score(score: Res<Score>, mut ui_q: Query<&mut Text, With<ScoreTag>>) {
    if !score.is_changed() {
        return;
    }

    if let Ok(mut text) = ui_q.get_single_mut() {
        text.sections[0].value = format!("Score: {}", score.0);
    }
}

#[derive(Resource)]
struct CustomFont(Handle<Font>);

fn build_gameover(
    score: Res<Score>,
    mut commands: Commands,
    font: Res<CustomFont>,
    bgm_q: Query<&AudioSink, With<MusicTag>>,
    audio_handles: Res<AudioHandles>,
    sound_toggle: Res<SoundToggle>,
) {
    //
    let score_string = format!("Score: {}", score.0);

    let style = TextStyle {
        font_size: 30.0,
        font: font.0.clone_weak(),
        ..default()
    };

    commands
        .spawn(
            TextBundle::from_sections([
                TextSection {
                    value: "Skill Issue\n".into(),
                    style: style.clone(),
                },
                TextSection {
                    value: score_string,
                    style: style.clone(),
                },
                TextSection {
                    value: "\n\nSpace to Restart".into(),
                    style: TextStyle {
                        font_size: 25.0,
                        font: font.0.clone_weak(),
                        ..default()
                    },
                },
            ])
            .with_style(Style {
                // how much can i be arsed with bevy's flexbox bits
                // every time i try it's a nightmare
                // i don't know why, every other part of the engine
                // feels super intuitive. but layout is a pain point.
                // could be wrong, i can't stand the most popular
                // js framework either.
                position_type: PositionType::Absolute,
                top: Val::Px(50.0),
                left: Val::Px(150.0),
                ..default()
            })
            .with_text_alignment(TextAlignment::Center),
        )
        .insert(GameOverTag);

    if let Ok(sink) = bgm_q.get_single() {
        sink.pause();
    }

    if !sound_toggle.0 {
        return;
    }

    commands
        .spawn(AudioBundle {
            source: audio_handles.game_over.clone_weak(),
            settings: PlaybackSettings {
                volume: Volume::Relative(VolumeLevel::new(0.7)),
                ..default()
            },
        })
        .insert(KillMeTimer(Timer::from_seconds(0.9, TimerMode::Once)));
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut window_q: Query<&mut Window>,
    asset_server: Res<AssetServer>,
) {
    let mut window = window_q.single_mut();

    window.resolution.set_scale_factor(1.0);
    window.resolution.set(480.0, 720.0);

    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            near: -1.0,
            far: 1000.0,
            scale: 0.01,
            ..default()
        },
        ..default()
    });

    let mut ball_sizes = vec![];

    for i in 1..=SIZE_COUNT {
        let radius = lerp(
            MIN_RADIUS,
            MAX_RADIUS,
            ease_in_sine(i as f32 / SIZE_COUNT as f32),
        );
        let mesh = meshes.add(shape::Circle::new(radius).into());
        let mat = materials.add(ColorMaterial::from(Color::rgba(
            0.5,
            0.5,
            i as f32 / SIZE_COUNT as f32,
            0.0, // DEBUG:  remove this line to see collider
        )));

        let img = asset_server.load(format!("{}.png", BALL_ORDER[i - 1]));

        ball_sizes.push((radius, mesh, mat, img));
    }

    commands.insert_resource(BallSizes(ball_sizes));
    commands.insert_resource(CustomFont(asset_server.load("Creepster-Regular.ttf")));

    // Audio
    commands.insert_resource(AudioHandles {
        merge: asset_server.load("pop-1.ogg"),
        drop: asset_server.load("drop-1.ogg"),
        game_over: asset_server.load("game-over.ogg"),
    });

    commands.insert_resource(SoundToggle(true));
    commands.insert_resource(MusicToggle(true));

    // BGM
    commands
        .spawn(AudioBundle {
            source: asset_server.load("spook.ogg"),
            settings: PlaybackSettings {
                mode: PlaybackMode::Loop,
                volume: Volume::Relative(VolumeLevel::new(0.7)),
                ..default()
            },
            ..default()
        })
        .insert(MusicTag);

    commands.spawn(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(4.8, 7.2)),
            ..default()
        },
        texture: asset_server.load("bg.png"),
        transform: Transform::from_xyz(0.0, 0.0, -1.0),
        ..default()
    });

    commands.spawn(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(4.8, 7.2)),
            ..default()
        },
        texture: asset_server.load("fg.png"),
        transform: Transform::from_xyz(0.0, 0.0, 1.0),
        ..default()
    });
}

fn add_walls(mut commands: Commands) {
    // TODO: seperate debugdraw fn instead of commented code
    //
    //let square_sprite = Sprite {
    //    color: Color::rgb_u8(200, 200, 200),
    //    custom_size: Some(Vec2::splat(1.0)),
    //    ..default()
    //};

    let wall_thickness = 1.0;

    // floor
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(BOX_WIDTH + wall_thickness, wall_thickness),
        Position(Vec2::new(0.0, TOP_OFFSET - BOX_HEIGHT / 2.0)),
        //SpriteBundle {
        //    sprite: square_sprite.clone(),
        //    transform: Transform::from_scale(Vec3::new(
        //        BOX_WIDTH + wall_thickness,
        //        wall_thickness,
        //        1.0,
        //    )),
        //    ..default()
        //},
        RunningTag,
    ));

    // left
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(wall_thickness, BOX_HEIGHT * 100.0), // walls are actually very tall, visually not
        Position(Vec2::new(-BOX_WIDTH / 2.0, TOP_OFFSET)),
        //SpriteBundle {
        //    sprite: square_sprite.clone(),
        //    transform: Transform::from_scale(Vec3::new(wall_thickness, BOX_HEIGHT, 1.0)),
        //    ..default()
        //},
        RunningTag,
    ));

    // right
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(wall_thickness, BOX_HEIGHT * 100.0),
        Position(Vec2::new(BOX_WIDTH / 2.0, TOP_OFFSET)),
        //SpriteBundle {
        //    sprite: square_sprite.clone(),
        //    transform: Transform::from_scale(Vec3::new(wall_thickness, BOX_HEIGHT, 1.0)),
        //    ..default()
        //},
        RunningTag,
    ));

    // roof (invisible, offscreen, saves from scammy explosion gameovers)
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(BOX_WIDTH + wall_thickness, wall_thickness),
        Position(Vec2::new(0.0, 6.0)),
        //SpriteBundle {
        //    sprite: square_sprite.clone(),
        //    transform: Transform::from_scale(Vec3::new(
        //        BOX_WIDTH + wall_thickness,
        //        wall_thickness,
        //        1.0,
        //    )),
        //    ..default()
        //},
        RunningTag,
    ));
}

#[derive(Component)]
struct FakeBall;

fn fake_ball_follow_mouse(
    mut fake_ball_q: Query<(&mut Position, &BallSize), With<FakeBall>>,
    cursor: Res<CursorWorldPos>,
    ball_sizes: Res<BallSizes>,
) {
    if let Ok((mut pos, size)) = fake_ball_q.get_single_mut() {
        let max = BOX_WIDTH / 2.0 - ball_sizes.0[size.0].0 - 0.5; // wall_thickness
        let min = -BOX_WIDTH / 2.0 + ball_sizes.0[size.0].0 + 0.5;
        pos.x = cursor.0.x.clamp(min, max);
    }
}

#[derive(Resource)]
struct NextBallTimer(Timer);

fn release_ball(
    mut next_ball_timer: ResMut<NextBallTimer>,
    mut fake_ball_q: Query<(Entity, &Position), With<FakeBall>>,
    mut commands: Commands,
    mouse: Res<Input<MouseButton>>,
    ball_sizes: Res<BallSizes>,
    mut next_ball_state: ResMut<NextState<NextBallState>>,
    next_ball_size: Res<NextBallSize>,
    mut multiplier: ResMut<Multiplier>,
    audio_handles: Res<AudioHandles>,
    sound_toggle: Res<SoundToggle>,
) {
    if !mouse.just_pressed(MouseButton::Left) || fake_ball_q.is_empty() {
        return;
    }

    multiplier.0 = 0;

    next_ball_timer.0.reset();

    let size = next_ball_size.0;

    if let Ok((entity, position)) = fake_ball_q.get_single_mut() {
        let av = -1.0 + fastrand::f32() * 2.0;
        commands
            .spawn(new_ball(position.0, size, &ball_sizes))
            .insert(AngularVelocity(av)); // prevents perfect stacking

        commands.entity(entity).despawn();

        next_ball_state.0 = Some(NextBallState::Pick);

        if !sound_toggle.0 {
            return;
        }

        let speed = lerp(0.2, 1.2, 1.0 - (size as f32 / SIZE_COUNT as f32));
        commands
            .spawn(AudioBundle {
                source: audio_handles.drop.clone_weak(),
                settings: PlaybackSettings {
                    volume: Volume::Relative(VolumeLevel::new(0.3)),
                    speed,
                    ..default()
                },
                ..default()
            })
            .insert(KillMeTimer(Timer::from_seconds(0.5, TimerMode::Once)));
    }
}

#[derive(Component)]
struct KillMeTimer(Timer);

fn do_kill_me(
    mut commands: Commands,
    mut audio_q: Query<(Entity, &mut KillMeTimer)>,
    time: Res<Time>,
) {
    for (entity, mut timer) in audio_q.iter_mut() {
        timer.0.tick(time.delta());
        if !timer.0.finished() {
            return;
        }

        commands.entity(entity).despawn();
    }
}

fn tick_next_ball(
    mut next_ball_timer: ResMut<NextBallTimer>,
    time: Res<Time>,
    mut commands: Commands,
    ball_sizes: Res<BallSizes>,
    fake_ball_q: Query<&FakeBall>,
    next_ball_size: Res<NextBallSize>,
    cursor: Res<CursorWorldPos>,
) {
    if next_ball_timer.0.finished() && fake_ball_q.is_empty() {
        commands
            .spawn(new_ball(
                Vec2::new(cursor.0.x, DROP_LINE),
                next_ball_size.0,
                &ball_sizes,
            ))
            .insert((RigidBody::Static, FakeBall)); // TODO: remove collision from fake ball

        return;
    }

    next_ball_timer.0.tick(time.delta());
}

#[derive(Resource, Default)]
struct CursorWorldPos(Vec2);

fn cursor_to_world(
    mut pos: ResMut<CursorWorldPos>,
    // query to get the window (so we can read the current cursor position)
    q_window: Query<&Window>,
    // query to get camera transform
    q_camera: Query<(&Camera, &GlobalTransform)>,
) {
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so Query::single() is OK
    let (camera, camera_transform) = q_camera.single();

    // There is only one primary window, so we can similarly get it from the query:
    let window = q_window.single();

    // check if the cursor is inside the window and get its position
    // then, ask bevy to convert into world coordinates, and truncate to discard Z
    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        pos.0 = world_position;
    }
}

#[derive(Resource, Default)]
struct Multiplier(usize);

fn merge_on_collision(
    mut collision_event_reader: EventReader<Collision>,
    ballsize_q: Query<(&BallSize, &Position, &LinearVelocity, &AngularVelocity)>,
    mut commands: Commands,
    ball_sizes: Res<BallSizes>,
    mut score: ResMut<Score>,
    mut multiplier: ResMut<Multiplier>,
    audio_handles: Res<AudioHandles>,
    sound_toggle: Res<SoundToggle>,
) {
    for Collision(contact) in collision_event_reader.iter() {
        // Check BallSize component on entities. If present and equal, remove the two contacting
        // entities and spawn a ball with the next size at the midpoint of the contacting ball's
        // positions.
        let entity1 = contact.entity1;
        let entity2 = contact.entity2;

        if let Ok((ball1, pos1, lv1, av1)) = ballsize_q.get(entity1) {
            if let Ok((ball2, pos2, lv2, av2)) = ballsize_q.get(entity2) {
                if ball1.0 == ball2.0 {
                    let size = ball1.0 + 1;

                    if size >= ball_sizes.0.len() {
                        continue;
                    }

                    multiplier.0 += 1;

                    score.0 += size * multiplier.0;

                    // Magic numbers to stop insane velocities
                    let _lv = (lv1.0 + lv2.0) / 10.0;
                    let av = (av1.0 + av2.0) / 4.0;

                    //println!("AV {:?}, LV {:?}, POS {:?}", av, lv, position);

                    let position = (pos1.0 + pos2.0) / 2.0;

                    commands
                        .spawn(new_ball(position, size, &ball_sizes))
                        .insert(AngularVelocity(av));

                    commands.entity(entity1).despawn();
                    commands.entity(entity2).despawn();

                    if !sound_toggle.0 {
                        return;
                    }
                    let speed = lerp(0.2, 1.2, 1.0 - (size as f32 / SIZE_COUNT as f32));
                    commands
                        .spawn(AudioBundle {
                            source: audio_handles.merge.clone_weak(),
                            settings: PlaybackSettings { speed, ..default() },
                            ..default()
                        })
                        .insert(KillMeTimer(Timer::from_seconds(0.5, TimerMode::Once)));
                    // one merge per frame to prevent doubling stuffs
                    return;
                }
            }
        }
    }
}

fn new_ball(
    pos: Vec2,
    size: usize,
    ball_sizes: &BallSizes,
) -> (
    RigidBody,
    Collider,
    MaterialMesh2dBundle<ColorMaterial>,
    Position,
    LinearDamping,
    AngularDamping,
    BallSize,
    Friction,
    RunningTag,
    SettleTimer,
    Restitution,
    Handle<Image>,
    Sprite,
) {
    let radius = ball_sizes.0[size].0;
    let matmesh = MaterialMesh2dBundle {
        mesh: ball_sizes.0[size].1.clone_weak().into(),
        material: ball_sizes.0[size].2.clone_weak(),
        transform: Transform::from_scale(Vec3::new(
            (2.1 / 512.0) * radius,
            (2.1 / 512.0) * radius,
            1.0,
        )),
        ..default()
    };
    (
        RigidBody::Dynamic,
        Collider::ball(radius),
        matmesh,
        Position(pos),
        LinearDamping(LINEAR_DAMPING),
        AngularDamping(ANGULAR_DAMPING),
        BallSize(size),
        Friction::new(FRICTION),
        RunningTag,
        SettleTimer(Timer::from_seconds(OVERTOP_TIMER, TimerMode::Once)),
        Restitution::new(RESTITUTION),
        ball_sizes.0[size].3.clone_weak(),
        Sprite {
            //custom_size: Some(Vec2::splat(radius * 2.0)),
            ..default()
        },
    )
}

#[derive(Resource, Default)]
struct NextBallSize(usize);
#[derive(Resource, Default)]
struct NextNextBallSize(usize);

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
enum NextBallState {
    Pick,
    #[default]
    Selected,
}

fn set_next_size(
    mut next_size: ResMut<NextBallSize>,
    mut next_next_size: ResMut<NextNextBallSize>,
    mut next_state: ResMut<NextState<NextBallState>>,
) {
    let x: usize = fastrand::usize(..SIZE_COUNT / 2); //rng.gen_range(0..SIZE_COUNT / 2);
    next_size.0 = next_next_size.0;
    next_next_size.0 = x;
    next_state.0 = Some(NextBallState::Selected);
}

#[derive(Resource, Default)]
struct Score(usize);

#[derive(Component)]
struct SettleTimer(Timer);

fn check_over_top(
    mut ball_q: Query<(&Position, &mut SettleTimer, &BallSize), Without<FakeBall>>,
    ball_sizes: Res<BallSizes>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (pos, mut timer, size) in ball_q.iter_mut() {
        let ball_top = pos.y + ball_sizes.0[size.0].0;
        if ball_top > DEATH_LINE {
            timer.0.tick(time.delta());
            if timer.0.finished() {
                next_state.0 = Some(GameState::GameOver);
            }
        } else {
            timer.0.reset();
        }
    }
}
