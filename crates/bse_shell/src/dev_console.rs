use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use bse_sim::components::GridDirection;
use bse_sim::messages::{
  DebugSpawnCustomerRequest, DebugSpawnStaffRequest, RequestDemolishAppliance, RequestPlaceChair,
  RequestPlaceRegister, RequestPlaceStove, RequestPlaceTable,
};

#[derive(Resource)]
pub struct DevConsoleState {
  pub anchor_x: i32,
  pub anchor_z: i32,
  pub direction: GridDirection,
}

impl Default for DevConsoleState {
  fn default() -> Self {
    Self {
      anchor_x: 16,
      anchor_z: 16,
      direction: GridDirection::PosZ,
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

fn direction_section(ui: &mut egui::Ui, direction: &mut GridDirection) {
  ui.group(|ui| {
    ui.label("Direction");
    ui.separator();
    ui.horizontal(|ui| {
      ui.selectable_value(direction, GridDirection::PosZ, "+Z");
      ui.selectable_value(direction, GridDirection::NegX, "-X");
      ui.selectable_value(direction, GridDirection::NegZ, "-Z");
      ui.selectable_value(direction, GridDirection::PosX, "+X");
    });
  });
}

fn placement_section(
  ui: &mut egui::Ui,
  anchor: (i32, i32),
  direction: GridDirection,
  table_writer: &mut MessageWriter<RequestPlaceTable>,
  chair_writer: &mut MessageWriter<RequestPlaceChair>,
  register_writer: &mut MessageWriter<RequestPlaceRegister>,
  stove_writer: &mut MessageWriter<RequestPlaceStove>,
) {
  ui.group(|ui| {
    ui.label("Place Appliance");
    ui.separator();

    if ui.button("Table").clicked() {
      table_writer.write(RequestPlaceTable { anchor, direction });
    }
    if ui.button("Chair").clicked() {
      chair_writer.write(RequestPlaceChair { anchor, direction });
    }
    if ui.button("Register").clicked() {
      register_writer.write(RequestPlaceRegister { anchor, direction });
    }
    if ui.button("Stove").clicked() {
      stove_writer.write(RequestPlaceStove { anchor, direction });
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
  mut stove_writer: MessageWriter<RequestPlaceStove>,
  mut demolish_writer: MessageWriter<RequestDemolishAppliance>,
  mut spawn_staff_writer: MessageWriter<DebugSpawnStaffRequest>,
  mut spawn_customer_writer: MessageWriter<DebugSpawnCustomerRequest>,
) {
  // copy out of ResMut to avoid borrow conflicts through DerefMut
  let mut anchor_x = state.anchor_x;
  let mut anchor_z = state.anchor_z;
  let mut direction = state.direction;

  let Ok(ctx) = contexts.ctx_mut() else {
    return;
  };

  egui::Window::new("Debug Console")
    .default_width(280.0)
    .show(ctx, |ui| {
      anchor_section(ui, &mut anchor_x, &mut anchor_z);
      ui.add_space(8.0);

      direction_section(ui, &mut direction);
      ui.add_space(8.0);

      let anchor = (anchor_x, anchor_z);
      placement_section(
        ui,
        anchor,
        direction,
        &mut table_writer,
        &mut chair_writer,
        &mut register_writer,
        &mut stove_writer,
      );
      ui.add_space(8.0);

      demolish_section(ui, anchor, &mut demolish_writer);
      ui.add_space(8.0);

      spawn_section(
        ui,
        anchor,
        &mut spawn_staff_writer,
        &mut spawn_customer_writer,
      );
    });

  // sync back
  state.anchor_x = anchor_x;
  state.anchor_z = anchor_z;
  state.direction = direction;
}

fn coord_row(ui: &mut egui::Ui, label: &str, value: &mut i32) {
  ui.horizontal(|ui| {
    ui.label(label);
    ui.add(egui::Slider::new(value, GRID_RANGE.clone()));
  });
}
