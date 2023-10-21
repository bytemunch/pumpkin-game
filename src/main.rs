use std::f32::{consts::PI, INFINITY};

use bevy::{
    audio::{PlaybackMode, Volume, VolumeLevel},
    input::touch::TouchPhase,
    prelude::*,
    sprite::MaterialMesh2dBundle,
    window::WindowResized,
};
use bevy_xpbd_2d::prelude::*;

const DROP_LINE: f32 = 3.0;

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
            DefaultPlugins
                .set(ImagePlugin::default_linear())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                }),
            PhysicsPlugins::default(),
        ))
        .insert_resource(Gravity(Vec2::NEG_Y * G))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (
                    set_ball_sizes,
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
                    change_quality,
                    adaptive_quality,
                    spawn_ball,
                )
                    .run_if(in_state(GameState::Running)),
                enter_running.run_if(in_state(GameState::Splash)),
                enter_running.run_if(in_state(GameState::GameOver)),
                music_button,
                sfx_button,
                do_kill_me,
                set_scale_from_window,
                play_button,
                tick_debounce,
                get_framerate,
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
        .init_resource::<BallSizes>()
        .init_resource::<Framerate>()
        .init_resource::<AdaptiveQualityTimer>()
        .insert_resource(NextBallTimer(Timer::from_seconds(0.5, TimerMode::Once)))
        .insert_resource(DebounceTimer(Timer::from_seconds(0.3, TimerMode::Once)))
        .add_state::<AppState>()
        .add_state::<GameState>()
        .add_state::<NextBallState>()
        .add_event::<SpawnBallEvent>()
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

#[derive(Component)]
struct UiRoot;

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

fn change_quality(keys: Res<Input<KeyCode>>, mut quality: ResMut<Quality>) {
    if keys.just_pressed(KeyCode::Q) {
        quality.0 = match quality.0 {
            32 => 64,
            64 => 128,
            128 => 256,
            256 => 512,
            _ => 32,
        }
    }
}

#[derive(Component)]
struct MusicTag;

#[derive(Resource)]
struct MusicToggle(bool);

#[derive(Resource)]
struct SoundToggle(bool);

#[derive(Resource)]
struct BallSizes(Vec<(f32, Handle<Mesh>, Handle<ColorMaterial>)>);

impl Default for BallSizes {
    fn default() -> Self {
        BallSizes(vec![
            (0.0, Handle::default(), Handle::default(),);
            BALL_ORDER.len()
        ])
    }
}

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

fn set_scale_from_window(
    mut ev: EventReader<WindowResized>,
    mut projection: Query<&mut OrthographicProjection>,
) {
    for e in ev.iter() {
        let mut camera_scale = 1. / (e.width / 480.) * (1. / 100.);

        camera_scale = camera_scale.max(1. / (e.height / 720.) * (1. / 100.));

        projection.single_mut().scale = camera_scale;
    }
}

#[derive(Component)]
struct ScoreTag;

fn build_splash(mut commands: Commands, font: Res<CustomFont>) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    flex_wrap: FlexWrap::Wrap,
                    padding: UiRect::top(Val::Px(50.0)),
                    ..default()
                },
                background_color: Color::rgb_u8(52, 52, 52).into(),
                ..default()
            },
            SplashTag,
        ))
        .with_children(|root| {
            root.spawn((TextBundle::from_sections([TextSection {
                value: "Pumpkin Game!\n\n\n\n".into(),
                style: TextStyle {
                    font_size: 40.0,
                    font: font.0.clone_weak(),
                    ..default()
                },
            }])
            .with_text_alignment(TextAlignment::Center)
            .with_style(Style { ..default() }),));

            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                ..default()
            })
            .with_children(|button_box| {
                button_box
                    .spawn((
                        ButtonBundle {
                            background_color: Color::GREEN.into(),
                            border_color: Color::DARK_GREEN.into(),
                            style: Style {
                                width: Val::Px(150.0),
                                height: Val::Px(64.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(5.0)),
                                flex_basis: Val::Percent(100.0),
                                max_width: Val::Px(150.0),
                                margin: UiRect::all(Val::Px(15.0)),
                                ..default()
                            },

                            ..default()
                        },
                        PlayButton,
                    ))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "Play",
                            TextStyle {
                                font: font.0.clone_weak(),
                                font_size: 30.0,
                                color: Color::WHITE,
                            },
                        ));
                    });

                button_box
                    .spawn((
                        ButtonBundle {
                            background_color: Color::BLUE.into(),
                            border_color: Color::MIDNIGHT_BLUE.into(),
                            style: Style {
                                width: Val::Px(150.0),
                                height: Val::Px(64.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(5.0)),
                                flex_basis: Val::Percent(100.0),
                                max_width: Val::Px(150.0),
                                margin: UiRect::all(Val::Px(15.0)),
                                ..default()
                            },

                            ..default()
                        },
                        MusicButton,
                    ))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "Music",
                            TextStyle {
                                font: font.0.clone_weak(),
                                font_size: 30.0,
                                color: Color::WHITE,
                            },
                        ));
                    });
                button_box
                    .spawn((
                        ButtonBundle {
                            background_color: Color::BLUE.into(),
                            border_color: Color::MIDNIGHT_BLUE.into(),
                            style: Style {
                                width: Val::Px(150.0),
                                height: Val::Px(64.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(5.0)),
                                flex_basis: Val::Percent(100.0),
                                max_width: Val::Px(150.0),
                                margin: UiRect::all(Val::Px(15.0)),
                                ..default()
                            },

                            ..default()
                        },
                        SfxButton,
                    ))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "Sounds",
                            TextStyle {
                                font: font.0.clone_weak(),
                                font_size: 30.0,
                                color: Color::WHITE,
                            },
                        ));
                    });
            });
        });
}

#[derive(Component)]
struct PlayButton;

#[derive(Component)]
struct MusicButton;

#[derive(Component)]
struct SfxButton;

fn play_button(
    button_q: Query<&Interaction, With<PlayButton>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Ok(interaction) = button_q.get_single() {
        if *interaction == Interaction::Pressed {
            next_state.0 = Some(GameState::Running);
        }
    }
}

fn tick_debounce(mut timer: ResMut<DebounceTimer>, time: Res<Time>) {
    timer.0.tick(time.delta());
}

fn music_button(
    button_q: Query<&Interaction, With<MusicButton>>,
    keys: Res<Input<KeyCode>>,
    bgm_q: Query<&mut AudioSink, With<MusicTag>>,
    mut toggle: ResMut<MusicToggle>,
    mut debounce: ResMut<DebounceTimer>,
) {
    if keys.just_pressed(KeyCode::M) {
        toggle.0 = !toggle.0;
        if let Ok(sink) = bgm_q.get_single() {
            sink.toggle();
        }
        return;
    }
    if let Ok(interaction) = button_q.get_single() {
        // TODO: wait for unpress instead of this debouncing
        // maybe match does this automatically?
        if *interaction == Interaction::Pressed && debounce.0.finished() {
            debounce.0.reset();
            toggle.0 = !toggle.0;
            if let Ok(sink) = bgm_q.get_single() {
                sink.toggle();
            }
        }
    }
}
fn sfx_button(
    keys: Res<Input<KeyCode>>,
    button_q: Query<&Interaction, With<SfxButton>>,
    mut toggle: ResMut<SoundToggle>,
    mut debounce: ResMut<DebounceTimer>,
) {
    if let Ok(interaction) = button_q.get_single() {
        if *interaction == Interaction::Pressed && debounce.0.finished() {
            debounce.0.reset();
            toggle.0 = !toggle.0;
            return;
        }
    }

    if keys.just_pressed(KeyCode::S) {
        toggle.0 = !toggle.0;
    }
}

fn build_running(
    mut score: ResMut<Score>,
    mut commands: Commands,
    next_ball_size: Res<NextBallSize>,
    font: Res<CustomFont>,
    bgm_q: Query<&AudioSink, With<MusicTag>>,
    bgm_toggle: Res<MusicToggle>,
    mut next_ball_timer: ResMut<NextBallTimer>,
    ball_images: Res<BallImageHandles>,
    quality: Res<Quality>,
) {
    next_ball_timer.0.reset();

    score.0 = 0;

    if let Ok(sink) = bgm_q.get_single() {
        if bgm_toggle.0 {
            sink.play();
        }
    }

    let margin = UiRect {
        left: Val::Px(10.0),
        right: Val::Px(10.0),
        top: Val::Px(10.0),
        bottom: Val::Px(10.0),
    };

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    justify_items: JustifyItems::Start,
                    ..default()
                },
                ..default()
            },
            RunningTag,
        ))
        .with_children(|root| {
            root.spawn((
                TextBundle::from_section(
                    "Score: 0",
                    TextStyle {
                        font_size: 30.0,
                        font: font.0.clone_weak(),
                        ..default()
                    },
                )
                .with_style(Style {
                    margin: margin.clone(),
                    ..default()
                }),
                ScoreTag,
            ));

            root.spawn((NodeBundle {
                style: Style {
                    width: Val::Px(50.0),
                    height: Val::Px(50.0 + 20.0),
                    margin,
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                ..default()
            },))
                .with_children(|next| {
                    next.spawn((TextBundle::from_section(
                        "Next:",
                        TextStyle {
                            font_size: 30.0,
                            font: font.0.clone_weak(),
                            ..default()
                        },
                    )
                    .with_style(Style { ..default() }),));

                    next.spawn((
                        NodeBundle {
                            background_color: Color::WHITE.into(),
                            style: Style {
                                width: Val::Px(50.0),
                                height: Val::Px(50.0),
                                ..default()
                            },
                            ..default()
                        },
                        UiImage::new(
                            ball_images.0[q_idx(quality.0)].0[next_ball_size.0].clone_weak(),
                        ),
                        NextUpTag,
                    ));
                });
        });
}

#[derive(Component)]
struct NextUpTag;

fn update_next_up(
    next_q: Query<Entity, With<NextUpTag>>,
    next_ball_size: Res<NextNextBallSize>,
    mut commands: Commands,
    ball_images: Res<BallImageHandles>,
    quality: Res<Quality>,
) {
    if !next_ball_size.is_changed() {
        return;
    }

    if let Ok(entity) = next_q.get_single() {
        commands.entity(entity).insert(UiImage::new(
            ball_images.0[q_idx(quality.0)].0[next_ball_size.0].clone_weak(),
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
    let gameover_text = (TextBundle::from_sections([
        TextSection {
            value: "Skill Issue\n".into(),
            style: style.clone(),
        },
        TextSection {
            value: score_string,
            style: style.clone(),
        },
    ])
    .with_text_alignment(TextAlignment::Center),);

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    flex_wrap: FlexWrap::Wrap,
                    padding: UiRect::top(Val::Px(15.0)),
                    ..default()
                },
                background_color: Color::rgb_u8(52, 52, 52).into(),
                ..default()
            },
            GameOverTag,
        ))
        .with_children(|root| {
            root.spawn(gameover_text);
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                ..default()
            })
            .with_children(|button_box| {
                button_box
                    .spawn((
                        ButtonBundle {
                            background_color: Color::GREEN.into(),
                            border_color: Color::DARK_GREEN.into(),
                            style: Style {
                                width: Val::Px(200.0),
                                height: Val::Px(64.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(5.0)),
                                flex_basis: Val::Percent(100.0),
                                max_width: Val::Px(150.0),
                                margin: UiRect::all(Val::Px(15.0)),
                                ..default()
                            },

                            ..default()
                        },
                        PlayButton,
                    ))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "Play Again",
                            TextStyle {
                                font: font.0.clone_weak(),
                                font_size: 30.0,
                                color: Color::WHITE,
                            },
                        ));
                    });
            });
        });

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

#[derive(Resource)]
struct Quality(usize);

#[derive(Resource, Default)]
struct Framerate(f32);

const FRAME_SMOOTHING: f32 = 0.99;

fn get_framerate(mut framerate: ResMut<Framerate>, time: Res<Time>) {
    let mut current = 1.0 / time.delta().as_secs_f32();

    if current == INFINITY {
        current = 1.0;
    }

    framerate.0 = (framerate.0 * FRAME_SMOOTHING) + (current * (1.0 - FRAME_SMOOTHING));

    //web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(
    //    format!("CFR: {} | FR: {}", current, framerate.0).as_str(),
    //));
}

#[derive(Resource)]
struct AdaptiveQualityTimer(Timer);

impl Default for AdaptiveQualityTimer {
    fn default() -> Self {
        AdaptiveQualityTimer(Timer::from_seconds(5.0, TimerMode::Once))
    }
}

fn adaptive_quality(
    mut quality: ResMut<Quality>,
    frames: Res<Framerate>,
    mut debounce: ResMut<AdaptiveQualityTimer>,
    time: Res<Time>,
) {
    debounce.0.tick(time.delta());

    if !(debounce.0.finished()) {
        return;
    }

    if frames.0 >= 59.0 && quality.0 < 512 {
        quality.0 <<= 1;
        debounce.0.reset();
    }

    if frames.0 <= 45.0 && quality.0 > 32 {
        quality.0 >>= 1;
        debounce.0.reset();
    }
}

fn set_ball_sizes(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut ball_sizes: ResMut<BallSizes>,
    quality: Res<Quality>,
    mut ball_q: Query<(&BallSize, &mut Handle<Image>)>,
    mut next_up_q: Query<&mut UiImage, With<NextUpTag>>,
    next_up: Res<NextBallSize>,
    ball_images: Res<BallImageHandles>,
) {
    if !quality.is_changed() && !quality.is_added() {
        return;
    }

    for i in 1..=BALL_ORDER.len() {
        let radius = lerp(
            MIN_RADIUS,
            MAX_RADIUS,
            ease_in_sine(i as f32 / BALL_ORDER.len() as f32),
        );
        let mesh = meshes.add(shape::Circle::new(radius).into());
        let mat = materials.add(ColorMaterial::from(Color::rgba(
            0.5,
            0.5,
            i as f32 / BALL_ORDER.len() as f32,
            0.0, // DEBUG:  remove this line to see collider
        )));

        ball_sizes.0[i - 1] = (radius, mesh, mat);
    }

    for (size, mut img) in ball_q.iter_mut() {
        *img = ball_images.0[q_idx(quality.0)].0[size.0].clone_weak();
    }

    for mut img in next_up_q.iter_mut() {
        img.texture = ball_images.0[q_idx(quality.0)].0[next_up.0].clone_weak();
    }
}

// TODO: this on impl Quality
fn q_idx(i: usize) -> usize {
    (i.ilog2() - 5) as usize
}

/// quality then size
#[derive(Clone)]
struct BallImageHandleList(Vec<Handle<Image>>);

/// quality then size
#[derive(Resource)]
struct BallImageHandles(Vec<BallImageHandleList>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut image_handles = vec![BallImageHandleList(vec![Handle::default(); BALL_ORDER.len()]); 5];

    for i in 5..=9 {
        let quality = 2usize.pow(i);
        let idx = i as usize - 5;
        for j in 0..BALL_ORDER.len() {
            let name = BALL_ORDER[j];
            image_handles[idx].0[j] = asset_server.load(format!("{}@{}.png", name, quality));
        }
    }

    commands.insert_resource(BallImageHandles(image_handles));

    commands.insert_resource(ClearColor(Color::rgb_u8(52, 52, 52)));

    commands.insert_resource(Quality(512)); //start at max quality, drop as needed

    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            near: -1.0,
            far: 1000.0,
            scale: 0.01,
            ..default()
        },
        ..default()
    });

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
    mut fake_ball_q: Query<(&mut Transform, &BallSize), With<FakeBall>>,
    cursor: Res<CursorWorldPos>,
    ball_sizes: Res<BallSizes>,
) {
    if let Ok((mut transform, size)) = fake_ball_q.get_single_mut() {
        let max = BOX_WIDTH / 2.0 - ball_sizes.0[size.0].0 - 0.5; // wall_thickness
        let min = -BOX_WIDTH / 2.0 + ball_sizes.0[size.0].0 + 0.5;
        transform.translation.x = cursor.0.x.clamp(min, max);
    }
}

#[derive(Resource)]
struct NextBallTimer(Timer);
#[derive(Resource)]
struct DebounceTimer(Timer);

fn release_ball(
    mut next_ball_timer: ResMut<NextBallTimer>,
    mut fake_ball_q: Query<(Entity, &Transform), With<FakeBall>>,
    mut commands: Commands,
    mouse: Res<Input<MouseButton>>,
    mut touch_evr: EventReader<TouchInput>,
    mut next_ball_state: ResMut<NextState<NextBallState>>,
    next_ball_size: Res<NextBallSize>,
    mut multiplier: ResMut<Multiplier>,
    audio_handles: Res<AudioHandles>,
    sound_toggle: Res<SoundToggle>,
    mut ew: EventWriter<SpawnBallEvent>,
) {
    let mut touch_ended = false;

    for touch in touch_evr.iter() {
        if touch.phase != TouchPhase::Ended {
            continue;
        } else {
            touch_ended = true;
            break;
        }
    }

    if !touch_ended && !mouse.just_pressed(MouseButton::Left) || fake_ball_q.is_empty() {
        return;
    }

    multiplier.0 = 0;

    next_ball_timer.0.reset();

    let size = next_ball_size.0;

    if let Ok((entity, position)) = fake_ball_q.get_single_mut() {
        let av = -1.0 + fastrand::f32() * 2.0;

        ew.send(SpawnBallEvent {
            position: position.translation.truncate(),
            size,
            av,
        });

        commands.entity(entity).despawn();

        next_ball_state.0 = Some(NextBallState::Pick);

        if !sound_toggle.0 {
            return;
        }

        let speed = lerp(0.2, 1.2, 1.0 - (size as f32 / BALL_ORDER.len() as f32));
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
    ball_images: Res<BallImageHandles>,
    quality: Res<Quality>,
) {
    if next_ball_timer.0.finished() && fake_ball_q.is_empty() {
        let radius = ball_sizes.0[next_ball_size.0].0;

        commands.spawn((
            FakeBall,
            BallSize(next_ball_size.0),
            RunningTag,
            SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(radius * 2.0)),
                    ..default()
                },
                transform: Transform::from_xyz(cursor.0.x, DROP_LINE, 0.0),
                texture: ball_images.0[q_idx(quality.0)].0[next_ball_size.0].clone_weak(),
                ..default()
            },
        ));

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
    mut touches_evr: EventReader<TouchInput>,
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
    } else {
        for touch in touches_evr.iter() {
            if let Some(world_position) = camera
                .viewport_to_world(camera_transform, touch.position)
                .map(|ray| ray.origin.truncate())
            {
                pos.0 = world_position;
            }
        }
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
    mut ew: EventWriter<SpawnBallEvent>,
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

                    ew.send(SpawnBallEvent { position, size, av });

                    commands.entity(entity1).despawn();
                    commands.entity(entity2).despawn();

                    if !sound_toggle.0 {
                        return;
                    }
                    let speed = lerp(0.2, 1.2, 1.0 - (size as f32 / BALL_ORDER.len() as f32));
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

#[derive(Event)]
struct SpawnBallEvent {
    position: Vec2,
    size: usize,
    av: f32,
}

fn spawn_ball(
    mut er: EventReader<SpawnBallEvent>,
    ball_images: Res<BallImageHandles>,
    ball_sizes: Res<BallSizes>,
    quality: Res<Quality>,
    mut commands: Commands,
) {
    for ev in er.iter() {
        let radius = ball_sizes.0[ev.size].0;

        let matmesh = MaterialMesh2dBundle {
            mesh: ball_sizes.0[ev.size].1.clone_weak().into(),
            material: ball_sizes.0[ev.size].2.clone_weak(),
            ..default()
        };

        commands.spawn((
            RigidBody::Dynamic,
            Collider::ball(radius),
            matmesh,
            Position(ev.position),
            LinearDamping(LINEAR_DAMPING),
            AngularDamping(ANGULAR_DAMPING),
            BallSize(ev.size),
            Friction::new(FRICTION),
            RunningTag,
            SettleTimer(Timer::from_seconds(OVERTOP_TIMER, TimerMode::Once)),
            Restitution::new(RESTITUTION),
            ball_images.0[q_idx(quality.0)].0[ev.size].clone_weak(),
            AngularVelocity(ev.av),
            Sprite {
                custom_size: Some(Vec2::splat(radius * 2.0)),
                ..default()
            },
        ));
    }
}

#[derive(Resource, Default)]
struct NextBallSize(usize);

#[derive(Resource)]
struct NextNextBallSize(usize);

impl Default for NextNextBallSize {
    fn default() -> Self {
        Self(fastrand::usize(..3))
    }
}

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
    let x: usize = fastrand::usize(..BALL_ORDER.len() / 2); //rng.gen_range(0..SIZE_COUNT / 2);
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
