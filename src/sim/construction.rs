use bevy::prelude::*;
use smallvec::SmallVec;

use super::components::{
  GridDirection, GridFootprint, GridPosition, GridSize, RegisterQueue, get_footprint,
};
use super::world::{GridLayer, GridLayers};

// =============================================================================
// define_appliances! macro
// =============================================================================
//
// Generates one `pub fn place_X(&mut World, anchor, direction)` per entry.
// Caller (e.g. dev_console) invokes these directly. No message layer.

macro_rules! define_appliances {
    (
        $(
            $place_fn:ident {
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
            pub fn $place_fn(
                world: &mut World,
                anchor: (i32, i32),
                direction: GridDirection,
            ) {
                let geometry = GridSize {
                    right: $right,
                    forward: $forward,
                };
                let footprint_cells: Vec<(i32, i32)> =
                    get_footprint(&geometry, direction, anchor);
                let footprint_sv: SmallVec<[(i32, i32); 8]> =
                    footprint_cells.iter().copied().collect();

                let entity = world
                    .spawn((
                        GridPosition { x: anchor.0, y: anchor.1 },
                        direction,
                        GridFootprint {
                            cells: footprint_sv.clone(),
                        },
                        GridLayer::$layer,
                        geometry,
                        <$identity>::default(),
                        $($extra),*
                    ))
                    .id();

                let ok = {
                    let mut grid = world.resource_mut::<GridLayers>();
                    match GridLayer::$layer {
                        GridLayer::Floor => grid.try_place_floor(&footprint_cells, entity),
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
                    }
                };

                if !ok {
                    world.despawn(entity);
                    return;
                }

                info!(
                    concat!("Placed ", stringify!($place_fn), " at ({},{}), direction {:?}"),
                    anchor.0, anchor.1, direction,
                );
            }
        )*
    };
}

define_appliances! {
    place_table {
        right: 1, forward: 1,
        layer: Floor,
        identity: super::components::TableState,
        extra_components: [],
    },
    place_chair {
        right: 1, forward: 1,
        layer: Floor,
        identity: super::components::ChairState,
        extra_components: [],
    },
    place_register {
        right: 2, forward: 1,
        layer: Floor,
        identity: super::components::RegisterState,
        extra_components: [RegisterQueue::default()],
    },
    place_stove {
        right: 2, forward: 1,
        layer: Floor,
        identity: super::components::StoveState,
        extra_components: [],
    },
}

// =============================================================================
// Demolish
// =============================================================================

/// Demolish the appliance whose footprint contains `click`.
pub fn demolish_appliance(world: &mut World, click: (i32, i32)) {
  // Collect target first to release the QueryState borrow before mutating.
  let target: Option<(Entity, SmallVec<[(i32, i32); 8]>, GridLayer)> = {
    let mut state = world.query::<(Entity, &GridFootprint, &GridLayer)>();
    let mut hit = None;
    for (entity, footprint, layer) in state.iter(world) {
      if footprint.cells.contains(&click) {
        hit = Some((entity, footprint.cells.clone(), *layer));
        break;
      }
    }
    hit
  };

  let Some((entity, cells, layer)) = target else {
    return;
  };

  {
    let mut grid = world.resource_mut::<GridLayers>();
    match layer {
      GridLayer::Floor => grid.remove_floor(&cells, entity),
      GridLayer::Ceiling => {
        if let Some(&c) = cells.first() {
          grid.remove_ceiling(c, entity);
        }
      }
      GridLayer::Surface => {
        if let Some(&c) = cells.first() {
          grid.remove_surface(c, entity);
        }
      }
    }
  }

  world.despawn(entity);
  info!(
    "Demolished appliance at ({},{}), footprint {:?}",
    click.0, click.1, &*cells,
  );
}

// =============================================================================
// Plugin (registers no systems; place/demolish are direct calls)
// =============================================================================

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ConstructionSet;

pub struct ConstructionPlugin;

impl Plugin for ConstructionPlugin {
  fn build(&self, _app: &mut App) {
    // No systems to register: construction is invoked directly by callers.
  }
}
