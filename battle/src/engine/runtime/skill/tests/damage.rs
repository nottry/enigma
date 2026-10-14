use super::*;

enum ActionTargetRoute {
    Direct,
    FromCondition,
}

mod lifecycle;
mod linked_damage;
mod modifiers;
mod routing_and_crit;
