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

/// How long a reservation lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReservationDuration {
  /// Decays by 1 each tick, removed at 0.
  Timed(u8),
  /// Persists until explicitly released.
  Permanent,
}

/// A cell reservation for dynamic obstacle avoidance.
#[derive(Clone, Debug)]
pub struct ReservationEntry {
  pub entity: Entity,
  pub duration: ReservationDuration,
  pub intent: CellIntent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellIntent {
  /// Agent intends to move here but hasn't entered yet. Can be bumped.
  Intend,
  /// Agent is actively passing through to the next cell.
  Transient,
  /// Agent is stationary in this cell, highest priority.
  Occupy,
}

/// Result of attempting to reserve a cell.
#[derive(Debug)]
pub enum ReserveResult {
  Claimed,
  Blocked,
  Preempted(Entity),
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
  reserved: Vec<Option<ReservationEntry>>,
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

  fn index(&self, x: i32, y: i32) -> Option<usize> {
    if x < 0 || x >= self.width || y < 0 || y >= self.height {
      return None;
    }
    Some((y * self.width + x) as usize)
  }

  // ── Floor ──────────────────────────────────────────────

  pub fn try_place_floor(&mut self, cells: &[Cell], entity: Entity) -> bool {
    if cells
      .iter()
      .any(|&(x, y)| self.index(x, y).map_or(true, |i| self.floor[i].is_some()))
    {
      return false;
    }
    for &(x, y) in cells {
      if let Some(i) = self.index(x, y) {
        self.floor[i] = Some(entity);
      }
    }
    true
  }

  pub fn remove_floor(&mut self, cells: &[Cell], entity: Entity) {
    for &(x, y) in cells {
      if let Some(i) = self.index(x, y) {
        if self.floor[i] == Some(entity) {
          self.floor[i] = None;
        }
      }
    }
  }

  pub fn floor_entity_at(&self, x: i32, y: i32) -> Option<Entity> {
    self.index(x, y).and_then(|i| self.floor[i])
  }

  /// 4-neighbor floor entities (used by interaction-point refresh).
  pub fn floor_neighbors(&self, x: i32, y: i32) -> impl Iterator<Item = Entity> + '_ {
    [(1, 0), (-1, 0), (0, 1), (0, -1)]
      .into_iter()
      .filter_map(move |(dx, dy)| self.floor_entity_at(x + dx, y + dy))
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

  pub fn reserve_cell(&mut self, x: i32, y: i32, entity: Entity) -> bool {
    let Some(i) = self.index(x, y) else {
      return false;
    };
    if self.reserved[i].is_some() {
      return false;
    }
    self.reserved[i] = Some(ReservationEntry {
      entity,
      duration: ReservationDuration::Permanent,
      intent: CellIntent::Occupy,
    });
    true
  }

  pub fn release_cell(&mut self, x: i32, y: i32, entity: Entity) {
    if let Some(i) = self.index(x, y) {
      if self.reserved[i]
        .as_ref()
        .map_or(false, |r| r.entity == entity)
      {
        self.reserved[i] = None;
      }
    }
  }

  /// Release every cell currently reserved by `entity`.
  pub fn release_all(&mut self, entity: Entity) {
    for slot in self.reserved.iter_mut() {
      if slot.as_ref().map_or(false, |r| r.entity == entity) {
        *slot = None;
      }
    }
  }

  /// Upgrade an existing reservation to Permanent (e.g. on arrival at destination).
  pub fn make_permanent(&mut self, x: i32, y: i32, entity: Entity) -> bool {
    let Some(i) = self.index(x, y) else {
      return false;
    };
    match &self.reserved[i] {
      Some(entry) if entry.entity == entity => {
        self.reserved[i] = Some(ReservationEntry {
          entity,
          duration: ReservationDuration::Permanent,
          intent: CellIntent::Occupy,
        });
        true
      }
      _ => false,
    }
  }

  // ── Pathfinding ────────────────────────────────────────

  /// A cell is walkable when it is inside bounds and neither the floor layer
  /// nor the reservation layer blocks it.
  pub fn is_walkable(&self, x: i32, y: i32) -> bool {
    self.index(x, y).map_or(false, |i| {
      self.floor[i].is_none() && self.reserved[i].is_none()
    })
  }

  /// Like `is_walkable` but excludes the given entity's own reservations.
  pub fn is_walkable_for(&self, x: i32, y: i32, entity: Entity) -> bool {
    self.index(x, y).map_or(false, |i| {
      self.floor[i].is_none()
        && self.reserved[i]
          .as_ref()
          .map_or(true, |r| r.entity == entity)
    })
  }

  /// Atomic check-and-claim with priority.  Computes TTL from speed.
  pub fn try_reserve(
    &mut self,
    x: i32,
    y: i32,
    entity: Entity,
    intent: CellIntent,
    speed: f32,
    tick_delta: f32,
  ) -> ReserveResult {
    let Some(i) = self.index(x, y) else {
      return ReserveResult::Blocked;
    };
    if speed <= 0.0 || tick_delta <= 0.0 {
      return ReserveResult::Blocked;
    }
    let ttl = (1.0 / speed / tick_delta).ceil() as u8;

    match &self.reserved[i] {
      None => {
        self.reserved[i] = Some(ReservationEntry {
          entity,
          duration: ReservationDuration::Timed(ttl),
          intent,
        });
        ReserveResult::Claimed
      }
      Some(existing) if existing.entity == entity => {
        // Already ours — refresh TTL
        self.reserved[i] = Some(ReservationEntry {
          entity,
          duration: ReservationDuration::Timed(ttl),
          intent,
        });
        ReserveResult::Claimed
      }
      Some(existing) => {
        // Priority: higher intent + higher entity low-bits as tiebreak
        let our_priority = priority_value(entity, intent);
        let their_priority = priority_value(existing.entity, existing.intent);
        if our_priority > their_priority {
          let bumped = existing.entity;
          self.reserved[i] = Some(ReservationEntry {
            entity,
            duration: ReservationDuration::Timed(ttl),
            intent,
          });
          ReserveResult::Preempted(bumped)
        } else {
          ReserveResult::Blocked
        }
      }
    }
  }

  /// Decay all TTLs, expire stale reservations.
  pub fn tick_reservations(&mut self) {
    for entry in self.reserved.iter_mut() {
      let mut should_clear = false;
      if let Some(r) = entry {
        match r.duration {
          ReservationDuration::Timed(ref mut ttl) => {
            *ttl = ttl.saturating_sub(1);
            if *ttl == 0 {
              should_clear = true;
            }
          }
          ReservationDuration::Permanent => {}
        }
      }
      if should_clear {
        *entry = None;
      }
    }
  }
}

fn priority_value(entity: Entity, intent: CellIntent) -> u8 {
  let base = match intent {
    CellIntent::Occupy => 3,
    CellIntent::Transient => 2,
    CellIntent::Intend => 1,
  };
  // Use low 4 bits of entity ID as tiebreaker (stable, deterministic)
  (base << 4) | (entity.to_bits() as u8 & 0x0F)
}
