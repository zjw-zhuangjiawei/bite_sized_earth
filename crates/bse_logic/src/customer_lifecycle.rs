use bevy::prelude::*;
use bse_core::components::{
  ChairState, Customer, CustomerState, GridPosition, MovementProgress,
};
use crate::navigation_cmd::NavigateTo;

/// System 1: Entering → WalkingToSeat
/// Finds an unoccupied chair, reserves it, dispatches a NavigateTo.
pub fn customer_find_seat_system(
  mut commands: Commands,
  mut customer_q: Query<
    (Entity, &GridPosition, &mut Customer),
    Without<MovementProgress>,
  >,
  mut chair_q: Query<(&GridPosition, &mut ChairState)>,
) {
  for (c_entity, c_pos, mut customer) in customer_q.iter_mut() {
    if customer.state != CustomerState::Entering {
      continue;
    }

    let Some((chair_pos, mut state)) = chair_q.iter_mut().find(|(_, s)| **s == ChairState::Available) else {
      continue;
    };

    *state = ChairState::Reserved;

    commands.entity(c_entity).queue(NavigateTo {
      target: (chair_pos.x, chair_pos.z),
      speed: 3.0,
    });

    customer.state = CustomerState::WalkingToSeat;

    info!(
      "Customer at ({},{}) reserved chair at ({},{}) and is walking",
      c_pos.x, c_pos.z, chair_pos.x, chair_pos.z,
    );
  }
}

/// System 2: WalkingToSeat → Eating
/// When movement completes and customer is at a chair position, start eating.
pub fn customer_arrive_at_seat_system(
  mut customer_q: Query<(&GridPosition, &mut Customer), Without<MovementProgress>>,
  chair_q: Query<&GridPosition, With<ChairState>>,
) {
  for (c_pos, mut customer) in customer_q.iter_mut() {
    if customer.state != CustomerState::WalkingToSeat {
      continue;
    }

    let is_at_chair = chair_q
      .iter()
      .any(|chair_pos| chair_pos.x == c_pos.x && chair_pos.z == c_pos.z);

    if is_at_chair {
      info!("Customer arrived at seat, starting meal (5s)");
      customer.state = CustomerState::Eating(5.0);
    }
  }
}

/// System 3: Eating → Leaving
/// Decrement eating timer. When it expires, release chair and navigate to exit.
pub fn customer_eating_system(
  mut commands: Commands,
  time: Res<Time>,
  mut customer_q: Query<(Entity, &GridPosition, &mut Customer)>,
  mut chair_q: Query<(&GridPosition, &mut ChairState)>,
) {
  for (c_entity, c_pos, mut customer) in customer_q.iter_mut() {
    let should_leave = match customer.state {
      CustomerState::Eating(ref mut remaining) => {
        *remaining -= time.delta_secs();
        *remaining <= 0.0
      }
      _ => false,
    };

    if !should_leave {
      continue;
    }

    info!("Customer finished meal, releasing chair");

    // Release the chair at the customer's current position
    for (chair_pos, mut state) in chair_q.iter_mut() {
      if chair_pos.x == c_pos.x && chair_pos.z == c_pos.z {
        *state = ChairState::Available;
        break;
      }
    }

    commands.entity(c_entity).queue(NavigateTo {
      target: (0, 0),
      speed: 3.0,
    });

    customer.state = CustomerState::Leaving;

    info!("Customer finished eating, leaving");
  }
}

/// System 4: Leaving → Despawn
/// When customer reaches (0, 0), remove them from the world.
pub fn customer_exit_and_despawn_system(
  mut commands: Commands,
  customer_q: Query<
    (Entity, &GridPosition, &Customer),
    Without<MovementProgress>,
  >,
) {
  for (entity, pos, customer) in customer_q.iter() {
    if customer.state == CustomerState::Leaving && pos.x == 0 && pos.z == 0 {
      info!("Customer reached exit, despawning");
      commands.entity(entity).despawn();
    }
  }
}
