use sqlx::{Sqlite, Transaction};

use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::model::id::UserId;

pub struct GameServer {
	pub guild_id: UserId,
	pub name: String,
	pub width: u8,
	pub height: u8,
}

#[serenity::async_trait]
pub trait DBGame {
	async fn get_game(
		&mut self,
		guild_id_: i64,
		ctx_msg: Option<(&Context, &Message)>,
	) -> anyhow::Result<GameServer>;
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
				guild_id: UserId(game.guild_id as u64),
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
}
