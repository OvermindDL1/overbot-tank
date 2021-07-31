use sqlx::{Sqlite, SqlitePool, Transaction};

use anyhow::Context as AnyHowContext;
use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::model::id::{GuildId, UserId};
use serenity::prelude::TypeMapKey;
use sqlx::pool::PoolConnection;

pub struct DB(SqlitePool);
impl TypeMapKey for DB {
	type Value = SqlitePool;
}
impl DB {
	pub async fn pool(ctx: &Context) -> anyhow::Result<SqlitePool> {
		let datas = ctx.data.read().await;
		Ok(datas
			.get::<DB>()
			.context("db missing from TypeMap")?
			.clone())
	}

	pub async fn acquire(ctx: &Context) -> anyhow::Result<PoolConnection<Sqlite>> {
		let db = Self::pool(ctx).await?;
		Ok(db.acquire().await?)
	}

	pub async fn begin(ctx: &Context) -> anyhow::Result<Transaction<'_, Sqlite>> {
		let db = Self::pool(ctx).await?;
		Ok(db.begin().await?)
	}
}

#[derive(Debug)]
pub struct GameServer {
	pub guild_id: GuildId,
	pub name: String,
	pub width: u8,
	pub height: u8,
}

#[derive(Debug)]
pub struct GamePlayer {
	pub guild_id: GuildId,
	pub user_id: UserId,
	pub pos_x: u8,
	pub pos_y: u8,
	pub health: u8,
	pub actions: u8,
	pub range: u8,
}

#[serenity::async_trait]
pub trait DBGame {
	async fn get_game(
		&mut self,
		guild_id_: i64,
		ctx_msg: Option<(&Context, &Message)>,
	) -> anyhow::Result<GameServer>;

	async fn get_player(
		&mut self,
		guild_id_: i64,
		user_id_: i64,
		ctx_msg: Option<(&Context, &Message)>,
	) -> anyhow::Result<GamePlayer>;
}

#[serenity::async_trait]
impl DBGame for Transaction<'_, Sqlite> {
	async fn get_game(
		&mut self,
		guild_id_: i64,
		ctx_msg: Option<(&Context, &Message)>,
	) -> anyhow::Result<GameServer> {
		match sqlx::query!("SELECT * FROM game_servers WHERE guild_id = ?", guild_id_)
			.fetch_one(self)
			.await
		{
			Ok(game) => Ok(GameServer {
				guild_id: GuildId(game.guild_id as u64),
				name: game.name,
				width: game.width as u8,
				height: game.height as u8,
			}),
			Err(reason) => {
				if let Some((ctx, msg)) = ctx_msg {
					msg.reply(ctx, "Game is not in progress").await?;
				}
				anyhow::bail!(reason);
			}
		}
	}

	async fn get_player(
		&mut self,
		guild_id_: i64,
		user_id_: i64,
		ctx_msg: Option<(&Context, &Message)>,
	) -> anyhow::Result<GamePlayer> {
		match sqlx::query!(
			"SELECT * FROM game_server_players WHERE guild_id = ? AND user_id = ?",
			guild_id_,
			user_id_
		)
		.fetch_one(self)
		.await
		{
			Ok(player) => Ok(GamePlayer {
				guild_id: GuildId(player.guild_id as u64),
				user_id: UserId(player.user_id as u64),
				pos_x: player.pos_x as u8,
				pos_y: player.pos_y as u8,
				health: player.health as u8,
				actions: player.actions as u8,
				range: player.range as u8,
			}),
			Err(reason) => {
				if let Some((ctx, msg)) = ctx_msg {
					msg.reply(ctx, "Player is not in a game").await?;
				}
				anyhow::bail!(reason);
			}
		}
	}
}
