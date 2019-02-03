use crate::components::Loading;
use crate::resources::MyWorld;
use crate::game_state::COLLIDER_MARGIN;

use amethyst::renderer::MeshData;
use specs::Entities;
use amethyst::{
    ecs::{Join, Read, ReadStorage, System, Write, WriteStorage},
    core::{Transform},
    assets::AssetStorage,
    renderer::{
        MeshHandle,
        Mesh,
    },
};
use na::Point3;
use na::{Isometry3, Vector3};
use ncollide3d::{
    bounding_volume::{HasBoundingVolume, AABB},
    shape::{ConvexHull, Cuboid, Cylinder, ShapeHandle},
    transformation::ToTriMesh,
};
use nphysics3d::{
    object::{BodyHandle, BodyStatus, Material as PhysicsMaterial},
    volumetric::Volumetric,
};

#[derive(Default)]
pub struct LoadedSystem;

impl LoadedSystem {
    fn mesh_data_vertices(&self, obj: &MeshData) -> Vec<Point3<f32>> {
        match obj {
            MeshData::PosNormTex(ref v) => {
                v.iter().map(|v| Point3::new(v.position.x as f32, v.position.y as f32, v.position.z as f32)).collect()
            },
            _ => unimplemented!(),
        }
    }

    fn create_mesh(&mut self, t: &Transform, obj: &MeshData, physics_world: &mut MyWorld) {
        let points = self.mesh_data_vertices(obj);

        let geom = ShapeHandle::new(ConvexHull::try_from_points(&points).unwrap());
        let inertia = geom.inertia(1.0);
        let center_of_mass = geom.center_of_mass();

        let pos = Isometry3::new(
            {
                let translation = t.translation();
                Vector3::new(translation[0], translation[1], translation[2])
            },
            Vector3::new(0.9, 0.1, 0.0),
        );
        let handle = physics_world
            .get_mut()
            .add_rigid_body(pos, inertia, center_of_mass);
        let body_handle = physics_world.get_mut().add_collider(
            COLLIDER_MARGIN,
            geom.clone(),
            handle,
            Isometry3::identity(),
            PhysicsMaterial::default(),
        );
    }
}

impl<'s> System<'s> for LoadedSystem {
    type SystemData = (
        Entities<'s>,
        WriteStorage<'s, Loading>,
        ReadStorage<'s, MeshData>,
        Write<'s, MyWorld>,
        ReadStorage<'s, Transform>,
    );
    fn run(&mut self, (ents, mut loading, mesh_data, mut physics_world, transforms): Self::SystemData) {
        let mut loadings_to_remove = vec![];
        for (ent, _, m, t) in (&*ents, &loading, &mesh_data, &transforms).join() {
            self.create_mesh(t, &m, &mut physics_world);
            loadings_to_remove.push(ent);
        }
        for ent in loadings_to_remove {
            loading.remove(ent);
        }
    }
}