use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_xpbd_2d::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .insert_resource(Gravity(Vec2::NEG_Y * 29.81))
        .add_systems(Startup, (setup, add_walls))
        .add_systems(
            Update,
            (
                play,
                fake_ball_follow_mouse,
                cursor_to_world,
                release_ball,
                tick_next_ball,
                merge_on_collision,
            ),
        )
        .add_systems(OnEnter(AppState::Running), resume)
        .add_systems(OnExit(AppState::Running), pause)
        .init_resource::<CursorWorldPos>()
        .insert_resource(NextBallTimer(Timer::from_seconds(0.5, TimerMode::Once)))
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

#[derive(Resource)]
struct BallSizes(Vec<(f32, Handle<Mesh>, Handle<ColorMaterial>)>);

#[derive(Component)]
struct BallSize(usize);

const DROP_LINE: f32 = 3.0;

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

    let mut ball_sizes = vec![];

    let max_radius = 0.5;
    let size_count = 9;
    for i in 1..size_count + 1 {
        let radius = max_radius * (i as f32 / size_count as f32);
        let mesh = meshes.add(shape::Circle::new(radius).into());
        let mat = materials.add(ColorMaterial::from(Color::rgb(
            0.5,
            0.5,
            i as f32 / size_count as f32,
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

    let floor_width = 4.0;
    let wall_thickness = 0.3;
    let wall_height = 5.0;
    let top_offset = -0.8;

    // floor
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(floor_width + wall_thickness, wall_thickness),
        Position(Vec2::new(0.0, top_offset - wall_height / 2.0)),
        SpriteBundle {
            sprite: square_sprite.clone(),
            transform: Transform::from_scale(Vec3::new(
                floor_width + wall_thickness,
                wall_thickness,
                1.0,
            )),
            ..default()
        },
    ));

    // left
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(wall_thickness, wall_height),
        Position(Vec2::new(-floor_width / 2.0, top_offset)),
        SpriteBundle {
            sprite: square_sprite.clone(),
            transform: Transform::from_scale(Vec3::new(wall_thickness, wall_height, 1.0)),
            ..default()
        },
    ));

    // right
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(wall_thickness, wall_height),
        Position(Vec2::new(floor_width / 2.0, top_offset)),
        SpriteBundle {
            sprite: square_sprite.clone(),
            transform: Transform::from_scale(Vec3::new(wall_thickness, wall_height, 1.0)),
            ..default()
        },
    ));
}

#[derive(Component)]
struct FakeBall;

fn fake_ball_follow_mouse(
    mut fake_ball_q: Query<&mut Position, With<FakeBall>>,
    cursor: Res<CursorWorldPos>,
) {
    if let Ok(mut ball) = fake_ball_q.get_single_mut() {
        ball.x = cursor.0.x;
    }
}

#[derive(Resource)]
struct NextBallTimer(Timer);

const SIZE: usize = 4;

fn release_ball(
    mut next_ball_timer: ResMut<NextBallTimer>,
    mut fake_ball_q: Query<(Entity, &Position), With<FakeBall>>,
    mut commands: Commands,
    mouse: Res<Input<MouseButton>>,
    ball_sizes: Res<BallSizes>,
) {
    if !mouse.just_pressed(MouseButton::Left) || fake_ball_q.is_empty() {
        return;
    }

    next_ball_timer.0.reset();

    if let Ok((entity, position)) = fake_ball_q.get_single_mut() {
        let radius = ball_sizes.0[SIZE].0;

        let matmesh = MaterialMesh2dBundle {
            mesh: ball_sizes.0[SIZE].1.clone().into(),
            material: ball_sizes.0[SIZE].2.clone(),
            ..default()
        };

        commands.spawn((
            RigidBody::Dynamic,
            Collider::ball(radius),
            matmesh.clone(),
            Position(position.0),
            BallSize(SIZE),
            LinearDamping(5.0),
        ));

        commands.entity(entity).despawn();
    }
}

fn tick_next_ball(
    mut next_ball_timer: ResMut<NextBallTimer>,
    time: Res<Time>,
    mut commands: Commands,
    ball_sizes: Res<BallSizes>,
    fake_ball_q: Query<&FakeBall>,
) {
    if next_ball_timer.0.finished() && fake_ball_q.is_empty() {
        commands.spawn((
            RigidBody::Static,
            FakeBall,
            MaterialMesh2dBundle {
                mesh: ball_sizes.0[SIZE].1.clone().into(),
                material: ball_sizes.0[SIZE].2.clone(),
                ..default()
            },
            Position(Vec2::new(0.0, DROP_LINE)),
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
                        return;
                    }

                    let radius = ball_sizes.0[size].0;

                    let matmesh = MaterialMesh2dBundle {
                        mesh: ball_sizes.0[size].1.clone().into(),
                        material: ball_sizes.0[size].2.clone(),
                        ..default()
                    };

                    // Magic numbers to stop insane velocities
                    let position = (pos1.0 + pos2.0) / 2.0;
                    let _lv = (lv1.0 + lv2.0) / 10.0;
                    let av = (av1.0 + av2.0) / 4.0;

                    //println!("AV {:?}, LV {:?}, POS {:?}", av, lv, position);

                    commands.spawn((
                        RigidBody::Dynamic,
                        Collider::ball(radius),
                        matmesh.clone(),
                        Position(position),
                        //LinearVelocity(lv),
                        AngularVelocity(av),
                        LinearDamping(5.0),
                        BallSize(size),
                    ));

                    commands.entity(entity1).despawn();
                    commands.entity(entity2).despawn();
                    // one merge per frame to prevent doubling stuffs
                    return;
                }
            }
        }
    }
}
