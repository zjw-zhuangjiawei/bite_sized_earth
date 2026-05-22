use bevy_ecs::prelude::Resource;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum GridOccupancy {
  #[default]
  Empty,
  Occupied,
}

#[derive(Clone, Resource)]
pub struct WorldGridMap {
  pub width: i32,
  pub height: i32,
  pub occupancy: Vec<GridOccupancy>,
}

impl WorldGridMap {
  pub fn new(width: i32, height: i32) -> Self {
    let size = (width * height) as usize;
    Self {
      width,
      height,
      occupancy: vec![GridOccupancy::Empty; size],
    }
  }

  fn index(&self, x: i32, z: i32) -> Option<usize> {
    if x >= 0 && x < self.width && z >= 0 && z < self.height {
      Some((z * self.width + x) as usize)
    } else {
      None
    }
  }

  pub fn is_walkable(&self, x: i32, z: i32) -> bool {
    self
      .index(x, z)
      .map(|i| matches!(self.occupancy[i], GridOccupancy::Empty))
      .unwrap_or(false)
  }

  pub fn set_occupancy(&mut self, x: i32, z: i32, occ: GridOccupancy) {
    if let Some(i) = self.index(x, z) {
      self.occupancy[i] = occ;
    }
  }

  pub fn get_occupancy(&self, x: i32, z: i32) -> Option<GridOccupancy> {
    self.index(x, z).map(|i| self.occupancy[i])
  }

  // ===== 新架构：原子化多格操作 =====

  /// 检查一片区域是否全部为空
  pub fn is_area_empty(&self, footprint: &[(i32, i32)]) -> bool {
    footprint
      .iter()
      .all(|&(x, z)| self.get_occupancy(x, z) == Some(GridOccupancy::Empty))
  }

  /// 批量写入占用
  pub fn fill_area(&mut self, footprint: &[(i32, i32)], occ: GridOccupancy) {
    for &(x, z) in footprint {
      self.set_occupancy(x, z, occ);
    }
  }

  /// 批量清空
  pub fn clear_area(&mut self, footprint: &[(i32, i32)]) {
    for &(x, z) in footprint {
      self.set_occupancy(x, z, GridOccupancy::Empty);
    }
  }
}
