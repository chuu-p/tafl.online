use toasty::Db;

#[derive(Debug, toasty::Model)]
pub struct User {
    #[key]
    #[auto]
    pub id: u64,
    pub google_id: String,
    pub username: String,
    pub email: String,
    pub avatar_url: String,
    pub rating: u32,
}

#[derive(Debug, toasty::Model)]
pub struct Game {
    #[key]
    #[auto]
    pub id: u64,
    pub board: String,
    pub attacker_player: String,
    pub defender_player: String,
    pub attacker_elo: u32,
    pub defender_elo: u32,
    pub current_side: String,
    pub moves: String,
    pub result: String,
    pub variant: String,
    pub base_time_seconds: u32,
    pub increment_seconds: u32,
    pub is_private: bool,
    pub status: String,
}

#[derive(Debug, toasty::Model)]
pub struct MatchmakingEntry {
    #[key]
    #[auto]
    pub id: u64,
    pub user_id: u64,
    pub variant: String,
    pub max_rating_diff: u32,
}

pub async fn create_game(db: &mut Db) -> Result<Game, Box<dyn std::error::Error + Send + Sync>> {
    let initial_board = tafl_game::Game::new()
        .ten()
        .split(' ')
        .next()
        .unwrap_or("")
        .to_string();
    let game = toasty::create!(Game {
        board: initial_board,
        attacker_player: "",
        defender_player: "",
        attacker_elo: 0,
        defender_elo: 0,
        current_side: "M",
        moves: "[]",
        result: "?",
        variant: "tablut",
        base_time_seconds: 600,
        increment_seconds: 5,
        is_private: false,
        status: "active",
    })
    .exec(db)
    .await?;
    println!("Created: {:?}", game.id);
    Ok(game)
}

pub async fn select_game(
    db: &mut Db,
    id: u64,
) -> Result<Game, Box<dyn std::error::Error + Send + Sync>> {
    let found = Game::get_by_id(db, &id).await?;
    println!("Found: {:?}", found.id);
    Ok(found)
}

pub async fn update_game(
    db: &mut Db,
    game_id: u64,
    new_board: &str,
    new_moves: &str,
    new_current_side: &str,
    new_result: &str,
) -> Result<Game, Box<dyn std::error::Error + Send + Sync>> {
    let mut game = Game::get_by_id(db, &game_id).await?;
    toasty::update!(game {
        board: new_board,
        moves: new_moves,
        current_side: new_current_side,
        result: new_result,
    })
    .exec(db)
    .await?;
    Ok(game)
}

pub async fn get_db(url: &str) -> Result<Db, toasty::Error> {
    toasty::Db::builder()
        .models(toasty::models!(crate::*))
        .connect(url)
        .await
}

// ponytail: TEN board encoding — same format as tafl_game::Game::ten() without trailing metadata
fn board_to_ten(g: &tafl_game::Game) -> String {
    let full = g.ten();
    full.split(' ').next().unwrap_or(&full).to_string()
}

fn side_to_char(s: tafl_game::Side) -> &'static str {
    match s {
        tafl_game::Side::Attacker => "M",
        tafl_game::Side::Defender => "S",
    }
}

fn result_to_char(r: tafl_game::Result) -> &'static str {
    match r {
        tafl_game::Result::Pending => "?",
        tafl_game::Result::AttackerWin => "a",
        tafl_game::Result::DefenderWin => "d",
        tafl_game::Result::Draw => "=",
    }
}

impl From<tafl_game::Game> for Game {
    fn from(g: tafl_game::Game) -> Self {
        let board = board_to_ten(&g);
        let side = side_to_char(g.current_side).to_string();
        let moves = serde_json::to_string(&g.moves).unwrap_or_default();
        let r = result_to_char(g.result);
        Game {
            id: 0,
            board,
            attacker_player: g.attacker_player,
            defender_player: g.defender_player,
            attacker_elo: g.attacker_elo,
            defender_elo: g.defender_elo,
            current_side: side,
            moves,
            result: r.to_string(),
            variant: "tablut".to_string(),
            base_time_seconds: 600,
            increment_seconds: 5,
            is_private: false,
            status: match r {
                "?" => "active",
                _ => "finished",
            }
            .to_string(),
        }
    }
}

impl From<Game> for tafl_game::Game {
    fn from(db_g: Game) -> Self {
        let full_ten = format!("{} {} - 0 1", db_g.board, db_g.current_side);
        let mut g = tafl_game::Game::from_ten(&full_ten);
        g.attacker_player = db_g.attacker_player;
        g.defender_player = db_g.defender_player;
        g.attacker_elo = db_g.attacker_elo;
        g.defender_elo = db_g.defender_elo;
        g.result = match db_g.result.as_str() {
            "a" => tafl_game::Result::AttackerWin,
            "d" => tafl_game::Result::DefenderWin,
            "=" => tafl_game::Result::Draw,
            _ => tafl_game::Result::Pending,
        };
        g.moves = serde_json::from_str(&db_g.moves).unwrap_or_default();
        g
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_game() -> toasty::Result<()> {
        let mut db = toasty::Db::builder()
            .models(toasty::models!(crate::*))
            .connect("sqlite::memory:")
            .await?;
        db.push_schema().await?;

        let game = toasty::create!(Game {
            board: "",
            attacker_player: "",
            defender_player: "",
            attacker_elo: 0,
            defender_elo: 0,
            current_side: "",
            moves: "",
            result: "",
            variant: "tablut",
            base_time_seconds: 600,
            increment_seconds: 5,
            is_private: false,
            status: "active",
        })
        .exec(&mut db)
        .await?;

        let found = Game::get_by_id(&mut db, &game.id).await?;
        assert_eq!(game.id, found.id);
        Ok(())
    }

    #[tokio::test]
    async fn test_game_roundtrip() -> toasty::Result<()> {
        let db = toasty::Db::builder()
            .models(toasty::models!(crate::*))
            .connect("sqlite::memory:")
            .await?;
        db.push_schema().await?;

        let original = tafl_game::Game::new();
        let ten = original.ten();
        let db_game: Game = original.into();
        let back: tafl_game::Game = db_game.into();
        assert_eq!(ten, back.ten());
        Ok(())
    }

    #[tokio::test]
    async fn test_create_user() -> toasty::Result<()> {
        let mut db = toasty::Db::builder()
            .models(toasty::models!(crate::*))
            .connect("sqlite::memory:")
            .await?;
        db.push_schema().await?;

        let user = toasty::create!(User {
            google_id: "test-google-id",
            username: "testuser",
            email: "test@example.com",
            avatar_url: "",
            rating: 1500,
        })
        .exec(&mut db)
        .await?;

        assert_eq!(user.username, "testuser");
        Ok(())
    }
}
