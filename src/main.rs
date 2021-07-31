mod db;
mod helpers;

use helpers::*;

use crate::db::*;
use anyhow::Context as AnyHowContext;
use image::png::PngEncoder;
use image::{ColorType, RgbImage};
use plotters::prelude::*;
use plotters_backend::BackendColor;
use rand::Rng;
use serenity::client::bridge::gateway::GatewayIntents;
use serenity::framework::standard::macros::*;
use serenity::framework::standard::{
	help_commands, Args, CommandGroup, CommandResult, DispatchError, HelpOptions, Reason,
};
use serenity::framework::StandardFramework;
use serenity::http::{AttachmentType, Http};
use serenity::model::prelude::*;
use serenity::prelude::*;
use sqlx::SqlitePool;
use std::borrow::Cow;
use std::collections::HashSet;
use std::time::Duration;

#[tokio::main]
async fn main() {
	let db = SqlitePool::connect(
		&std::env::var("DATABASE_URL").unwrap_or("sqlite:overbot-tank.db".to_string()),
	)
	.await
	.expect("unable to connect to database");

	let token = std::env::var("DISCORD_TOKEN").expect("need DISCORD_TOKEN environment variable");
	let http = Http::new_with_token(&token);

	// We will fetch your bot's owners and id
	let (owners, bot_id) = match http.get_current_application_info().await {
		Ok(info) => {
			let mut owners = HashSet::new();
			if let Some(team) = info.team {
				owners.insert(team.owner_user_id);
			} else {
				owners.insert(info.owner.id);
			}
			match http.get_current_user().await {
				Ok(bot_id) => (owners, bot_id.id),
				Err(why) => panic!("Could not access the bot id: {:?}", why),
			}
		}
		Err(why) => panic!("Could not access application info: {:?}", why),
	};

	let framework = StandardFramework::new()
		.configure(|c| {
			c.prefix("?")
				.with_whitespace(true)
				.on_mention(Some(bot_id))
				.delimiters(vec![" ", ", ", ","])
				.owners(owners)
		})
		//.before(before) // Called before each command
		//.after(after) // Called after each command
		.unrecognised_command(unknown_command)
		//.normal_message(normal_message) // Called whenever a message is not a command
		.on_dispatch_error(dispatch_error)
		.bucket("ShowBoard", |b| {
			b.delay(15)
				.check(|ctx, msg| Box::pin(async move { msg.is_admin(ctx).await.is_err() }))
		})
		.await
		.help(&HELP)
		.group(&TANKGAME_GROUP);

	let mut client = Client::builder(token)
		.event_handler(Handler)
		.framework(framework)
		.type_map_insert::<DB>(db)
		.cache_update_timeout(Duration::from_secs(15))
		.intents(GatewayIntents::all())
		.await
		.expect("unable to initialize the discord client");

	println!("Starting discord client");
	if let Err(reason) = client.start().await {
		eprintln!("error running discord client: {:?}", reason);
	}
}

#[hook]
async fn unknown_command(_ctx: &Context, msg: &Message, unknown_command_name: &str) {
	println!(
		"Could not find command named '{}' by: {:?}",
		unknown_command_name, msg
	);
}

#[hook]
async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) {
	println!("DispatchError of `{:?}` for: {:?}", &error, msg);
	match error {
		DispatchError::CheckFailed(check, reason) => match reason {
			Reason::Unknown => {
				let _ = msg.reply(ctx, format!("Failed {} check", check)).await;
			}
			Reason::User(reason) => {
				let _ = msg
					.reply(ctx, format!("Failed {} check because: {}", check, reason))
					.await;
			}
			Reason::Log(_reason) => {}
			Reason::UserAndLog {
				user: reason,
				log: _reason,
			} => {
				let _ = msg
					.reply(ctx, format!("Failed {} check because: {}", check, reason))
					.await;
			}
			_ => {}
		},
		DispatchError::Ratelimited(info) => {
			// We notify them only once.
			if info.is_first_try {
				let _ = msg
					.reply(
						ctx,
						format!("Try this again in {} seconds.", info.as_secs()),
					)
					.await;
			}
		}
		DispatchError::CommandDisabled(_) => {}
		DispatchError::BlockedUser => {}
		DispatchError::BlockedGuild => {}
		DispatchError::BlockedChannel => {}
		DispatchError::OnlyForDM => {
			let _ = msg
				.reply(
					ctx,
					"This command is only for use inside of direct messages",
				)
				.await;
		}
		DispatchError::OnlyForGuilds => {
			let _ = msg
				.reply(ctx, "This command is only for use inside of a server")
				.await;
		}
		DispatchError::OnlyForOwners => {
			let _ = msg
				.reply(ctx, "This command is only for use by owners")
				.await;
		}
		DispatchError::LackingRole => {
			let _ = msg
				.reply(ctx, "Missing required role to use this command")
				.await;
		}
		DispatchError::LackingPermissions(_permissions) => {
			let _ = msg
				.reply(ctx, "Lacking required permissions to use this command")
				.await;
		}
		DispatchError::NotEnoughArguments { min, given } => {
			let _ = msg
				.reply(
					ctx,
					format!(
						"Not enough arguments, minimum required is {} but only supplied {}",
						min, given
					),
				)
				.await;
		}
		DispatchError::TooManyArguments { max, given } => {
			let _ = msg
				.reply(
					ctx,
					format!(
						"Too many arguments, maximum accepted is {} but supplied {}",
						max, given
					),
				)
				.await;
		}
		_ => {}
	}
}

struct Handler;

#[serenity::async_trait]
impl EventHandler for Handler {}

#[group]
#[prefixes("tank", "t")]
#[summary = "Tank Game"]
#[description = "Tank Game"]
#[commands(ping, init, destroy, join, board, supply, move_)] // attack, give, vote
struct TankGame;

#[help]
#[individual_command_tip = "For more information about a specific command then just pass that command as an argument to `help`"]
#[command_not_found_text = "Could not find: `{}`"]
#[max_levenshtein_distance(3)]
#[indention_prefix = "-"]
#[lacking_permissions = "Strike"]
#[lacking_role = "Strike"]
#[wrong_channel = "Strike"]
async fn help(
	context: &Context,
	msg: &Message,
	args: Args,
	help_options: &'static HelpOptions,
	groups: &[&'static CommandGroup],
	owners: HashSet<UserId>,
) -> CommandResult {
	let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
	Ok(())
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
	println!("Ping: {:?}", msg);
	msg.reply(ctx, "Pong!").await?;
	Ok(())
}

#[command]
#[description("Initialize a new game")]
#[usage("<game-name:Game> <width:16> <height:16>")]
#[example("\"Game Name\" 16 16")]
#[min_args(0)]
#[max_args(3)]
#[required_permissions("ADMINISTRATOR")]
#[only_in(guilds)]
async fn init(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	println!("init: {:?}", msg);
	let guild = if let Some(guild) = msg.guild_id {
		guild
	} else {
		msg.reply(ctx, "Can only init in a server").await?;
		return Ok(());
	};
	let guild_id = guild.0 as i64;
	let name = match args.single_quoted::<String>() {
		Ok(name) => name,
		Err(_) => "Game".to_string(),
	};
	let width = args.single::<u8>().unwrap_or(16) as i16;
	let height = args.single::<u8>().unwrap_or(16) as i16;
	if width < 8 || height < 8 {
		msg.reply(ctx, "Error: Minimum width*height is 8x8").await?;
	}
	let mut db = DB::begin(ctx).await?;
	let results = sqlx::query!(
		"INSERT INTO game_servers (guild_id, name, width, height) VALUES (?, ?, ?, ?)",
		guild_id,
		name,
		width,
		height
	)
	.execute(&mut db)
	.await;
	dbg!(&results);
	if results.is_err() {
		msg.reply(
			ctx,
			"A Game already exists, destroy it first before creating another",
		)
		.await?;
		return Ok(());
	}
	msg.reply(
		ctx,
		format!("Created new game `{}` of size {}x{}", name, width, height),
	)
	.await?;
	db.commit().await?;
	Ok(())
}

#[command]
#[description("Destroy the existing game")]
#[min_args(0)]
#[max_args(0)]
#[required_permissions("ADMINISTRATOR")]
#[only_in(guilds)]
async fn destroy(ctx: &Context, msg: &Message) -> CommandResult {
	println!("Destroy game: {:?}", msg);
	let guild = if let Some(guild) = msg.guild_id {
		guild
	} else {
		msg.reply(ctx, "Can only init in a server").await?;
		return Ok(());
	};
	let guild_id = guild.0 as i64;
	// sqlx bug prevents this from working...
	// let mut db = DB::acquire(ctx).await?;
	// let result = sqlx::query_scalar!(
	// 	"DELETE FROM game_servers WHERE guild_id = ? RETURNING name",
	// 	guild_id
	// )
	// .fetch_one(&mut db)
	// .await;
	// if let Ok(name) = result {
	// 	msg.reply(ctx, format_args!("Destroyed game: {}", name))
	// 		.await;
	// } else {
	// 	msg.reply(ctx, "No game existed to destroy").await;
	// }
	// So doing this slower version instead
	let mut db = DB::begin(ctx).await?;
	let result = sqlx::query_scalar!("SELECT name FROM game_servers WHERE guild_id = ?", guild_id)
		.fetch_one(&mut db)
		.await;
	if result.is_err() {
		msg.reply(ctx, "No game exists to destroy").await?;
		result?;
		return Ok(());
	}
	let name: String = result.unwrap();
	if let Ok(res) = sqlx::query!("DELETE FROM game_servers WHERE guild_id = ?", guild_id)
		.execute(&mut db)
		.await
	{
		if res.rows_affected() == 0 {
			msg.reply(
				ctx,
				format!("Failed to delete game, report to admin: {}", name),
			)
			.await?;
			return Err("rows_affected is 0")?;
		}
		sqlx::query!(
			"DELETE from game_server_players WHERE guild_id = ?",
			guild_id
		)
		.execute(&mut db)
		.await?;
		db.commit().await?;
		msg.reply(ctx, format!("Game destroyed: {}", name)).await?;
	} else {
		msg.reply(ctx, format!("Unable to destroy game: {}", name))
			.await?;
	}
	Ok(())
}

#[command]
#[description("Join the current game board")]
#[min_args(0)]
#[max_args(0)]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
	let mut db = DB::begin(ctx).await?;
	let guild_id = if let Some(guild_id) = msg.guild_id {
		guild_id
	} else {
		msg.reply(ctx, "Can only init in a server").await?;
		return Ok(());
	};
	let guild_id_ = guild_id.0 as i64;
	let user_id = msg.author.id;
	let user_id_ = user_id.0 as i64;
	if sqlx::query_scalar!(
		"SELECT 1 FROM game_server_players WHERE guild_id = ? AND user_id = ?",
		guild_id_,
		user_id_
	)
	.fetch_one(&mut db)
	.await
	.is_ok()
	{
		msg.reply(ctx, "Already joined to this game").await?;
		return Ok(());
	}

	let game = if let Ok(game) = sqlx::query!(
		"SELECT name, width, height FROM game_servers WHERE guild_id = ?",
		guild_id_
	)
	.fetch_one(&mut DB::acquire(ctx).await?)
	.await
	{
		game
	} else {
		msg.reply(ctx, "Game not in progress").await?;
		return Ok(());
	};

	let health = 3;
	let actions = 0;
	let range = 1;

	for _attempt in 0..32 {
		let pos_x = rand::thread_rng().gen_range(0..game.width);
		let pos_y = rand::thread_rng().gen_range(0..game.height);
		let result = sqlx::query!(
			"
			INSERT INTO game_server_players
			(guild_id, user_id, pos_x, pos_y, health, actions, range)
			VALUES (?, ?, ?, ?, ?, ?, ?)
			",
			guild_id_,
			user_id_,
			pos_x,
			pos_y,
			health,
			actions,
			range
		)
		.execute(&mut db)
		.await;
		match result {
			Ok(v) if v.rows_affected() == 1 => {
				db.commit().await?;
				println!(
					"Successfully joined to game `{}`: `{:?}`",
					game.name, msg.author
				);
				msg.reply(ctx, "You joined the game").await?;
				return Ok(());
			}
			result => println!("Failed inserting a user joining: {:?}", result),
		}
	}

	msg.reply(ctx, "Board appears to be too full to join, try again later")
		.await?;
	Ok(())
}

#[command]
#[description("Show the game board")]
#[min_args(0)]
#[max_args(0)]
#[only_in(guilds)]
#[bucket("ShowBoard")]
async fn board(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	println!("Board: {:?}", msg);
	let guild = if let Some(guild) = msg.guild_id {
		guild
	} else {
		msg.reply(ctx, "Can only init in a server").await?;
		return Ok(());
	};
	let guild_id = guild.0 as i64;

	let game = if let Ok(game) = sqlx::query!(
		"SELECT name, width, height FROM game_servers WHERE guild_id = ?",
		guild_id
	)
	.fetch_one(&mut DB::acquire(ctx).await?)
	.await
	{
		game
	} else {
		msg.reply(ctx, "Game not in progress").await?;
		return Ok(());
	};

	let players = sqlx::query!("SELECT user_id, pos_x, pos_y, health, actions, range FROM game_server_players WHERE guild_id = ? ORDER BY user_id", guild_id).fetch_all(&mut DB::acquire(ctx).await?).await?;

	let tile_size = 25u32;
	let image_width = game.width as u32 * tile_size + 1;
	let image_height = game.height as u32 * tile_size + 1;
	let mut image = RgbImage::new(image_width, image_height);
	{
		let mut image = BitMapBackend::with_buffer(&mut image, (image_width, image_height));
		let image_width = image_width as i32;
		let image_height = image_height as i32;
		let tile_size = tile_size as i32;
		let bg_style = BackendColor {
			alpha: 1.0,
			rgb: (255, 255, 255),
		};
		let line_style = BackendColor {
			alpha: 1.0,
			rgb: (0, 0, 0),
		};
		let tank_health = [
			BackendColor {
				alpha: 0.5,
				rgb: (196, 196, 196),
			},
			BackendColor {
				alpha: 1.0,
				rgb: (196, 0, 0),
			},
			BackendColor {
				alpha: 1.0,
				rgb: (196, 196, 0),
			},
			BackendColor {
				alpha: 1.0,
				rgb: (0, 196, 0),
			},
		];
		let text_id_size = (tile_size * 2) / 3;
		let text_id_style = &("sans-serif", text_id_size)
			.into_text_style(&image.get_size())
			.color(&BLACK);
		let range_style = [
			BackendColor {
				alpha: 0.25,
				rgb: (196, 196, 196),
			},
			BackendColor {
				alpha: 0.25,
				rgb: (196, 196, 0),
			},
			BackendColor {
				alpha: 0.25,
				rgb: (196, 0, 0),
			},
		];

		// Board itself
		image.draw_rect((1, 1), (image_width - 2, image_height - 2), &bg_style, true)?;
		(0..game.width as i32).for_each(|x| {
			let _ = image.draw_line(
				(x * tile_size, 0),
				(x * tile_size, image_height - 1),
				&line_style,
			);
		});
		(0..game.height as i32).for_each(|y| {
			let _ = image.draw_line(
				(0, y * tile_size),
				(image_width - 1, y * tile_size),
				&line_style,
			);
		});

		// Range indicators
		for range in (1..=3).into_iter().rev() {
			for player in players.iter().filter(|p| p.range == range) {
				let dist = range as i32 * tile_size + (tile_size / 3);
				// Range
				let c = (
					player.pos_x as i32 * tile_size + (tile_size / 2),
					player.pos_y as i32 * tile_size + (tile_size / 2),
				);
				image.draw_rect(
					(c.0 - dist, c.1 - dist),
					(c.0 + dist, c.1 + dist),
					&range_style[range as usize - 1],
					true,
				)?;
			}
		}

		for (i, player) in players.iter().enumerate() {
			dbg!((i, player));
			// Health
			let center = (
				player.pos_x as i32 * tile_size + (tile_size / 2),
				player.pos_y as i32 * tile_size + (tile_size / 2),
			);
			if player.health < 0 || player.health > 3 {
				eprintln!("Invalid player data in game {}: {:?}", game.name, player);
			} else {
				image.draw_circle(
					center,
					tile_size as u32 / 3,
					&tank_health[player.health as usize],
					true,
				)?;
			}
			// Player ID#
			let text_offset_x = if i < 10 { (3 * text_id_size) / 7 } else { 0 };
			let center = (
				player.pos_x as i32 * tile_size + 2 + text_offset_x,
				player.pos_y as i32 * tile_size + 2,
			);
			image.draw_text(&i.to_string(), text_id_style, center)?;
			// Player Actions
			let text_offset_x = if player.actions < 10 {
				(3 * text_id_size) / 7
			} else {
				0
			};
			let center = (
				player.pos_x as i32 * tile_size + 2 + text_offset_x,
				player.pos_y as i32 * tile_size + (tile_size / 2) + 1,
			);
			image.draw_text(&player.actions.to_string(), text_id_style, center)?;
		}
	}

	// Leaving off the 3 or 4 for color as it should compress smaller than that anyway
	let mut data = Vec::with_capacity(image_width as usize * image_height as usize);
	{
		PngEncoder::new(&mut data).encode(
			image.as_raw().as_slice(),
			image_width,
			image_height,
			ColorType::Rgb8,
		)?;
	}

	let now = chrono::Utc::now();
	let guild = ctx
		.cache
		.guild_field(guild, |g| g.members.clone())
		.await
		.context("Guild access missing")?;
	msg.channel_id
		.send_message(ctx, |m| {
			m
				//.content("Current Board State")
				.add_file(AttachmentType::Bytes {
					data: Cow::Owned(data),
					filename: format!("board-{}.png", now.format("%s")),
				})
				.embed(|e| {
					e.title("Players").timestamp(now.to_rfc3339()).fields(
						players.iter().enumerate().map(|(i, p)| {
							let name = guild
								.get(&UserId(p.user_id as u64))
								.map(|m| m.user.name.clone())
								.unwrap_or_else(|| format!("<@{}>", p.user_id));
							(
								format!("{}: {}", i, name),
								format!("{}h {}a {}r", p.health, p.actions, p.range),
								false,
							)
						}),
					)
					//.description("Current State of the game Board")
				})
		})
		.await?;
	Ok(())
}

#[command]
#[description("Supply action points to a player, max of 9 points at once, \"all\" for all players")]
#[usage("<points:1>? <player-or-\"all\">+")]
#[example("@SomeName")]
#[example("@SomeName @AnotherName @MoreName")]
#[example("2 @SomeName @AnotherName")]
#[min_args(1)]
//#[required_permissions("ADMINISTRATOR")]
#[checks(Supply)]
#[only_in(guilds)]
async fn supply(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	let guild_id_ = msg.guild_id.unwrap().0 as i64;
	let mut db = DB::begin(ctx).await?;
	let game = db.get_game(guild_id_, Some((ctx, msg))).await?;

	let actions = args
		.single::<i8>()
		.ok()
		.filter(|a| *a <= 9 && *a >= -9)
		.unwrap_or_else(|| {
			args.rewind();
			1
		});
	// Just grab it from the Mentions
	// args.iter::<String>()
	// 	.map(|u| u.unwrap())
	// 	.flat_map(|u| {
	// 		if u.starts_with("<@!") && u.ends_with(">") {
	// 			if let Ok(user_id) = u[3..u.len() - 1].parse::<UserId>() {
	// 				return Some(user_id);
	// 			}
	// 		}
	// 		None
	// 	})
	// 	.for_each(|u| {
	// 		dbg!(u);
	// 	});
	if msg.mentions.is_empty() && args.current() == Some("all") {
		if let Ok(_success) = sqlx::query!(
			"UPDATE game_server_players SET actions = actions + ? WHERE guild_id = ?",
			actions,
			guild_id_
		)
		.execute(&mut db)
		.await
		{
			msg.reply(
				ctx,
				format!(
					"Supply {} action{} to all is complete",
					actions,
					if actions == 0 { "" } else { "s" }
				),
			)
			.await?;
			db.commit().await?;
		} else {
			msg.reply(ctx, "Failed setting actions, check log").await?;
		}
	} else {
		// TODO: Parse the rest of the args as user names perhaps?
		let mut users_added = Vec::with_capacity(msg.mentions.len());
		for u in msg.mentions.iter() {
			let user_id_ = u.id.0 as i64;
			match sqlx::query!("UPDATE game_server_players SET actions = actions + ? WHERE guild_id = ? AND user_id = ?", actions, guild_id_, user_id_).execute(&mut db).await {
				Ok(r) if r.rows_affected() != 0 => {
					users_added.push(u.name.as_str());
				}
				_error => {
					let _ = msg.reply(ctx, format!("{} is not a current player", u.name)).await;
				}
			}
		}
		msg.reply(
			ctx,
			format!(
				"Supply {} action{} to each complete: {}",
				actions,
				if actions == 0 { "" } else { "s" },
				users_added.join(", "),
			),
		)
		.await?;
		db.commit().await?;
	}
	Ok(())
}

#[command("move")]
#[description("Move a single direction in any of the 8 surrounding squares.  Format can be
 * Like the keyboard number where 2 is down, 8 is up, 3 is lower-right, etc...
 * A direction name as a single character like r, l, u, or d, or dr for down-right, ul for up-left, etc...
 * A directional name like right/left/up/down/up-right/down-left/etc...
 * A cardinal direction as a single character like N for up, E for right, NW for up-left, SE for down-right, etc..
 * A cardinal direction as a full name like north, east, south, west, or north-east, south-west, etc...")]
#[usage("<direction>{1,2}")]
#[example("N")]
#[example("up-right")]
#[example("9")]
#[min_args(1)]
#[max_args(1)]
#[only_in(guilds)]
async fn move_(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	let direction = match args.single::<Direction>() {
		Ok(direction) => direction,
		Err(reason) => {
			msg.reply(ctx, format!("Invalid direction: {:?}", reason))
				.await?;
			return Err(anyhow::anyhow!("unsupported argument"))?;
		}
	};
	let guild_id_ = msg.guild_id.unwrap().0 as i64;
	let user_id_ = msg.author.id.0 as i64;
	let mut db = DB::begin(ctx).await?;
	let game = db.get_game(guild_id_, Some((ctx, msg))).await?;
	let player = db.get_player(guild_id_, user_id_, Some((ctx, msg))).await?;
	if player.actions == 0 {
		msg.reply(ctx, "Out of actions, cannot move").await?;
		return Err(anyhow::anyhow!("unable to move due to out of actions"))?;
	}
	let (pos_x, pos_y) =
		match direction.offset_values(player.pos_x, player.pos_y, game.width, game.height) {
			Some((x, y)) => (x, y),
			None => {
				msg.reply(ctx, "Cannot move past a wall").await?;
				return Err(anyhow::anyhow!("cannot move past a wall"))?;
			}
		};
	sqlx::query!(
		"UPDATE game_server_players SET actions = actions - 1, pos_x = ?, pos_y = ? WHERE guild_id = ? AND user_id = ?",
		pos_x,
		pos_y,
		guild_id_,
		user_id_
	)
	.execute(&mut db)
	.await?;
	db.commit().await?;
	println!(
		"Successfully moved {} in server {} to {}:{}",
		user_id_, guild_id_, pos_x, pos_y
	);
	msg.reply(ctx, "Successfully moved, showing board").await?;
	board(ctx, msg, args).await?;
	Ok(())
}
