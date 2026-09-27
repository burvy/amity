use amity_protocol::stuff::{
    definition::Interactable,
    ibox::definition::{InteractableBox, BOX_SIZE},
};
use bevy::prelude::*;

/// Draw interactable entities
/// TODO: for the mesh and material, eventually use an enum that determines what to draw
/// instead of hardcoding it like it is right now
pub fn draw_interactables(
    mut cmds: Commands,
    interactables: Query<Entity, Added<Interactable>>,
    mut mesh: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    for interactable in &interactables {
        cmds.entity(interactable).insert((
            // TODO: use enum based on component to determine what to draw
            InteractableBox::default(),
            Mesh3d(mesh.add(Cuboid::new(BOX_SIZE, BOX_SIZE, BOX_SIZE))),
            MeshMaterial3d::<StandardMaterial>(mats.add(Color::srgb_u8(0, 128, 255))),
        ));
    }
}
