use super::*;

#[test]
fn configured_device_group_change_follows_its_after_hit_effects() {
    crate::test_support::init_config();
    let entity = |uid, model_id, team_type| FightEntityInfo {
        uid: Some(uid),
        model_id: Some(model_id),
        team_type: Some(team_type),
        current_hp: Some(100_000),
        attr: Some(HeroAttribute {
            hp: Some(100_000),
            attack: Some(1_000),
            ..Default::default()
        }),
        ..Default::default()
    };
    for (skill_id, initial_group, switches_group) in [
        (31450181, 1, true),
        (31450182, 1, true),
        (31450181, 2, false),
    ] {
        let fight = Fight {
            attacker: Some(FightTeam {
                entitys: vec![entity(10, 3145, 1)],
                ..Default::default()
            }),
            defender: Some(FightTeam {
                entitys: vec![entity(-1, 1001, 2)],
                ..Default::default()
            }),
            ..Default::default()
        };
        let pool = TargetPool::from_fight(&fight);
        let mut managers = BattleManagers::seeded(&fight);
        managers
            .conduit
            .execute(
                crate::engine::manager::conduit::ConduitCommand::SelectGroup {
                    source_uid: 10,
                    group: initial_group,
                },
            )
            .unwrap();
        let catalog = SkillEffectCatalog::from_roots(config::configs::get(), [skill_id], []);
        let mut invocation: SkillInvocation = SkillRequest {
            source_uid: 10,
            skill_id,
        }
        .into();
        invocation.mode = SkillExecutionMode::Device;
        invocation.target = SkillTarget::Explicit(10);

        let result = run_action(
            &mut managers,
            &pool,
            &catalog,
            &mut RoundDeterminism::default(),
            TargetContext::default(),
            [],
            invocation,
        )
        .unwrap();
        let step = crate::engine::packet::timeline::project(&result.frames)
            .unwrap()
            .into_iter()
            .find(|step| step.act_id == Some(skill_id))
            .expect("the configured device skill projects a step");
        let relevant = step
            .act_effect
            .iter()
            .filter_map(|effect| match effect.effect_type? {
                kind if kind
                    == sonettobuf::effect_type_enum::EffectType::Devicepowerchange as i32 =>
                {
                    Some((kind, effect.config_effect))
                }
                kind if kind == sonettobuf::effect_type_enum::EffectType::Buffadd as i32
                    && effect.buff.as_ref().and_then(|buff| buff.buff_id) == Some(31450121) =>
                {
                    Some((kind, effect.config_effect))
                }
                kind if kind
                    == sonettobuf::effect_type_enum::EffectType::Deviceskillindex as i32 =>
                {
                    Some((kind, effect.config_effect))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut expected = vec![
            (
                sonettobuf::effect_type_enum::EffectType::Devicepowerchange as i32,
                Some(0),
            ),
            (
                sonettobuf::effect_type_enum::EffectType::Buffadd as i32,
                Some(0),
            ),
        ];
        if switches_group {
            expected.push((
                sonettobuf::effect_type_enum::EffectType::Deviceskillindex as i32,
                Some(60293),
            ));
        }

        assert_eq!(relevant, expected);
        assert_eq!(
            managers.conduit.selected_group(10),
            Some(if switches_group { 2 } else { initial_group })
        );
    }
}
