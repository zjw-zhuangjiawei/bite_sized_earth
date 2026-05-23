use bevy::prelude::*;
use bse_core::components::{
  BelongsToTable, ChairState, Customer, CustomerState, GridPosition, NavigationComplete, SeatedAt,
  TableState, EXIT_POSITION,
};
use crate::navigation_cmd::NavigateTo;

/// Find available chair bound to an Empty table, reserve it.
pub fn customer_find_seat_system(
    mut commands: Commands,
    mut customer_q: Query<
        (Entity, &GridPosition, &mut Customer),
        Without<NavigationComplete>,
    >,
    mut chair_q: Query<(
        Entity,
        &GridPosition,
        &mut ChairState,
        &BelongsToTable,
    )>,
    mut table_q: Query<&mut TableState>,
) {
    for (c_entity, c_pos, mut customer) in customer_q.iter_mut() {
        if customer.state != CustomerState::Entering {
            continue;
        }

        for (chair_entity, chair_pos, mut chair_state, bound) in chair_q.iter_mut() {
            // Check table is empty
            let mut table_empty = false;
            if let Ok(ts) = table_q.get(bound.table) {
                if *ts == TableState::Empty {
                    table_empty = true;
                }
            }
            if !table_empty {
                continue;
            }

            // Mark table as Occupied
            if let Ok(mut ts) = table_q.get_mut(bound.table) {
                *ts = TableState::Occupied;
            }

            *chair_state = ChairState::Reserved;

            commands.entity(c_entity).queue(NavigateTo {
                target: (chair_pos.x, chair_pos.z),
                speed: 3.0,
            });
            commands.entity(c_entity).insert(SeatedAt { chair: chair_entity });

            customer.state = CustomerState::WalkingToSeat;
            info!(
                "Customer at ({},{}) reserved chair at ({},{}) bound to table {:?}",
                c_pos.x, c_pos.z, chair_pos.x, chair_pos.z, bound.table,
            );
            break;
        }
    }
}

/// Customer arrived at chair via NavigationComplete -> place order.
pub fn customer_arrive_at_seat_system(
    mut commands: Commands,
    mut order_queue: ResMut<bse_core::OrderQueue>,
    mut customer_q: Query<(Entity, &mut Customer, &SeatedAt), With<NavigationComplete>>,
    chair_q: Query<&BelongsToTable>,
    mut table_q: Query<&mut TableState>,
) {
    for (entity, mut customer, seated) in customer_q.iter_mut() {
        if customer.state != CustomerState::WalkingToSeat {
            continue;
        }

        let Ok(bound) = chair_q.get(seated.chair) else {
            continue;
        };

        // Mark table as Ordered and push to queue (only once)
        if let Ok(mut ts) = table_q.get_mut(bound.table) {
            if *ts != TableState::Occupied {
                continue;
            }
            *ts = TableState::Ordered;
            order_queue.pending.push_back(bound.table);
            info!("Customer arrived at seat, table {:?} ordered", bound.table);
        }

        customer.state = CustomerState::WaitingForFood;
        commands.entity(entity).remove::<NavigationComplete>();
    }
}

/// Customer waiting for food; start eating when table is Served; tick timer.
pub fn customer_eating_system(
    mut commands: Commands,
    time: Res<Time>,
    mut customer_q: Query<(Entity, &mut Customer, &SeatedAt), Without<NavigationComplete>>,
    chair_q: Query<&BelongsToTable>,
    table_q: Query<&TableState>,
) {
    for (c_entity, mut customer, seated) in customer_q.iter_mut() {
        // Transition: WaitingForFood -> Eating(5.0) when table is Served
        if customer.state == CustomerState::WaitingForFood {
            if let Ok(bound) = chair_q.get(seated.chair) {
                if let Ok(ts) = table_q.get(bound.table) {
                    if *ts == TableState::Served {
                        info!("Food served, customer starting to eat");
                        customer.state = CustomerState::Eating(5.0);
                    }
                }
            }
        }

        // Tick eating timer
        let should_leave = match customer.state {
            CustomerState::Eating(ref mut remaining) => {
                *remaining -= time.delta_secs();
                *remaining <= 0.0
            }
            _ => false,
        };

        if should_leave {
            commands.entity(c_entity).queue(NavigateTo {
                target: EXIT_POSITION,
                speed: 3.0,
            });

            customer.state = CustomerState::Leaving;
            info!("Customer finished eating, leaving");
        }
    }
}

/// Customer reached exit -> despawn, mark table Dirty.
pub fn customer_exit_and_despawn_system(
    mut commands: Commands,
    customer_q: Query<(Entity, &Customer, &SeatedAt), With<NavigationComplete>>,
    chair_q: Query<&BelongsToTable>,
    mut table_q: Query<&mut TableState>,
) {
    for (entity, customer, seated) in customer_q.iter() {
        if customer.state != CustomerState::Leaving {
            continue;
        }

        let Ok(bound) = chair_q.get(seated.chair) else {
            continue;
        };

        if let Ok(mut ts) = table_q.get_mut(bound.table) {
            *ts = TableState::Dirty;
        }
        info!(
            "Customer reached exit, table {:?} dirty, despawning",
            bound.table
        );
        commands.entity(entity).remove::<NavigationComplete>();
        commands.entity(entity).despawn();
    }
}

/// Dirty tables -> Empty, release all bound chairs.
pub fn cleanup_table_system(
    mut chair_q: Query<(&mut ChairState, &BelongsToTable)>,
    mut table_q: Query<(Entity, &mut TableState)>,
) {
    for (te, mut ts) in table_q.iter_mut() {
        if *ts != TableState::Dirty {
            continue;
        }
        *ts = TableState::Empty;

        // Release all chairs bound to this table
        for (mut chair_state, bound) in chair_q.iter_mut() {
            if bound.table == te {
                *chair_state = ChairState::Available;
            }
        }
    }
}
