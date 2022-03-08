use domain_types::LayerServerEvent;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tdn::types::{
    primitives::{PeerId, Result},
    rpc::{json, RpcParam},
};

use crate::storage::{delete_avatar, get_pool, read_avatar, write_avatar};

/// User Model.
pub struct User {
    /// db auto-increment id.
    pub id: i64,
    /// name.
    name: String,
    /// user ID
    pub pid: PeerId,
    /// bio.
    bio: String,
    /// avatar.
    avatar: Vec<u8>,
    /// is actived.
    is_actived: bool,
    /// created time.
    datetime: i64,
}

impl User {
    pub fn new(name: String, pid: PeerId, bio: String, avatar: Vec<u8>) -> Self {
        let start = SystemTime::now();
        let datetime = start
            .duration_since(UNIX_EPOCH)
            .map(|s| s.as_secs())
            .unwrap_or(0) as i64; // safe for all life.

        Self {
            datetime,
            name,
            pid,
            bio,
            avatar,
            is_actived: true,
            id: 0,
        }
    }

    pub fn to_rpc(self) -> RpcParam {
        json!([
            self.id,
            self.name,
            self.pid.to_hex(),
            self.is_actived,
            self.datetime
        ])
    }

    pub fn to_info(self) -> LayerServerEvent {
        LayerServerEvent::Info(self.pid, self.name, self.bio, self.avatar)
    }

    pub async fn list(base: &PathBuf) -> Result<Vec<Self>> {
        let recs = sqlx::query!(
            "SELECT id, name, pid, bio, is_actived, datetime FROM users WHERE is_deleted = false ORDER BY id",
        )
            .fetch_all(get_pool()?).await.map_err(|_| anyhow!("database failure."))?;

        let mut users = vec![];

        for res in recs {
            let avatar = read_avatar(base, &res.id).await?;

            users.push(Self {
                avatar,
                id: res.id,
                name: res.name,
                pid: PeerId::from_hex(res.pid).unwrap_or(PeerId::default()),
                bio: res.bio,
                is_actived: res.is_actived,
                datetime: res.datetime,
            });
        }

        Ok(users)
    }

    pub async fn search(base: &PathBuf, name: &str) -> Result<User> {
        let res = sqlx::query!(
            "SELECT id, name, pid, bio, is_actived, datetime FROM users WHERE is_actived = true AND name = $1",
            name
        ).fetch_one(get_pool()?).await.map_err(|_| anyhow!("database failure."))?;

        let avatar = read_avatar(base, &res.id).await?;

        Ok(Self {
            avatar,
            id: res.id,
            name: res.name,
            pid: PeerId::from_hex(res.pid).unwrap_or(PeerId::default()),
            bio: res.bio,
            is_actived: res.is_actived,
            datetime: res.datetime,
        })
    }

    pub async fn get_by_name(base: &PathBuf, name: &str) -> Result<User> {
        let res = sqlx::query!(
            "SELECT id, name, pid, bio, is_actived, datetime FROM users WHERE is_deleted = false AND name = $1",
            name
        ).fetch_one(get_pool()?).await.map_err(|_| anyhow!("database failure."))?;

        let avatar = read_avatar(base, &res.id).await?;

        Ok(Self {
            avatar,
            id: res.id,
            name: res.name,
            pid: PeerId::from_hex(res.pid).unwrap_or(PeerId::default()),
            bio: res.bio,
            is_actived: res.is_actived,
            datetime: res.datetime,
        })
    }

    pub async fn _get(base: &PathBuf, id: &i64) -> Result<User> {
        let res = sqlx::query!(
            "SELECT id, name, pid, bio, is_actived, datetime FROM users WHERE is_deleted = false and id = $1",
            id
        ).fetch_one(get_pool()?).await.map_err(|_| anyhow!("database failure."))?;

        let avatar = read_avatar(base, id).await?;

        Ok(Self {
            avatar,
            id: res.id,
            name: res.name,
            pid: PeerId::from_hex(res.pid).unwrap_or(PeerId::default()),
            bio: res.bio,
            is_actived: res.is_actived,
            datetime: res.datetime,
        })
    }

    pub async fn insert(&mut self, base: &PathBuf) -> Result<()> {
        // check if unique group id.
        let unique_check = sqlx::query!("SELECT id from users WHERE name = $1", self.name)
            .fetch_optional(get_pool()?)
            .await
            .map_err(|_| anyhow!("database failure."))?;
        if unique_check.is_some() {
            return Err(anyhow!("unique username."));
        }

        let rec = sqlx::query!(
            "INSERT INTO users (name, pid, bio, is_actived, datetime) VALUES ($1, $2, $3, $4, $5) RETURNING id",
            self.name,
            self.pid.to_hex(),
            self.bio,
            self.is_actived,
            self.datetime
        ).fetch_one(get_pool()?).await.map_err(|_| anyhow!("database failure."))?;

        self.id = rec.id;
        let _ = write_avatar(base, &self.id, &self.avatar).await;

        Ok(())
    }

    pub async fn update(id: &i64, bio: &str, avatar: &Vec<u8>, base: &PathBuf) -> Result<()> {
        let _ = sqlx::query!("UPDATE users SET bio = $1 WHERE id = $2", bio, id)
            .execute(get_pool()?)
            .await
            .map_err(|_| anyhow!("database failure."))?;

        let _ = write_avatar(base, id, avatar).await;

        Ok(())
    }

    pub async fn active(id: &i64, active: bool) -> Result<()> {
        let _ = sqlx::query!("UPDATE users SET is_actived = $1 WHERE id = $2", active, id)
            .execute(get_pool()?)
            .await
            .map_err(|_| anyhow!("database failure."))?;

        Ok(())
    }

    pub async fn delete(id: &i64, base: &PathBuf) -> Result<()> {
        let _ = sqlx::query!(
            "UPDATE users SET is_actived = false, is_deleted = true WHERE id = $1",
            id
        )
        .execute(get_pool()?)
        .await
        .map_err(|_| anyhow!("database failure."))?;

        let _ = delete_avatar(base, id).await;

        Ok(())
    }
}

//pub struct Request {}
