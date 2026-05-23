use bevy::prelude::*;
use smallvec::SmallVec;

/// World-grid coordinate.
pub type Cell = (i32, i32);

/// Which spatial layer an entity belongs to.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridLayer {
    /// Single-entity-per-cell, blocks pathfinding.
    Floor,
    /// Single-entity-per-cell, does not block pathfinding.
    Ceiling,
    /// Multi-entity-per-cell, does not block pathfinding (e.g. table decorations).
    Surface,
}

/// Flat-array spatial index.  One parallel array per layer, O(1) array lookup, no heap fragmentation.
#[derive(Resource)]
pub struct GridLayers {
    pub width: i32,
    pub height: i32,
    floor: Vec<Option<Entity>>,
    ceiling: Vec<Option<Entity>>,
    surface: Vec<SmallVec<[Entity; 4]>>,
    /// Dynamic-agent reservation.  Checked alongside `floor` in `is_walkable`.
    reserved: Vec<Option<Entity>>,
}

impl GridLayers {
    pub fn new(width: i32, height: i32) -> Self {
        let size = (width * height) as usize;
        Self {
            width,
            height,
            floor: vec![None; size],
            ceiling: vec![None; size],
            surface: vec![SmallVec::new(); size],
            reserved: vec![None; size],
        }
    }

    fn index(&self, x: i32, z: i32) -> Option<usize> {
        if x < 0 || x >= self.width || z < 0 || z >= self.height {
            return None;
        }
        Some((z * self.width + x) as usize)
    }

    // ── Floor ──────────────────────────────────────────────

    pub fn try_place_floor(&mut self, cells: &[Cell], entity: Entity) -> bool {
        if cells
            .iter()
            .any(|&(x, z)| self.index(x, z).map_or(true, |i| self.floor[i].is_some()))
        {
            return false;
        }
        for &(x, z) in cells {
            if let Some(i) = self.index(x, z) {
                self.floor[i] = Some(entity);
            }
        }
        true
    }

    pub fn remove_floor(&mut self, cells: &[Cell], entity: Entity) {
        for &(x, z) in cells {
            if let Some(i) = self.index(x, z) {
                if self.floor[i] == Some(entity) {
                    self.floor[i] = None;
                }
            }
        }
    }

    pub fn floor_entity_at(&self, x: i32, z: i32) -> Option<Entity> {
        self.index(x, z).and_then(|i| self.floor[i])
    }

    /// 4-neighbor floor entities (used by interaction-point refresh).
    pub fn floor_neighbors(&self, x: i32, z: i32) -> impl Iterator<Item = Entity> + '_ {
        [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .into_iter()
            .filter_map(move |(dx, dz)| self.floor_entity_at(x + dx, z + dz))
    }

    // ── Ceiling ────────────────────────────────────────────

    pub fn try_place_ceiling(&mut self, cell: Cell, entity: Entity) -> bool {
        let Some(i) = self.index(cell.0, cell.1) else {
            return false;
        };
        if self.ceiling[i].is_some() {
            return false;
        }
        self.ceiling[i] = Some(entity);
        true
    }

    pub fn remove_ceiling(&mut self, cell: Cell, entity: Entity) {
        if let Some(i) = self.index(cell.0, cell.1) {
            if self.ceiling[i] == Some(entity) {
                self.ceiling[i] = None;
            }
        }
    }

    // ── Surface ────────────────────────────────────────────

    pub fn add_surface(&mut self, cell: Cell, entity: Entity) -> bool {
        let Some(i) = self.index(cell.0, cell.1) else {
            return false;
        };
        self.surface[i].push(entity);
        true
    }

    pub fn remove_surface(&mut self, cell: Cell, entity: Entity) {
        if let Some(i) = self.index(cell.0, cell.1) {
            self.surface[i].retain(|e| *e != entity);
        }
    }

    // ── Reservation ────────────────────────────────────────

    pub fn reserve_cell(&mut self, x: i32, z: i32, entity: Entity) -> bool {
        let Some(i) = self.index(x, z) else {
            return false;
        };
        if self.reserved[i].is_some() {
            return false;
        }
        self.reserved[i] = Some(entity);
        true
    }

    pub fn release_cell(&mut self, x: i32, z: i32, entity: Entity) {
        if let Some(i) = self.index(x, z) {
            if self.reserved[i] == Some(entity) {
                self.reserved[i] = None;
            }
        }
    }

    /// Release every cell currently reserved by `entity`.
    pub fn release_all(&mut self, entity: Entity) {
        for slot in self.reserved.iter_mut() {
            if *slot == Some(entity) {
                *slot = None;
            }
        }
    }

    // ── Pathfinding ────────────────────────────────────────

    /// A cell is walkable when it is inside bounds and neither the floor layer
    /// nor the reservation layer blocks it.
    pub fn is_walkable(&self, x: i32, z: i32) -> bool {
        self.index(x, z).map_or(false, |i| {
            self.floor[i].is_none() && self.reserved[i].is_none()
        })
    }
}
