CREATE TABLE users (
    id TEXT PRIMARY KEY,               -- e.g., 'caissa_king99' (username or UUID)
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')), -- Unix timestamp
    rating INTEGER NOT NULL DEFAULT 1500, -- Default Glicko/Elo rating
    games_played INTEGER NOT NULL DEFAULT 0
);
