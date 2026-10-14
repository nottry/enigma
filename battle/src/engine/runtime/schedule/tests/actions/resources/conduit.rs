use super::*;

mod device_ultimate;

#[test]
fn conduit_attack_does_not_begin_without_a_living_enemy() {
    init_config();
    let entity = |uid, model_id| FightEntityInfo {
        uid: Some(uid),
        model_id: Some(model_id),
        current_hp: Some(100),
        attr: Some(HeroAttribute {
            hp: Some(100),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut twins = entity(10, 3149);
    twins.ex_skill = Some(31490151);
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![twins],
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
    managers.hp.lose(-1, 100, 0);
    managers
        .conduit
        .execute(
            crate::engine::manager::conduit::ConduitCommand::SelectGroup {
                source_uid: 10,
                group: 3,
            },
        )
        .unwrap();
    let mut catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);

    let result = run_conduit_phase(
        managers.catalog(),
        &fight,
        &mut managers,
        &pool,
        &mut catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        &[sonettobuf::FightDeviceOper {
            uid: Some(10),
            index: Some(3),
        }],
    )
    .unwrap();

    assert!(result.frames.is_empty());
    assert_eq!(managers.conduit.uses(10), 0);
}

#[test]
fn regulus_device_ultimate_selects_the_group_for_the_living_enemy_count() {
    init_config();
    let entity = |uid, model_id| FightEntityInfo {
        uid: Some(uid),
        model_id: Some(model_id),
        current_hp: Some(100_000),
        attr: Some(HeroAttribute {
            hp: Some(100_000),
            ..Default::default()
        }),
        ..Default::default()
    };

    for (enemy_count, expected_group) in [(1, 1), (2, 2)] {
        let mut regulus = entity(10, 3025);
        regulus.ex_skill_level = Some(3);
        regulus.destiny_stone = Some(302502);
        regulus.destiny_rank = Some(4);
        let fight = Fight {
            attacker: Some(FightTeam {
                entitys: vec![regulus],
                ..Default::default()
            }),
            defender: Some(FightTeam {
                entitys: (0..enemy_count)
                    .map(|index| entity(-1 - i64::from(index), 1001))
                    .collect(),
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
                    group: 3,
                },
            )
            .unwrap();
        let mut catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);

        run_conduit_phase(
            managers.catalog(),
            &fight,
            &mut managers,
            &pool,
            &mut catalog,
            &mut RoundDeterminism::default(),
            TargetContext::default(),
            &[sonettobuf::FightDeviceOper {
                uid: Some(10),
                index: Some(3),
            }],
        )
        .unwrap();

        assert_eq!(managers.conduit.selected_group(10), Some(expected_group));
    }
}

#[test]
fn conduit_source_target_uses_the_first_living_main_ally_as_its_frame_anchor() {
    init_config();
    let entity = |uid, model_id| FightEntityInfo {
        uid: Some(uid),
        model_id: Some(model_id),
        current_hp: Some(100_000),
        attr: Some(HeroAttribute {
            hp: Some(100_000),
            ..Default::default()
        }),
        ..Default::default()
    };
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![entity(20, 3134), entity(10, 3149)],
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
    let mut catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);

    let result = run_conduit_phase(
        managers.catalog(),
        &fight,
        &mut managers,
        &pool,
        &mut catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        &[sonettobuf::FightDeviceOper {
            uid: Some(10),
            index: Some(1),
        }],
    )
    .unwrap();

    assert!(matches!(
        result.frames.first().map(|frame| &frame.owner),
        Some(crate::engine::runtime::record::FrameOwner::ConduitAction {
            target_uid: Some(20),
            ..
        })
    ));
    assert!(result.frames[0].items.iter().any(|item| matches!(
        item,
        crate::engine::runtime::record::FrameItem::Child(frame)
            if matches!(
                frame.owner,
                crate::engine::runtime::record::FrameOwner::ConduitSkill {
                    source_uid: 10,
                    skill_id: 31490111,
                    device_index: 1,
                    target_uid: Some(20),
                    ..
                }
            )
    )));
    assert_eq!(managers.conduit.power(1, 1), 1);
}

#[test]
fn device_power_card_keeps_ally_action_without_granting_loop_chain() {
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
            entitys: vec![
                entity(10, 3149),
                FightEntityInfo {
                    passive_skill: vec![31430151],
                    skill_group1: vec![31430111],
                    ..entity(30, 3143)
                },
            ],
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
    managers
        .execute_card(CardCommand::Setup(CardSetup {
            hand: vec![
                CardInfo {
                    uid: Some(10),
                    skill_id: Some(31446011),
                    temp_card: Some(false),
                    card_type: Some(0),
                    hero_id: Some(3149),
                    ..Default::default()
                },
                CardInfo {
                    uid: Some(30),
                    skill_id: Some(31430111),
                    hero_id: Some(3143),
                    ..Default::default()
                },
            ],
            draw_pile: Vec::new(),
            deck_num: 2,
        }))
        .unwrap();
    let mut catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);
    managers
        .catalog()
        .extend_skill_roots(&mut catalog, [31446011], std::iter::empty());
    let play = || CardPlay {
        origin: CARD_PLAY_ORIGIN,
        hand_index: 0,
        target_uid: None,
        chosen_skill_id: None,
        choice: None,
        recorded_skill: None,
    };

    let device = run_player_action_queue(
        &mut managers,
        &pool,
        &catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        [play()],
        1,
        crate::engine::manager::emitter::UID,
    )
    .unwrap();

    assert!(device.outcomes.iter().any(|outcome| matches!(
        outcome,
        RuleOutcome::SkillLifecycle(SkillLifecycle::ActionCompleted(action))
            if action.skill_id == 31446011 && action.mode == SkillExecutionMode::DeviceCard
    )));
    assert!(device.events.iter().any(|event| matches!(
        event,
        BattleEvent::AllyAction(action)
            if action.skill_id == 31446011 && action.mode == SkillExecutionMode::DeviceCard
    )));
    assert_eq!(managers.buff.buff_id_amount(30, 31430151), 0);

    let active = run_player_action_queue(
        &mut managers,
        &pool,
        &catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        [play()],
        1,
        crate::engine::manager::emitter::UID,
    )
    .unwrap();

    assert!(active.events.iter().any(|event| matches!(
        event,
        BattleEvent::AllyAction(action)
            if action.skill_id == 31430111 && action.mode == SkillExecutionMode::Active
    )));
    assert_eq!(managers.buff.buff_id_amount(30, 31430151), 1);

    fn find_step(step: &sonettobuf::FightStep, act_id: i32) -> Option<&sonettobuf::FightStep> {
        (step.act_id == Some(act_id)).then_some(step).or_else(|| {
            step.act_effect
                .iter()
                .filter_map(|effect| effect.fight_step.as_ref())
                .find_map(|nested| find_step(nested, act_id))
        })
    }

    let steps = crate::engine::packet::timeline::project(&active.frames).unwrap();
    let reaction = steps
        .iter()
        .find_map(|step| find_step(step, 31430151))
        .unwrap();
    assert_eq!(reaction.to_id, Some(-1));
    assert!(reaction.act_effect.iter().any(|effect| {
        effect.target_id == Some(30)
            && effect
                .buff
                .as_ref()
                .is_some_and(|buff| buff.buff_id == Some(31430151))
    }));
}

#[test]
fn conduit_attacks_complete_loop_chain_and_grant_moxie() {
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
            entitys: vec![
                entity(10, 3149),
                FightEntityInfo {
                    ex_point: Some(4),
                    ex_skill: Some(31430131),
                    passive_skill: vec![31430141, 31430151],
                    ..entity(30, 3143)
                },
            ],
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
    let origin = CommandOrigin {
        domain: RuleDomain::Behavior,
        key: DefinitionKey::new(1, "AddBuff"),
    };
    managers
        .execute_buff(crate::engine::manager::buff::BuffCommand::Grant(
            crate::engine::manager::buff::BuffGrant {
                origin,
                source_uid: 30,
                target_uid: 30,
                buff_id: 31430151,
                amount: Some(3),
                occurrences: 1,
                child_uid_reservations: 0,
            },
        ))
        .unwrap();
    managers
        .conduit
        .execute(
            crate::engine::manager::conduit::ConduitCommand::ChangePower(
                crate::engine::manager::conduit::ConduitPowerChange {
                    origin,
                    source_uid: 10,
                    team: 1,
                    power_id: 1,
                    delta: 3,
                    kind: crate::engine::manager::conduit::ConduitPowerChangeKind::Standard,
                },
            ),
        )
        .unwrap();
    let mut catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);

    let result = run_conduit_phase(
        managers.catalog(),
        &fight,
        &mut managers,
        &pool,
        &mut catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        &[sonettobuf::FightDeviceOper {
            uid: Some(10),
            index: Some(1),
        }],
    )
    .unwrap();

    for skill_id in [31490111, 31490121] {
        assert!(result.events.iter().any(|event| matches!(
            event,
            BattleEvent::AllyAction(action)
                if action.skill_id == skill_id && action.mode == SkillExecutionMode::Device
        )));
    }
    assert_eq!(managers.buff.buff_id_amount(30, 31430151), 1);
    assert_eq!(managers.ex_point.get(30), 5);

    fn find_step(step: &sonettobuf::FightStep, act_id: i32) -> Option<&sonettobuf::FightStep> {
        (step.act_id == Some(act_id)).then_some(step).or_else(|| {
            step.act_effect
                .iter()
                .filter_map(|effect| effect.fight_step.as_ref())
                .find_map(|nested| find_step(nested, act_id))
        })
    }
    fn collect_loop_chain_layers(step: &sonettobuf::FightStep, layers: &mut Vec<i32>) {
        for effect in &step.act_effect {
            if let Some(buff) = effect
                .buff
                .as_ref()
                .filter(|buff| buff.buff_id == Some(31430151))
            {
                layers.push(buff.layer.unwrap_or_default());
            }
            if let Some(nested) = effect.fight_step.as_ref() {
                collect_loop_chain_layers(nested, layers);
            }
        }
    }

    let steps = crate::engine::packet::timeline::project(&result.frames).unwrap();
    let mut layers = Vec::new();
    for step in &steps {
        collect_loop_chain_layers(step, &mut layers);
    }
    assert!(layers.windows(2).any(|layers| layers == [3, 4]));
    assert!(layers.windows(2).any(|layers| layers == [4, 5]));
    let threshold = steps
        .iter()
        .find_map(|step| find_step(step, 31430141))
        .unwrap();
    assert!(threshold.act_effect.iter().any(|effect| {
        effect
            .fight_step
            .as_ref()
            .is_some_and(|step| step.act_id == Some(31430181))
    }));
}

#[test]
fn conduit_attacks_keep_their_lifecycle_without_becoming_active_incantations() {
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
            entitys: vec![entity(20, 3134), entity(10, 3149)],
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
    managers
        .execute_buff(crate::engine::manager::buff::BuffCommand::Grant(
            crate::engine::manager::buff::BuffGrant {
                origin: CommandOrigin {
                    domain: RuleDomain::Behavior,
                    key: DefinitionKey::new(1, "AddBuff"),
                },
                source_uid: 20,
                target_uid: 20,
                buff_id: 31340006,
                amount: Some(1),
                occurrences: 1,
                child_uid_reservations: 0,
            },
        ))
        .unwrap();
    assert!(managers.emanation.select(20, 10));
    let mut catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);

    let result = run_conduit_phase(
        managers.catalog(),
        &fight,
        &mut managers,
        &pool,
        &mut catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        &[sonettobuf::FightDeviceOper {
            uid: Some(10),
            index: Some(1),
        }],
    )
    .unwrap();
    assert!(result.outcomes.iter().any(|outcome| matches!(
        outcome,
        RuleOutcome::SkillLifecycle(SkillLifecycle::ActionCompleted(action))
            if action.mode == SkillExecutionMode::Device && action.card_index == 0
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        BattleEvent::SkillAction(action)
            if action.mode == SkillExecutionMode::Device
                && action.is_attack
    )));
    assert_eq!(managers.buff.buff_id_amount(-1, 31340001), 0);
}

#[test]
fn conduit_repeats_a_paid_skill_until_its_energy_is_spent() {
    init_config();
    let entity = |uid, model_id| FightEntityInfo {
        uid: Some(uid),
        model_id: Some(model_id),
        current_hp: Some(100_000),
        attr: Some(HeroAttribute {
            hp: Some(100_000),
            ..Default::default()
        }),
        ..Default::default()
    };
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![entity(10, 3149)],
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
    let origin = crate::engine::skill::rule::CommandOrigin {
        domain: crate::engine::skill::rule::RuleDomain::Behavior,
        key: crate::engine::skill::rule::DefinitionKey::new(60291, "AddDevicePower"),
    };
    managers
        .conduit
        .execute(
            crate::engine::manager::conduit::ConduitCommand::ChangePower(
                crate::engine::manager::conduit::ConduitPowerChange {
                    origin,
                    source_uid: 10,
                    team: 1,
                    power_id: 1,
                    delta: 2,
                    kind: crate::engine::manager::conduit::ConduitPowerChangeKind::Standard,
                },
            ),
        )
        .unwrap();
    let mut catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);

    let result = run_conduit_phase(
        managers.catalog(),
        &fight,
        &mut managers,
        &pool,
        &mut catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        &[sonettobuf::FightDeviceOper {
            uid: Some(10),
            index: Some(1),
        }],
    )
    .unwrap();

    assert_eq!(managers.conduit.power(1, 1), 0);
    assert_eq!(managers.conduit.consumed(1, 1), 6);
    assert_eq!(managers.conduit.uses(10), 3);
    let running = crate::engine::packet::timeline::project(&result.frames)
        .unwrap()
        .into_iter()
        .filter_map(|step| {
            step.act_effect
                .iter()
                .find(|effect| {
                    effect.effect_type
                        == Some(sonettobuf::effect_type_enum::EffectType::Devicerunning as i32)
                })
                .map(|effect| {
                    (
                        step.from_id,
                        step.card_index,
                        step.support_hero_id,
                        effect.effect_num,
                    )
                })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        running,
        [
            (Some(10), Some(1), Some(1), Some(1)),
            (Some(10), Some(1), Some(2), Some(1)),
            (Some(10), Some(1), Some(1), Some(0)),
            (Some(10), Some(1), Some(2), Some(1)),
            (Some(10), Some(1), Some(1), Some(0)),
            (Some(10), Some(1), Some(2), Some(0)),
        ]
    );
}

#[test]
fn conduit_devices_share_power_in_team_turns() {
    init_config();
    let entity = |uid, model_id| FightEntityInfo {
        uid: Some(uid),
        model_id: Some(model_id),
        current_hp: Some(100_000),
        attr: Some(HeroAttribute {
            hp: Some(100_000),
            attack: Some(100),
            ..Default::default()
        }),
        ex_skill_level: Some(2),
        destiny_stone: Some(302502),
        destiny_rank: Some(4),
        ..Default::default()
    };
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![entity(10, 3025), entity(11, 3025)],
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
    managers
        .conduit
        .execute(
            crate::engine::manager::conduit::ConduitCommand::ChangePower(
                crate::engine::manager::conduit::ConduitPowerChange {
                    origin: CommandOrigin {
                        domain: RuleDomain::Behavior,
                        key: DefinitionKey::new(60291, "AddDevicePower"),
                    },
                    source_uid: 10,
                    team: 1,
                    power_id: 2,
                    delta: 4,
                    kind: crate::engine::manager::conduit::ConduitPowerChangeKind::Standard,
                },
            ),
        )
        .unwrap();
    let mut catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);

    let result = run_conduit_phase(
        managers.catalog(),
        &fight,
        &mut managers,
        &pool,
        &mut catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        &[
            sonettobuf::FightDeviceOper {
                uid: Some(10),
                index: Some(1),
            },
            sonettobuf::FightDeviceOper {
                uid: Some(11),
                index: Some(1),
            },
        ],
    )
    .unwrap();

    let sources = result
        .frames
        .iter()
        .filter_map(|frame| match frame.owner {
            crate::engine::runtime::record::FrameOwner::ConduitAction { source_uid, .. } => {
                Some(source_uid)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(sources, [10, 11, 10, 11]);
    assert_eq!(managers.conduit.uses(10), 2);
    assert_eq!(managers.conduit.uses(11), 2);
}

#[test]
fn a_later_device_can_enable_an_earlier_device_on_the_next_team_pass() {
    init_config();
    let entity = |uid, model_id| FightEntityInfo {
        uid: Some(uid),
        model_id: Some(model_id),
        current_hp: Some(100_000),
        attr: Some(HeroAttribute {
            hp: Some(100_000),
            attack: Some(100),
            ..Default::default()
        }),
        ..Default::default()
    };
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![entity(10, 3149), entity(11, 3149)],
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
    let origin = CommandOrigin {
        domain: RuleDomain::Behavior,
        key: DefinitionKey::new(60294, "StopDeviceSkill"),
    };
    for (source_uid, skill_id) in [(10, 31490111), (11, 31490121)] {
        managers
            .conduit
            .execute(crate::engine::manager::conduit::ConduitCommand::StopSkill {
                origin,
                source_uid,
                team: 1,
                skill_id,
            })
            .unwrap();
    }
    let mut catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);

    let result = run_conduit_phase(
        managers.catalog(),
        &fight,
        &mut managers,
        &pool,
        &mut catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        &[
            sonettobuf::FightDeviceOper {
                uid: Some(10),
                index: Some(1),
            },
            sonettobuf::FightDeviceOper {
                uid: Some(11),
                index: Some(1),
            },
        ],
    )
    .unwrap();

    let sources = result
        .frames
        .iter()
        .filter_map(|frame| match frame.owner {
            crate::engine::runtime::record::FrameOwner::ConduitAction { source_uid, .. } => {
                Some(source_uid)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(sources, [11, 10]);
    assert_eq!(managers.conduit.uses(10), 1);
    assert_eq!(managers.conduit.uses(11), 1);
}

#[test]
fn everecho_interval_consumes_mend_and_restarts_only_its_device() {
    init_config();
    let entity = |uid| FightEntityInfo {
        uid: Some(uid),
        model_id: Some(3145),
        team_type: Some(1),
        current_hp: Some(100_000),
        attr: Some(HeroAttribute {
            hp: Some(100_000),
            attack: Some(100),
            ..Default::default()
        }),
        passive_skill: vec![31450141],
        buffs: vec![BuffInfo {
            uid: Some(uid + 100),
            buff_id: Some(31450142),
            from_uid: Some(uid),
            layer: Some(4),
            ..Default::default()
        }],
        ..Default::default()
    };

    for (skill_id, expected_amount, expected_restarts) in
        [(31450111, 0, vec![10]), (31450121, 4, Vec::new())]
    {
        let fight = Fight {
            attacker: Some(FightTeam {
                entitys: vec![entity(10), entity(11)],
                ..Default::default()
            }),
            defender: Some(FightTeam {
                entitys: vec![FightEntityInfo {
                    uid: Some(-1),
                    model_id: Some(1001),
                    current_hp: Some(100_000),
                    attr: Some(HeroAttribute {
                        hp: Some(100_000),
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        let pool = TargetPool::from_fight(&fight);
        let mut managers = BattleManagers::seeded(&fight);
        let catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);
        let mut invocation = crate::engine::skill::action::SkillInvocation::from(
            crate::engine::skill::action::SkillRequest {
                source_uid: 10,
                skill_id,
            },
        );
        invocation.mode = SkillExecutionMode::Device;
        invocation.target = crate::engine::skill::action::SkillTarget::Explicit(-1);

        let result = crate::engine::runtime::drain::run_action(
            &mut managers,
            &pool,
            &catalog,
            &mut RoundDeterminism::default(),
            TargetContext::default(),
            [],
            invocation,
        )
        .unwrap();

        assert_eq!(managers.buff.buff_id_amount(10, 31450142), expected_amount);
        assert_eq!(managers.buff.buff_id_amount(11, 31450142), 4);
        let restarts = result
            .outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                RuleOutcome::Conduit(
                    crate::engine::manager::conduit::ConduitChange::DeviceRestarted {
                        source_uid,
                        ..
                    },
                ) => Some(*source_uid),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(restarts, expected_restarts);
    }
}

#[test]
fn conduit_does_not_start_another_device_after_battle_ends() {
    init_config();
    let entity = |uid, model_id, hp| FightEntityInfo {
        uid: Some(uid),
        model_id: Some(model_id),
        current_hp: Some(hp),
        attr: Some(HeroAttribute {
            hp: Some(hp),
            attack: Some(10_000),
            ..Default::default()
        }),
        ..Default::default()
    };
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![entity(10, 3149, 100_000), entity(11, 3149, 100_000)],
            ..Default::default()
        }),
        defender: Some(FightTeam {
            entitys: vec![entity(-1, 1001, 1)],
            ..Default::default()
        }),
        ..Default::default()
    };
    let pool = TargetPool::from_fight(&fight);
    let mut managers = BattleManagers::seeded(&fight);
    let origin = crate::engine::skill::rule::CommandOrigin {
        domain: crate::engine::skill::rule::RuleDomain::Behavior,
        key: crate::engine::skill::rule::DefinitionKey::new(60291, "AddDevicePower"),
    };
    managers
        .conduit
        .execute(
            crate::engine::manager::conduit::ConduitCommand::ChangePower(
                crate::engine::manager::conduit::ConduitPowerChange {
                    origin,
                    source_uid: 10,
                    team: 1,
                    power_id: 1,
                    delta: 2,
                    kind: crate::engine::manager::conduit::ConduitPowerChangeKind::Standard,
                },
            ),
        )
        .unwrap();
    let mut catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);

    run_conduit_phase(
        managers.catalog(),
        &fight,
        &mut managers,
        &pool,
        &mut catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        &[
            sonettobuf::FightDeviceOper {
                uid: Some(10),
                index: Some(1),
            },
            sonettobuf::FightDeviceOper {
                uid: Some(11),
                index: Some(1),
            },
        ],
    )
    .unwrap();

    assert_eq!(managers.hp.current(-1), 0);
    assert_eq!(managers.conduit.uses(11), 0);
}
