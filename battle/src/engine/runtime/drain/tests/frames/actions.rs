use super::*;

#[test]
fn owner_event_runs_only_the_selected_side() {
    let entity = |uid| FightEntityInfo {
        uid: Some(uid),
        current_hp: Some(100),
        ex_point: Some(0),
        passive_skill: vec![100],
        ..Default::default()
    };
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![entity(10)],
            ..Default::default()
        }),
        defender: Some(FightTeam {
            entitys: vec![entity(-1)],
            ..Default::default()
        }),
        ..Default::default()
    };
    let pool = TargetPool::from_fight(&fight);
    let mut managers = BattleManagers::seeded(&fight);
    let mut slot = SkillEffectSlot::new(
        ParsedBehavior::from_spec(BehaviorSpec::new(20002, "AddExPoint"), vec![1], Vec::new()),
        TargetRequest::self_only(),
    );
    slot.conditions = vec![ParsedCondition {
        opcode: 208,
        type_name: "None".to_owned(),
        kind: ParsedConditionKind::None(NoneMode::SkillAction),
        raw_args: Vec::new(),
    }];
    slot.compiled_route = ConditionRoute::compile(&slot.conditions);
    let mut catalog = SkillEffectCatalog::default();
    catalog.insert(ParsedSkillEffect {
        skill_id: 100,
        slots: vec![slot],
    });

    let result = run_owner_event(
        &mut managers,
        &pool,
        &catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        BattleEvent::Kind(crate::engine::event::kind::EventKind::SkillAction),
        &[10],
    )
    .unwrap();

    assert_eq!(result.outcomes.len(), 1);
    assert_eq!(managers.ex_point.get(10), 1);
    assert_eq!(managers.ex_point.get(-1), 0);
}

#[test]
fn active_skill_commits_immediate_ops_and_keeps_cast_state() {
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![FightEntityInfo {
                uid: Some(10),
                current_hp: Some(100),
                attr: Some(HeroAttribute {
                    hp: Some(100),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        }),
        defender: Some(FightTeam {
            entitys: vec![FightEntityInfo {
                uid: Some(-1),
                current_hp: Some(1000),
                attr: Some(HeroAttribute {
                    hp: Some(1000),
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
    managers.attribute.override_ex(
        10,
        &HeroExAttribute {
            cri_dmg: Some(1000),
            ..Default::default()
        },
    );
    let mut catalog = SkillEffectCatalog::default();
    catalog.insert(ParsedSkillEffect {
        skill_id: 100,
        slots: vec![
            SkillEffectSlot::new(
                ParsedBehavior::from_spec(
                    BehaviorSpec::new(60189, "AddEnergyToCard"),
                    vec![1, 2, 1],
                    Vec::new(),
                ),
                TargetRequest::self_only(),
            ),
            SkillEffectSlot::new(
                ParsedBehavior::from_spec(
                    BehaviorSpec::new(10004, "AttrFix"),
                    vec![AttrId::CriticalDmg as i32, 500],
                    Vec::new(),
                ),
                TargetRequest::self_only(),
            ),
            SkillEffectSlot::new(
                ParsedBehavior::from_spec(
                    BehaviorSpec::new(30015, "OriginDamageCanCrit"),
                    vec![0, AttrId::CurrentHp as i32, 1000],
                    Vec::new(),
                ),
                TargetRequest::self_only(),
            ),
        ],
    });

    let mut determinism = RoundDeterminism::default();
    determinism.enqueue_hidden_crits(100, 10, [true]);
    let mut invocation: SkillInvocation = SkillRequest {
        source_uid: 10,
        skill_id: 100,
    }
    .into();
    invocation.target = SkillTarget::Explicit(-1);
    let result = run(
        &mut managers,
        &pool,
        &catalog,
        &mut determinism,
        TargetContext::default(),
        [RuleOp::Skill(invocation)],
    )
    .unwrap();

    assert!(result.outcomes.iter().any(|outcome| matches!(
        outcome,
        RuleOutcome::Card(card) if card.kind == CardChangeKind::EnergyChanged
    )));
    assert!(result.outcomes.iter().any(|outcome| matches!(
        outcome,
        RuleOutcome::Hp(execution)
            if execution
                .changes
                .hp
                .is_some_and(|change| change.delta == -150)
    )));
}

#[test]
fn after_hit_settles_the_acting_owners_take_stage_buff() {
    crate::test_support::init_config();
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![FightEntityInfo {
                uid: Some(10),
                current_hp: Some(100),
                buffs: vec![BuffInfo {
                    buff_id: Some(31260211),
                    uid: Some(2),
                    from_uid: Some(10),
                    duration: Some(1),
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
    let mut catalog = SkillEffectCatalog::default();
    catalog.insert(ParsedSkillEffect {
        skill_id: 100,
        slots: Vec::new(),
    });

    let result = run(
        &mut managers,
        &pool,
        &catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        [RuleOp::Skill(
            SkillRequest {
                source_uid: 10,
                skill_id: 100,
            }
            .into(),
        )],
    )
    .unwrap();

    assert!(managers.buff.snapshot(10, 2).is_none());
    assert!(result.outcomes.iter().any(|outcome| matches!(
        outcome,
        RuleOutcome::Buff(changes)
            if changes.change.removed.iter().any(|removed| removed.buff.uid == Some(2))
    )));
}

#[test]
fn settled_player_death_removes_owned_cards_in_the_same_frame() {
    let entity = |uid, team_type| FightEntityInfo {
        uid: Some(uid),
        team_type: Some(team_type),
        current_hp: Some(1),
        attr: Some(HeroAttribute {
            hp: Some(1),
            ..Default::default()
        }),
        ..Default::default()
    };
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![entity(10, 1)],
            ..Default::default()
        }),
        defender: Some(FightTeam {
            entitys: vec![entity(-1, 2)],
            ..Default::default()
        }),
        ..Default::default()
    };
    let pool = TargetPool::from_fight(&fight);
    let mut managers = BattleManagers::seeded(&fight);
    let card = |skill_id| CardInfo {
        uid: Some(10),
        skill_id: Some(skill_id),
        ..Default::default()
    };
    managers
        .execute_card(CardCommand::Setup(CardSetup {
            hand: vec![card(100)],
            draw_pile: vec![card(200)],
            deck_num: 2,
        }))
        .unwrap();
    managers.card.add_temp_card_for(10, 300, None, 0, 1);
    managers
        .execute_card(CardCommand::SetTeamCards(
            crate::engine::manager::card::CardSetTeamCards {
                origin: CommandOrigin {
                    domain: RuleDomain::Lifecycle,
                    key: DefinitionKey::new(0, "TestTeamCards"),
                },
                cards: vec![card(400)],
            },
        ))
        .unwrap();
    managers
        .execute_card(CardCommand::SetAiQueue(
            crate::engine::manager::card::CardSetAiQueue {
                origin: CommandOrigin {
                    domain: RuleDomain::Lifecycle,
                    key: DefinitionKey::new(0, "TestAiQueue"),
                },
                cards: vec![CardInfo {
                    uid: Some(-1),
                    skill_id: Some(500),
                    ..Default::default()
                }],
            },
        ))
        .unwrap();

    let mut catalog = SkillEffectCatalog::default();
    catalog.insert(ParsedSkillEffect {
        skill_id: 100,
        slots: vec![
            SkillEffectSlot::new(
                ParsedBehavior::from_spec(
                    BehaviorSpec::new(30005, "LostLife"),
                    vec![1, AttrId::CurrentHp as i32, 1000],
                    Vec::new(),
                ),
                TargetRequest::self_only(),
            ),
            SkillEffectSlot::new(
                ParsedBehavior::from_spec(
                    BehaviorSpec::new(30014, "OriginDamage"),
                    vec![0, AttrId::CurrentHp as i32, 0],
                    Vec::new(),
                ),
                TargetRequest::self_only(),
            ),
        ],
    });
    let mut invocation: SkillInvocation = SkillRequest {
        source_uid: -1,
        skill_id: 100,
    }
    .into();
    invocation.target = SkillTarget::Explicit(10);
    let result = run(
        &mut managers,
        &pool,
        &catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        [RuleOp::Skill(invocation)],
    )
    .unwrap();

    assert!(managers.card.hand().is_empty());
    assert!(managers.card.draw_pile().is_empty());
    assert!(managers.card.generated().is_empty());
    assert!(managers.card.team_cards().is_empty());
    assert_eq!(managers.card.ai_queue()[0].uid, Some(-1));

    let steps = crate::engine::packet::timeline::project(&result.frames).unwrap();
    let effects = &steps[0].act_effect;
    let death = effects
        .iter()
        .position(|effect| {
            effect.effect_type == Some(sonettobuf::effect_type_enum::EffectType::Dead as i32)
                && effect.target_id == Some(10)
        })
        .unwrap();
    let removal = effects[death + 1].fight_step.as_ref().unwrap();
    assert_eq!(
        effects[death + 1].effect_type,
        Some(sonettobuf::effect_type_enum::EffectType::Fightstep as i32)
    );
    assert_eq!(
        removal
            .act_effect
            .iter()
            .map(|effect| (effect.effect_type, effect.target_id, effect.team_type))
            .collect::<Vec<_>>(),
        vec![(
            Some(sonettobuf::effect_type_enum::EffectType::Removeentitycards as i32),
            Some(10),
            Some(1),
        )]
    );
}

#[test]
fn non_action_death_settles_after_its_reaction_and_card_cleanup() {
    crate::test_support::init_config();
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![FightEntityInfo {
                uid: Some(10),
                model_id: Some(3002),
                team_type: Some(1),
                current_hp: Some(10_000),
                ..Default::default()
            }],
            ..Default::default()
        }),
        defender: Some(FightTeam {
            entitys: vec![FightEntityInfo {
                uid: Some(-1),
                model_id: Some(100101),
                team_type: Some(2),
                current_hp: Some(1),
                passive_skill: vec![2345],
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    };
    let pool = TargetPool::from_fight(&fight);
    let mut managers = BattleManagers::seeded(&fight);
    managers
        .execute_card(CardCommand::SetAiQueue(
            crate::engine::manager::card::CardSetAiQueue {
                origin: CommandOrigin {
                    domain: RuleDomain::Lifecycle,
                    key: DefinitionKey::new(0, "TestAiQueue"),
                },
                cards: vec![CardInfo {
                    uid: Some(-1),
                    skill_id: Some(500),
                    ..Default::default()
                }],
            },
        ))
        .unwrap();
    let origin = CommandOrigin {
        domain: RuleDomain::Lifecycle,
        key: DefinitionKey::new(0, "TestDeathSettlement"),
    };
    let result = run(
        &mut managers,
        &pool,
        &SkillEffectCatalog::from_fight(config::configs::get(), &fight),
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        [RuleOp::Command(BattleCommand::Hp(
            crate::engine::manager::hp::HpCommand::Kill(crate::engine::manager::hp::HpKill {
                origin,
                source_uid: 10,
                target_uid: -1,
                config_effect: 1,
            }),
        ))],
    )
    .unwrap();

    fn collect(step: &sonettobuf::FightStep, order: &mut Vec<&'static str>) {
        if step.act_id == Some(2345) {
            order.push("reaction");
        }
        for effect in &step.act_effect {
            match effect.effect_type {
                Some(value)
                    if value
                        == sonettobuf::effect_type_enum::EffectType::Removeentitycards as i32 =>
                {
                    order.push("cleanup");
                }
                Some(value) if value == sonettobuf::effect_type_enum::EffectType::Dead as i32 => {
                    order.push("death");
                }
                _ => {}
            }
            if let Some(child) = effect.fight_step.as_ref() {
                collect(child, order);
            }
        }
    }

    let mut order = Vec::new();
    for step in crate::engine::packet::timeline::project(&result.frames).unwrap() {
        collect(&step, &mut order);
    }
    assert_eq!(order, ["reaction", "cleanup", "death"]);
}

#[test]
fn ally_action_settles_the_acting_owners_take_stage_buff() {
    crate::test_support::init_config();
    let fight = Fight {
        attacker: Some(FightTeam {
            entitys: vec![
                FightEntityInfo {
                    uid: Some(10),
                    current_hp: Some(100),
                    buffs: vec![BuffInfo {
                        buff_id: Some(30940191),
                        uid: Some(2),
                        from_uid: Some(10),
                        duration: Some(1),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                FightEntityInfo {
                    uid: Some(20),
                    current_hp: Some(100),
                    buffs: vec![BuffInfo {
                        buff_id: Some(30940191),
                        uid: Some(3),
                        from_uid: Some(20),
                        duration: Some(1),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            ],
            ..Default::default()
        }),
        ..Default::default()
    };
    let pool = TargetPool::from_fight(&fight);
    let mut managers = BattleManagers::seeded(&fight);
    let mut catalog = SkillEffectCatalog::default();
    catalog.insert(ParsedSkillEffect {
        skill_id: 100,
        slots: Vec::new(),
    });

    let result = run(
        &mut managers,
        &pool,
        &catalog,
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        [RuleOp::Skill(SkillInvocation {
            mode: crate::engine::skill::action::SkillExecutionMode::Active,
            ..SkillRequest {
                source_uid: 10,
                skill_id: 100,
            }
            .into()
        })],
    )
    .unwrap();

    assert!(managers.buff.snapshot(10, 2).is_none());
    assert!(managers.buff.snapshot(20, 3).is_some());
    assert!(result.outcomes.iter().any(|outcome| matches!(
        outcome,
        RuleOutcome::Buff(changes)
            if changes.origin.key.opcode == 212
                && changes.change.removed.iter().any(|removed| removed.buff.uid == Some(2))
    )));
}

#[test]
fn queued_reactions_preserve_dispatch_pass_order() {
    use crate::engine::{
        event::{kind::EventKind, subscription::SubscriptionKey},
        skill::subscriber::SkillSubscriber,
    };

    let reaction = |owner_uid, skill_id, key| {
        (
            SkillSubscriber {
                owner_uid,
                skill_id,
                slot_index: Some(0),
                key: SubscriptionKey::new(EventKind::SkillEffectStarted, key),
            },
            RuleOp::Skill(
                SkillRequest {
                    source_uid: owner_uid,
                    skill_id,
                }
                .into(),
            ),
        )
    };
    let started = BattleEvent::SkillEffectStarted(crate::engine::skill::action::SkillActionEvent {
        source_uid: 10,
        skill_id: 31050151,
        target_uid: -1,
        target_uids: vec![-1],
        attacked_target_uids: Vec::new(),
        phase: crate::engine::skill::action::SkillPhase::Immediate,
        skill_slot: 0,
        is_attack: true,
        rank: 1,
        skill_type: 1,
        effect_tag: 1,
        assassinate: false,
        ignore_riposte: false,
        damage_amount: 0,
        kill_count: 0,
        crit_count: 0,
        guard_break_count: 0,
        additional_moxie: 0,
        extra_skill_kind: 1,
        mode: crate::engine::skill::action::SkillExecutionMode::Active,
        teammate_injury_count: 0,
        teammate_injury_count_not_reset: 0,
        team_injury_count_round: 0,
        card_enchants: Vec::new(),
        buff_additions: Vec::new(),
    });
    let queued = queued_reactions(
        &TargetPool::default(),
        dispatcher::DispatchBatch {
            skills: vec![reaction(
                -1,
                109380003,
                DefinitionKey::new(502202, "ActiveUseSkill"),
            )],
            following: vec![dispatcher::DispatchBatch {
                skills: vec![reaction(
                    10,
                    433911,
                    DefinitionKey::new(403203, "SkillExtraType"),
                )],
                ..Default::default()
            }],
            ..Default::default()
        },
        &started,
        Some(&[0]),
        None,
        None,
        None,
        None,
    )
    .unwrap();

    assert_eq!(
        queued
            .iter()
            .map(|reaction| reaction.subscriber_owner_uid)
            .collect::<Vec<_>>(),
        vec![Some(-1), Some(10)]
    );
}
