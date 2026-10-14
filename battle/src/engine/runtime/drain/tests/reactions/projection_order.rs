use super::*;

#[test]
fn bendith_buff_drain_projects_add_remove_and_recast_in_capture_order() {
    crate::test_support::init_config();
    let base_group1 = vec![31460114, 31460115, 31460116];
    let base_group2 = vec![31460127, 31460128, 31460129];
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![FightEntityInfo {
                uid: Some(10),
                model_id: Some(3146),
                team_type: Some(1),
                current_hp: Some(100),
                skill_group1: base_group1,
                skill_group2: base_group2,
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    };
    let pool = TargetPool::from_fight(&fight);
    let mut managers = BattleManagers::seeded(&fight);
    managers
        .execute_card(CardCommand::Setup(CardSetup {
            hand: vec![
                CardInfo {
                    uid: Some(10),
                    hero_id: Some(3146),
                    skill_id: Some(31460114),
                    ..Default::default()
                },
                CardInfo {
                    uid: Some(10),
                    hero_id: Some(3146),
                    skill_id: Some(31460127),
                    ..Default::default()
                },
            ],
            draw_pile: Vec::new(),
            deck_num: 2,
        }))
        .unwrap();
    let catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);
    let origin = CommandOrigin {
        domain: RuleDomain::Behavior,
        key: DefinitionKey::new(60001, "AddBuff"),
    };
    let grant = || {
        RuleOp::Command(BattleCommand::Buff(BuffCommand::Grant(BuffGrant {
            origin,
            source_uid: 10,
            target_uid: 10,
            buff_id: 31460137,
            amount: None,
            occurrences: 1,
            child_uid_reservations: 0,
        })))
    };
    let remove = || {
        RuleOp::Command(BattleCommand::Buff(BuffCommand::Remove(BuffRemove {
            origin,
            target_uid: 10,
            selector: BuffRemoveSelector::ExactId(31460137),
        })))
    };
    let projected_types = |result: &crate::engine::runtime::drain::DrainResult| {
        fn collect(effect: &sonettobuf::ActEffect, types: &mut Vec<i32>) {
            let relevant = [
                sonettobuf::effect_type_enum::EffectType::Cardaconvertcardb as i32,
                sonettobuf::effect_type_enum::EffectType::Heroupgrade as i32,
                sonettobuf::effect_type_enum::EffectType::Buffadd as i32,
                sonettobuf::effect_type_enum::EffectType::Buffdel as i32,
            ];
            if effect
                .effect_type
                .is_some_and(|kind| relevant.contains(&kind))
            {
                types.push(effect.effect_type.unwrap());
            }
            if let Some(step) = &effect.fight_step {
                for nested in &step.act_effect {
                    collect(nested, types);
                }
            }
        }

        let mut types = Vec::new();
        for effect in crate::engine::packet::timeline::project(&result.frames)
            .unwrap()
            .iter()
            .flat_map(|step| &step.act_effect)
        {
            collect(effect, &mut types);
        }
        types
    };
    let converted = sonettobuf::effect_type_enum::EffectType::Cardaconvertcardb as i32;
    let upgraded = sonettobuf::effect_type_enum::EffectType::Heroupgrade as i32;
    let added = sonettobuf::effect_type_enum::EffectType::Buffadd as i32;
    let deleted = sonettobuf::effect_type_enum::EffectType::Buffdel as i32;

    let add = run(
        &mut managers,
        &pool,
        &catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        [grant()],
    )
    .unwrap();
    assert_eq!(
        projected_types(&add),
        vec![converted, converted, upgraded, added]
    );

    let recast = run(
        &mut managers,
        &pool,
        &catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        [remove(), grant()],
    )
    .unwrap();
    assert_eq!(
        projected_types(&recast),
        vec![
            converted, converted, upgraded, deleted, converted, converted, upgraded, added,
        ]
    );

    let remove = run(
        &mut managers,
        &pool,
        &catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        [remove()],
    )
    .unwrap();
    assert_eq!(
        projected_types(&remove),
        vec![converted, converted, upgraded, deleted]
    );
}
