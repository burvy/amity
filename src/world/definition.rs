
use bevy::prelude::*;
use lightyear::prelude::MessageReceiver;
use shooter_protocol::{
    stuff,
    world::chunk,
};

pub fn spawn_light(mut cmds: Commands) {
    cmds.spawn((
        DirectionalLight::default(),
        Transform::from_translation(Vec3::new(5.0, 5.0, 10.0)).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Spawns chunks sent by the server
pub fn receive_chunks(
    mut cmds: Commands,
    mut receiver: Single<&mut MessageReceiver<chunk::ChunkData>>,
) {
    for chunk::ChunkData { pos, chunk } in receiver.receive() {
        info!("received chunk {:?}", pos.0);
        cmds.spawn(chunk::chunk_body(pos, chunk));
    }
}

/// Draw interactable entities
/// TODO: for the mesh and material, eventually use an enum that determines what to draw
/// instead of hardcoding it like it is right now
pub fn draw_interactables(
    mut cmds: Commands,
    interactables: Query<Entity, Added<stuff::definition::Interactable>>,
    mut mesh: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    for interactable in &interactables {
        cmds.entity(interactable).insert((
            // TODO: use enum based on component to determine what to draw
            stuff::ibox::definition::InteractableBox::default(),
            Mesh3d(mesh.add(Cuboid::new(
                stuff::ibox::definition::BOX_SIZE,
                stuff::ibox::definition::BOX_SIZE,
                stuff::ibox::definition::BOX_SIZE,
            ))),
            MeshMaterial3d::<StandardMaterial>(mats.add(Color::srgb_u8(0, 128, 255))),
        ));
    }
}
