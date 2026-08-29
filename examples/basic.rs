use std::f32::consts::TAU;

use bevy::{
    camera::Exposure,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    color::palettes::css::ALICE_BLUE,
    image::ImageLoaderSettings,
    light::{
        Atmosphere, AtmosphereEnvironmentMapLight, SunDisk, atmosphere::ScatteringMedium,
        light_consts::lux,
    },
    pbr::AtmosphereSettings,
    post_process::bloom::Bloom,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        storage::ShaderBuffer,
    },
};

use bevy_clipmap::{Clipmap, ClipmapCutoutGridParams, ClipmapPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FreeCameraPlugin)
        .add_plugins(ClipmapPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    commands.spawn(Atmosphere::earth(
        scattering_mediums.add(ScatteringMedium::earth(256, 256)),
    ));

    // Dummy all-zero splatmap so the shader falls back to the color texture
    let dummy_splatmap = images.add(Image::new(
        Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        vec![0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        default(),
    ));
    // Dummy 2D array texture (required by the `layers` binding which expects D2Array)
    let dummy_layers = images.add(Image::new(
        Extent3d { width: 1, height: 1, depth_or_array_layers: 2 },
        TextureDimension::D2,
        vec![255, 255, 255, 255, 255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        default(),
    ));
    // CAGE: zero-filled cutout placeholders (no road footprints) — one empty
    // region record and a 1x1 grid cell so the storage bindings validate.
    let cutout_regions = buffers.add(ShaderBuffer::new(&[0u8; 32], default()));
    let cutout_grid = buffers.add(ShaderBuffer::new(&[0u8; 8], default()));

    let target = commands
        .spawn((
            Camera3d::default(),
            Projection::from(PerspectiveProjection {
                fov: 90.0_f32.to_radians(),
                ..Default::default()
            }),
            Bloom::NATURAL,
            AtmosphereSettings {
                aerial_view_lut_max_distance: 16384.0,
                ..Default::default()
            },
            AtmosphereEnvironmentMapLight::default(),
            Exposure::SUNLIGHT,
            Transform::from_xyz(0.0, 150.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
            FreeCamera {
                walk_speed: 500.0,
                run_speed: 1000.0,
                ..Default::default()
            },
        ))
        .id();

    for _ in 0..2 {
        commands.spawn((
            DirectionalLight {
                shadow_maps_enabled: true,
                illuminance: lux::RAW_SUNLIGHT,
                color: ALICE_BLUE.into(),
                ..Default::default()
            },
            SunDisk {
                angular_size: SunDisk::EARTH.angular_size * 3.0,
                intensity: 30.0,
            },
            Transform::default(),
        ));
    }

    commands.spawn(Clipmap {
        half_width: 128,
        levels: 7,
        base_scale: 1.0,
        texel_size: 8.0,
        target,
        color: asset_server.load("color_2048x2048.png"),
        heightmap: asset_server
            .load_builder()
            .with_settings(|settings: &mut ImageLoaderSettings| {
                settings.is_srgb = false;
            })
            .load("heightmap_1024x1024.ktx2"),
        horizon: asset_server
            .load_builder()
            .with_settings(|settings: &mut ImageLoaderSettings| {
                settings.is_srgb = false;
            })
            .load("heightmap_horizon_512x512_8.ktx2"),
        horizon_coeffs: 8,
        min: -1312.5,
        max: 1312.5,
        wireframe: false,
        // CAGE: splatmap defaults (all-zero → shader falls back to color texture)
        splatmap: dummy_splatmap,
        layers: dummy_layers,
        layer_uv_scale: 1.0,
        // CAGE: cutout defaults (empty regions, one grid cell spanning the world)
        cutout_regions,
        cutout_grid,
        cutout_grid_params: ClipmapCutoutGridParams {
            grid_dims: UVec2::ONE,
            cell_size: 2625.0,
            world_origin: Vec2::splat(-1312.5),
        },
    });
}

fn update(mut lights: Query<&mut Transform, With<DirectionalLight>>, time: Res<Time>) {
    let cnt = lights.count();
    for (i, mut transform) in lights.iter_mut().enumerate() {
        let angle = 0.1 * time.elapsed_secs() + (TAU * i as f32 / cnt as f32);
        *transform =
            Transform::from_translation(Vec3::new(angle.cos(), angle.sin(), angle.sin() * 0.1))
                .looking_at(Vec3::ZERO, Vec3::Y);
    }
}
