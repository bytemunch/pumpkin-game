use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_xpbd_2d::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .insert_resource(Gravity(Vec2::NEG_Y * 9.81))
        .add_systems(Startup, setup)
        .add_systems(Update, (track_ball_position, play))
        .add_systems(OnEnter(AppState::Running), resume)
        .add_systems(OnExit(AppState::Running), pause)
        .add_state::<AppState>()
        .add_state::<GameState>()
        .run();
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
enum AppState {
    #[default]
    Paused,
    Running,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
enum GameState {
    #[default]
    Splash,
    Running,
    GameOver,
}

fn play(keys: Res<Input<KeyCode>>, mut next_state: ResMut<NextState<AppState>>) {
    if keys.just_pressed(KeyCode::Space) {
        next_state.0 = Some(AppState::Running)
    }
}

#[derive(Component)]
struct TrackMe;

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut window_q: Query<&mut Window>,
    mut physics_loop: ResMut<PhysicsLoop>,
) {
    physics_loop.pause();

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

    let radius = 0.5;

    commands.spawn((
        RigidBody::Dynamic,
        Collider::ball(radius),
        TrackMe,
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(radius).into()).into(),
            material: materials.add(ColorMaterial::from(Color::rgb_u8(128, 128, 255))),
            ..default()
        },
        Position(Vec2::new(0., 2.)),
    ));
}

fn track_ball_position(ball_q: Query<&Position, With<TrackMe>>) {
    for b in ball_q.iter() {
        println!("{:?}", b);
    }
}
