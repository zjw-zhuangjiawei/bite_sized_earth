use bevy::prelude::*;
use bevy::camera::ScalingMode;
use bevy_enhanced_input::prelude::*;
/// Marker component for the main isometric camera entity.
#[derive(Component)]
pub struct MainCamera;

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

#[derive(InputAction)]
#[action_output(Vec2)]
struct PanCamera;

#[derive(InputAction)]
#[action_output(f32)]
struct ZoomCamera;

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

#[derive(Component)]
struct CameraContext;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct CameraControlPlugin;

impl Plugin for CameraControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<CameraContext>();
        app.add_systems(Startup, setup_camera);
        app.add_systems(Update, camera_controller_system);
    }
}

// ---------------------------------------------------------------------------
// Camera setup
// ---------------------------------------------------------------------------

fn setup_camera(mut commands: Commands) {
    let angle_y = std::f32::consts::FRAC_PI_4;
    let angle_x = -(1.0 / 3.0_f32.sqrt()).asin();
    let isometric_rotation = Quat::from_rotation_y(angle_y) * Quat::from_rotation_x(angle_x);

    let grid_center = Vec3::new(15.5, 0.0, 15.5);
    let camera_back_dir = isometric_rotation * Vec3::Z;
    let camera_position = grid_center + camera_back_dir * 50.0;

    let camera_transform =
        Transform::from_translation(camera_position).with_rotation(isometric_rotation);

    commands.spawn((
        MainCamera,
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scale: 0.08,
            scaling_mode: ScalingMode::WindowSize,
            ..OrthographicProjection::default_3d()
        }),
        camera_transform,
        CameraContext,
        actions!(CameraContext[
            (
                Action::<PanCamera>::new(),
                Bindings::spawn(Cardinal::arrows()),
            ),
            (
                Action::<ZoomCamera>::new(),
                Scale::splat(0.1),
                bindings![(Binding::mouse_wheel(), SwizzleAxis::YXZ)],
            ),
        ]),
    ));
}

// ---------------------------------------------------------------------------
// Camera controller
// ---------------------------------------------------------------------------

fn camera_controller_system(
    time: Res<Time>,
    pan: Query<&Action<PanCamera>>,
    zoom: Query<&Action<ZoomCamera>>,
    camera: Single<(&mut Projection, &mut Transform), With<MainCamera>>,
) {
    let (mut projection, mut transform) = camera.into_inner();

    let Projection::Orthographic(ortho) = &mut *projection else {
        return;
    };

    let Ok(pan) = pan.single() else { return };
    let Ok(zoom_val) = zoom.single() else { return };

    let pan_value = **pan;
    if pan_value != Vec2::ZERO {
        let input = pan_value.normalize_or_zero();
        let rotation = Quat::from_rotation_y(45.0_f32.to_radians());
        let isometric_dir = rotation * Vec3::new(input.x, 0.0, -input.y);
        let delta = isometric_dir * 500.0 * time.delta_secs() * ortho.scale;
        transform.translation += delta;
    }

    let zoom_delta = **zoom_val;
    if zoom_delta != 0.0 {
        ortho.scale -= zoom_delta * 0.1;
        ortho.scale = ortho.scale.clamp(0.02, 0.2);
    }
}
