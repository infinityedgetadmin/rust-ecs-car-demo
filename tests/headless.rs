//! Headless end-to-end tests: real ECS systems, simulated keyboard, manual clock. No window
//! or GPU is needed, so these run in CI on every platform.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use open_world_game::GameLogicPlugin;
use open_world_game::components::{Heading, Player, Route, RunPhase, RunState, Velocity};
use open_world_game::systems::setup::player_car_bundle;
use std::time::Duration;

const DT: Duration = Duration::from_micros(16_667);

fn app_with(route: Route) -> App {
    let mut app = App::new();
    app.insert_resource(route)
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<Time>()
        .add_plugins(GameLogicPlugin);
    let route = *app.world().resource::<Route>();
    app.world_mut().spawn(player_car_bundle(&route));
    app
}

/// Advances the manual clock by one frame and runs `Update`.
fn step(app: &mut App) {
    app.world_mut().resource_mut::<Time>().advance_by(DT);
    app.update();
    // Clear just_pressed/just_released like Bevy's input plugin would.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
}

fn car(app: &mut App) -> (Transform, Velocity, Heading) {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Transform, &Velocity, &Heading), With<Player>>();
    let (t, v, h) = q.single(app.world()).unwrap();
    (*t, *v, *h)
}

fn keys(app: &mut App) -> Mut<'_, ButtonInput<KeyCode>> {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>()
}

#[test]
fn holding_w_drives_forward_then_damping_stops_the_car() {
    let mut app = app_with(Route::default());
    keys(&mut app).press(KeyCode::KeyW);
    for _ in 0..120 {
        step(&mut app);
    }
    let (t, v, _) = car(&mut app);
    assert!(
        t.translation.z < -15.0,
        "car should have moved toward -Z: {:?}",
        t.translation
    );
    assert!(v.length() > 15.0);

    keys(&mut app).release(KeyCode::KeyW);
    let mut frames = 0;
    while car(&mut app).1.0 != Vec3::ZERO {
        step(&mut app);
        frames += 1;
        assert!(frames < 60 * 60, "car never stopped");
    }
}

#[test]
fn steering_with_a_turns_left() {
    let mut app = app_with(Route::default());
    keys(&mut app).press(KeyCode::KeyW);
    keys(&mut app).press(KeyCode::KeyA);
    for _ in 0..90 {
        step(&mut app);
    }
    let (t, _, h) = car(&mut app);
    assert!(h.0 > 0.2, "heading {h:?}");
    assert!(t.translation.x < 0.0);
}

#[test]
fn reaching_b_finishes_the_run_and_r_resets() {
    // Put B straight ahead so holding W is enough.
    let route = Route {
        finish: Vec3::new(0.0, 0.0, -40.0),
        finish_radius: 5.0,
        ..Default::default()
    };
    let mut app = app_with(route);
    keys(&mut app).press(KeyCode::KeyW);
    let mut frames = 0;
    while app.world().resource::<RunState>().phase == RunPhase::Driving {
        step(&mut app);
        frames += 1;
        assert!(frames < 60 * 20, "never reached B");
    }
    let run = *app.world().resource::<RunState>();
    let RunPhase::Finished(t) = run.phase else {
        unreachable!()
    };
    assert!(t > 1.0 && t < 6.0, "finish time {t}");
    assert_eq!(run.best, Some(t));

    // Input is ignored after finishing: the car coasts rather than accelerating.
    let v_before = car(&mut app).1.length();
    for _ in 0..30 {
        step(&mut app);
    }
    assert!(car(&mut app).1.length() < v_before);

    keys(&mut app).release(KeyCode::KeyW);
    keys(&mut app).press(KeyCode::KeyR);
    step(&mut app);
    let (t, v, h) = car(&mut app);
    assert_eq!(t.translation, route.start);
    assert_eq!(v.0, Vec3::ZERO);
    assert_eq!(h.0, route.start_yaw);
    let run = *app.world().resource::<RunState>();
    assert_eq!(run.phase, RunPhase::Driving);
    assert!(run.best.is_some(), "best time survives reset");
}

/// Smoke test for the full game plugin without a window or GPU: the scene spawns, and the
/// camera and HUD systems run against it.
#[test]
fn full_game_plugin_spawns_scene_and_runs_headless() {
    use open_world_game::GamePlugin;
    use open_world_game::components::{ChaseCamera, GoalMarker, Hud};

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<bevy::mesh::Mesh>()
        .init_asset::<StandardMaterial>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_plugins(GamePlugin);
    keys(&mut app).press(KeyCode::KeyW);
    for _ in 0..5 {
        app.update();
    }
    let world = app.world_mut();
    assert_eq!(world.query::<&GoalMarker>().iter(world).count(), 2);
    assert_eq!(world.query::<&ChaseCamera>().iter(world).count(), 1);
    assert_eq!(world.query::<&Player>().iter(world).count(), 1);
    let hud = world
        .query_filtered::<&Text, With<Hud>>()
        .single(world)
        .unwrap();
    assert!(hud.0.contains("Distance to B"), "HUD text: {}", hud.0);
}
