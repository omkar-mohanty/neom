#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    run().await;
}

#[derive(Debug, Eq, PartialEq)]
enum MaterialType {
    Position,
    Normal,
    Color,
    Depth,
    Orm,
    Uv,
    Forward,
    Deferred,
}

use std::sync::{Arc, RwLock};

use three_d::*;
use viewer::gui::{Config, Gui, IGui, MainMenu};
use viewer::{load_models, ModelEntry, RenderMode, Resources};

pub async fn run() {
    let window = Window::new(WindowSettings {
        title: "Lighting!".to_string(),
        max_size: Some((1280, 720)),
        ..Default::default()
    })
    .unwrap();
    let context = window.gl();
    let mut camera = Camera::new_perspective(
        window.viewport(),
        vec3(2.0, 2.0, 5.0),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        degrees(45.0),
        0.1,
        30.0,
    );
    let mut control = OrbitControl::new(*camera.target(), 1.0, 10000000.0);
    let mut gui = three_d::GUI::new(&context);

    // Source: https://github.com/KhronosGroup/glTF-Sample-Models/tree/master/2.0
    let mut cpu_model: CpuModel = three_d_asset::io::load_async(&["assets/DamagedHelmet.glb"])
        .await
        .unwrap()
        .deserialize("")
        .unwrap();
    cpu_model
        .geometries
        .iter_mut()
        .for_each(|m| m.compute_tangents());
    let mut model = Model::<PhysicalMaterial>::new(&context, &cpu_model)
        .unwrap()
        .remove(0);

    let mut loaded = three_d_asset::io::load_async(&["./assets/studio.hdr"])
        .await
        .unwrap();
    let res = Arc::new(RwLock::new(Resources::new(context.clone())));
    let mut main_menu = Gui::new(res);

    let mut ambient = AmbientLight::new(&context, 0.2, Srgba::WHITE);
    let mut directional0 = DirectionalLight::new(&context, 1.0, Srgba::RED, &vec3(0.0, -1.0, 0.0));
    let mut directional1 =
        DirectionalLight::new(&context, 1.0, Srgba::GREEN, &vec3(0.0, -1.0, 0.0));
    let mut spot0 = SpotLight::new(
        &context,
        2.0,
        Srgba::BLUE,
        &vec3(0.0, 0.0, 0.0),
        &vec3(0.0, -1.0, 0.0),
        degrees(25.0),
        Attenuation {
            constant: 0.1,
            linear: 0.001,
            quadratic: 0.0001,
        },
    );
    let mut point0 = PointLight::new(
        &context,
        1.0,
        Srgba::GREEN,
        &vec3(0.0, 0.0, 0.0),
        Attenuation {
            constant: 0.5,
            linear: 0.05,
            quadratic: 0.005,
        },
    );
    let mut point1 = PointLight::new(
        &context,
        1.0,
        Srgba::RED,
        &vec3(0.0, 0.0, 0.0),
        Attenuation {
            constant: 0.5,
            linear: 0.05,
            quadratic: 0.005,
        },
    );

    // main loop
    let mut shadows_enabled = true;
    let mut config = Config::default();

    let model_wireframe = false;

    window.render_loop(move |mut frame_input| {
        let mut panel_width = 0.0;
        gui.update(
            &mut frame_input.events,
            frame_input.accumulated_time,
            frame_input.viewport,
            frame_input.device_pixel_ratio,
            |gui_context| {
                main_menu.show(&mut config, &gui_context);
                panel_width = gui_context.used_rect().width();
            },
        );

        control.handle_events(&mut camera, &mut frame_input.events);

        model.material = PhysicalMaterial::new_opaque(
            &context,
            &CpuMaterial {
                albedo: Srgba::new_opaque(128, 200, 70),
                ..Default::default()
            },
        );

        // Draw
        if shadows_enabled {
            for model in &models {
                directional0.generate_shadow_map(1024, &*model.normal_mesh);
                directional1.generate_shadow_map(1024, &*model.normal_mesh);
                spot0.generate_shadow_map(1024, &*model.normal_mesh);
            }
        }

        let lights = [
            &ambient as &dyn Light,
            &spot0,
            &directional0,
            &directional1,
            &point0,
            &point1,
        ];

        let screen = frame_input.screen();
        screen.clear(ClearState::default());

        for model in &mut models {
            model.render(&screen, &camera, &lights);
        }

        screen.write(|| gui.render()).unwrap();

        FrameOutput::default()
    });
}
