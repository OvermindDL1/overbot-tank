use anyhow::Context as AnyHowContext;
use serenity::client::Context;
use serenity::framework::standard::macros::*;
use serenity::framework::standard::{Args, CommandOptions, Reason};
use serenity::model::channel::Message;

#[serenity::async_trait]
pub trait Access {
	async fn is_admin(&self, ctx: &Context) -> anyhow::Result<()>;
}

#[serenity::async_trait]
impl Access for Message {
	async fn is_admin(&self, ctx: &Context) -> anyhow::Result<()> {
		let guild = self
			.guild(ctx)
			.await
			.context("not called within a server")?;
		let permissions = guild
			.member_permissions(ctx, &self.author.id)
			.await
			.context("no permissions in server")?;
		if permissions.administrator() {
			return Ok(());
		}
		anyhow::bail!("not an admin")
	}
}

#[check]
#[name = "GuildAdmin"]
async fn guild_admin_check(
	ctx: &Context,
	msg: &Message,
	_args: &mut Args,
	_opts: &CommandOptions,
) -> Result<(), Reason> {
	if msg.is_admin(ctx).await.is_ok() {
		return Ok(());
	}
	Err(Reason::UserAndLog {
		user: "Not a server admin".to_string(),
		log: format!(
			"User {} attempted a guild admin command but is not a guild admin",
			&msg.author.name
		),
	})
}

#[check]
#[name = "Supply"]
async fn supply_check(
	ctx: &Context,
	msg: &Message,
	_args: &mut Args,
	_opts: &CommandOptions,
) -> Result<(), Reason> {
	if msg.is_admin(ctx).await.is_ok() {
		return Ok(());
	}
	// TODO:  DB lookup for authorized people or groups?
	Err(Reason::UserAndLog {
		user: "Not a server admin".to_string(),
		log: format!(
			"User {} attempted a supply guild admin command but is not a guild admin",
			&msg.author.name
		),
	})
}
