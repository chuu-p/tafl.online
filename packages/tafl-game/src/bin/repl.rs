use repl_rs::{Command, Convert, Parameter, Repl, Result, Value};
use std::collections::HashMap;
use tafl_game::Game;

#[derive(Default)]
struct Context {
    game: Game,
}

fn print(_args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
    context.game.board.print(None);
    Ok(Some(format!("Printed")))
}

fn game(_args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
    let json = serde_json::to_string(&context.game).unwrap();
    Ok(Some(format!("{}", json)))
}
fn load_command(args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
    let ten: String = args["ten"].convert()?;
    context.game = Game::from_ten(&ten);
    context.game.board.print(None);
    Ok(Some(format!("Loaded")))
}

fn move_command(args: HashMap<String, Value>, context: &mut Context) -> Result<Option<String>> {
    let move_str: String = args["move"].convert()?;

    let from = tafl_game::Game::algebraic_to_pos(&move_str[..2]);
    let to = tafl_game::Game::algebraic_to_pos(&move_str[2..]);
    match (from, to) {
        (Some(f), Some(t)) => match std::mem::take(&mut context.game).make_move(f, t) {
            Some(g) => {
                context.game = g;
                context.game.board.print(None);
                Ok(Some(format!("Moved")))
            }
            None => Ok(Some(format!("Invalid move"))),
        },
        _ => Ok(Some(format!("Invalid notation"))),
    }
}

fn valid_moves_command(
    args: HashMap<String, Value>,
    context: &mut Context,
) -> Result<Option<String>> {
    let piece_str: String = args["piece"].convert()?;
    let pos = tafl_game::Game::algebraic_to_pos(&piece_str);
    match pos {
        Some(p) => {
            let moves = context.game.board.valid_moves(p);
            let moves_pos: Vec<tafl_game::Pos> = moves
                .iter()
                .filter_map(|&(r, c)| tafl_game::Pos::from_index((r * 9 + c) as u8))
                .collect();
            let moves_notation = moves
                .iter()
                .map(|&(r, c)| format!("{}{}", (b'a' + c as u8) as char, (b'1' + r as u8) as char))
                .collect::<Vec<_>>();
            context.game.board.print(Some(&moves_pos));
            Ok(Some(format!("Valid moves: {:?}", moves_notation)))
        }
        None => Ok(Some(format!("Invalid piece notation"))),
    }
}

fn main() -> Result<()> {
    let mut repl = Repl::new(Context::default())
        .with_name("tafl")
        .with_version("v0.1.0")
        .with_description("repl for the tafl-game engine")
        .add_command(Command::new("p", print))
        .add_command(
            Command::new("l", load_command)
                .with_parameter(Parameter::new("ten").set_required(true)?)?
                .with_help("Load a position from TEN, e.g. l 4k4/9/9/9/9/9/9/9/9"),
        )
        .add_command(Command::new("game", game))
        .add_command(
            Command::new("m", move_command)
                .with_parameter(Parameter::new("move").set_required(true)?)?
                .with_help("Make a move, notation is like a4a3"),
        )
        .add_command(
            Command::new("v", valid_moves_command)
                .with_parameter(Parameter::new("piece").set_required(true)?)?
                .with_help("Print valid moves for a piece, notation is like a4"),
        );
    repl.run()
}
