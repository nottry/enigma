use super::*;

#[test]
fn a_same_skill_buff_reentry_uses_the_new_events_target() {
    crate::test_support::init_config();
    let entity = |uid, model_id| FightEntityInfo {
        uid: Some(uid),
        model_id: Some(model_id),
        current_hp: Some(100_000),
        attr: Some(HeroAttribute {
            hp: Some(100_000),
            attack: Some(1_000),
            technic: Some(1_000),
            ..Default::default()
        }),
        ..Default::default()
    };
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![
                FightEntityInfo {
                    passive_skill: vec![312301533],
                    buffs: vec![BuffInfo {
                        uid: Some(30),
                        buff_id: Some(2295013),
                        from_uid: Some(10),
                        layer: Some(7),
                        ..Default::default()
                    }],
                    ..entity(10, 3123)
                },
                entity(11, 3124),
                entity(12, 3105),
            ],
            ..Default::default()
        }),
        defender: Some(FightTeam {
            entitys: vec![entity(-1, 1)],
            ..Default::default()
        }),
        ..Default::default()
    };
    let pool = TargetPool::from_fight(&fight);
    let mut managers = BattleManagers::seeded(&fight);
    let catalog =
        SkillEffectCatalog::from_roots(config::configs::get(), [312431214, 312301533], []);
    let mut invocation: SkillInvocation = SkillRequest {
        source_uid: 11,
        skill_id: 312431214,
    }
    .into();
    invocation.target = SkillTarget::Explicit(-1);
    invocation.mode = SkillExecutionMode::Active;
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

    fn master_assassin_grant(step: &sonettobuf::FightStep) -> Option<&sonettobuf::FightStep> {
        if step.act_id == Some(312301533)
            && step.act_effect.iter().any(|effect| {
                effect
                    .buff
                    .as_ref()
                    .is_some_and(|buff| buff.buff_id == Some(2295033))
            })
        {
            return Some(step);
        }
        step.act_effect
            .iter()
            .filter_map(|effect| effect.fight_step.as_ref())
            .find_map(master_assassin_grant)
    }

    let steps = crate::engine::packet::timeline::project(&result.frames).unwrap();
    let grant = steps
        .iter()
        .find_map(master_assassin_grant)
        .expect("Master Assassin is granted after Apprentice Assassin reaches 8 stacks");
    assert_eq!((grant.from_id, grant.to_id), (Some(10), Some(10)));
}
