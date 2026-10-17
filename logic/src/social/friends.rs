use super::SocialManager;
use crate::error::AppError;
use database::db::game::{friends, player_card, player_infos};
use sonettobuf::{
    AddBlacklistReply, FriendExtInfo, FriendModuleFriendInfo, GetApplyListReply, GetBlacklistReply,
    GetFriendInfoListReply, GetRecommendedFriendsReply, LoadFriendInfosReply, PlayerCardExtInfo,
    RemoveBlacklistReply, RemoveFriendReply, SearchReply,
};
use sqlx::SqlitePool;
use std::collections::HashSet;

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
            info: friends::get_friend_ids(db, self.player_id)
                .await?
                .into_iter()
                .map(friend_info)
                .collect(),
        })
    }

    pub async fn get_blacklist(&self, db: &SqlitePool) -> Result<GetBlacklistReply, AppError> {
        Ok(GetBlacklistReply {
            info: friends::get_blacklist_ids(db, self.player_id)
                .await?
                .into_iter()
                .map(friend_info)
                .collect(),
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

fn friend_info(user_id: u64) -> FriendModuleFriendInfo {
    FriendModuleFriendInfo {
        user_id: Some(user_id),
        ..Default::default()
    }
}

async fn friend_ext_info(db: &SqlitePool, user_id: u64) -> Result<FriendExtInfo, AppError> {
    let player_info = player_infos::get_player_info_data(db, user_id as i64)
        .await?
        .map(Into::into);

    let player_card_info = match player_info {
        Some(_) => Some(
            player_card::get_player_card_info(db, user_id as i64)
                .await?
                .into(),
        ),
        None => None,
    };

    Ok(FriendExtInfo {
        friend_info: Some(friend_info(user_id)),
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
