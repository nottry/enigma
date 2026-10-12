use crate::engine::{
    entity::attr::AttrId,
    manager::{
        buff::BuffManager,
        hp::{FakeHpAbsorption, HpCommandError, HpManager},
    },
};

use super::registry::BuffActKind;

pub fn supports(args: &[i32]) -> bool {
    matches!(args, [raw_attr, cap_rate]
        if AttrId::from_raw(*raw_attr) == Some(AttrId::Attack) && *cap_rate > 0)
}

pub fn initial_state(source_attack: i32, args: &[i32]) -> Option<Vec<i32>> {
    supports(args).then(|| {
        vec![
            0,
            (i64::from(source_attack.max(0)) * i64::from(args[1]) / 1000)
                .clamp(0, i64::from(i32::MAX)) as i32,
        ]
    })
}

pub(crate) fn zero_reserve(
    buffs: &BuffManager,
    hp: &HpManager,
    target_uid: i64,
) -> Result<Option<FakeHpAbsorption>, HpCommandError> {
    let Some(feature) = buffs.active_features(hp).into_iter().find(|feature| {
        feature.owner_uid == target_uid
            && feature.owner_alive
            && super::is_kind(feature, BuffActKind::LostHpToFakeHp)
    }) else {
        return Ok(None);
    };
    let buff_act_id = feature.act_id().ok_or(HpCommandError::InvalidFakeHpState)?;
    let buff = buffs
        .snapshot(feature.owner_uid, feature.buff_uid)
        .ok_or(HpCommandError::InvalidFakeHpState)?;
    let state = buff
        .act_info
        .iter()
        .find(|state| state.act_id == Some(buff_act_id))
        .ok_or(HpCommandError::InvalidFakeHpState)?;
    let [current, maximum, ..] = state.param.as_slice() else {
        return Err(HpCommandError::InvalidFakeHpState);
    };
    if *current != 0 || *maximum <= 0 {
        return Err(HpCommandError::UnsupportedFakeHpState);
    }
    Ok(Some(FakeHpAbsorption {
        buff_uid: feature.buff_uid,
        consumed: 0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_only_attack_based_positive_caps() {
        assert!(supports(&[AttrId::Attack.id(), 4000]));
        assert!(!supports(&[AttrId::Hp.id(), 4000]));
        assert!(!supports(&[AttrId::Attack.id(), 0]));
        assert!(!supports(&[AttrId::Attack.id()]));
        assert_eq!(
            initial_state(1703, &[AttrId::Attack.id(), 4000]),
            Some(vec![0, 6812])
        );
    }
}
