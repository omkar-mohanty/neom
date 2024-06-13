use anyhow::Result;
use rand::Rng;
use std::{
    collections::BTreeMap,
    fs,
    ops::RangeInclusive,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use three_d::{
    core::Context, vec3, Camera, ColorMaterial, CpuMaterial, CpuMesh, Cull, FromCpuMaterial,
    Geometry, Gm, InnerSpace, InstancedMesh, Instances, Light, Mat4, Material, Mesh, Object,
    PhysicalMaterial, Quat, RenderTarget, Srgba, Vec3,
};

pub mod gui;

static DARK: RangeInclusive<u8> = 0..=125;
static BRIGHT: RangeInclusive<u8> = 126..=255;

pub struct Resources {
    pub models: Vec<ModelEntry>,
    pub ctx: Context,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderMode {
    Normal,
    Wireframe,
}

impl Resources {
    pub fn new(ctx: Context) -> Self {
        Self {
            models: Vec::new(),
            ctx,
        }
    }
}

fn get_rand_rgba() -> [u8; 4] {
    let mut rng = rand::thread_rng();
    let r = rng.gen_range(BRIGHT.clone());
    let g = rng.gen_range(BRIGHT.clone());
    let b = rng.gen_range(BRIGHT.clone());
    let a = rng.gen_range(BRIGHT.clone());
    [r, g, b, a]
}

fn new_phys_mat(ctx: &Context) -> PhysicalMaterial {
    let [r, g, b, _a] = get_rand_rgba();
    let defualt_physical_mat = PhysicalMaterial::from_cpu_material(
        &ctx,
        &CpuMaterial {
            albedo: Srgba::new_opaque(r, g, b),
            roughness: 1.0,
            ..Default::default()
        },
    );

    defualt_physical_mat
}

pub struct ModelEntry {
    cpu_mesh: CpuMesh,
    pub normal_mesh: Gm<Mesh, PhysicalMaterial>,
    pub wireframe_vertices: Gm<InstancedMesh, PhysicalMaterial>,
    pub wireframe_edges: Gm<InstancedMesh, PhysicalMaterial>,
    pub render_mode: RenderMode,
}

impl ModelEntry {
    pub fn new(ctx: &Context, mut cpu_mesh: CpuMesh) -> Self {
        let mut wireframe_material = PhysicalMaterial::new_opaque(
            &ctx,
            &CpuMaterial {
                albedo: Srgba::new_opaque(220, 50, 50),
                roughness: 0.7,
                metallic: 0.8,
                ..Default::default()
            },
        );
        wireframe_material.render_states.cull = Cull::Back;
        let mut cylinder = CpuMesh::cylinder(10);
        cylinder
            .transform(&Mat4::from_nonuniform_scale(1.0, 0.007, 0.007))
            .unwrap();
        let wireframe_edges = Gm::new(
            InstancedMesh::new(&ctx, &edge_transformations(&cpu_mesh), &cylinder),
            wireframe_material.clone(),
        );

        let mut sphere = CpuMesh::sphere(8);
        sphere.transform(&Mat4::from_scale(0.015)).unwrap();
        let wireframe_vertices = Gm::new(
            InstancedMesh::new(&ctx, &vertex_transformations(&cpu_mesh), &sphere),
            wireframe_material,
        );

        if cpu_mesh.normals.is_none() {
            cpu_mesh.compute_normals();
        }

        let model_material = new_phys_mat(ctx);
        let normal_mesh = Gm::new(Mesh::new(&ctx, &cpu_mesh), model_material);

        Self {
            cpu_mesh,
            normal_mesh,
            wireframe_vertices,
            wireframe_edges,
            render_mode: RenderMode::Normal,
        }
    }

    pub fn render(&mut self, target: &RenderTarget, camera: &Camera, lights: &[&dyn Light]) {
        match self.render_mode {
            RenderMode::Normal => {
                let objects = self.normal_mesh.into_iter();
                target.render(camera, objects, lights);
            }
            RenderMode::Wireframe => {
                let objects = self
                    .normal_mesh
                    .into_iter()
                    .chain(&self.wireframe_edges)
                    .chain(&self.wireframe_vertices);
                target.render(camera, objects, lights);
            }
        };
    }
}

fn get_model(ctx: &three_d::Context, path: PathBuf) -> Result<ModelEntry> {
    let model: CpuMesh = three_d_asset::io::load(&[path]).unwrap().deserialize("")?;
    let model = ModelEntry::new(ctx, model);
    return Ok(model);
}

pub fn load_models(ctx: &three_d::Context, path: PathBuf) -> Result<Vec<ModelEntry>> {
    if !path.is_dir() {
        let model: CpuMesh = three_d_asset::io::load(&[path]).unwrap().deserialize("")?;
        let model = ModelEntry::new(ctx, model);
        return Ok(vec![model]);
    }

    let models = Arc::new(RwLock::new(Vec::new()));

    for entry in fs::read_dir(path).unwrap() {
        let entry = entry?;
        std::thread::scope(|_| {
            let models = Arc::clone(&models);
            match get_model(&ctx.clone(), entry.path()) {
                Ok(model) => {
                    let mut models_write = models.write().unwrap();
                    models_write.push(model);
                }
                Err(msg) => {
                    println!("Couldnt load {msg}");
                }
            }
        });
    }

    let mut models_write = models.write().unwrap();
    let models = std::mem::take(&mut *models_write);
    Ok(models)
}

fn edge_transformations(cpu_mesh: &CpuMesh) -> Instances {
    let indices = cpu_mesh.indices.to_u32().unwrap();
    let positions = cpu_mesh.positions.to_f32();
    let mut transformations = Vec::new();
    for f in 0..indices.len() / 3 {
        let i1 = indices[3 * f] as usize;
        let i2 = indices[3 * f + 1] as usize;
        let i3 = indices[3 * f + 2] as usize;

        if i1 < i2 {
            transformations.push(edge_transform(positions[i1], positions[i2]));
        }
        if i2 < i3 {
            transformations.push(edge_transform(positions[i2], positions[i3]));
        }
        if i3 < i1 {
            transformations.push(edge_transform(positions[i3], positions[i1]));
        }
    }
    Instances {
        transformations,
        ..Default::default()
    }
}

fn edge_transform(p1: Vec3, p2: Vec3) -> Mat4 {
    Mat4::from_translation(p1)
        * Into::<Mat4>::into(Quat::from_arc(
            vec3(1.0, 0.0, 0.0),
            (p2 - p1).normalize(),
            None,
        ))
        * Mat4::from_nonuniform_scale((p1 - p2).magnitude(), 1.0, 1.0)
}

fn vertex_transformations(cpu_mesh: &CpuMesh) -> Instances {
    Instances {
        transformations: cpu_mesh
            .positions
            .to_f32()
            .into_iter()
            .map(Mat4::from_translation)
            .collect(),
        ..Default::default()
    }
}
