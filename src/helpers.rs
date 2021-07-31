use anyhow::Context as AnyHowContext;
use serenity::client::Context;
use serenity::framework::standard::macros::*;
use serenity::framework::standard::{Args, CommandOptions, Reason};
use serenity::model::channel::Message;
use std::borrow::Cow;
use std::str::FromStr;

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

#[derive(Debug, Clone, Copy)]
pub enum Direction {
	North,
	NorthEast,
	East,
	SouthEast,
	South,
	SouthWest,
	West,
	NorthWest,
}

impl FromStr for Direction {
	type Err = Cow<'static, str>;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		use Direction::*;
		let s = s.trim().to_lowercase();
		if let Some((left, right)) = s.split_once('-') {
			let left = Direction::from_str(left)?;
			let right = Direction::from_str(right)?;
			if !left.is_axial() || !right.is_axial() {
				return Err("can only have a max of two axial direction".into());
			}
			return Ok(match (left, right) {
				(North, East) | (East, North) => NorthEast,
				(South, East) | (East, South) => SouthEast,
				(North, West) | (West, North) => NorthWest,
				(South, West) | (West, South) => SouthWest,
				(l, r) => return Err(format!("can not offset {:?} and {:?} together", l, r).into()),
			});
		}
		Ok(match s.as_str() {
			"n" | "north" | "8" | "up" | "u" => North,
			"e" | "east" | "6" | "right" | "r" => East,
			"s" | "south" | "2" | "down" | "d" => South,
			"w" | "west" | "4" | "left" | "l" => West,
			"ne" | "9" | "ur" => NorthEast,
			"se" | "3" | "dr" => SouthEast,
			"nw" | "7" | "ul" => NorthWest,
			"sw" | "1" | "dl" => SouthWest,
			_ => {
				return Err("invalid string".into());
			}
		})
	}
}

impl Direction {
	pub fn is_axial(self) -> bool {
		use Direction::*;
		match self {
			North | East | South | West => true,
			_ => false,
		}
	}

	pub fn as_offsets(self) -> (i8, i8) {
		match self {
			Direction::North => (0, -1),
			Direction::NorthEast => (1, -1),
			Direction::East => (1, 0),
			Direction::SouthEast => (1, 1),
			Direction::South => (0, 1),
			Direction::SouthWest => (-1, 1),
			Direction::West => (-1, 0),
			Direction::NorthWest => (-1, -1),
		}
	}

	pub fn offset_values(self, x: u8, y: u8, width: u8, height: u8) -> Option<(u8, u8)> {
		let (ox, oy) = self.as_offsets();
		if ox == -1 && x == 0 {
			return None;
		}
		if ox == 1 && x >= (width - 1) {
			return None;
		}
		if oy == -1 && y == 0 {
			return None;
		}
		if oy == 1 && y >= (height - 1) {
			return None;
		}
		Some(((x as i16 + ox as i16) as u8, (y as i16 + oy as i16) as u8))
	}
}
