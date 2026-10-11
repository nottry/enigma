use std::{env, fs};

fn main() -> anyhow::Result<()> {
    let path = env::args().nth(1).expect("capture path");
    let mut value: serde_json::Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    battle_preview::expand_compressed_fight_steps(&mut value)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
