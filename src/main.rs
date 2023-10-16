use std::f32::consts::PI;

use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_xpbd_2d::prelude::*;
use rand::prelude::*;

const DROP_LINE: f32 = 3.0;
const SIZE_COUNT: usize = 11;

const BOX_WIDTH: f32 = 4.4;
const BOX_HEIGHT: f32 = 5.0;

const MAX_RADIUS: f32 = (BOX_WIDTH / 2.2) / 2.0;
const MIN_RADIUS: f32 = 0.15;

const LINEAR_DAMPING: f32 = 10.0;
const FRICTION: f32 = 1.0;

const G: f32 = 90.0;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
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
                )
                    .run_if(in_state(GameState::Running)),
                enter_running.run_if(in_state(GameState::Splash)),
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
        .init_resource::<Score>()
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

#[derive(Resource)]
struct BallSizes(Vec<(f32, Handle<Mesh>, Handle<ColorMaterial>)>);

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

fn build_splash() {}

fn build_running(mut score: ResMut<Score>, mut commands: Commands) {
    score.0 = 0;

    commands
        .spawn(
            TextBundle::from_section(
                "Score: 0",
                TextStyle {
                    font_size: 30.0,
                    ..default()
                },
            )
            .with_style(Style {
                left: Val::Px(5.0),
                top: Val::Px(5.0),
                ..default()
            }),
        )
        .insert((ScoreTag, RunningTag));
}

fn update_score(score: Res<Score>, mut ui_q: Query<&mut Text, With<ScoreTag>>) {
    if !score.is_changed() {
        return;
    }

    if let Ok(mut text) = ui_q.get_single_mut() {
        text.sections[0].value = format!("Score: {}", score.0);
    }
}

fn build_gameover() {}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut window_q: Query<&mut Window>,
) {
    let mut window = window_q.single_mut();

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
        let mat = materials.add(ColorMaterial::from(Color::rgb(
            0.5,
            0.5,
            i as f32 / SIZE_COUNT as f32,
        )));

        ball_sizes.push((radius, mesh, mat));
    }

    commands.insert_resource(BallSizes(ball_sizes));
}

fn add_walls(mut commands: Commands) {
    let square_sprite = Sprite {
        color: Color::rgb_u8(200, 200, 200),
        custom_size: Some(Vec2::splat(1.0)),
        ..default()
    };

    let wall_thickness = 1.0;
    let top_offset = -0.8;

    // floor
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(BOX_WIDTH + wall_thickness, wall_thickness),
        Position(Vec2::new(0.0, top_offset - BOX_HEIGHT / 2.0)),
        SpriteBundle {
            sprite: square_sprite.clone(),
            transform: Transform::from_scale(Vec3::new(
                BOX_WIDTH + wall_thickness,
                wall_thickness,
                1.0,
            )),
            ..default()
        },
        RunningTag,
    ));

    // left
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(wall_thickness, BOX_HEIGHT),
        Position(Vec2::new(-BOX_WIDTH / 2.0, top_offset)),
        SpriteBundle {
            sprite: square_sprite.clone(),
            transform: Transform::from_scale(Vec3::new(wall_thickness, BOX_HEIGHT, 1.0)),
            ..default()
        },
        RunningTag,
    ));

    // right
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(wall_thickness, BOX_HEIGHT),
        Position(Vec2::new(BOX_WIDTH / 2.0, top_offset)),
        SpriteBundle {
            sprite: square_sprite.clone(),
            transform: Transform::from_scale(Vec3::new(wall_thickness, BOX_HEIGHT, 1.0)),
            ..default()
        },
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
) {
    if !mouse.just_pressed(MouseButton::Left) || fake_ball_q.is_empty() {
        return;
    }

    next_ball_timer.0.reset();

    let size = next_ball_size.0;

    if let Ok((entity, position)) = fake_ball_q.get_single_mut() {
        commands.spawn(new_ball(position.0, size, &ball_sizes));

        commands.entity(entity).despawn();

        next_ball_state.0 = Some(NextBallState::Pick);
    }
}

fn tick_next_ball(
    mut next_ball_timer: ResMut<NextBallTimer>,
    time: Res<Time>,
    mut commands: Commands,
    ball_sizes: Res<BallSizes>,
    fake_ball_q: Query<&FakeBall>,
    next_ball_size: Res<NextBallSize>,
) {
    if next_ball_timer.0.finished() && fake_ball_q.is_empty() {
        commands.spawn((
            RigidBody::Static,
            FakeBall,
            MaterialMesh2dBundle {
                mesh: ball_sizes.0[next_ball_size.0].1.clone().into(),
                material: ball_sizes.0[next_ball_size.0].2.clone(),
                ..default()
            },
            BallSize(next_ball_size.0),
            Position(Vec2::new(0.0, DROP_LINE)),
            RunningTag,
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

fn merge_on_collision(
    mut collision_event_reader: EventReader<Collision>,
    ballsize_q: Query<(&BallSize, &Position, &LinearVelocity, &AngularVelocity)>,
    mut commands: Commands,
    ball_sizes: Res<BallSizes>,
    mut score: ResMut<Score>,
) {
    for Collision(contact) in collision_event_reader.iter() {
        // Check BallSize component on entities. If present and equal, remove the two contacting
        // entities and spawn a ball with the next size at the midpoint of the contacting ball's
        // positions.
        //println!("{:?} + {:?} contacting", contact.entity1, contact.entity2);
        let entity1 = contact.entity1;
        let entity2 = contact.entity2;

        if let Ok((ball1, pos1, lv1, av1)) = ballsize_q.get(entity1) {
            if let Ok((ball2, pos2, lv2, av2)) = ballsize_q.get(entity2) {
                if ball1.0 == ball2.0 {
                    let size = ball1.0 + 1;

                    if size >= ball_sizes.0.len() {
                        continue;
                    }

                    score.0 += size;

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
    BallSize,
    Friction,
    RunningTag,
) {
    let radius = ball_sizes.0[size].0;
    let matmesh = MaterialMesh2dBundle {
        mesh: ball_sizes.0[size].1.clone().into(),
        material: ball_sizes.0[size].2.clone(),
        ..default()
    };
    (
        RigidBody::Dynamic,
        Collider::ball(radius),
        matmesh,
        Position(pos),
        LinearDamping(LINEAR_DAMPING),
        BallSize(size),
        Friction::new(FRICTION),
        RunningTag,
    )
}

#[derive(Resource, Default)]
struct NextBallSize(usize);

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
enum NextBallState {
    Pick,
    #[default]
    Selected,
}

fn set_next_size(
    mut next_size: ResMut<NextBallSize>,
    mut next_state: ResMut<NextState<NextBallState>>,
) {
    let mut rng = rand::thread_rng();
    let x: usize = rng.gen_range(0..SIZE_COUNT / 2);
    next_size.0 = x;
    next_state.0 = Some(NextBallState::Selected);
}

#[derive(Resource, Default)]
struct Score(usize);
