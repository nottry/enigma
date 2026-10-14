use super::*;

fn find_step(step: &sonettobuf::FightStep, act_id: i32) -> Option<&sonettobuf::FightStep> {
    (step.act_id == Some(act_id)).then_some(step).or_else(|| {
        step.act_effect
            .iter()
            .filter_map(|effect| effect.fight_step.as_ref())
            .find_map(|child| find_step(child, act_id))
    })
}

#[test]
fn conduit_skill_finishes_after_its_hit_passive_reaction() {
    init_config();
    let entity = |uid, model_id| FightEntityInfo {
        uid: Some(uid),
        model_id: Some(model_id),
        current_hp: Some(100_000),
        attr: Some(HeroAttribute {
            hp: Some(100_000),
            attack: Some(1_000),
            ..Default::default()
        }),
        ..Default::default()
    };
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![FightEntityInfo {
                ex_skill_level: Some(2),
                passive_skill: vec![422811],
                ..entity(10, 3144)
            }],
            ..Default::default()
        }),
        defender: Some(FightTeam {
            entitys: vec![entity(-1, 1001)],
            ..Default::default()
        }),
        ..Default::default()
    };
    let pool = TargetPool::from_fight(&fight);
    let mut managers = BattleManagers::seeded(&fight);
    let catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);
    managers
        .conduit
        .execute(
            crate::engine::manager::conduit::ConduitCommand::SelectGroup {
                source_uid: 10,
                group: 2,
            },
        )
        .unwrap();

    let result = crate::engine::runtime::drain::run_conduit_action(
        &mut managers,
        &pool,
        &catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        10,
        2,
        1,
        2,
        31441121,
        None,
        None,
    )
    .unwrap();

    let steps = crate::engine::packet::timeline::project(&result.frames).unwrap();
    let skill = steps
        .iter()
        .find_map(|step| find_step(step, 31441121))
        .unwrap();
    let reaction = skill
        .act_effect
        .iter()
        .position(|effect| effect.fight_step.as_ref().and_then(|step| step.act_id) == Some(422811))
        .unwrap();
    let completion = skill
        .act_effect
        .iter()
        .position(|effect| {
            effect.effect_type
                == Some(sonettobuf::effect_type_enum::EffectType::Counterchange as i32)
                && effect.effect_num == Some(63)
        })
        .unwrap();

    assert!(reaction < completion);
}
