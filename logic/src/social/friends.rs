use super::SocialManager;
use crate::error::AppError;
use database::db::game::player_infos::PlayerInfoData;
use database::db::game::{friends, player_card, player_infos};
use sonettobuf::{
    AddBlacklistReply, FriendExtInfo, FriendModuleFriendInfo, GetApplyListReply, GetBlacklistReply,
    GetFriendInfoListReply, GetRecommendedFriendsReply, HeroInfo, LoadFriendInfosReply,
    PlayerCardExtInfo, RemoveBlacklistReply, RemoveFriendReply, SearchReply,
};
use sqlx::SqlitePool;
use std::collections::HashSet;

#[cfg(test)]
mod tests;

impl SocialManager {
    pub async fn assist_candidates(
        &self,
        db: &SqlitePool,
        limit: i64,
    ) -> Result<Vec<(i64, bool)>, AppError> {
        let friends = friends::get_friend_ids(db, self.player_id).await?;
        let friend_ids = friends.iter().copied().collect::<HashSet<_>>();
        let mut candidates = friends;
        candidates.extend(friends::get_recommended_ids(db, self.player_id, limit).await?);

        let mut seen = HashSet::new();
        Ok(candidates
            .into_iter()
            .filter(|id| seen.insert(*id))
            .filter_map(|id| {
                i64::try_from(id)
                    .ok()
                    .map(|id| (id, friend_ids.contains(&(id as u64))))
            })
            .collect())
    }

    pub async fn load_friend_infos(
        &self,
        db: &SqlitePool,
    ) -> Result<LoadFriendInfosReply, AppError> {
        Ok(LoadFriendInfosReply {
            friend_ids: friends::get_friend_ids(db, self.player_id).await?,
            black_list_ids: friends::get_blacklist_ids(db, self.player_id).await?,
        })
    }

    pub async fn get_friend_info_list(
        &self,
        db: &SqlitePool,
    ) -> Result<GetFriendInfoListReply, AppError> {
        Ok(GetFriendInfoListReply {
            info: friend_infos(db, friends::get_friend_ids(db, self.player_id).await?).await?,
        })
    }

    pub async fn get_blacklist(&self, db: &SqlitePool) -> Result<GetBlacklistReply, AppError> {
        Ok(GetBlacklistReply {
            info: friend_infos(db, friends::get_blacklist_ids(db, self.player_id).await?).await?,
        })
    }

    pub async fn get_recommended_friends(
        &self,
        db: &SqlitePool,
    ) -> Result<GetRecommendedFriendsReply, AppError> {
        let mut info = Vec::new();
        for id in friends::get_recommended_ids(db, self.player_id, 20).await? {
            info.push(friend_ext_info(db, id).await?);
        }

        Ok(GetRecommendedFriendsReply {
            info,
            message: None,
        })
    }

    pub fn get_apply_list(&self) -> GetApplyListReply {
        GetApplyListReply { info: vec![] }
    }

    pub async fn search(&self, db: &SqlitePool, value: String) -> Result<SearchReply, AppError> {
        let mut info = Vec::new();
        for id in friends::search_user_ids(db, self.player_id, value).await? {
            info.push(friend_ext_info(db, id).await?);
        }

        Ok(SearchReply { info })
    }

    pub async fn remove_friend(
        &self,
        db: &SqlitePool,
        friend_id: u64,
    ) -> Result<RemoveFriendReply, AppError> {
        let friend_id = checked_target_id(self.player_id, friend_id)?;
        friends::remove_friend(db, self.player_id, friend_id).await?;
        friends::remove_friend(db, friend_id, self.player_id).await?;

        Ok(RemoveFriendReply {
            friend_id: Some(friend_id as u64),
        })
    }

    pub async fn add_blacklist(
        &self,
        db: &SqlitePool,
        friend_id: u64,
    ) -> Result<AddBlacklistReply, AppError> {
        let friend_id = checked_target_id(self.player_id, friend_id)?;
        friends::add_to_blacklist(db, self.player_id, friend_id).await?;
        friends::remove_friend(db, self.player_id, friend_id).await?;
        friends::remove_friend(db, friend_id, self.player_id).await?;

        Ok(AddBlacklistReply {
            friend_id: Some(friend_id as u64),
        })
    }

    pub async fn remove_blacklist(
        &self,
        db: &SqlitePool,
        friend_id: u64,
    ) -> Result<RemoveBlacklistReply, AppError> {
        let friend_id = checked_target_id(self.player_id, friend_id)?;
        friends::remove_from_blacklist(db, self.player_id, friend_id).await?;

        Ok(RemoveBlacklistReply {
            friend_id: Some(friend_id as u64),
        })
    }
}

async fn friend_infos(
    db: &SqlitePool,
    user_ids: Vec<u64>,
) -> Result<Vec<FriendModuleFriendInfo>, AppError> {
    let mut infos = Vec::with_capacity(user_ids.len());
    for user_id in user_ids {
        let data = player_infos::get_player_info_data(db, user_id as i64).await?;
        infos.push(friend_info(db, user_id, data.as_ref()).await?);
    }
    Ok(infos)
}

// time (0 = online, else last logout ms) and desc (the viewer's remark) stay unset:
// the server tracks neither.
async fn friend_info(
    db: &SqlitePool,
    user_id: u64,
    data: Option<&PlayerInfoData>,
) -> Result<FriendModuleFriendInfo, AppError> {
    let mut info = FriendModuleFriendInfo {
        user_id: Some(user_id),
        ..Default::default()
    };
    let Some(data) = data else {
        return Ok(info);
    };

    info.name = Some(data.user_info.username.clone());
    info.level = u32::try_from(data.user_info.level).ok();
    info.portrait = u32::try_from(data.player_info.portrait).ok();
    info.bg = Some(data.player_info.bg);
    info.infos = show_hero_infos(db, data.player_id).await?;
    Ok(info)
}

async fn show_hero_infos(db: &SqlitePool, user_id: i64) -> Result<Vec<HeroInfo>, AppError> {
    let rows = sqlx::query_as::<_, (i64, i32, i32, i32, i32, i32)>(
        "SELECT heroes.uid, shown.hero_id, shown.level, shown.rank, shown.ex_skill_level, shown.skin
         FROM player_show_heroes AS shown
         JOIN heroes ON heroes.user_id = shown.player_id AND heroes.hero_id = shown.hero_id
         WHERE shown.player_id = ?
         ORDER BY shown.display_order",
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(uid, hero_id, level, rank, ex_skill_level, skin)| HeroInfo {
                uid,
                user_id,
                hero_id,
                level: Some(level),
                rank: Some(rank),
                ex_skill_level: Some(ex_skill_level),
                skin: Some(skin),
                ..Default::default()
            },
        )
        .collect())
}

async fn friend_ext_info(db: &SqlitePool, user_id: u64) -> Result<FriendExtInfo, AppError> {
    let data = player_infos::get_player_info_data(db, user_id as i64).await?;
    let friend_info = friend_info(db, user_id, data.as_ref()).await?;
    let player_info: Option<sonettobuf::PlayerInfo> = data.map(Into::into);

    let player_card_info = match player_info {
        Some(_) => Some(
            player_card::get_player_card_info(db, user_id as i64)
                .await?
                .into(),
        ),
        None => None,
    };

    Ok(FriendExtInfo {
        friend_info: Some(friend_info),
        player_card_ext_info: Some(PlayerCardExtInfo {
            player_info,
            player_card_info,
        }),
    })
}

fn checked_target_id(player_id: i64, target_id: u64) -> Result<i64, AppError> {
    let target_id = i64::try_from(target_id).map_err(|_| AppError::InvalidRequest)?;
    if target_id <= 0 || target_id == player_id {
        return Err(AppError::InvalidRequest);
    }

    Ok(target_id)
}
