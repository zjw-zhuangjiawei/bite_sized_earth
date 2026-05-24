use bevy::prelude::*;
use smallvec::SmallVec;

use crate::components::{
  get_footprint, ApplianceGeometry, GridDirection, GridFootprint, GridPosition, RegisterQueue,
};
use crate::messages::GridChangedMessage;
use crate::world::{GridLayer, GridLayers};

// =============================================================================
// define_appliances! macro
// =============================================================================

macro_rules! define_appliances {
    (
        $(
            $(#[$meta:meta])*
            $handler:ident : $msg:ident {
                right: $right:literal,
                forward: $forward:literal,
                layer: $layer:ident,
                identity: $identity:ty,
                extra_components: [$($extra:expr),* $(,)?],
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
                    mut grid: ResMut<GridLayers>,
                    mut writer: MessageWriter<GridChangedMessage>,
                    mut reader: MessageReader<$msg>,
                ) {
                    for req in reader.read() {
                        let geometry = ApplianceGeometry {
                            right: $right,
                            forward: $forward,
                            direction: req.direction,
                        };
                        let footprint_cells: Vec<(i32, i32)> =
                            get_footprint(&geometry, req.anchor);

                        let footprint_sv: SmallVec<[(i32, i32); 8]> =
                            footprint_cells.iter().copied().collect();

                        let entity = commands
                            .spawn((
                                GridPosition { x: req.anchor.0, z: req.anchor.1 },
                                GridFootprint {
                                    cells: footprint_sv.clone(),
                                },
                                GridLayer::$layer,
                                geometry,
                                <$identity>::default(),
                                $($extra),*
                            ))
                            .id();

                        let ok = match GridLayer::$layer {
                            GridLayer::Floor => {
                                grid.try_place_floor(&footprint_cells, entity)
                            }
                            GridLayer::Ceiling => {
                                if let Some(&c) = footprint_cells.first() {
                                    grid.try_place_ceiling(c, entity)
                                } else {
                                    false
                                }
                            }
                            GridLayer::Surface => {
                                if let Some(&c) = footprint_cells.first() {
                                    grid.add_surface(c, entity)
                                } else {
                                    false
                                }
                            }
                        };

                        if !ok {
                            commands.entity(entity).despawn();
                            continue;
                        }

                        let changed: SmallVec<[(i32, i32); 8]> =
                            footprint_cells.iter().copied().collect();
                        writer.write(GridChangedMessage {
                            cells: changed,
                            layer: GridLayer::$layer,
                        });

                        info!(
                            concat!("Placed ", stringify!($msg), " at ({},{}), direction {:?}"),
                            req.anchor.0, req.anchor.1, req.direction,
                        );
                    }
                }
        )*
    };

}

// =============================================================================
// Invoke the macro
// =============================================================================

define_appliances! {
    handle_place_table: RequestPlaceTable {
        right: 1, forward: 1,
        layer: Floor,
        identity: crate::components::TableState,
        extra_components: [],
    },
    handle_place_chair: RequestPlaceChair {
        right: 1, forward: 1,
        layer: Floor,
        identity: crate::components::ChairState,
        extra_components: [],
    },
    handle_place_register: RequestPlaceRegister {
        right: 2, forward: 1,
        layer: Floor,
        identity: crate::components::RegisterState,
        extra_components: [RegisterQueue::default()],
    },
    handle_place_stove: RequestPlaceStove {
        right: 2, forward: 1,
        layer: Floor,
        identity: crate::components::StoveState,
        extra_components: [],
    },
}

// =============================================================================
// Demolish
// =============================================================================

#[derive(Message, Debug)]
pub struct RequestDemolishAppliance {
  pub click: (i32, i32),
}

pub fn handle_demolish_appliance(
  mut commands: Commands,
  mut grid: ResMut<GridLayers>,
  mut writer: MessageWriter<GridChangedMessage>,
  mut reader: MessageReader<RequestDemolishAppliance>,
  query: Query<(Entity, &GridFootprint, &GridLayer)>,
) {
  for req in reader.read() {
    for (entity, footprint, layer) in query.iter() {
      if !footprint.cells.contains(&req.click) {
        continue;
      }
      match layer {
        GridLayer::Floor => grid.remove_floor(&footprint.cells, entity),
        GridLayer::Ceiling => {
          if let Some(&c) = footprint.cells.first() {
            grid.remove_ceiling(c, entity);
          }
        }
        GridLayer::Surface => {
          if let Some(&c) = footprint.cells.first() {
            grid.remove_surface(c, entity);
          }
        }
      }
      let changed: SmallVec<[(i32, i32); 8]> = footprint.cells.iter().copied().collect();
      writer.write(GridChangedMessage {
        cells: changed,
        layer: *layer,
      });
      commands.entity(entity).despawn();
      info!(
        "Demolished appliance at ({},{}), footprint {:?}",
        req.click.0, req.click.1, &*footprint.cells,
      );
      break;
    }
  }
}
