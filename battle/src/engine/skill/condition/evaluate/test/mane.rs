use super::*;

fn target_pool() -> TargetPool {
    TargetPool::from_fight(&Fight {
        defender: Some(FightTeam {
            entitys: vec![FightEntityInfo {
                uid: Some(-1),
                current_hp: Some(1),
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    })
}

#[test]
fn natural_buff_expiry_rejects_other_removals() {
    init_config();
    let condition = exact_condition(515005, "BuffIdExpireOnly", &["4004"]);
    let pool = target_pool();
    let matches = |duration_expired| {
        conditions_match(
            std::slice::from_ref(&condition),
            10,
            &[-1],
            None,
            &pool,
            TargetContext {
                removed_buff_id: 4004,
                removed_buff_target_uid: -1,
                removed_buff_duration_expired: duration_expired,
                ..Default::default()
            },
        )
    };

    assert!(matches(true));
    assert!(!matches(false));
}

#[test]
fn buff_category_condition_matches_only_the_added_status() {
    init_config();
    let condition = exact_condition(88, "BuffTypeAdd", &["4"]);
    let pool = target_pool();
    let matches = |status_id| {
        conditions_match(
            std::slice::from_ref(&condition),
            10,
            &[-1],
            None,
            &pool,
            TargetContext {
                added_buff_amount: 1,
                added_buff_target_uid: -1,
                added_buff_status_id: status_id,
                ..Default::default()
            },
        )
    };

    assert!(matches(4));
    assert!(!matches(6));
}

#[test]
fn received_target_count_kind_matches_only_the_passive_owner() {
    init_config();
    let condition = exact_condition(53209, "HurtNumType", &["2"]);
    let pool = target_pool();
    let matches = |hit_target_uid| {
        conditions_match(
            std::slice::from_ref(&condition),
            -1,
            &[-1],
            None,
            &pool,
            TargetContext {
                damage_target_count_kind: 2,
                hit_target_uid,
                ..Default::default()
            },
        )
    };

    assert!(matches(-1));
    assert!(!matches(-2));
}

#[test]
fn damage_target_count_kind_rejects_non_attacks() {
    init_config();
    let condition = exact_condition(53208, "HurtNumType", &["1"]);
    let pool = target_pool();
    let matches = |active_skill_is_attack| {
        conditions_match(
            std::slice::from_ref(&condition),
            10,
            &[10],
            None,
            &pool,
            TargetContext {
                active_skill_is_attack,
                damage_target_count_kind: 1,
                ..Default::default()
            },
        )
    };

    assert!(matches(true));
    assert!(!matches(false));
}
