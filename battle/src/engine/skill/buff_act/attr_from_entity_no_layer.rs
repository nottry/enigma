use crate::engine::{
    entity::attr::AttrId,
    manager::buff::{ActiveBuffFeature, BuffManager},
};

pub fn supports(args: &[i32]) -> bool {
    matches!(
        args,
        [source_attr, target_attr, rate]
            if *source_attr == AttrId::Attack.id()
                && AttrId::from_raw(*target_attr).is_some()
                && *rate > 0
    )
}

pub fn snapshot(source_attack: i32, values: &[i32]) -> Option<(AttrId, i32)> {
    let [_, source_attr, target_attr, rate] = values else {
        return None;
    };
    supports(&[*source_attr, *target_attr, *rate]).then_some(())?;
    let target_attr = AttrId::from_raw(*target_attr)?;
    let value = (i64::from(source_attack.max(0)) * i64::from(*rate) / 1000)
        .clamp(0, i64::from(i32::MAX)) as i32;
    Some((target_attr, value))
}

pub fn attribute_delta(feature: &ActiveBuffFeature, attr_id: AttrId, buffs: &BuffManager) -> i32 {
    if !super::is_kind(feature, super::registry::BuffActKind::AttrFromEntityNoLayer) {
        return 0;
    }
    let [act_id, _, configured_attr, _] = feature.values.as_slice() else {
        return 0;
    };
    if AttrId::from_raw(*configured_attr) != Some(attr_id) {
        return 0;
    }
    buffs
        .snapshot(feature.owner_uid, feature.buff_uid)
        .and_then(|buff| {
            buff.act_info
                .into_iter()
                .find(|info| info.act_id == Some(*act_id))
        })
        .and_then(|info| info.str_param)
        .and_then(|raw| {
            let (attr, value) = raw.split_once('#')?;
            (attr.parse::<i32>().ok()? == *configured_attr)
                .then(|| value.parse::<i32>().ok())
                .flatten()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use sonettobuf::{
        Fight, FightEntityInfo, FightTeam, HeroAttribute, effect_type_enum::EffectType,
    };

    use super::*;
    use crate::engine::{
        manager::{
            BattleManagers,
            buff::{BuffCommand, BuffGrant},
        },
        packet::effect::EffectPacket,
        skill::rule::{CommandOrigin, DefinitionKey, RuleDomain},
    };

    #[test]
    fn past_shadows_snapshots_source_attack_once_and_projects_both_attributes() {
        crate::test_support::init_config();
        let fight = Fight {
            attacker: Some(FightTeam {
                entitys: vec![
                    FightEntityInfo {
                        uid: Some(10),
                        current_hp: Some(1_000),
                        attr: Some(HeroAttribute {
                            hp: Some(1_000),
                            attack: Some(1_700),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    FightEntityInfo {
                        uid: Some(11),
                        current_hp: Some(1_000),
                        attr: Some(HeroAttribute {
                            hp: Some(1_000),
                            attack: Some(300),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut managers = BattleManagers::seeded(&fight);
        let command = || {
            BuffCommand::Grant(BuffGrant {
                origin: CommandOrigin {
                    domain: RuleDomain::Behavior,
                    key: DefinitionKey::new(1, "AddBuff"),
                },
                source_uid: 10,
                target_uid: 11,
                buff_id: 31450111,
                amount: None,
                occurrences: 1,
                child_uid_reservations: 0,
            })
        };

        let changes = managers.execute_buff(command()).unwrap();
        let added = changes.change.added.as_ref().unwrap();
        assert_eq!(
            added
                .buff
                .act_info
                .iter()
                .find(|info| info.act_id == Some(1152))
                .and_then(|info| info.str_param.as_deref()),
            Some("205#102")
        );
        assert_eq!(
            EffectPacket::buff_changes(&changes.change)
                .iter()
                .map(|effect| effect.effect_type.unwrap_or_default())
                .collect::<Vec<_>>(),
            vec![
                EffectType::Buffactinfoupdate as i32,
                EffectType::Buffadd as i32,
                EffectType::Attr as i32,
                EffectType::Attr as i32,
            ]
        );
        assert_eq!(
            managers.persistent_attribute_delta(11, AttrId::DmgBonus),
            112
        );

        managers.execute_buff(command()).unwrap();
        assert_eq!(managers.buff.buff_id_amount(11, 31450111), 2);
        assert_eq!(
            managers.persistent_attribute_delta(11, AttrId::DmgBonus),
            122
        );
    }
}
