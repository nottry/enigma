use crate::engine::{
    damage::{handler::modified_heal, scale_permille},
    manager::hp::{HpCommand, HpHeal, HpHealKind},
    skill::{
        behavior::{BehaviorOpContext, registry::BehaviorHandler},
        effect::ParsedBehavior,
        rule::output::{BattleCommand, RuleOp},
    },
};

pub struct Handler;

impl BehaviorHandler for Handler {
    const VALIDATES_ARGUMENTS: bool = true;

    fn supports(behavior: &ParsedBehavior) -> bool {
        matches!(behavior.args.as_slice(), [rate, 0] if *rate > 0)
    }

    fn emit_ops(context: BehaviorOpContext<'_>, behavior: &ParsedBehavior) -> Option<Vec<RuleOp>> {
        let [rate, 0] = behavior.args.as_slice() else {
            return None;
        };
        let state = crate::engine::skill::buff_act::lost_hp_to_fake_hp::state(
            context.managers,
            context.target_uid,
        )
        .ok()?;
        let Some(state) = state.filter(|state| state.current > 0) else {
            return Some(Vec::new());
        };
        let amount = modified_heal(
            scale_permille(state.current, *rate),
            context.source_uid,
            context.target_uid,
            context.managers,
        );
        if amount <= 0 {
            return Some(Vec::new());
        }
        Some(vec![
            RuleOp::Command(BattleCommand::Hp(HpCommand::Heal(HpHeal {
                origin: super::command_origin(behavior)?,
                source_uid: context.source_uid,
                target_uid: context.target_uid,
                amount,
                config_effect: behavior.config_effect,
                kind: HpHealKind::Normal,
            }))),
            RuleOp::BuffActInfoMarker(crate::engine::manager::buff::BuffActInfoMarkerResult {
                target_uid: context.target_uid,
                buff_uid: state.buff_uid,
                act_id: state.act_id,
                params: vec![state.current, state.maximum],
                str_param: Some(String::new()),
                team_type: 0,
            }),
        ])
    }
}

#[cfg(test)]
mod tests {
    use sonettobuf::{BuffActInfo, BuffInfo, Fight, FightEntityInfo, FightTeam, HeroAttribute};

    use crate::engine::{
        manager::{BattleManagers, hp::HpCommand},
        runtime::determinism::RoundDeterminism,
        skill::{
            action::SkillModifiers,
            behavior::{BehaviorOpContext, classify::BehaviorSpec},
            effect::ParsedBehavior,
            rule::output::{BattleCommand, RuleOp},
            target::{TargetContext, TargetPool},
        },
    };

    use super::*;

    #[test]
    fn fake_hp_heal_reads_remembrance_without_consuming_it() {
        crate::test_support::init_config();
        let fight = Fight {
            attacker: Some(FightTeam {
                entitys: vec![
                    FightEntityInfo {
                        uid: Some(10),
                        team_type: Some(1),
                        current_hp: Some(1_000),
                        attr: Some(HeroAttribute {
                            hp: Some(1_000),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    FightEntityInfo {
                        uid: Some(20),
                        team_type: Some(1),
                        current_hp: Some(8_117),
                        attr: Some(HeroAttribute {
                            hp: Some(10_903),
                            ..Default::default()
                        }),
                        buffs: vec![BuffInfo {
                            uid: Some(30),
                            buff_id: Some(31450143),
                            from_uid: Some(10),
                            layer: Some(1),
                            act_info: vec![BuffActInfo {
                                act_id: Some(1150),
                                param: vec![2_071, 8_108],
                                str_param: Some(String::new()),
                            }],
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }),
            ..Default::default()
        };
        let managers = BattleManagers::seeded(&fight);
        let pool = TargetPool::from_fight(&fight);
        let mut determinism = RoundDeterminism::default();
        let mut modifiers = SkillModifiers::default();
        let mut target = TargetContext::default();
        let behavior = ParsedBehavior::from_spec(
            BehaviorSpec::new(60309, "FakeHpToHeal"),
            vec![200, 0],
            Vec::new(),
        );

        let ops = super::super::rule_ops(
            BehaviorOpContext {
                source_uid: 10,
                source_team: 1,
                target_uid: 20,
                active_skill_id: 31450151,
                transfer_count: 1,
                event: None,
                managers: &managers,
                pool: &pool,
                determinism: &mut determinism,
                modifiers: &mut modifiers,
                target: &mut target,
            },
            &behavior,
        )
        .expect("FakeHpToHeal should be registered");

        assert!(matches!(
            ops.as_slice(),
            [
                RuleOp::Command(BattleCommand::Hp(HpCommand::Heal(heal))),
                RuleOp::BuffActInfoMarker(marker)
            ] if heal.source_uid == 10
                && heal.target_uid == 20
                && heal.amount == 414
                && heal.config_effect == 60309
                && marker.target_uid == 20
                && marker.buff_uid == 30
                && marker.act_id == 1150
                && marker.params == [2_071, 8_108]
        ));
        assert_eq!(
            crate::engine::skill::buff_act::lost_hp_to_fake_hp::state(&managers, 20)
                .unwrap()
                .unwrap()
                .current,
            2_071
        );

        for (rate, expected) in [(350, 724), (500, 1_035)] {
            let behavior = ParsedBehavior::from_spec(
                BehaviorSpec::new(60309, "FakeHpToHeal"),
                vec![rate, 0],
                Vec::new(),
            );
            let ops = super::super::rule_ops(
                BehaviorOpContext {
                    source_uid: 10,
                    source_team: 1,
                    target_uid: 20,
                    active_skill_id: 31450151,
                    transfer_count: 1,
                    event: None,
                    managers: &managers,
                    pool: &pool,
                    determinism: &mut determinism,
                    modifiers: &mut modifiers,
                    target: &mut target,
                },
                &behavior,
            )
            .expect("configured FakeHpToHeal rate should be supported");
            assert!(matches!(
                ops.first(),
                Some(RuleOp::Command(BattleCommand::Hp(HpCommand::Heal(heal))))
                    if heal.amount == expected
            ));
        }
    }
}
