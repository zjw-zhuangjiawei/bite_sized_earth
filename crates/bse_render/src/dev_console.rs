use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use bse_core::components::GridRotation;
use bse_core::messages::{
  DebugSpawnCustomerRequest, DebugSpawnStaffRequest, RequestDemolishAppliance, RequestPlaceChair,
  RequestPlaceRegister, RequestPlaceTable,
};

#[derive(Resource)]
pub struct DevConsoleState {
  pub anchor_x: i32,
  pub anchor_z: i32,
  pub rotation: GridRotation,
}

impl Default for DevConsoleState {
  fn default() -> Self {
    Self {
      anchor_x: 16,
      anchor_z: 16,
      rotation: GridRotation::North,
    }
  }
}

const GRID_RANGE: std::ops::RangeInclusive<i32> = 0..=31;

fn anchor_section(ui: &mut egui::Ui, anchor_x: &mut i32, anchor_z: &mut i32) {
  ui.group(|ui| {
    ui.label("Anchor Position");
    ui.separator();
    coord_row(ui, "X:", anchor_x);
    coord_row(ui, "Z:", anchor_z);
    ui.horizontal(|ui| {
      ui.label(format!("  ({}, {})", anchor_x, anchor_z));
    });
  });
}

fn rotation_section(ui: &mut egui::Ui, rotation: &mut GridRotation) {
  ui.group(|ui| {
    ui.label("Rotation");
    ui.separator();
    ui.horizontal(|ui| {
      ui.selectable_value(rotation, GridRotation::North, "N");
      ui.selectable_value(rotation, GridRotation::East, "E");
      ui.selectable_value(rotation, GridRotation::South, "S");
      ui.selectable_value(rotation, GridRotation::West, "W");
    });
  });
}

fn placement_section(
  ui: &mut egui::Ui,
  anchor: (i32, i32),
  rotation: GridRotation,
  table_writer: &mut MessageWriter<RequestPlaceTable>,
  chair_writer: &mut MessageWriter<RequestPlaceChair>,
  register_writer: &mut MessageWriter<RequestPlaceRegister>,
) {
  ui.group(|ui| {
    ui.label("Place Appliance");
    ui.separator();

    if ui.button("Table").clicked() {
      table_writer.write(RequestPlaceTable { anchor, rotation });
    }
    if ui.button("Chair").clicked() {
      chair_writer.write(RequestPlaceChair { anchor, rotation });
    }
    if ui.button("Register").clicked() {
      register_writer.write(RequestPlaceRegister { anchor, rotation });
    }
  });
}

fn demolish_section(
  ui: &mut egui::Ui,
  anchor: (i32, i32),
  demolish_writer: &mut MessageWriter<RequestDemolishAppliance>,
) {
  ui.group(|ui| {
    ui.label("Demolish");
    ui.separator();
    if ui
      .button(format!("Demolish at ({}, {})", anchor.0, anchor.1))
      .clicked()
    {
      demolish_writer.write(RequestDemolishAppliance { click: anchor });
    }
  });
}

fn spawn_section(
  ui: &mut egui::Ui,
  anchor: (i32, i32),
  staff_writer: &mut MessageWriter<DebugSpawnStaffRequest>,
  customer_writer: &mut MessageWriter<DebugSpawnCustomerRequest>,
) {
  ui.group(|ui| {
    ui.label("Spawn Entity");
    ui.separator();

    if ui.button("Customer").clicked() {
      customer_writer.write(DebugSpawnCustomerRequest {
        grid_x: anchor.0,
        grid_z: anchor.1,
      });
    }

    ui.add_space(4.0);

    if ui.button("Staff").clicked() {
      staff_writer.write(DebugSpawnStaffRequest {
        grid_x: anchor.0,
        grid_z: anchor.1,
      });
    }
  });
}

pub fn render_egui_console(
  mut contexts: EguiContexts,
  mut state: ResMut<DevConsoleState>,
  mut table_writer: MessageWriter<RequestPlaceTable>,
  mut chair_writer: MessageWriter<RequestPlaceChair>,
  mut register_writer: MessageWriter<RequestPlaceRegister>,
  mut demolish_writer: MessageWriter<RequestDemolishAppliance>,
  mut spawn_staff_writer: MessageWriter<DebugSpawnStaffRequest>,
  mut spawn_customer_writer: MessageWriter<DebugSpawnCustomerRequest>,
) {
  // copy out of ResMut to avoid borrow conflicts through DerefMut
  let mut anchor_x = state.anchor_x;
  let mut anchor_z = state.anchor_z;
  let mut rotation = state.rotation;

  let Ok(ctx) = contexts.ctx_mut() else {
    return;
  };

  egui::Window::new("Debug Console")
    .default_width(280.0)
    .show(ctx, |ui| {
      anchor_section(ui, &mut anchor_x, &mut anchor_z);
      ui.add_space(8.0);

      rotation_section(ui, &mut rotation);
      ui.add_space(8.0);

      let anchor = (anchor_x, anchor_z);
      placement_section(
        ui, anchor, rotation,
        &mut table_writer, &mut chair_writer, &mut register_writer,
      );
      ui.add_space(8.0);

      demolish_section(ui, anchor, &mut demolish_writer);
      ui.add_space(8.0);

      spawn_section(ui, anchor, &mut spawn_staff_writer, &mut spawn_customer_writer);
    });

  // sync back
  state.anchor_x = anchor_x;
  state.anchor_z = anchor_z;
  state.rotation = rotation;
}

fn coord_row(ui: &mut egui::Ui, label: &str, value: &mut i32) {
  ui.horizontal(|ui| {
    ui.label(label);
    ui.add(egui::Slider::new(value, GRID_RANGE.clone()));
  });
}
