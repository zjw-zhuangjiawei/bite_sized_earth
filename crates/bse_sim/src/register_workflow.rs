use bevy::prelude::*;
use smallvec::SmallVec;

use crate::components::{
  ApplianceGeometry, Customer, CustomerState, CustomerZone, GridDirection, GridPosition,
  MovementProgress, NavigationComplete, ProcessingCustomer, RegisterQueue, RegisterState, Staff,
  StaffState, StaffZone, EXIT_POSITION,
};
use crate::navigation_cmd::NavigateTo;

/// Register customer queue zone: L-shaped line from front-right corner.
/// Goes right 2 cells then forward for remaining length.
pub fn register_queue_zone(
  anchor: (i32, i32),
  geo: &ApplianceGeometry,
  length: u32,
) -> SmallVec<[(i32, i32); 8]> {
  let mut cells = SmallVec::new();
  let (right_dir, forward_dir) = match geo.direction {
    GridDirection::PosZ => ((1, 0), (0, 1)),
    GridDirection::NegZ => ((-1, 0), (0, -1)),
    GridDirection::PosX => ((0, -1), (1, 0)),
    GridDirection::NegX => ((0, 1), (-1, 0)),
  };
  let right_count = 2.min(length);
  for i in 1..=right_count {
    cells.push((
      anchor.0 + right_dir.0 * (geo.right + i as i32 - 1),
      anchor.1 + right_dir.1 * (geo.right + i as i32 - 1),
    ));
  }
  let last = *cells.last().unwrap_or(&(anchor.0, anchor.1));
  for i in 1..=(length.saturating_sub(right_count)) {
    cells.push((
      last.0 + forward_dir.0 * i as i32,
      last.1 + forward_dir.1 * i as i32,
    ));
  }
  cells
}

/// Register staff zone: single cell behind the register center.
pub fn register_staff_zone(
  anchor: (i32, i32),
  geo: &ApplianceGeometry,
) -> SmallVec<[(i32, i32); 8]> {
  let back_dir = match geo.direction {
    GridDirection::PosZ => (0, -1),
    GridDirection::NegZ => (0, 1),
    GridDirection::PosX => (-1, 0),
    GridDirection::NegX => (1, 0),
  };
  SmallVec::from_slice(&[(anchor.0 + back_dir.0, anchor.1 + back_dir.1)])
}

/// Zone system: runs on Added<RegisterState> (init) and Changed<RegisterQueue> (queue updates).
/// CustomerZone stores cells currently free (not occupied by queued customers).
pub fn update_register_zones(
  mut commands: Commands,
  query: Query<
    (
      Entity,
      &GridPosition,
      &ApplianceGeometry,
      Option<&RegisterQueue>,
    ),
    With<RegisterState>,
  >,
) {
  for (entity, pos, geo, queue) in query.iter() {
    let mut cells = register_queue_zone((pos.x, pos.z), geo, 5);
    // Filter out cells "taken" by queued customers (first N cells where N = queue length)
    if let Some(q) = queue {
      let taken = q.customers.len().min(cells.len());
      cells.drain(..taken);
    }
    commands.entity(entity).insert(CustomerZone {
      cells: cells.clone(),
    });
    commands.entity(entity).insert(StaffZone {
      cells: register_staff_zone((pos.x, pos.z), geo),
    });
  }
}

/// Customer arrived at register zone cell -> add to per-register queue.
pub fn customer_arrive_at_register_system(
  mut commands: Commands,
  customer_q: Query<(Entity, &GridPosition, &Customer), With<NavigationComplete>>,
  mut register_q: Query<(Entity, &CustomerZone, &mut RegisterQueue)>,
) {
  for (customer_entity, pos, customer) in customer_q.iter() {
    if customer.state != CustomerState::WalkingToRegister {
      continue;
    }
    // Find the register whose CustomerZone contains this customer's position
    for (_reg_entity, zone, mut queue) in register_q.iter_mut() {
      if zone.cells.contains(&(pos.x, pos.z)) {
        queue.customers.push(customer_entity);
        commands
          .entity(customer_entity)
          .remove::<NavigationComplete>();
        commands.entity(customer_entity).insert(Customer {
          state: CustomerState::WaitingForPayment,
        });
        info!(
          "Customer {:?} queued at register ({} in line)",
          customer_entity,
          queue.customers.len()
        );
        break;
      }
    }
  }
}

/// Staff handles three sub-stages:
/// 1. Idle + non-empty RegisterQueue -> pop customer, go to register
/// 2. WalkingToRegister + NavigationComplete -> start CheckingOut timer
/// 3. CheckingOut timer done -> customer leaves, staff idle
pub fn staff_checkout_system(
  mut commands: Commands,
  time: Res<Time>,
  mut staff_q: Query<
    (
      Entity,
      &GridPosition,
      &mut Staff,
      Option<&NavigationComplete>,
      Option<&ProcessingCustomer>,
    ),
    Without<MovementProgress>,
  >,
  mut register_q: Query<(Entity, &StaffZone, &mut RegisterQueue)>,
  mut customer_q: Query<&mut Customer>,
) {
  for (staff_entity, staff_pos, mut staff, nav_complete, processing) in staff_q.iter_mut() {
    match staff.state {
      StaffState::Idle => {
        for (_reg_entity, staff_zone, mut queue) in register_q.iter_mut() {
          if queue.customers.is_empty() {
            continue;
          }
          let customer_entity = queue.customers.remove(0);
          let target = staff_zone
            .cells
            .first()
            .copied()
            .unwrap_or((staff_pos.x, staff_pos.z));
          commands.entity(staff_entity).insert(ProcessingCustomer {
            customer: customer_entity,
          });
          commands
            .entity(staff_entity)
            .queue(NavigateTo { target, speed: 3.0 });
          staff.state = StaffState::WalkingToRegister;
          info!(
            "Staff heading to register for customer {:?}",
            customer_entity
          );
          break;
        }
      }
      StaffState::WalkingToRegister if nav_complete.is_some() => {
        commands.entity(staff_entity).remove::<NavigationComplete>();
        staff.state = StaffState::CheckingOut(2.0);
      }
      StaffState::CheckingOut(ref mut remaining) => {
        *remaining -= time.delta_secs();
        if *remaining <= 0.0 {
          if let Some(pc) = processing {
            if let Ok(mut customer) = customer_q.get_mut(pc.customer) {
              customer.state = CustomerState::Leaving;
            }
            commands.entity(pc.customer).queue(NavigateTo {
              target: EXIT_POSITION,
              speed: 3.0,
            });
            // TODO: advance remaining queued customers forward one cell
            // (C2->C1, C3->C2, etc.) so the line physically moves up.
          }
          commands.entity(staff_entity).remove::<ProcessingCustomer>();
          staff.state = StaffState::Idle;
          info!("Staff finished checkout, idle");
        }
      }
      _ => {}
    }
  }
}
