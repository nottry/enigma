use super::*;

#[test]
fn attack_conditions_keep_their_exact_routes() {
    let incoming = find_key(502202, "ActiveUseSkill").unwrap();
    assert_eq!(
        incoming.role,
        ConditionRole::Trigger {
            event: EventKind::SkillEffectStarted,
            phase: Some(SkillPhase::Immediate),
        }
    );
    assert_eq!(
        incoming.skill_action_observer,
        SkillActionObserver::AttackTarget
    );
    assert_eq!(
        incoming.attack_modifier_side,
        Some(AttackModifierSide::IncomingTarget)
    );

    for (opcode, type_name) in [(501209, "UseHurtSkill"), (792209, "UseDeviceSkill")] {
        assert_eq!(
            find_key(opcode, type_name).map(|definition| definition.role),
            Some(ConditionRole::Trigger {
                event: EventKind::TargetAttacked,
                phase: None,
            })
        );
    }
}

#[test]
fn tuning_practice_observes_matching_incantations_at_action_start() {
    assert_eq!(
        parse(34201, "UseSkillEffectTag", &["4".into(), "6".into()]),
        Some(ParsedConditionKind::ActiveSkillEffectTag(vec![4, 6]))
    );
    assert_eq!(
        find_key(34201, "UseSkillEffectTag").map(|definition| definition.role),
        Some(ConditionRole::Trigger {
            event: EventKind::SkillAction,
            phase: Some(SkillPhase::Immediate),
        })
    );
}

#[test]
fn mass_attack_received_keeps_its_target_attacked_route() {
    assert_eq!(
        parse(53209, "HurtNumType", &["2".into()]),
        Some(ParsedConditionKind::ReceivedDamageTargetCountKind(2))
    );
    let definition = find_key(53209, "HurtNumType").unwrap();
    assert_eq!(definition.role, ConditionRole::Predicate);
    assert_eq!(definition.dependencies, &[EventKind::TargetAttacked]);
}

#[test]
fn completed_attack_target_count_keeps_its_after_damage_route() {
    assert_eq!(
        parse(53208, "HurtNumType", &["1".into()]),
        Some(ParsedConditionKind::DamageTargetCountKind(1))
    );
    assert_eq!(
        find_key(53208, "HurtNumType").map(|definition| definition.role),
        Some(ConditionRole::Trigger {
            event: EventKind::SkillAction,
            phase: Some(SkillPhase::AfterDamage),
        })
    );
}

#[test]
fn attack_modifier_target_count_keeps_its_exact_predicate_route() {
    assert_eq!(
        parse(53203, "HurtNumType", &["1".into()]),
        Some(ParsedConditionKind::DamageTargetCountKind(1))
    );
    assert_eq!(
        find_key(53203, "HurtNumType").map(|definition| definition.role),
        Some(ConditionRole::Predicate)
    );
}

#[test]
fn buff_and_obscurity_conditions_keep_their_exact_routes() {
    assert_eq!(
        parse(88, "BuffTypeAdd", &["4".into()]),
        Some(ParsedConditionKind::BuffTypeAdded(vec![4]))
    );
    assert_eq!(
        find_key(88, "BuffTypeAdd").map(|definition| definition.role),
        Some(ConditionRole::Trigger {
            event: EventKind::BuffAdded,
            phase: None,
        })
    );
    assert_eq!(
        find_key(749209, "PowerRatio").map(|definition| definition.role),
        Some(ConditionRole::Trigger {
            event: EventKind::EurekaChanged,
            phase: None,
        })
    );
    let absent = find_key(57209, "NoBuffId").unwrap();
    assert_eq!(absent.role, ConditionRole::Predicate);
    assert_eq!(absent.dependencies, &[EventKind::TargetAttacked]);
}

#[test]
fn share_damage_observes_the_share_hurt_split() {
    assert_eq!(
        find_key(695213, "ShareDamage").map(|definition| definition.role),
        Some(ConditionRole::Trigger {
            event: EventKind::DamageShared,
            phase: None,
        })
    );
}

#[test]
fn settlement_and_round_start_conditions_keep_their_exact_routes() {
    assert_eq!(
        find_key(750307, "PlayerHasBuff").map(|definition| definition.role),
        Some(ConditionRole::Trigger {
            event: EventKind::RoundEndAfterFinalSettlement,
            phase: None,
        })
    );
    assert_eq!(
        find_key(57101, "NoBuffId").map(|definition| definition.role),
        Some(ConditionRole::Setup {
            stage: SetupStage::RoundStartCondition,
            priority: 101,
        })
    );
}

#[test]
fn natural_buff_expiry_observes_buff_removal() {
    assert_eq!(
        parse(515005, "BuffIdExpireOnly", &["4004".into()]),
        Some(ParsedConditionKind::BuffExpired(vec![4004]))
    );
    assert_eq!(
        find_key(515005, "BuffIdExpireOnly").map(|definition| definition.role),
        Some(ConditionRole::Trigger {
            event: EventKind::BuffRemoved,
            phase: None,
        })
    );
}
