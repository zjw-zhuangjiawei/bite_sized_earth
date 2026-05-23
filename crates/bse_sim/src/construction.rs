use bevy::prelude::*;
use crate::components::{ApplianceGeometry, GridDirection, GridPosition, get_footprint};
use crate::world::{GridOccupancy, WorldGridMap};

/// Check footprint and fill grid; returns true if placement succeeded.
fn try_place(
    grid_map: &mut WorldGridMap,
    anchor: (i32, i32),
    geometry: &ApplianceGeometry,
) -> bool {
    let footprint = get_footprint(geometry, anchor);
    if !grid_map.is_area_empty(&footprint) {
        return false;
    }
    grid_map.fill_area(&footprint, GridOccupancy::Occupied);
    true
}

// =============================================================================
// define_appliances! macro -- generates one message type + one handler per appliance
// =============================================================================

macro_rules! define_appliances {
    (
        $(
            $(#[$meta:meta])*
            $handler:ident : $msg:ident {
                right: $right:literal,
                forward: $forward:literal,
                identity: $identity:ty,
            }
        ),+
        $(,)?
    ) => {
        $(
            #[derive(Message, Debug)]
            pub struct $msg {
                pub anchor: (i32, i32),
                pub direction: GridDirection,
            }

            impl $msg {
                pub const RIGHT: i32 = $right;
                pub const FORWARD: i32 = $forward;
            }

            pub fn $handler(
                mut commands: Commands,
                mut grid_map: ResMut<WorldGridMap>,
                mut reader: MessageReader<$msg>,
            ) {
                for req in reader.read() {
                    let geometry = ApplianceGeometry {
                        right: $right,
                        forward: $forward,
                        direction: req.direction,
                    };
                    let anchor = req.anchor;
                    if try_place(&mut grid_map, anchor, &geometry) {
                        commands.spawn((
                            GridPosition { x: anchor.0, z: anchor.1 },
                            geometry,
                            <$identity>::default(),
                        ));
                        info!(
                            concat!("Placed ", stringify!($msg), " at ({},{}), direction {:?}"),
                            anchor.0, anchor.1, req.direction,
                        );
                    }
                }
            }
        )*
    };
}

// Invoke the macro for all appliances
define_appliances! {
    handle_place_table: RequestPlaceTable {
        right: 2, forward: 1,
        identity: crate::components::TableState,
    },
    handle_place_chair: RequestPlaceChair {
        right: 1, forward: 1,
        identity: crate::components::ChairState,
    },
    handle_place_register: RequestPlaceRegister {
        right: 2, forward: 1,
        identity: crate::components::RegisterState,
    },
    handle_place_stove: RequestPlaceStove {
        right: 2, forward: 1,
        identity: crate::components::StoveState,
    },
}

// =============================================================================
// Demolish -- shared by both game systems and debug tools
// =============================================================================

#[derive(Message, Debug)]
pub struct RequestDemolishAppliance {
    pub click: (i32, i32),
}

pub fn handle_demolish_appliance(
    mut commands: Commands,
    mut grid_map: ResMut<WorldGridMap>,
    mut reader: MessageReader<RequestDemolishAppliance>,
    query: Query<(Entity, &GridPosition, &ApplianceGeometry)>,
) {
    for req in reader.read() {
        for (entity, pos, geometry) in query.iter() {
            let footprint = get_footprint(geometry, (pos.x, pos.z));
            if footprint.contains(&req.click) {
                grid_map.clear_area(&footprint);
                commands.entity(entity).despawn();
                info!("Demolished appliance at ({},{}), footprint {:?}", req.click.0, req.click.1, footprint);
                break;
            }
        }
    }
}
