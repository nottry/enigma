use crate::engine::manager::{
    buff::{ActiveBuffFeature, BuffManager},
    hp::HpManager,
};

use super::{is_kind, registry::BuffActKind};

pub fn supports(args: &[i32]) -> bool {
    matches!(args, [permille] if (1..=1000).contains(permille))
}

pub fn max_loss(buffs: &BuffManager, hp: &HpManager, owner_uid: i64) -> Option<i32> {
    buffs
        .active_features(hp)
        .into_iter()
        .filter(|feature| feature.owner_uid == owner_uid)
        .filter_map(|feature| floor_permille(&feature))
        .max()
        .map(|floor_permille| {
            let floor = i64::from(hp.max(owner_uid)) * i64::from(floor_permille) / 1000;
            (i64::from(hp.current(owner_uid)) - floor).clamp(0, i64::from(i32::MAX)) as i32
        })
}

fn floor_permille(feature: &ActiveBuffFeature) -> Option<i32> {
    if !is_kind(feature, BuffActKind::LockHp) {
        return None;
    }
    let [_, permille] = feature.values.as_slice() else {
        return None;
    };
    Some(*permille)
}
