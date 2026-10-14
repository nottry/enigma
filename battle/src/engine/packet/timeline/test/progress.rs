use super::*;
use crate::engine::manager::fight_progress::{
    FightProgressChange, FightProgressState, FightProgressValueChange,
};

#[test]
fn progress_initialization_projects_max_before_value() {
    let effects = project_change_with_reduce_hp(
        &BattleChange::FightProgress(FightProgressChange {
            origin: CommandOrigin {
                domain: RuleDomain::Behavior,
                key: DefinitionKey::new(100015, "ProgressInitNew"),
            },
            state: FightProgressState {
                id: 8,
                max: 100,
                value: 0,
                show_id: 10,
                skill_id: 119381117,
            },
        }),
        true,
    )
    .unwrap();

    assert_eq!(effects.len(), 2);
    assert_eq!(
        effects[0].effect_type,
        Some(EffectType::Progressmaxchange as i32)
    );
    assert_eq!(effects[0].target_id, Some(8));
    assert_eq!(effects[0].buff_act_id, Some(8));
    assert_eq!(effects[0].effect_num, Some(100));
    assert_eq!(effects[0].effect_num1, Some(10));
    assert_eq!(effects[0].reserve_str.as_deref(), Some("119381117"));
    assert_eq!(
        effects[1].effect_type,
        Some(EffectType::Progresschange as i32)
    );
    assert_eq!(effects[1].target_id, Some(8));
    assert_eq!(effects[1].effect_num, Some(0));
}

#[test]
fn progress_value_change_projects_the_committed_delta() {
    let state = FightProgressState {
        id: 8,
        max: 100,
        value: 100,
        show_id: 10,
        skill_id: 119381117,
    };
    let [effect] = project_change_with_reduce_hp(
        &BattleChange::FightProgressValue(FightProgressValueChange {
            origin: CommandOrigin {
                domain: RuleDomain::Behavior,
                key: DefinitionKey::new(100016, "ProgressChangeNew"),
            },
            before: state,
            after: FightProgressState { value: 0, ..state },
            delta: -100,
        }),
        true,
    )
    .unwrap()
    .try_into()
    .unwrap();

    assert_eq!(effect.effect_type, Some(EffectType::Progresschange as i32));
    assert_eq!(effect.target_id, Some(8));
    assert_eq!(effect.buff_act_id, Some(8));
    assert_eq!(effect.effect_num, Some(-100));
}
