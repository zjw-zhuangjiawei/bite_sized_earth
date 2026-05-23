use bevy::prelude::*;
use smallvec::SmallVec;

use crate::components::{
  get_footprint, ApplianceGeometry, GridDirection, GridFootprint, GridPosition, InteractionPoints,
  InteractionRule,
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
                interaction: $interaction:ident $( { $($ikey:ident : $ival:expr),* $(,)? } )?,
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

                    // Pre-compute interaction cells
                    let bounds = (grid.width, grid.height);
                    let interaction_rule = define_appliances!(@interaction_rule
                        $interaction $( { $($ikey : $ival),* } )?
                    );
                    let (_initial_points, filtered_points): (Vec<(i32, i32)>, SmallVec<[(i32, i32); 8]>) = match &interaction_rule {
                        InteractionRule::Front => {
                            let temp_pos = GridPosition { x: req.anchor.0, z: req.anchor.1 };
                            let pts = crate::interaction::compute_front_cells(&temp_pos, &geometry, bounds).into_vec();
                            let filtered = pts.iter().copied()
                                .filter(|&(x, z)| grid.floor_entity_at(x, z).is_none())
                                .collect();
                            (pts, filtered)
                        }
                        InteractionRule::AllAdjacent { range } => {
                            let footprint = GridFootprint {
                                cells: footprint_cells.iter().copied().collect(),
                            };
                            let pts = crate::interaction::compute_adjacent_cells(&footprint, *range, bounds).into_vec();
                            let filtered = pts.iter().copied()
                                .filter(|&(x, z)| grid.floor_entity_at(x, z).is_none())
                                .collect();
                            (pts, filtered)
                        }
                        InteractionRule::OnSite => {
                            // Interaction cells are the entity's own footprint.
                            let pts = footprint_cells.clone();
                            let filtered = pts.iter().copied().collect();
                            (pts, filtered)
                        }
                    };

                    let footprint_sv: SmallVec<[(i32, i32); 8]> =
                        footprint_cells.iter().copied().collect();

                    // Spawn first to get entity id, then write into GridLayers
                    let entity = commands
                        .spawn((
                            GridPosition { x: req.anchor.0, z: req.anchor.1 },
                            GridFootprint {
                                cells: footprint_sv.clone(),
                            },
                            GridLayer::$layer,
                            geometry,
                            <$identity>::default(),
                            interaction_rule,
                            InteractionPoints { cells: filtered_points },
                        ))
                        .id();

                    // Write into grid with the real entity
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

                    // Broadcast
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

    (@interaction_rule Front) => { InteractionRule::Front };
    (@interaction_rule Adjacent { range: $range:expr }) => {
        InteractionRule::AllAdjacent { range: $range }
    };
    (@interaction_rule OnSite) => { InteractionRule::OnSite };
}

// =============================================================================
// Invoke the macro
// =============================================================================

define_appliances! {
    handle_place_table: RequestPlaceTable {
        right: 1, forward: 1,
        layer: Floor,
        identity: crate::components::TableState,
        interaction: Adjacent { range: 1 },
    },
    handle_place_chair: RequestPlaceChair {
        right: 1, forward: 1,
        layer: Floor,
        identity: crate::components::ChairState,
        interaction: OnSite,
    },
    handle_place_register: RequestPlaceRegister {
        right: 2, forward: 1,
        layer: Floor,
        identity: crate::components::RegisterState,
        interaction: Adjacent { range: 1 },
    },
    handle_place_stove: RequestPlaceStove {
        right: 2, forward: 1,
        layer: Floor,
        identity: crate::components::StoveState,
        interaction: Front,
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
