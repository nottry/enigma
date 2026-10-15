use crate::engine::{
    event::payload::BattleEvent,
    manager::fight_progress::FightProgressValueCommand,
    skill::{
        rule::output::{BattleCommand, RuleOp},
        subscriber::BuffActSubscriber,
    },
};

pub fn supports(args: &[i32]) -> bool {
    matches!(args, [progress_id, damage_permille] if *progress_id > 0 && *damage_permille > 0)
}

pub fn rule_ops(subscriber: &BuffActSubscriber, event: &BattleEvent) -> Option<Vec<RuleOp>> {
    let BattleEvent::Hit(hit) = event else {
        return None;
    };
    let [progress_id, damage_permille] = subscriber.args.as_slice() else {
        return None;
    };
    if !super::subscriber_is_kind(subscriber, super::registry::BuffActKind::HurtToProgress)
        || hit.target_uid != subscriber.owner_uid
    {
        return Some(Vec::new());
    }
    let delta = progress_from_damage(hit.resolved_damage, *damage_permille);
    if delta <= 0 {
        return Some(Vec::new());
    }
    Some(vec![RuleOp::Command(BattleCommand::FightProgressValue(
        FightProgressValueCommand {
            origin: super::command_origin(subscriber)?,
            progress_id: *progress_id,
            delta,
        },
    ))])
}

fn progress_from_damage(damage: i32, permille: i32) -> i32 {
    ((i64::from(damage.max(0)) * i64::from(permille.max(0))) / 1000).min(i64::from(i32::MAX)) as i32
}

#[cfg(test)]
mod tests {
    use crate::engine::{
        event::subscription::SubscriptionKey,
        manager::hp::HurtDamageFromType,
        skill::rule::{CommandOrigin, DefinitionKey, RuleDomain},
    };

    use super::*;

    fn subscriber() -> BuffActSubscriber {
        BuffActSubscriber {
            owner_uid: -1,
            source_uid: -1,
            buff_uid: 100,
            buff_id: 119381118,
            team_type: 2,
            owner_alive: true,
            amount: 1,
            key: SubscriptionKey::new(
                crate::engine::event::kind::EventKind::BeAttacked,
                DefinitionKey::new(1148, "HurtToProgress"),
            ),
            act_type: "HurtToProgress".to_owned(),
            effect_time: 0,
            effect_condition: 0,
            args: vec![8, 9],
            raw: "1148#8#9".to_owned(),
        }
    }

    #[test]
    fn converts_damage_using_configured_permille() {
        let event = BattleEvent::Hit(crate::engine::event::payload::HitEvent {
            origin: CommandOrigin {
                domain: RuleDomain::Skill,
                key: DefinitionKey::new(1, "SkillDamage"),
            },
            source_uid: 10,
            target_uid: -1,
            skill_id: 1,
            amount: 1_000,
            resolved_damage: 3_874,
            shield_absorbed: 0,
            career_restraint: false,
            damage_from: HurtDamageFromType::Skill,
            share_count: 0,
            assassinate: false,
            ignore_riposte: false,
        });
        assert!(matches!(
            rule_ops(&subscriber(), &event).as_deref(),
            Some([RuleOp::Command(BattleCommand::FightProgressValue(
                FightProgressValueCommand {
                    progress_id: 8,
                    delta: 34,
                    ..
                }
            ))])
        ));
    }
}
