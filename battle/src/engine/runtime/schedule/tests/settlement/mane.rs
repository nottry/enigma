use super::*;

#[test]
fn final_settlement_advances_final_stage_durations() {
    init_config();
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![FightEntityInfo {
                uid: Some(10),
                current_hp: Some(100),
                buffs: vec![BuffInfo {
                    uid: Some(20),
                    buff_id: Some(109320002),
                    duration: Some(1),
                    from_uid: Some(10),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    };
    let pool = TargetPool::from_fight(&fight);
    let mut managers = BattleManagers::seeded(&fight);

    let result = run_round_end_final_settlement(
        &mut managers,
        &pool,
        &SkillEffectCatalog::default(),
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        Vec::new(),
    )
    .unwrap();

    assert!(managers.buff.snapshot(10, 20).is_none());
    let settled = result
        .events
        .iter()
        .position(|event| matches!(event, BattleEvent::BuffsSettled(_)))
        .unwrap();
    let (removed, duration_expired) = result
        .events
        .iter()
        .enumerate()
        .find_map(|(index, event)| match event {
            BattleEvent::BuffRemoved(change) if change.buff_id == 109320002 => {
                Some((index, change.duration_expired))
            }
            _ => None,
        })
        .unwrap();
    assert!(settled < removed);
    assert!(duration_expired);
}

#[test]
fn obscurity_break_enters_the_witness_round_on_the_next_round() {
    init_config();
    let entity = |uid, buffs, passive_skill| FightEntityInfo {
        uid: Some(uid),
        current_hp: Some(10_000),
        attr: Some(HeroAttribute {
            hp: Some(10_000),
            ..Default::default()
        }),
        buffs,
        passive_skill,
        ..Default::default()
    };
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![entity(10, Vec::new(), Vec::new())],
            ..Default::default()
        }),
        defender: Some(FightTeam {
            entitys: vec![entity(
                -1,
                vec![BuffInfo {
                    uid: Some(20),
                    buff_id: Some(109320001),
                    duration: Some(1),
                    from_uid: Some(-1),
                    ..Default::default()
                }],
                vec![109320102, 109320105],
            )],
            ..Default::default()
        }),
        ..Default::default()
    };
    let pool = TargetPool::from_fight(&fight);
    let catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);
    let mut managers = BattleManagers::seeded(&fight);

    run_round_end_final_settlement(
        &mut managers,
        &pool,
        &catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        Vec::new(),
    )
    .unwrap();

    // "When Obscurity reaches zero, you'll enter a bonus Witness Round during the following
    // round", in which "enemies cannot take active actions".
    assert!(managers.buff.has_active_buff_id(-1, 109320002));
    assert!(managers.buff.has_active_buff_id(-1, 109320103));
}

#[test]
fn defender_round_end_keeps_buff_acts_and_owner_passives_in_one_frame() {
    init_config();
    let burn = |uid| BuffInfo {
        uid: Some(uid),
        buff_id: Some(4150001),
        count: Some(1),
        layer: Some(1),
        from_uid: Some(10),
        duration: Some(2),
        ..Default::default()
    };
    let defender = |uid, passive_skill, buff_uid| FightEntityInfo {
        uid: Some(uid),
        current_hp: Some(120_000),
        attr: Some(HeroAttribute {
            attack: Some(if uid == -1 { 2_000 } else { 1_000 }),
            hp: Some(120_000),
            ..Default::default()
        }),
        passive_skill,
        buffs: vec![burn(buff_uid)],
        ..Default::default()
    };
    let fight = Fight {
        version: Some(7),
        attacker: Some(FightTeam {
            entitys: vec![FightEntityInfo {
                uid: Some(10),
                current_hp: Some(10_000),
                attr: Some(HeroAttribute {
                    attack: Some(1_000),
                    hp: Some(10_000),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        }),
        defender: Some(FightTeam {
            entitys: vec![
                defender(-1, Vec::new(), 31),
                defender(-2, vec![116330101], 32),
                defender(-3, vec![116330101], 33),
            ],
            ..Default::default()
        }),
        ..Default::default()
    };
    let pool = TargetPool::from_fight(&fight);
    let catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);
    let mut managers = BattleManagers::seeded(&fight);

    let result = run_after_ai_round_end(
        &mut managers,
        &pool,
        &catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
    )
    .unwrap();
    let steps = crate::engine::packet::timeline::project(&result.frames).unwrap();
    let round_end = steps
        .iter()
        .find(|step| {
            step.act_effect
                .iter()
                .filter(|effect| {
                    effect
                        .fight_step
                        .as_ref()
                        .is_some_and(|step| step.act_id == Some(4150001))
                })
                .count()
                == 3
        })
        .unwrap();
    assert_eq!(round_end.act_effect.len(), 5);

    let passives = round_end.act_effect[3..]
        .iter()
        .map(|effect| {
            effect.fight_step.as_ref().unwrap().act_effect[0]
                .fight_step
                .as_ref()
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        passives
            .iter()
            .map(|step| (step.from_id, step.to_id, step.act_id))
            .collect::<Vec<_>>(),
        vec![
            (Some(-2), Some(-2), Some(116330101)),
            (Some(-3), Some(-3), Some(116330101)),
        ]
    );
    let buffs = passives
        .iter()
        .map(|step| {
            step.act_effect
                .iter()
                .find_map(|effect| effect.buff.as_ref())
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        buffs
            .iter()
            .map(|buff| (buff.buff_id, buff.layer))
            .collect::<Vec<_>>(),
        vec![(Some(116330002), Some(1)), (Some(116330002), Some(2))]
    );
    assert_eq!(buffs[0].uid, buffs[1].uid);
    assert_eq!(managers.buff.buff_id_amount(-1, 116330002), 2);
    assert_eq!(managers.buff.buff_id_amount(-2, 116330002), 0);
    assert_eq!(managers.buff.buff_id_amount(-3, 116330002), 0);
}
