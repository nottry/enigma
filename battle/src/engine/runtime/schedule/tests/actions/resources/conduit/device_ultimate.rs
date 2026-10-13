use super::*;

fn fight(ex_point: i32) -> Fight {
    let entity = |uid, model_id, ex_point, ex_point_type| FightEntityInfo {
        uid: Some(uid),
        model_id: Some(model_id),
        current_hp: Some(100_000),
        ex_point: Some(ex_point),
        ex_point_type: Some(ex_point_type),
        attr: Some(HeroAttribute {
            hp: Some(100_000),
            attack: Some(1_000),
            ..Default::default()
        }),
        ..Default::default()
    };
    Fight {
        attacker: Some(FightTeam {
            entitys: vec![entity(
                10,
                3145,
                ex_point,
                crate::engine::manager::ex_point::ExPointKind::DevicePower.as_wire(),
            )],
            ..Default::default()
        }),
        defender: Some(FightTeam {
            entitys: vec![entity(-1, 1001, 0, 0)],
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn selected_unique_device(fight: &Fight) -> (TargetPool, BattleManagers, SkillEffectCatalog) {
    let pool = TargetPool::from_fight(fight);
    let mut managers = BattleManagers::seeded(fight);
    managers
        .conduit
        .execute(
            crate::engine::manager::conduit::ConduitCommand::SelectGroup {
                source_uid: 10,
                group: 3,
            },
        )
        .unwrap();
    let catalog = SkillEffectCatalog::from_fight(config::configs::get(), fight);
    (pool, managers, catalog)
}

fn operation() -> [sonettobuf::FightDeviceOper; 1] {
    [sonettobuf::FightDeviceOper {
        uid: Some(10),
        index: Some(3),
    }]
}

#[test]
fn conduit_ultimate_requires_full_personal_device_power() {
    init_config();
    let fight = fight(99);
    let (pool, mut managers, mut catalog) = selected_unique_device(&fight);

    let error = match run_conduit_phase(
        managers.catalog(),
        &fight,
        &mut managers,
        &pool,
        &mut catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        &operation(),
    ) {
        Ok(_) => panic!("an underfilled personal device gauge authorized its ultimate"),
        Err(error) => error,
    };

    assert_eq!(
        error,
        DrainError::InsufficientUltimateResource {
            owner_uid: 10,
            skill_id: 31450131,
            required: 100,
            current: 99,
        }
    );
}

#[test]
fn conduit_ultimate_spends_personal_device_power_before_casting() {
    init_config();
    let fight = fight(100);
    let (pool, mut managers, mut catalog) = selected_unique_device(&fight);

    let result = run_conduit_phase(
        managers.catalog(),
        &fight,
        &mut managers,
        &pool,
        &mut catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        &operation(),
    )
    .unwrap();

    let steps = crate::engine::packet::timeline::project(&result.frames).unwrap();
    let action = steps
        .iter()
        .find(|step| {
            step.act_effect.iter().any(|effect| {
                effect
                    .fight_step
                    .as_ref()
                    .is_some_and(|skill| skill.act_id == Some(31450131))
            })
        })
        .unwrap();
    let spend = action
        .act_effect
        .iter()
        .position(|effect| {
            effect.effect_type
                == Some(sonettobuf::effect_type_enum::EffectType::Expointchange as i32)
                && effect.target_id == Some(10)
                && effect.effect_num == Some(-100)
        })
        .unwrap();
    let skill = action
        .act_effect
        .iter()
        .position(|effect| {
            effect
                .fight_step
                .as_ref()
                .is_some_and(|skill| skill.act_id == Some(31450131))
        })
        .unwrap();
    assert!(spend < skill);
    assert_eq!(action.card_index, Some(1));
    assert_eq!(action.support_hero_id, Some(1));
    assert_eq!(
        action.act_effect[skill]
            .fight_step
            .as_ref()
            .unwrap()
            .card_index,
        Some(1)
    );
}
