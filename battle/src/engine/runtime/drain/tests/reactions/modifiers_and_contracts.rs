use super::*;

#[test]
fn pre_attack_carrier_settles_before_skill_rate_conditions() {
    crate::test_support::init_config();

    let run_case = |starting_burn, target_hp| {
        let fight = Fight {
            version: Some(7),
            attacker: Some(FightTeam {
                entitys: vec![FightEntityInfo {
                    uid: Some(10),
                    model_id: Some(3127),
                    current_hp: Some(10_000),
                    attr: Some(HeroAttribute {
                        attack: Some(1_000),
                        hp: Some(10_000),
                        ..Default::default()
                    }),
                    skill_group1: vec![31270111],
                    buffs: vec![BuffInfo {
                        uid: Some(40),
                        buff_id: Some(4150002),
                        count: Some(1),
                        layer: Some(3),
                        from_uid: Some(10),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            defender: Some(FightTeam {
                entitys: vec![FightEntityInfo {
                    uid: Some(-1),
                    current_hp: Some(target_hp),
                    attr: Some(HeroAttribute {
                        hp: Some(target_hp),
                        ..Default::default()
                    }),
                    buffs: vec![BuffInfo {
                        uid: Some(41),
                        buff_id: Some(4150001),
                        layer: Some(starting_burn),
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
        let catalog = SkillEffectCatalog::from_fight(config::configs::get(), &fight);
        let mut invocation: SkillInvocation = SkillRequest {
            source_uid: 10,
            skill_id: 31270111,
        }
        .into();
        invocation.target = SkillTarget::Explicit(-1);

        run(
            &mut managers,
            &pool,
            &catalog,
            &mut RoundDeterminism::default(),
            TargetContext::default(),
            [RuleOp::Skill(invocation)],
        )
        .unwrap();
        assert!(!managers.buff.has_buff_id(10, 4150002));
        (
            target_hp - managers.hp.current(-1),
            managers.buff.buff_id_amount(-1, 4150001),
        )
    };

    let below_threshold = run_case(4, 10_000);
    let crosses_threshold = run_case(6, 10_000);
    assert_eq!(below_threshold, (2_000, 10));
    assert_eq!(crosses_threshold, (3_000, 12));
    assert_eq!(run_case(4, 1_000), (1_000, 7));
}

#[test]
fn rhiannon_ultimate_snapshots_ally_attributes_before_each_buff_grant() {
    crate::test_support::init_config();
    let ally = |uid| FightEntityInfo {
        uid: Some(uid),
        team_type: Some(1),
        current_hp: Some(2_000),
        attr: Some(HeroAttribute {
            hp: Some(2_000),
            attack: Some(1_000),
            ..Default::default()
        }),
        ..Default::default()
    };
    let fight = Fight {
        version: Some(7),
        attacker: Some(FightTeam {
            entitys: vec![
                FightEntityInfo {
                    uid: Some(10),
                    model_id: Some(3146),
                    team_type: Some(1),
                    current_hp: Some(12_763),
                    attr: Some(HeroAttribute {
                        hp: Some(12_763),
                        attack: Some(2_140),
                        ..Default::default()
                    }),
                    ex_skill: Some(31460131),
                    ..Default::default()
                },
                ally(11),
                ally(12),
            ],
            ..Default::default()
        }),
        defender: Some(FightTeam {
            entitys: vec![FightEntityInfo {
                uid: Some(-1),
                team_type: Some(2),
                current_hp: Some(10_000),
                attr: Some(HeroAttribute {
                    hp: Some(10_000),
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
    let mut invocation: SkillInvocation = SkillRequest {
        source_uid: 10,
        skill_id: 31460131,
    }
    .into();
    invocation.target = SkillTarget::Explicit(-1);

    let result = run(
        &mut managers,
        &pool,
        crate::engine::skill::effect::catalog::global(),
        &mut RoundDeterminism::default(),
        TargetContext::default(),
        [RuleOp::Skill(invocation)],
    )
    .unwrap();

    let steps = crate::engine::packet::timeline::project(&result.frames).unwrap();
    let effects = &steps
        .iter()
        .find(|step| step.act_id == Some(31460131))
        .unwrap()
        .act_effect;
    for target_uid in [11, 12] {
        let add = effects
            .iter()
            .position(|effect| {
                effect.target_id == Some(target_uid)
                    && effect.buff.as_ref().and_then(|buff| buff.buff_id) == Some(31460131)
            })
            .unwrap();
        let buff_uid = effects[add].buff.as_ref().unwrap().uid.unwrap();
        let markers = effects[..add]
            .iter()
            .filter(|effect| {
                effect.effect_type
                    == Some(sonettobuf::effect_type_enum::EffectType::Buffactinfoupdate as i32)
                    && effect.reserve_id == Some(buff_uid)
            })
            .collect::<Vec<_>>();
        assert_eq!(markers.len(), 2);
        assert!(markers.iter().all(|effect| {
            effect.target_id == Some(0)
                && effect.reserve_id == Some(buff_uid)
                && effect.buff_act_info.as_ref().and_then(|info| info.act_id) == Some(1131)
        }));
        assert_eq!(
            markers
                .iter()
                .filter_map(|effect| effect.buff_act_info.as_ref()?.str_param.as_deref())
                .collect::<Vec<_>>(),
            ["102#171", "101#1021"]
        );
        assert_eq!(
            effects[add]
                .buff
                .as_ref()
                .unwrap()
                .act_info
                .iter()
                .filter_map(|info| info.str_param.as_deref())
                .collect::<Vec<_>>(),
            ["102#171", "101#1021"]
        );
        assert_eq!(managers.origin_attribute(target_uid, AttrId::Attack), 1_171);
        assert_eq!(managers.hp.max(target_uid), 3_021);
    }
}

#[test]
fn actual_contract_bound_death_clears_buffs_and_cards_through_the_drain() {
    crate::test_support::init_config();

    for passive_skill in [31000141, 31000142] {
        let card = |uid, skill_id| CardInfo {
            uid: Some(uid),
            skill_id: Some(skill_id),
            ..Default::default()
        };
        let fight = Fight {
            version: Some(7),
            attacker: Some(FightTeam {
                entitys: vec![
                    FightEntityInfo {
                        uid: Some(10),
                        model_id: Some(3100),
                        position: Some(1),
                        team_type: Some(1),
                        current_hp: Some(100),
                        attr: Some(HeroAttribute {
                            hp: Some(100),
                            attack: Some(100),
                            ..Default::default()
                        }),
                        passive_skill: vec![passive_skill],
                        ..Default::default()
                    },
                    FightEntityInfo {
                        uid: Some(20),
                        model_id: Some(3086),
                        position: Some(2),
                        team_type: Some(1),
                        career: Some(1),
                        current_hp: Some(100),
                        attr: Some(HeroAttribute {
                            hp: Some(100),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    FightEntityInfo {
                        uid: Some(30),
                        model_id: Some(3001),
                        position: Some(3),
                        team_type: Some(1),
                        current_hp: Some(100),
                        attr: Some(HeroAttribute {
                            hp: Some(100),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }),
            defender: Some(FightTeam {
                entitys: vec![FightEntityInfo {
                    uid: Some(-1),
                    position: Some(1),
                    team_type: Some(2),
                    current_hp: Some(100),
                    attr: Some(HeroAttribute {
                        hp: Some(100),
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
        let effect = catalog
            .get(passive_skill)
            .unwrap_or_else(|| panic!("missing actual passive effect {passive_skill}"));
        let dead_key = DefinitionKey::new(8, "Dead");
        let none_key = DefinitionKey::new(52, "None");
        let lanes = catalog
            .compiled_subscription_lanes(passive_skill)
            .unwrap_or_else(|error| panic!("passive {passive_skill} route failed: {error:?}"));
        assert!(
            lanes.iter().any(|(slot_index, subscription)| {
                *slot_index == 3
                    && subscription.event == crate::engine::event::kind::EventKind::EntityDied
                    && subscription.definition == dead_key
            }),
            "passive {passive_skill} did not discover slot 3 Dead subscription: {lanes:?}"
        );
        assert!(
            !lanes
                .iter()
                .any(|(_, subscription)| { subscription.definition == none_key })
        );

        let end_slot = effect
            .slots
            .get(3)
            .unwrap_or_else(|| panic!("passive {passive_skill} has no slot 3"));
        assert_eq!(end_slot.behavior.spec.key.opcode, 60093);
        assert_eq!(end_slot.behavior.spec.key.type_name, "ContractEndClearBuff");
        let owner_buff_ids = end_slot
            .behavior
            .arg_list(0)
            .expect("actual contract cleanup has owner buff arguments");
        let bound_buff_ids = end_slot
            .behavior
            .arg_list(1)
            .expect("actual contract cleanup has bound buff arguments");

        let contract_origin = CommandOrigin {
            domain: RuleDomain::Behavior,
            key: DefinitionKey::new(60092, "NotifyHeroContract"),
        };
        managers
            .contract
            .execute(crate::engine::manager::contract::ContractCommand::Offer {
                origin: contract_origin,
                owner_uid: 10,
                candidates: vec![20],
            })
            .unwrap();
        managers
            .contract
            .execute(
                crate::engine::manager::contract::ContractCommand::SelectOwner {
                    owner_uid: 10,
                    bound_uid: 20,
                },
            )
            .unwrap();
        managers
            .contract
            .execute(
                crate::engine::manager::contract::ContractCommand::SelectBound {
                    owner_uid: 10,
                    bound_uid: 20,
                },
            )
            .unwrap();
        assert_eq!(managers.contract.bound_uid(10), Some(20));

        let seed_buff = |managers: &mut BattleManagers, target_uid, buff_id| {
            managers
                .execute_buff(BuffCommand::Grant(BuffGrant {
                    origin: CommandOrigin {
                        domain: RuleDomain::Behavior,
                        key: DefinitionKey::new(60093, "ContractEndClearBuff"),
                    },
                    source_uid: target_uid,
                    target_uid,
                    buff_id,
                    amount: Some(1),
                    occurrences: 1,
                    child_uid_reservations: 0,
                }))
                .unwrap();
        };
        for buff_id in owner_buff_ids.iter().copied() {
            seed_buff(&mut managers, 10, buff_id);
        }
        for buff_id in bound_buff_ids.iter().copied() {
            seed_buff(&mut managers, 20, buff_id);
        }
        seed_buff(&mut managers, 10, 31000141);
        seed_buff(&mut managers, 20, 31000142);

        managers
            .execute_card(CardCommand::Setup(CardSetup {
                hand: vec![card(20, 200), card(10, 300), card(30, 400)],
                draw_pile: vec![card(20, 201), card(10, 301), card(30, 401)],
                deck_num: 6,
            }))
            .unwrap();

        for buff_id in owner_buff_ids.iter().copied() {
            assert!(managers.buff.has_buff_id(10, buff_id));
        }
        for buff_id in bound_buff_ids.iter().copied() {
            assert!(managers.buff.has_buff_id(20, buff_id));
        }
        assert!(managers.buff.has_buff_id(10, 31000141));
        assert!(managers.buff.has_buff_id(20, 31000142));
        assert!(managers.card.hand().iter().any(|card| card.uid == Some(20)));
        assert!(
            managers
                .card
                .draw_pile()
                .iter()
                .any(|card| card.uid == Some(20))
        );

        let death = BattleEvent::EntityDied(crate::engine::event::payload::EntityDiedEvent {
            source_uid: 10,
            target_uid: 20,
        });
        let dispatched = crate::engine::event::dispatcher::dispatch_event(
            &pool.runtime_view(&managers),
            &managers,
            &catalog,
            &mut RoundDeterminism::default(),
            &death,
        )
        .unwrap();
        assert!(
            dispatched
                .passes()
                .flat_map(|pass| &pass.skills)
                .any(|(subscriber, _)| {
                    subscriber.owner_uid == 10
                        && subscriber.skill_id == passive_skill
                        && subscriber.slot_index == Some(3)
                        && subscriber.key.definition == dead_key
                })
        );
        assert!(
            !dispatched
                .passes()
                .flat_map(|pass| &pass.skills)
                .any(|(subscriber, _)| subscriber.key.definition == none_key)
        );

        let result = run_command_group(
            &mut managers,
            &pool,
            &catalog,
            &mut RoundDeterminism::default(),
            TargetContext {
                current_round: 1,
                ..Default::default()
            },
            [RuleOp::Command(BattleCommand::Hp(
                crate::engine::manager::hp::HpCommand::Damage(
                    crate::engine::manager::hp::HpDamage {
                        origin: CommandOrigin {
                            domain: RuleDomain::Behavior,
                            key: DefinitionKey::new(1, "TestDamage"),
                        },
                        source_uid: 10,
                        target_uid: 20,
                        amount: 100,
                        config_effect: 1,
                        effect_kind: crate::engine::manager::hp::DamageEffectKind::Normal,
                        assassinate: false,
                        ignore_riposte: false,
                        hurt: crate::engine::manager::hp::HurtInfoData {
                            from_uid: 10,
                            is_crit: false,
                            career_restraint: false,
                            reduce_hp: 0,
                            effect_id: 1,
                            skill_id: 1,
                            damage_from: crate::engine::manager::hp::HurtDamageFromType::Skill,
                            buff_act_id: 0,
                            buff_uid: 0,
                            hurt_effect_type: 0,
                            display_amount: None,
                        },
                    },
                ),
            ))],
        )
        .unwrap();

        assert_eq!(managers.hp.current(20), 0);
        assert!(result.events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::EntityDied(death)
                    if death.source_uid == 10 && death.target_uid == 20
            )
        }));
        assert!(result.outcomes.iter().any(|outcome| {
            matches!(
                outcome,
                RuleOutcome::Contract(crate::engine::manager::contract::ContractChange::Cleared {
                    owner_uid: 10,
                    bound_uid: 20,
                })
            )
        }));
        assert_eq!(managers.contract.bound_uid(10), None);

        for buff_id in owner_buff_ids.iter().copied() {
            assert!(!managers.buff.has_buff_id(10, buff_id));
        }
        for buff_id in bound_buff_ids.iter().copied() {
            assert!(!managers.buff.has_buff_id(20, buff_id));
        }
        assert!(!managers.buff.has_buff_id(20, 31000151));
        assert!(!managers.buff.has_buff_id(10, 31000141));
        assert!(managers.buff.has_buff_id(20, 31000142));

        let remaining_cards = managers
            .card
            .hand()
            .iter()
            .chain(managers.card.draw_pile())
            .collect::<Vec<_>>();
        assert!(remaining_cards.iter().all(|card| card.uid != Some(20)));
        assert!(
            remaining_cards
                .iter()
                .any(|card| card.uid == Some(10) && card.skill_id == Some(300))
        );
        assert!(
            remaining_cards
                .iter()
                .any(|card| card.uid == Some(30) && card.skill_id == Some(400))
        );
    }
}
