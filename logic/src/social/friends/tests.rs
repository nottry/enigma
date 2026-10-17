use super::*;
use database::models::game::heros::UserHeroModel;
use sqlx::sqlite::SqlitePoolOptions;

async fn pool_with_friend() -> (SqlitePool, i64) {
    let data_dir = format!("{}/../data/excel2json", env!("CARGO_MANIFEST_DIR"));
    let _ = config::init(&data_dir);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    database::run_migrations(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO users (id, username, level, created_at, updated_at)
             VALUES (31, 'viewer', 1, 0, 0), (32, 'friend', 47, 0, 0)",
    )
    .execute(&pool)
    .await
    .unwrap();
    for id in [31, 32] {
        player_infos::create_player_info(&pool, id, 0)
            .await
            .unwrap();
    }
    player_infos::set_portrait(&pool, 32, 170001).await.unwrap();
    player_infos::set_player_bg(&pool, 32, 210001)
        .await
        .unwrap();
    let uid = UserHeroModel::new(32, pool.clone())
        .create_hero(3003)
        .await
        .unwrap();
    player_infos::set_show_hero(&pool, 32, vec![uid])
        .await
        .unwrap();
    friends::add_friend(&pool, 31, 32).await.unwrap();
    (pool, uid)
}

fn assert_friend(info: &FriendModuleFriendInfo, uid: i64) {
    assert_eq!(info.user_id, Some(32));
    assert_eq!(info.name.as_deref(), Some("friend"));
    assert_eq!(info.level, Some(47));
    assert_eq!(info.portrait, Some(170001));
    assert_eq!(info.bg, Some(210001));
    assert_eq!(info.time, None);
    assert_eq!(info.desc, None);
    assert_eq!(
        info.infos
            .iter()
            .map(|hero| (hero.uid, hero.user_id, hero.hero_id))
            .collect::<Vec<_>>(),
        [(uid, 32, 3003)]
    );
}

#[tokio::test]
async fn friend_list_carries_the_friend_profile_and_show_heroes() {
    let (pool, uid) = pool_with_friend().await;

    let reply = SocialManager::new(31)
        .get_friend_info_list(&pool)
        .await
        .unwrap();

    assert_eq!(reply.info.len(), 1);
    assert_friend(&reply.info[0], uid);
}

#[tokio::test]
async fn search_carries_the_friend_profile_and_show_heroes() {
    let (pool, uid) = pool_with_friend().await;

    let reply = SocialManager::new(31)
        .search(&pool, "friend".into())
        .await
        .unwrap();

    assert_eq!(reply.info.len(), 1);
    assert_friend(reply.info[0].friend_info.as_ref().unwrap(), uid);
}
