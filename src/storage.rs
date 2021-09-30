use config::Config as CFG;
use dotenv::dotenv;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Pool, Postgres};
use std::env;
use std::path::PathBuf;
use tdn::types::primitive::Result;
use tokio::fs;

#[derive(Debug, Deserialize)]
struct Config {
    database: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let mut cfg = CFG::new();

        // database url.
        let database = env::var("DATABASE_URL").expect("DATABASE_URL missing");

        cfg.set("database", database)
            .map_err(|_| anyhow!("set config error."))?;

        // others.
        for (key, value) in env::vars() {
            cfg.set(&key, value)
                .map_err(|_| anyhow!("set config error."))?;
        }
        cfg.try_into().map_err(|_| anyhow!("config init error."))
    }
}

pub static INSTANCE: OnceCell<Pool<Postgres>> = OnceCell::new();

#[inline]
pub fn get_pool<'a>() -> Result<&'a PgPool> {
    INSTANCE.get().ok_or(anyhow!("DB get error!"))
}

pub async fn init(base: &PathBuf) -> Result<()> {
    init_local_files(base).await?;

    dotenv().ok();
    let cfg = Config::from_env()?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&cfg.database)
        .await
        .map_err(|_| anyhow!("DB postgres connect failure! check database & user/password"))?;

    INSTANCE.set(pool).map_err(|_| anyhow!("DB set error!"))
}

const AVATAR_DIR: &'static str = "avatars";

pub(crate) async fn init_local_files(base: &PathBuf) -> Result<()> {
    let mut avatar_path = base.clone();
    avatar_path.push(AVATAR_DIR);
    if !avatar_path.exists() {
        fs::create_dir_all(avatar_path).await?;
    }
    Ok(())
}

pub(crate) async fn read_avatar(base: &PathBuf, id: &i64) -> Result<Vec<u8>> {
    let mut path = base.clone();
    path.push(AVATAR_DIR);
    path.push(format!("{}.png", id));
    if path.exists() {
        Ok(fs::read(path).await?)
    } else {
        Ok(vec![])
    }
}

pub(crate) async fn write_avatar(base: &PathBuf, id: &i64, bytes: &Vec<u8>) -> Result<()> {
    if bytes.len() < 1 {
        return Ok(());
    }
    let mut path = base.clone();
    path.push(AVATAR_DIR);
    path.push(format!("{}.png", id));
    Ok(fs::write(path, bytes).await?)
}

pub(crate) async fn delete_avatar(base: &PathBuf, id: &i64) -> Result<()> {
    let mut path = base.clone();
    path.push(AVATAR_DIR);
    path.push(format!("{}.png", id));
    if path.exists() {
        Ok(fs::remove_file(path).await?)
    } else {
        Ok(())
    }
}
