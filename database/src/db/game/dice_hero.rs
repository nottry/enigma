use crate::models::game::dice_hero::DiceHeroChapter;
use anyhow::Result;
use sqlx::SqlitePool;

pub async fn get_chapters(
    pool: &SqlitePool,
    user_id: i64,
    chapters: Vec<i32>,
) -> Result<Vec<DiceHeroChapter>> {
    if chapters.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = std::iter::repeat_n("?", chapters.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT user_id, chapter, current_hero_id, relic_ids, skill_card_ids,
                pass_level_ids, reward_items_json, updated_at
         FROM user_dice_hero_chapters
         WHERE user_id = ? AND chapter IN ({})
         ORDER BY chapter",
        placeholders
    );

    let mut query = sqlx::query_as::<_, DiceHeroChapter>(&sql).bind(user_id);
    for chapter in chapters {
        query = query.bind(chapter);
    }

    Ok(query.fetch_all(pool).await?)
}

pub async fn complete_level(
    pool: &SqlitePool,
    user_id: i64,
    chapter: i32,
    level_id: i32,
) -> Result<()> {
    let row = get_or_create_chapter(pool, user_id, chapter).await?;
    let mut pass_level_ids = serde_json::from_str::<Vec<i32>>(&row.pass_level_ids)?;
    if !pass_level_ids.contains(&level_id) {
        pass_level_ids.push(level_id);
        save_json_field(pool, user_id, chapter, "pass_level_ids", &pass_level_ids).await?;
    }

    Ok(())
}

pub async fn get_chapter(pool: &SqlitePool, user_id: i64, chapter: i32) -> Result<DiceHeroChapter> {
    Ok(sqlx::query_as::<_, DiceHeroChapter>(
        "SELECT user_id, chapter, current_hero_id, relic_ids, skill_card_ids,
                pass_level_ids, reward_items_json, updated_at
         FROM user_dice_hero_chapters
         WHERE user_id = ? AND chapter = ?",
    )
    .bind(user_id)
    .bind(chapter)
    .fetch_one(pool)
    .await?)
}

pub async fn save_reward_items(
    pool: &SqlitePool,
    user_id: i64,
    chapter: i32,
    reward_items_json: String,
) -> Result<()> {
    sqlx::query(
        "UPDATE user_dice_hero_chapters
         SET reward_items_json = ?, updated_at = ?
         WHERE user_id = ? AND chapter = ?",
    )
    .bind(reward_items_json)
    .bind(common::time::ServerTime::now_ms())
    .bind(user_id)
    .bind(chapter)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn save_reward_state(
    pool: &SqlitePool,
    user_id: i64,
    chapter: i32,
    current_hero_id: i32,
    relic_ids: &[i32],
    skill_card_ids: &[i32],
) -> Result<()> {
    sqlx::query(
        "UPDATE user_dice_hero_chapters
         SET current_hero_id = ?,
             relic_ids = ?,
             skill_card_ids = ?,
             reward_items_json = '[]',
             updated_at = ?
         WHERE user_id = ? AND chapter = ?",
    )
    .bind(current_hero_id)
    .bind(serde_json::to_string(relic_ids)?)
    .bind(serde_json::to_string(skill_card_ids)?)
    .bind(common::time::ServerTime::now_ms())
    .bind(user_id)
    .bind(chapter)
    .execute(pool)
    .await?;

    Ok(())
}

async fn get_or_create_chapter(
    pool: &SqlitePool,
    user_id: i64,
    chapter: i32,
) -> Result<DiceHeroChapter> {
    sync_chapters(pool, user_id, vec![chapter]).await?;

    Ok(sqlx::query_as::<_, DiceHeroChapter>(
        "SELECT user_id, chapter, current_hero_id, relic_ids, skill_card_ids,
                pass_level_ids, reward_items_json, updated_at
         FROM user_dice_hero_chapters
         WHERE user_id = ? AND chapter = ?",
    )
    .bind(user_id)
    .bind(chapter)
    .fetch_one(pool)
    .await?)
}

async fn save_json_field(
    pool: &SqlitePool,
    user_id: i64,
    chapter: i32,
    field: &'static str,
    value: &[i32],
) -> Result<()> {
    let json = serde_json::to_string(value)?;
    let sql = format!(
        "UPDATE user_dice_hero_chapters
         SET {field} = ?, updated_at = ?
         WHERE user_id = ? AND chapter = ?"
    );

    sqlx::query(&sql)
        .bind(json)
        .bind(common::time::ServerTime::now_ms())
        .bind(user_id)
        .bind(chapter)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn sync_chapters(pool: &SqlitePool, user_id: i64, chapters: Vec<i32>) -> Result<()> {
    let now = common::time::ServerTime::now_ms();

    for chapter in chapters {
        sqlx::query(
            "INSERT INTO user_dice_hero_chapters (user_id, chapter, updated_at)
             VALUES (?, ?, ?)
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(chapter)
        .bind(now)
        .execute(pool)
        .await?;
    }

    Ok(())
}
