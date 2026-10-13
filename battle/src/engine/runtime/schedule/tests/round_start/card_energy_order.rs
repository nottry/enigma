use super::*;

#[test]
fn round_start_generated_cards_exist_before_card_energy_allocation() {
    init_config();
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![FightEntityInfo {
                uid: Some(10),
                model_id: Some(3124),
                current_hp: Some(100),
                ex_point: Some(10),
                ex_point_type: Some(3),
                buffs: vec![
                    BuffInfo {
                        uid: Some(20),
                        buff_id: Some(312451407),
                        from_uid: Some(10),
                        ..Default::default()
                    },
                    BuffInfo {
                        uid: Some(21),
                        buff_id: Some(2240000),
                        from_uid: Some(10),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    };
    let pool = TargetPool::from_fight(&fight);
    let mut managers = BattleManagers::seeded(&fight);
    let features = managers.buff.active_features(&managers.hp);
    let enable = enable_rule_ops(
        crate::catalog::impromptu_definition(crate::test_support::game_data()),
        &managers.gauge,
        &features,
        10,
    )
    .pop()
    .unwrap();
    for op in [enable.team_energy, enable.inspiration] {
        let RuleOp::Command(BattleCommand::Gauge(command)) = op else {
            panic!("impromptu enable emits gauge commands");
        };
        managers.execute_gauge(command).unwrap();
    }
    let tag = features
        .iter()
        .find(|feature| buff_act::is_kind(feature, buff_act::registry::BuffActKind::EmitterTag))
        .unwrap();
    managers
        .execute_gauge(crate::engine::manager::gauge::GaugeCommand::new(
            buff_act::feature_command_origin(tag).unwrap(),
            team_energy_key(1),
            crate::engine::manager::gauge::GaugeOperation::ChangeValue { delta: 1 },
        ))
        .unwrap();
    let mut determinism = RoundDeterminism::default();
    determinism.enqueue_card_energy_snapshot(vec![CardInfo {
        uid: Some(10),
        skill_id: Some(312451035),
        energy: Some(1),
        ..Default::default()
    }]);

    let (_, next_round) = run_round_start_split(
        &mut managers,
        &pool,
        &SkillEffectCatalog::default(),
        &mut determinism,
        TargetContext::default(),
        1,
    )
    .unwrap();
    let steps = crate::engine::packet::timeline::project(&next_round.frames).unwrap();
    let reset = steps
        .iter()
        .position(|step| {
            step.act_effect.iter().any(|effect| {
                effect.fight_step.as_ref().is_some_and(|nested| {
                    nested.act_effect.iter().any(|effect| {
                        effect.effect_type
                            == Some(sonettobuf::effect_type_enum::EffectType::Expointchange as i32)
                            && effect.effect_num == Some(-10)
                    })
                })
            })
        })
        .unwrap();
    let allocation = steps
        .iter()
        .position(|step| {
            step.act_effect.iter().any(|effect| {
                effect.effect_type
                    == Some(sonettobuf::effect_type_enum::EffectType::Allocatecardenergy as i32)
                    && effect
                        .card_info_list
                        .iter()
                        .any(|card| card.skill_id == Some(312451035))
            })
        })
        .unwrap();

    assert!(reset < allocation);
    assert_eq!(managers.ex_point.get(10), 0);
}
