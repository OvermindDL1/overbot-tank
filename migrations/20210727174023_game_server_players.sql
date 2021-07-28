CREATE TABLE game_server_players (
    guild_id      INTEGER NOT NULL,
    user_id       INTEGER NOT NULL,
    pos_x         INTEGER NOT NULL,
    pos_y         INTEGER NOT NULL,
    health        INTEGER NOT NULL,
    actions       INTEGER NOT NULL,
    range         INTEGER NOT NULL,
    PRIMARY KEY (guild_id, user_id),
    UNIQUE(guild_id, pos_x, pos_y)
);