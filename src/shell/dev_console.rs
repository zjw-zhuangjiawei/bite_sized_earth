use crate::model::ModelAssetHandle;
use crate::sim::components::{GridDirection, GridPosition, GridSize};
use crate::sim::construction::{
  demolish_appliance, place_chair, place_register, place_stove, place_table,
};
use crate::sim::dev_systems::{spawn_customer, spawn_staff};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

#[cfg(feature = "dev")]
const DEV_L_SHAPE_ROW_Y: i32 = 8;

#[cfg(feature = "dev")]
pub fn spawn_dev_l_shape_test(mut commands: Commands, assets: Res<AssetServer>) {
  let scale = 1.0 / 32.0;
  for (i, dir) in [
    (10, GridDirection::PosX),
    (15, GridDirection::PosY),
    (20, GridDirection::NegX),
    (25, GridDirection::NegY),
  ] {
    commands.spawn((
      GridPosition {
        x: i,
        y: DEV_L_SHAPE_ROW_Y,
      },
      dir,
      GridSize {
        right: 1,
        forward: 1,
      },
      ModelAssetHandle(assets.load("models/dev_l_shape.ron")),
      Transform::from_xyz(DEV_L_SHAPE_ROW_Y as f32, 0.0, i as f32)
        .with_scale(Vec3::splat(scale))
        .with_rotation(dir.to_bevy_quat()),
    ));
  }
}

#[derive(Resource)]
pub struct DevConsoleState {
  pub anchor_x: i32,
  pub anchor_y: i32,
  pub direction: GridDirection,
}

impl Default for DevConsoleState {
  fn default() -> Self {
    Self {
      anchor_x: 16,
      anchor_y: 16,
      direction: GridDirection::PosX,
    }
  }
}

const GRID_RANGE: std::ops::RangeInclusive<i32> = 0..=31;

fn anchor_section(ui: &mut egui::Ui, anchor_x: &mut i32, anchor_y: &mut i32) {
  ui.group(|ui| {
    ui.label("Anchor Position");
    ui.separator();
    coord_row(ui, "X:", anchor_x);
    coord_row(ui, "Y:", anchor_y);
    ui.horizontal(|ui| {
      ui.label(format!("  ({}, {})", anchor_x, anchor_y));
    });
  });
}

fn direction_section(ui: &mut egui::Ui, direction: &mut GridDirection) {
  ui.group(|ui| {
    ui.label("Direction");
    ui.separator();
    ui.horizontal(|ui| {
      ui.selectable_value(direction, GridDirection::PosX, "+X");
      ui.selectable_value(direction, GridDirection::PosY, "+Y");
      ui.selectable_value(direction, GridDirection::NegX, "-X");
      ui.selectable_value(direction, GridDirection::NegY, "-Y");
    });
  });
}

fn placement_section(
  ui: &mut egui::Ui,
  commands: &mut Commands,
  anchor: (i32, i32),
  direction: GridDirection,
) {
  ui.group(|ui| {
    ui.label("Place Appliance");
    ui.separator();

    if ui.button("Table").clicked() {
      let a = anchor;
      let d = direction;
      commands.queue(move |world: &mut World| {
        place_table(world, a, d);
      });
    }
    if ui.button("Chair").clicked() {
      let a = anchor;
      let d = direction;
      commands.queue(move |world: &mut World| {
        place_chair(world, a, d);
      });
    }
    if ui.button("Register").clicked() {
      let a = anchor;
      let d = direction;
      commands.queue(move |world: &mut World| {
        place_register(world, a, d);
      });
    }
    if ui.button("Stove").clicked() {
      let a = anchor;
      let d = direction;
      commands.queue(move |world: &mut World| {
        place_stove(world, a, d);
      });
    }
  });
}

fn demolish_section(ui: &mut egui::Ui, commands: &mut Commands, anchor: (i32, i32)) {
  ui.group(|ui| {
    ui.label("Demolish");
    ui.separator();
    if ui
      .button(format!("Demolish at ({}, {})", anchor.0, anchor.1))
      .clicked()
    {
      let a = anchor;
      commands.queue(move |world: &mut World| {
        demolish_appliance(world, a);
      });
    }
  });
}

fn spawn_section(ui: &mut egui::Ui, commands: &mut Commands, anchor: (i32, i32)) {
  ui.group(|ui| {
    ui.label("Spawn Entity");
    ui.separator();

    if ui.button("Customer").clicked() {
      let (gx, gy) = anchor;
      commands.queue(move |world: &mut World| {
        spawn_customer(world, gx, gy);
      });
    }

    ui.add_space(4.0);

    if ui.button("Staff").clicked() {
      let (gx, gy) = anchor;
      commands.queue(move |world: &mut World| {
        spawn_staff(world, gx, gy);
      });
    }
  });
}

pub fn render_egui_console(
  mut contexts: EguiContexts,
  mut state: ResMut<DevConsoleState>,
  mut commands: Commands,
) {
  // copy out of ResMut to avoid borrow conflicts through DerefMut
  let mut anchor_x = state.anchor_x;
  let mut anchor_y = state.anchor_y;
  let mut direction = state.direction;

  let Ok(ctx) = contexts.ctx_mut() else {
    return;
  };

  egui::Window::new("Debug Console")
    .default_width(280.0)
    .show(ctx, |ui| {
      anchor_section(ui, &mut anchor_x, &mut anchor_y);
      ui.add_space(8.0);

      direction_section(ui, &mut direction);
      ui.add_space(8.0);

      let anchor = (anchor_x, anchor_y);
      placement_section(ui, &mut commands, anchor, direction);
      ui.add_space(8.0);

      demolish_section(ui, &mut commands, anchor);
      ui.add_space(8.0);

      spawn_section(ui, &mut commands, anchor);
    });

  // sync back
  state.anchor_x = anchor_x;
  state.anchor_y = anchor_y;
  state.direction = direction;
}

fn coord_row(ui: &mut egui::Ui, label: &str, value: &mut i32) {
  ui.horizontal(|ui| {
    ui.label(label);
    ui.add(egui::Slider::new(value, GRID_RANGE.clone()));
  });
}
