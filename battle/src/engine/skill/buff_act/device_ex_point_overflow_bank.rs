use sonettobuf::{BuffActInfo, effect_type_enum::EffectType};

use crate::engine::{
    event::{kind::EventKind, payload::BattleEvent},
    manager::{
        BattleManagers,
        buff::{BuffAccumulateCappedActState, BuffActInfoMarkerResult, BuffCommand, BuffSetState},
        ex_point::{ExPointChange, ExPointCommand, ExPointKind},
    },
    skill::{
        buff_act::BuffActRuleOp,
        rule::output::{BattleCommand, RuleOp},
        subscriber::BuffActSubscriber,
    },
};

pub fn rule_ops(
    managers: &BattleManagers,
    subscriber: &BuffActSubscriber,
    event: &BattleEvent,
) -> Option<Vec<BuffActRuleOp>> {
    if !super::subscriber_is_kind(
        subscriber,
        super::registry::BuffActKind::DeviceExPointOverflowBank,
    ) {
        return Some(Vec::new());
    }
    match event {
        BattleEvent::Kind(EventKind::PostActionRefill) => {
            replenishment_rule_ops(managers, subscriber)
        }
        BattleEvent::ExPointOverflow(change) => store_rule_ops(managers, subscriber, change),
        BattleEvent::ExPointChanged(change) => consume_rule_ops(managers, subscriber, change),
        _ => Some(Vec::new()),
    }
}

fn replenishment_rule_ops(
    managers: &BattleManagers,
    subscriber: &BuffActSubscriber,
) -> Option<Vec<BuffActRuleOp>> {
    let stored = current(
        &managers
            .buff
            .snapshot(subscriber.owner_uid, subscriber.buff_uid)?,
        subscriber.key.definition.opcode,
    )?;
    if stored <= 0
        || !subscriber.owner_alive
        || managers.ex_point.is_full(subscriber.owner_uid)
        || managers.buff.has_buff_act_kind(
            subscriber.owner_uid,
            super::registry::BuffActKind::ExPointCantAdd,
        )
        || managers.buff.has_buff_act_kind(
            subscriber.owner_uid,
            super::registry::BuffActKind::TransferAddExPoint,
        )
    {
        return Some(Vec::new());
    }
    let origin = super::command_origin(subscriber)?;
    Some(vec![BuffActRuleOp::subscriber(RuleOp::Command(
        BattleCommand::ExPoint(ExPointCommand::Change(ExPointChange {
            origin,
            source_uid: subscriber.owner_uid,
            target_uid: subscriber.owner_uid,
            delta: stored,
            config_effect: 0,
            effect_type: EffectType::Expointchange as i32,
        })),
    ))])
}

fn store_rule_ops(
    managers: &BattleManagers,
    subscriber: &BuffActSubscriber,
    change: &crate::engine::event::payload::ExPointChangeEvent,
) -> Option<Vec<BuffActRuleOp>> {
    let [capacity] = subscriber.args.as_slice() else {
        return None;
    };
    if change.overflow <= 0
        || change.kind != ExPointKind::DevicePower
        || change.target_uid != subscriber.owner_uid
        || change.origin == super::command_origin(subscriber)?
    {
        return Some(Vec::new());
    }
    let current = current(
        &managers
            .buff
            .snapshot(subscriber.owner_uid, subscriber.buff_uid)?,
        subscriber.key.definition.opcode,
    )?;
    let maximum = capacity.saturating_mul(subscriber.amount.max(1));
    let stored = change.overflow.min((maximum - current).max(0));
    if stored <= 0 {
        return Some(Vec::new());
    }
    Some(vec![BuffActRuleOp::grouped_event(RuleOp::Command(
        BattleCommand::Buff(BuffCommand::AccumulateCappedActState(
            BuffAccumulateCappedActState {
                origin: super::command_origin(subscriber)?,
                target_uid: subscriber.owner_uid,
                buff_uid: subscriber.buff_uid,
                act_id: subscriber.key.definition.opcode,
                delta: stored,
                maximum,
            },
        )),
    ))])
}

fn consume_rule_ops(
    managers: &BattleManagers,
    subscriber: &BuffActSubscriber,
    change: &crate::engine::event::payload::ExPointChangeEvent,
) -> Option<Vec<BuffActRuleOp>> {
    if change.origin != super::command_origin(subscriber)?
        || change.source_uid != subscriber.owner_uid
        || change.target_uid != subscriber.owner_uid
        || change.kind != ExPointKind::DevicePower
        || change.applied_delta <= 0
    {
        return Some(Vec::new());
    }
    let buff = managers
        .buff
        .snapshot(subscriber.owner_uid, subscriber.buff_uid)?;
    let current = current(&buff, subscriber.key.definition.opcode)?;
    let after = current - change.applied_delta.min(current);
    if after == current {
        return Some(Vec::new());
    }
    let mut act_info = buff.act_info;
    set_current(&mut act_info, subscriber.key.definition.opcode, after)?;
    let origin = super::command_origin(subscriber)?;
    Some(vec![
        BuffActRuleOp::causing(RuleOp::Command(BattleCommand::Buff(
            BuffCommand::SetInternalState(BuffSetState {
                origin,
                target_uid: subscriber.owner_uid,
                buff_uid: subscriber.buff_uid,
                ex_info: None,
                params: None,
                act_info: Some(act_info),
            }),
        ))),
        BuffActRuleOp::causing(RuleOp::BuffActInfoMarker(BuffActInfoMarkerResult {
            target_uid: subscriber.owner_uid,
            buff_uid: subscriber.buff_uid,
            act_id: subscriber.key.definition.opcode,
            params: vec![after],
            str_param: Some(String::new()),
            team_type: 0,
        })),
    ])
}

fn current(buff: &sonettobuf::BuffInfo, act_id: i32) -> Option<i32> {
    let mut matching = buff
        .act_info
        .iter()
        .filter(|info| info.act_id == Some(act_id));
    let info = matching.next()?;
    if matching.next().is_some() || info.str_param.as_deref() != Some("") {
        return None;
    }
    let [value] = info.param.as_slice() else {
        return None;
    };
    (*value >= 0).then_some(*value)
}

fn set_current(act_info: &mut [BuffActInfo], act_id: i32, value: i32) -> Option<()> {
    let mut matching = act_info
        .iter_mut()
        .filter(|info| info.act_id == Some(act_id));
    let info = matching.next()?;
    if matching.next().is_some() || info.str_param.as_deref() != Some("") {
        return None;
    }
    info.param = vec![value];
    info.str_param = Some(String::new());
    Some(())
}

#[cfg(test)]
mod tests {
    use sonettobuf::{BuffInfo, Fight, FightEntityInfo, FightTeam};

    use crate::engine::{
        event::payload::ExPointChangeEvent,
        manager::BattleManagers,
        skill::rule::{CommandOrigin, DefinitionKey, RuleDomain},
    };

    use super::*;

    fn managers(stored: i32, ex_point: i32) -> BattleManagers {
        crate::test_support::init_config();
        BattleManagers::seeded(&Fight {
            attacker: Some(FightTeam {
                entitys: vec![FightEntityInfo {
                    uid: Some(10),
                    model_id: Some(3145),
                    current_hp: Some(100),
                    ex_point: Some(ex_point),
                    ex_point_max: Some(100),
                    ex_point_type: Some(ExPointKind::DevicePower.as_wire()),
                    buffs: vec![BuffInfo {
                        uid: Some(20),
                        buff_id: Some(31450173),
                        from_uid: Some(10),
                        act_info: vec![BuffActInfo {
                            act_id: Some(1149),
                            param: vec![stored],
                            str_param: Some(String::new()),
                        }],
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        })
    }

    fn subscriber(managers: &BattleManagers, event: EventKind) -> BuffActSubscriber {
        let feature = managers
            .buff
            .active_features(&managers.hp)
            .into_iter()
            .find(|feature| {
                super::super::is_kind(
                    feature,
                    super::super::registry::BuffActKind::DeviceExPointOverflowBank,
                )
            })
            .expect("device overflow bank feature should be active");
        super::super::subscriber_from_feature(feature, event).unwrap()
    }

    #[test]
    fn device_overflow_accumulates_only_up_to_the_configured_reserve() {
        let managers = managers(48, 90);
        let subscriber = subscriber(&managers, EventKind::ExPointOverflow);
        let event = BattleEvent::ExPointOverflow(ExPointChangeEvent {
            origin: CommandOrigin {
                domain: RuleDomain::Behavior,
                key: DefinitionKey::new(60292, "AddDeviceExPoint"),
            },
            source_uid: 10,
            target_uid: 10,
            kind: ExPointKind::DevicePower,
            before: 90,
            requested_delta: 15,
            applied_delta: 10,
            after: 100,
            overflow: 5,
        });

        let ops = rule_ops(&managers, &subscriber, &event).expect("device overflow state is valid");
        let [op] = ops.as_slice() else {
            panic!("device overflow should emit one state command");
        };
        assert!(matches!(
            &op.op,
            RuleOp::Command(BattleCommand::Buff(BuffCommand::AccumulateCappedActState(
                BuffAccumulateCappedActState {
                    act_id: 1149,
                    delta: 2,
                    maximum: 50,
                    ..
                }
            )))
        ));
    }

    #[test]
    fn replenishment_uses_device_energy_and_reduces_only_the_applied_amount() {
        let managers = managers(15, 90);
        let subscriber = subscriber(&managers, EventKind::PostActionRefill);
        let replenishment = replenishment_rule_ops(&managers, &subscriber).unwrap();
        let [op] = replenishment.as_slice() else {
            panic!("stored device energy should schedule one replenishment");
        };
        assert!(matches!(
            &op.op,
            RuleOp::Command(BattleCommand::ExPoint(ExPointCommand::Change(
                ExPointChange { delta: 15, .. }
            )))
        ));
        let event = BattleEvent::ExPointChanged(ExPointChangeEvent {
            origin: super::super::command_origin(&subscriber).unwrap(),
            source_uid: 10,
            target_uid: 10,
            kind: ExPointKind::DevicePower,
            before: 90,
            requested_delta: 15,
            applied_delta: 10,
            after: 100,
            overflow: 5,
        });

        let ops = rule_ops(&managers, &subscriber, &event).unwrap();
        assert!(matches!(
            &ops[0].op,
            RuleOp::Command(BattleCommand::Buff(BuffCommand::SetInternalState(
                BuffSetState { act_info: Some(info), .. }
            ))) if info[0].param == [5]
        ));
        assert!(matches!(
            &ops[1].op,
            RuleOp::BuffActInfoMarker(BuffActInfoMarkerResult {
                act_id: 1149,
                params,
                ..
            }) if params == &[5]
        ));
    }
}
