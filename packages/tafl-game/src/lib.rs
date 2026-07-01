use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const SIZE: usize = 9;
pub const CASTLE: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum P {
    Empty,
    King,
    Defender,
    Attacker,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Board {
    pub squares: [P; SIZE * SIZE],
}

impl Default for Board {
    fn default() -> Self {
        Board::new()
    }
}

#[rustfmt::skip]
impl Board {
    pub fn new() -> Self {
        Self {
            squares: [
                P::Empty,   P::Empty,   P::Empty,   P::Attacker,P::Attacker,P::Attacker,P::Empty,   P::Empty,   P::Empty,
                P::Empty,   P::Empty,   P::Empty,   P::Empty,   P::Attacker,P::Empty,   P::Empty,   P::Empty,   P::Empty,
                P::Empty,   P::Empty,   P::Empty,   P::Empty,   P::Defender,P::Empty,   P::Empty,   P::Empty,   P::Empty,
                P::Attacker,P::Empty,   P::Empty,   P::Empty,   P::Defender,P::Empty,   P::Empty,   P::Empty,   P::Attacker,
                P::Attacker,P::Attacker,P::Defender,P::Defender,P::King,    P::Defender,P::Defender,P::Attacker,P::Attacker,
                P::Attacker,P::Empty,   P::Empty,   P::Empty,   P::Defender,P::Empty,   P::Empty,   P::Empty,   P::Attacker,
                P::Empty,   P::Empty,   P::Empty,   P::Empty,   P::Defender,P::Empty,   P::Empty,   P::Empty,   P::Empty,
                P::Empty,   P::Empty,   P::Empty,   P::Empty,   P::Attacker,P::Empty,   P::Empty,   P::Empty,   P::Empty,
                P::Empty,   P::Empty,   P::Empty,   P::Attacker,P::Attacker,P::Attacker,P::Empty,   P::Empty,   P::Empty,
            ],
        }
    }

    pub fn print(&self, moves: Option<&[Pos]>) {
        println!("{}", self.print_to_string(moves));
    }

    pub fn print_to_string(&self, moves: Option<&[Pos]>) -> String {
        let mut result = String::new();
        let moves_set: std::collections::HashSet<Pos> = moves
            .map(|m| m.iter().copied().collect())
            .unwrap_or_default();
        for col in (0..SIZE).rev() {
            for row in 0..SIZE {
                let piece_str = match self.squares[row * SIZE + col] {
                    P::Empty => " ",
                    P::King => "K",
                    P::Attacker => "A",
                    P::Defender => "D",
                };
                let pos = row * SIZE + col;
                if let Some(p) = Pos::from_index(pos as u8) {
                    if moves_set.contains(&p) {
                        result.push_str(&format!("{}*", piece_str));
                        continue;
                    }
                }
                result.push_str(&format!("{} ", piece_str));
            }
            result.push_str("\n");
        }
        return result;
    }

    pub fn get_captured_positions(&self, from: Pos, to: Pos) -> Vec<(usize, usize)> {
        // Simulate the move on a clone and check what gets captured
        let mut board = self.clone();
        let from_idx = from.index() as usize;
        let to_idx = to.index() as usize;
        let piece = board.squares[from_idx];
        board.squares[from_idx] = P::Empty;
        board.squares[to_idx] = piece;

        let mut captured = vec![];
        let to_r = to_idx / SIZE;
        let to_c = to_idx % SIZE;
        for (dr, dc) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let (er, ec) = match ((to_r as isize + dr), (to_c as isize + dc)) {
                (r, c) if r >= 0 && r < SIZE as isize && c >= 0 && c < SIZE as isize
                    => (r as usize, c as usize),
                _ => continue,
            };
            let enemy = board.squares[er * SIZE + ec];
            if Self::is_enemy(piece, enemy) {
                let (br, bc) = (er as isize + dr, ec as isize + dc);
                if Self::is_wall(br, bc)
                    || (br >= 0 && br < SIZE as isize && bc >= 0 && bc < SIZE as isize
                        && Self::is_friendly(piece, board.squares[br as usize * SIZE + bc as usize]))
                {
                    captured.push((er, ec));
                }
            }
        }
        captured
    }

    fn is_wall(r: isize, c: isize) -> bool {
        if r < 0 || r >= SIZE as isize || c < 0 || c >= SIZE as isize {
            return false; // edge is NOT a wall
        }
        let (ur, uc) = (r as usize, c as usize);
        (ur == 0 || ur == SIZE - 1) && (uc == 0 || uc == SIZE - 1) || (ur == CASTLE && uc == CASTLE)
    }

    fn is_friendly(piece: P, other: P) -> bool {
        matches!(
            (piece, other),
            (P::Attacker, P::Attacker)
                | (P::Defender, P::Defender)
                | (P::Defender, P::King)
                | (P::King, P::Defender)
                | (P::King, P::King)
        )
    }

    fn is_enemy(piece: P, other: P) -> bool {
        other != P::Empty && !Self::is_friendly(piece, other)
    }

    fn capture_after_move(&mut self, to_idx: usize, moved_piece: P) {
        let to_r = to_idx / SIZE;
        let to_c = to_idx % SIZE;
        let dirs: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for (dr, dc) in dirs {
            let enemy_r = to_r as isize + dr;
            let enemy_c = to_c as isize + dc;
            if enemy_r < 0 || enemy_r >= SIZE as isize || enemy_c < 0 || enemy_c >= SIZE as isize {
                continue;
            }
            let (er, ec) = (enemy_r as usize, enemy_c as usize);
            let enemy = self.squares[er * SIZE + ec];
            if Self::is_enemy(moved_piece, enemy) {
                let beyond_r = enemy_r + dr;
                let beyond_c = enemy_c + dc;
                if Self::is_wall(beyond_r, beyond_c)
                    || (beyond_r >= 0 && beyond_r < SIZE as isize
                        && beyond_c >= 0 && beyond_c < SIZE as isize
                        && Self::is_friendly(moved_piece, self.squares[beyond_r as usize * SIZE + beyond_c as usize]))
                {
                    self.squares[er * SIZE + ec] = P::Empty;
                }
            }
        }
    }

    pub fn make_move(&mut self, from: Pos, to: Pos) -> bool {
        let from_idx = from.index() as usize;
        let to_idx = to.index() as usize;

        let piece = self.squares[from_idx];
        if piece == P::Empty {
            return false;
        }

        let valid_moves = self.valid_moves(from);
        let (to_r, to_c) = (to_idx / SIZE, to_idx % SIZE);
        if !valid_moves.contains(&(to_r, to_c)) {
            return false;
        }

        self.squares[from_idx] = P::Empty;
        self.squares[to_idx] = piece;
        self.capture_after_move(to_idx, piece);
        true
    }

    pub fn valid_moves(&self, pos: Pos) -> Vec<(usize, usize)> {
        let idx = pos.index() as usize;
        let r = idx / SIZE;
        let c = idx % SIZE;
        let piece = self.squares[idx];
        if piece == P::Empty {
            return vec![];
        }
        let is_king = piece == P::King;
        let mut moves = vec![];
        let dirs: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for (dr, dc) in dirs {
            let mut nr = r as isize + dr;
            let mut nc = c as isize + dc;
            while nr >= 0 && nr < SIZE as isize && nc >= 0 && nc < SIZE as isize {
                let (ur, uc) = (nr as usize, nc as usize);
                let u_index = ur * SIZE + uc;
                if self.squares[u_index] == P::Empty {
                    // ponytail: only the king may enter corners or the castle
                    if is_king || !((ur == 0 || ur == SIZE - 1) && (uc == 0 || uc == SIZE - 1)
                        || (ur == CASTLE && uc == CASTLE))
                    {
                        moves.push((ur, uc));
                    }
                    nr += dr;
                    nc += dc;
                } else {
                    break;
                }
            }
        }
        moves
    }

    fn find_king(&self) -> Option<(usize, usize)> {
        for r in 0..SIZE {
            for c in 0..SIZE {
                if self.squares[r * SIZE + c] == P::King {
                    return Some((r, c));
                }
            }
        }
        None
    }

    fn is_corner(r: usize, c: usize) -> bool {
        (r == 0 || r == SIZE - 1) && (c == 0 || c == SIZE - 1)
    }

    fn king_escape_paths(&self) -> usize {
        let (kr, kc) = match self.find_king() {
            Some(pos) => pos,
            None => return 0,
        };

        let mut visited = [[false; SIZE]; SIZE];
        let mut queue = std::collections::VecDeque::new();
        let mut corner_count = 0;

        visited[kr][kc] = true;
        queue.push_back((kr, kc));

        let dirs: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

        while let Some((r, c)) = queue.pop_front() {
            if Self::is_corner(r, c) && (r, c) != (kr, kc) {
                corner_count += 1;
                continue;
            }

            for (dr, dc) in &dirs {
                let nr = r as isize + dr;
                let nc = c as isize + dc;
                if nr >= 0 && nr < SIZE as isize && nc >= 0 && nc < SIZE as isize {
                    let (ur, uc) = (nr as usize, nc as usize);
                    if !visited[ur][uc] && self.squares[ur * SIZE + uc] == P::Empty {
                        visited[ur][uc] = true;
                        queue.push_back((ur, uc));
                    }
                }
            }
        }

        corner_count
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Game {
    pub result: Result,
    pub attacker_player: String,
    pub defender_player: String,
    pub attacker_elo: u32,
    pub defender_elo: u32,
    pub moves: Vec<Move>,

    #[serde(skip)]
    pub board: Board,
    #[serde(skip)]
    pub current_side: Side,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Attacker,
    Defender,
}

impl Default for Side {
    fn default() -> Self {
        Side::Attacker
    }
}

impl Side {
    fn toggle(self) -> Self {
        match self {
            Side::Attacker => Side::Defender,
            Side::Defender => Side::Attacker,
        }
    }
}

pub struct Situation {
    pub raichi: bool,
    pub tuichi: bool,
    pub stalemate: bool,
    pub result: Result,
}

impl Game {
    fn new_game() -> Self {
        Game {
            result: Result::Pending,
            attacker_player: String::new(),
            defender_player: String::new(),
            attacker_elo: 0,
            defender_elo: 0,
            moves: Vec::new(),
            board: Board::default(),
            current_side: Side::default(),
        }
    }

    fn update_result(&mut self) {
        if let Some(last_move) = self.moves.last() {
            let (r, c) = last_move.to;
            if (r == 0 || r as usize == SIZE - 1) && (c == 0 || c as usize == SIZE - 1) {
                // ponytail: piece at `to` after move — king escapes to corner
                if self.board.squares[r as usize * SIZE + c as usize] == P::King {
                    self.result = Result::DefenderWin;
                }
            }
        }
        if self.board.find_king().is_none() {
            self.result = Result::AttackerWin;
        }
    }

    pub fn make_move(mut self, from: Pos, to: Pos) -> Option<Self> {
        if self.result != Result::Pending {
            return None;
        }
        let from_idx = from.index() as usize;
        let piece = self.board.squares[from_idx];
        if piece == P::Empty {
            return None;
        }
        // ponytail: only pieces belonging to the current side may move
        let correct_side = match (piece, self.current_side) {
            (P::King | P::Defender, Side::Defender) => true,
            (P::Attacker, Side::Attacker) => true,
            _ => false,
        };
        if !correct_side {
            return None;
        }
        if !self.board.make_move(from, to) {
            return None;
        }
        let to_idx = to.index() as usize;
        self.moves.push(Move {
            from: ((from_idx / SIZE) as u8, (from_idx % SIZE) as u8),
            to: ((to_idx / SIZE) as u8, (to_idx % SIZE) as u8),
        });
        self.current_side = self.current_side.toggle();
        self.update_result();
        Some(self)
    }

    pub fn situation(&self) -> Situation {
        if self.result != Result::Pending {
            return Situation {
                raichi: false,
                tuichi: false,
                stalemate: false,
                result: self.result.clone(),
            };
        }
        let paths = self.board.king_escape_paths();
        Situation {
            raichi: paths >= 2,
            tuichi: paths == 1,
            stalemate: self.all_legal_moves().is_empty(),
            result: self.result.clone(),
        }
    }

    pub fn algebraic_to_pos(s: &str) -> Option<Pos> {
        if s.len() < 2 {
            return None;
        }
        let bytes = s.as_bytes();
        let row = (bytes[0] - b'a') as usize;
        let col = (bytes[1] - b'1') as usize;
        if row >= SIZE || col >= SIZE {
            return None;
        }
        Pos::from_index((row * SIZE + col) as u8)
    }

    pub fn from_tpgn(tpgn: &str) -> Option<Self> {
        let mut game = Game::new();
        let tokens: Vec<&str> = tpgn.split_whitespace().collect();
        let mut i = 0;
        while i + 2 < tokens.len() {
            i += 1;
            let from_str = tokens.get(i)?;
            let to_str = tokens.get(i + 1)?;
            let from = Self::algebraic_to_pos(from_str)?;
            let to = Self::algebraic_to_pos(to_str)?;
            game = game.make_move(from, to)?;
            i += 2;
        }
        Some(game)
    }

    pub fn all_legal_moves(&self) -> HashMap<Pos, Vec<Pos>> {
        if self.result != Result::Pending {
            return HashMap::new();
        }
        let mut result = HashMap::with_capacity(20);
        for r in 0..SIZE {
            for c in 0..SIZE {
                let idx = r * SIZE + c;
                if self.board.squares[idx] == P::Empty {
                    continue;
                }
                if let Some(pos) = Pos::from_index(idx as u8) {
                    let moves = self.board.valid_moves(pos);
                    if !moves.is_empty() {
                        let pos_moves: Vec<Pos> = moves
                            .iter()
                            .filter_map(|&(r, c)| Pos::from_index((r * SIZE + c) as u8))
                            .collect();
                        result.insert(pos, pos_moves);
                    }
                }
            }
        }
        result
    }

    pub fn ten(&self) -> String {
        use std::fmt::Write;
        let mut result = String::with_capacity(128);
        for r in 0..SIZE {
            let mut empty_count = 0;
            for c in 0..SIZE {
                match self.board.squares[r * SIZE + c] {
                    P::Empty => empty_count += 1,
                    P::King | P::Defender | P::Attacker => {
                        if empty_count > 0 {
                            write!(result, "{}", empty_count).unwrap();
                            empty_count = 0;
                        }
                        result.push(match self.board.squares[r * SIZE + c] {
                            P::King => 'k',
                            P::Defender => 'd',
                            P::Attacker => 'a',
                            _ => unreachable!(),
                        });
                    }
                }
            }
            if empty_count > 0 {
                write!(result, "{}", empty_count).unwrap();
            }
            if r < SIZE - 1 {
                result.push('/');
            }
        }
        let to_move = match self.current_side {
            Side::Attacker => 'M',
            Side::Defender => 'S',
        };
        write!(result, " {} - 0 1", to_move).unwrap();
        result
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new_game()
    }
}

impl Game {
    pub fn new() -> Self {
        Self::new_game()
    }

    pub fn from_ten(ten: &str) -> Self {
        let board_part = ten.split(' ').next().unwrap_or(ten);
        let rows: Vec<&str> = board_part.split('/').collect();
        let mut board = Board::default();
        board.squares = [P::Empty; SIZE * SIZE];

        for (y, row) in rows.iter().enumerate() {
            let mut col = 0;
            for cell in row.chars() {
                match cell {
                    'a' => {
                        board.squares[y * SIZE + col] = P::Attacker;
                        col += 1;
                    }
                    'd' => {
                        board.squares[y * SIZE + col] = P::Defender;
                        col += 1;
                    }
                    'k' => {
                        board.squares[y * SIZE + col] = P::King;
                        col += 1;
                    }
                    n @ '1'..='9' => {
                        col += n.to_digit(10).unwrap() as usize;
                    }
                    _ => {}
                }
            }
        }

        let mut game = Self::new_game();
        game.board = board;
        // ponytail: parse side from TEN " M " or " S "
        if let Some(s) = ten.split(' ').nth(1) {
            game.current_side = match s {
                "S" => Side::Defender,
                _ => Side::Attacker,
            };
        }
        game
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Move {
    pub from: (u8, u8),
    pub to: (u8, u8),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Result {
    #[serde(rename = "?")]
    Pending,
    #[serde(rename = "a")]
    AttackerWin,
    #[serde(rename = "d")]
    DefenderWin,
    #[serde(rename = "=")]
    Draw,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Pos {
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
    A8,
    A9,
    B1,
    B2,
    B3,
    B4,
    B5,
    B6,
    B7,
    B8,
    B9,
    C1,
    C2,
    C3,
    C4,
    C5,
    C6,
    C7,
    C8,
    C9,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    D9,
    E1,
    E2,
    E3,
    E4,
    E5,
    E6,
    E7,
    E8,
    E9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    G1,
    G2,
    G3,
    G4,
    G5,
    G6,
    G7,
    G8,
    G9,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    H7,
    H8,
    H9,
    I1,
    I2,
    I3,
    I4,
    I5,
    I6,
    I7,
    I8,
    I9,
}

impl Pos {
    pub fn from_index(idx: u8) -> Option<Self> {
        match idx {
            0 => Some(Pos::A1),
            1 => Some(Pos::A2),
            2 => Some(Pos::A3),
            3 => Some(Pos::A4),
            4 => Some(Pos::A5),
            5 => Some(Pos::A6),
            6 => Some(Pos::A7),
            7 => Some(Pos::A8),
            8 => Some(Pos::A9),
            9 => Some(Pos::B1),
            10 => Some(Pos::B2),
            11 => Some(Pos::B3),
            12 => Some(Pos::B4),
            13 => Some(Pos::B5),
            14 => Some(Pos::B6),
            15 => Some(Pos::B7),
            16 => Some(Pos::B8),
            17 => Some(Pos::B9),
            18 => Some(Pos::C1),
            19 => Some(Pos::C2),
            20 => Some(Pos::C3),
            21 => Some(Pos::C4),
            22 => Some(Pos::C5),
            23 => Some(Pos::C6),
            24 => Some(Pos::C7),
            25 => Some(Pos::C8),
            26 => Some(Pos::C9),
            27 => Some(Pos::D1),
            28 => Some(Pos::D2),
            29 => Some(Pos::D3),
            30 => Some(Pos::D4),
            31 => Some(Pos::D5),
            32 => Some(Pos::D6),
            33 => Some(Pos::D7),
            34 => Some(Pos::D8),
            35 => Some(Pos::D9),
            36 => Some(Pos::E1),
            37 => Some(Pos::E2),
            38 => Some(Pos::E3),
            39 => Some(Pos::E4),
            40 => Some(Pos::E5),
            41 => Some(Pos::E6),
            42 => Some(Pos::E7),
            43 => Some(Pos::E8),
            44 => Some(Pos::E9),
            45 => Some(Pos::F1),
            46 => Some(Pos::F2),
            47 => Some(Pos::F3),
            48 => Some(Pos::F4),
            49 => Some(Pos::F5),
            50 => Some(Pos::F6),
            51 => Some(Pos::F7),
            52 => Some(Pos::F8),
            53 => Some(Pos::F9),
            54 => Some(Pos::G1),
            55 => Some(Pos::G2),
            56 => Some(Pos::G3),
            57 => Some(Pos::G4),
            58 => Some(Pos::G5),
            59 => Some(Pos::G6),
            60 => Some(Pos::G7),
            61 => Some(Pos::G8),
            62 => Some(Pos::G9),
            63 => Some(Pos::H1),
            64 => Some(Pos::H2),
            65 => Some(Pos::H3),
            66 => Some(Pos::H4),
            67 => Some(Pos::H5),
            68 => Some(Pos::H6),
            69 => Some(Pos::H7),
            70 => Some(Pos::H8),
            71 => Some(Pos::H9),
            72 => Some(Pos::I1),
            73 => Some(Pos::I2),
            74 => Some(Pos::I3),
            75 => Some(Pos::I4),
            76 => Some(Pos::I5),
            77 => Some(Pos::I6),
            78 => Some(Pos::I7),
            79 => Some(Pos::I8),
            80 => Some(Pos::I9),
            _ => None,
        }
    }

    pub fn index(self) -> u8 {
        self as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Original API tests =====

    #[test]
    fn tafl_has_expected_api_shape() {
        let mut new_game = Game::new();
        new_game.current_side = Side::Attacker;
        let ten = "3aaa3/4a4/4d4/a3d3a/aaddkddaa/a3d3a/4d4/4a4/3aaa3 M - 0 1";
        let new_game_custom = Game::from_ten(&ten);
        assert_eq!(new_game.ten(), ten);
        assert_eq!(new_game_custom.ten(), ten);

        let legal_moves = new_game.all_legal_moves();
        // ponytail: 5 because attacker cannot enter corner (0,0)
        assert_eq!(legal_moves[&Pos::A4].len(), 5);

        let new_game_after = new_game.make_move(Pos::A4, Pos::A3).expect("Invalid move");

        let situation = new_game_after.situation();
        assert!(!situation.raichi);
        assert!(!situation.tuichi);
        assert!(!situation.stalemate);
        assert!(situation.result != Result::Draw);
    }

    fn from_tpgn_helper(tpgn: &str) -> Game {
        let mut g = Game::from_tpgn(tpgn).expect("Invalid PGN");
        g.current_side = Side::Attacker;
        g
    }

    #[test]
    fn from_tpgn_works_raichi_happy() {
        let tpgn_text = "1. a4 a3 2. d5 d7 3. a3 a4 4. e5 d5 5. a4 a3 6. d5 d2 7. a3 a4 8. d2 a2";
        let game_after_pgn = from_tpgn_helper(tpgn_text);
        assert!(game_after_pgn.situation().raichi);
        assert!(!game_after_pgn.situation().tuichi);
    }

    #[test]
    fn from_tpgn_works_winner_happy() {
        // ponytail: with wall capture, king captured at move 9 when attacker corners it at a2
        let tpgn_text =
            "1. a4 a3 2. d5 d7 3. a3 a4 4. e5 d5 5. a4 a3 6. d5 d2 7. a3 a4 8. d2 a2 9. a4 a3";
        let game_after_pgn = from_tpgn_helper(tpgn_text);
        // King at a2 (0,1) is sandwiched between attacker at a3 (0,2) and corner a1 (0,0)
        assert_eq!(game_after_pgn.result, Result::AttackerWin);
    }

    #[test]
    fn pos_roundtrip() {
        for i in 0..=80u8 {
            let p = Pos::from_index(i).unwrap();
            assert_eq!(p.index(), i);
        }
        assert!(Pos::from_index(81).is_none());
    }

    // ===== Board Setup =====

    #[test]
    fn king_starts_at_center_castle() {
        let game = Game::new();
        assert_eq!(game.board.find_king(), Some((4, 4)));
    }

    #[test]
    fn initial_board_has_16_attackers() {
        let game = Game::new();
        let count = game
            .board
            .squares
            .iter()
            .filter(|&&p| p == P::Attacker)
            .count();
        assert_eq!(count, 16);
    }

    #[test]
    fn initial_board_has_8_defenders() {
        let game = Game::new();
        let count = game
            .board
            .squares
            .iter()
            .filter(|&&p| p == P::Defender)
            .count();
        assert_eq!(count, 8);
    }

    #[test]
    fn defenders_form_cross_around_king() {
        let game = Game::new();
        let b = &game.board.squares;
        assert_eq!(b[3 * 9 + 4], P::Defender); // E4
        assert_eq!(b[5 * 9 + 4], P::Defender); // E6
        assert_eq!(b[4 * 9 + 3], P::Defender); // D5
        assert_eq!(b[4 * 9 + 5], P::Defender); // F5
    }

    #[test]
    fn attackers_grouped_at_edge_centers() {
        let game = Game::new();
        let b = &game.board.squares;
        assert_eq!(b[0 * 9 + 3], P::Attacker); // D1
        assert_eq!(b[0 * 9 + 4], P::Attacker); // E1
        assert_eq!(b[0 * 9 + 5], P::Attacker); // F1
        assert_eq!(b[8 * 9 + 3], P::Attacker); // D9
        assert_eq!(b[8 * 9 + 4], P::Attacker); // E9
        assert_eq!(b[8 * 9 + 5], P::Attacker); // F9
        assert_eq!(b[3 * 9 + 0], P::Attacker); // A4
        assert_eq!(b[4 * 9 + 0], P::Attacker); // A5
        assert_eq!(b[5 * 9 + 0], P::Attacker); // A6
        assert_eq!(b[3 * 9 + 8], P::Attacker); // I4
        assert_eq!(b[4 * 9 + 8], P::Attacker); // I5
        assert_eq!(b[5 * 9 + 8], P::Attacker); // I6
    }

    // ===== Movement: Positive =====

    #[test]
    fn piece_moves_any_vacant_squares_in_line() {
        // King alone at a5 (row0,col4) on empty board
        let game = Game::from_ten("4k4/9/9/9/9/9/9/9/9");
        let moves = game.board.valid_moves(Pos::A5);
        assert!(moves.contains(&(0, 0))); // A1
        assert!(moves.contains(&(0, 8))); // I1
        assert!(moves.contains(&(8, 4))); // E9
    }

    #[test]
    fn piece_slides_until_blocked() {
        // Defender at D1 (row3,col0), attacker at I4 (row3,col8)
        let game = Game::from_ten("9/9/9/d7a/9/9/9/9/9");
        let moves = game.board.valid_moves(Pos::D1);
        assert!(moves.contains(&(3, 1))); // D2
        assert!(moves.contains(&(3, 7))); // H4
        assert!(!moves.contains(&(3, 8))); // I4: occupied
    }

    // ===== Movement: Negative =====

    #[test]
    fn piece_cannot_move_diagonally() {
        let game = Game::from_ten("4k4/9/9/9/9/9/9/9/9");
        let moves = game.board.valid_moves(Pos::A5);
        assert!(!moves.contains(&(1, 3))); // not diagonal
        assert!(!moves.contains(&(1, 5))); // not diagonal
    }

    #[test]
    fn piece_cannot_jump_over_own_piece() {
        // King at E5, defender at E4 blocks northward
        let game = Game::from_ten("9/9/9/4d4/4k4/9/9/9/9");
        let moves = game.board.valid_moves(Pos::E5);
        assert!(moves.contains(&(5, 4))); // E6: can go south
        assert!(!moves.contains(&(2, 4))); // E3: blocked by defender at E4
        assert!(!moves.contains(&(1, 4))); // E2
        assert!(!moves.contains(&(0, 4))); // E1
    }

    #[test]
    fn piece_cannot_jump_over_enemy_piece() {
        let game = Game::from_ten("9/9/9/4a4/4k4/9/9/9/9");
        let moves = game.board.valid_moves(Pos::E5);
        assert!(!moves.contains(&(2, 4))); // blocked by attacker at E4
    }

    #[test]
    fn empty_square_has_no_moves() {
        let game = Game::new();
        assert!(game.board.valid_moves(Pos::B2).is_empty());
    }

    // ===== Restricted squares: corners & castle =====

    #[test]
    fn defender_cannot_enter_corner() {
        // Defender at D5 (flat index 4*9+3=39) next to empty corner A1 (row0,col0)
        // ponytail: defender cannot step into (0,0)
        let game = Game::from_ten("d8/9/9/9/9/9/9/9/9");
        assert!(!game.board.valid_moves(Pos::A1).contains(&(0, 0)));
    }

    #[test]
    fn attacker_cannot_enter_corner() {
        // Attacker at A2 (row0,col1) next to empty corner A1
        let game = Game::from_ten("1a7/9/9/9/9/9/9/9/9");
        assert!(!game.board.valid_moves(Pos::A2).contains(&(0, 0)));
    }

    #[test]
    fn defender_cannot_enter_castle() {
        // Defender at D5 (row4,col3) next to empty castle E5 (row4,col4)
        let game = Game::from_ten("9/9/9/9/3d5/9/9/9/9");
        let moves = game.board.valid_moves(Pos::D5);
        assert!(!moves.contains(&(4, 4)));
    }

    #[test]
    fn attacker_cannot_enter_castle() {
        // Attacker at E4 (row3,col4) next to empty castle E5 (row4,col4)
        let game = Game::from_ten("9/9/9/4a4/9/9/9/9/9");
        let moves = game.board.valid_moves(Pos::E4);
        assert!(!moves.contains(&(4, 4)));
    }

    #[test]
    fn king_can_enter_corner() {
        // King at A2 (row0,col1) — can slide to corner A1 (row0,col0)
        let game = Game::from_ten("1k7/9/9/9/9/9/9/9/9");
        assert!(game.board.valid_moves(Pos::A2).contains(&(0, 0)));
    }

    #[test]
    fn king_can_enter_castle() {
        // King at E4 (row4,col3) — can move right into castle E5 (row4,col4)
        let game = Game::from_ten("9/9/9/9/3k5/9/9/9/9");
        let moves = game.board.valid_moves(Pos::E4);
        assert!(moves.contains(&(4, 4)));
    }

    // ===== Capture: Positive =====

    #[test]
    fn horizontal_capture() {
        // Row4: D5=attacker, E5=defender, I5=attacker. Move I5->F5 to sandwich E5.
        let mut game = Game::from_ten("9/9/9/9/3ad3a/9/9/9/9");
        game.current_side = Side::Attacker;
        let game = game.make_move(Pos::E9, Pos::E6).unwrap();
        assert_eq!(game.board.squares[4 * 9 + 4], P::Empty); // E5 captured
    }

    #[test]
    fn vertical_capture() {
        // Row3: E4=defender. Row4: E5=attacker. Row5: F6=defender. Move F6->F5.
        let mut game = Game::from_ten("9/9/9/4d4/4a4/5d3/9/9/9");
        game.current_side = super::Side::Defender;
        let game = game.make_move(Pos::F6, Pos::F5).unwrap();
        assert_eq!(game.board.squares[4 * 9 + 4], P::Empty); // E5 captured
    }

    #[test]
    fn king_participates_in_capture() {
        // Row1: E2=defender. Row3: E4=attacker. Row4: E5=king.
        // Move defender E2->E3, sandwiching attacker E4 between E3 and E5(king).
        let mut game = Game::from_ten("9/4d4/9/4a4/4k4/9/9/9/9");
        game.current_side = super::Side::Defender;
        let game = game.make_move(Pos::B5, Pos::C5).unwrap();
        assert_eq!(game.board.squares[3 * 9 + 4], P::Empty); // E4 captured
    }

    #[test]
    fn double_capture_in_one_move() {
        // Row1: B5=attacker (row1,col4). Row2: C5=attacker (row2,col2),
        // D5=defender (row2,col3), empty (row2,col4), F5=defender (row2,col5),
        // G5=attacker (row2,col6). Move B5->C5, capturing both D5 and F5.
        let mut game = Game::from_ten("9/4a4/2ad1da2/9/9/9/9/9/9");
        game.current_side = Side::Attacker;
        let game = game.make_move(Pos::B5, Pos::C5).unwrap();
        assert_eq!(game.board.squares[2 * 9 + 3], P::Empty); // D5 captured
        assert_eq!(game.board.squares[2 * 9 + 5], P::Empty); // F5 captured
    }

    #[test]
    fn edge_does_not_act_as_wall() {
        // Row0: 4 empty, defender at A5 (0,4), 1 empty, attacker at A7 (0,6), 2 empty.
        // Attacker at A7 moves to A6 (0,5). Defender at A5 should NOT be captured
        // by edge alone (no corner or center involved).
        let mut game = Game::from_ten("4d1a2/9/9/9/9/9/9/9/9");
        game.current_side = Side::Attacker;
        let game = game.make_move(Pos::A7, Pos::A6).unwrap();
        assert_eq!(game.board.squares[0 * 9 + 4], P::Defender); // A5 NOT captured
    }

    #[test]
    fn center_acts_as_wall_for_capture() {
        // Row4: empty, attacker at E2 (4,1), empty, defender at E4 (4,3), attacker at E6 (4,5), 3 empty
        // Attacker at E2 moves right to E3, sandwiching defender at E4 against center E5 (4,4)
        let mut game = Game::from_ten("9/9/9/9/1a1d1a3/9/9/9/9");
        game.current_side = Side::Attacker;
        let game = game.make_move(Pos::E2, Pos::E3).unwrap();
        assert_eq!(game.board.squares[4 * 9 + 3], P::Empty); // E4 captured by center wall
    }

    // ===== Capture: Negative =====

    #[test]
    fn no_capture_with_only_one_flanking_piece() {
        // Row3: E4=attacker. Row4: E5=defender.
        let mut game = Game::from_ten("9/9/9/4a4/4d4/9/9/9/9");
        game.current_side = Side::Attacker;
        let game = game.make_move(Pos::D5, Pos::D4).unwrap();
        assert_eq!(game.board.squares[4 * 9 + 4], P::Defender); // E5 not captured
    }

    #[test]
    fn no_capture_if_not_moved_into_flanking_position() {
        // Row3: E4=attacker. Row4: E5=defender. Row5: E6=attacker.
        // Move attacker E4->D4: no longer flanks E5.
        let mut game = Game::from_ten("9/9/9/4a4/4d4/4a4/9/9/9");
        game.current_side = Side::Attacker;
        let game = game.make_move(Pos::D5, Pos::D4).unwrap();
        assert_eq!(game.board.squares[4 * 9 + 4], P::Defender); // E5 still there
    }

    // ===== Goal: King Escapes =====

    #[test]
    fn king_to_corner_wins() {
        // King at A2 (row0,col1), move to A1 (row0,col0) = corner
        let mut game = Game::from_ten("1k7/9/9/9/9/9/9/9/9");
        game.current_side = super::Side::Defender;
        let game = game.make_move(Pos::A2, Pos::A1).unwrap();
        assert_eq!(game.result, Result::DefenderWin);
        assert!(game.all_legal_moves().is_empty());
    }

    #[test]
    fn king_to_non_corner_no_win() {
        // King at A2 (row0,col1), move to A3 (row0,col2) = not corner
        let mut game = Game::from_ten("1k7/9/9/9/9/9/9/9/9");
        game.current_side = super::Side::Defender;
        let game = game.make_move(Pos::A2, Pos::A3).unwrap();
        assert_eq!(game.result, Result::Pending);
    }

    // ===== Goal: King Captured =====

    #[test]
    fn king_captured_attacker_wins() {
        // Row4: D5=attacker, E5=king, I5=attacker. Move I5->F5 sandwiching king.
        let mut game = Game::from_ten("9/9/9/9/3ak3a/9/9/9/9");
        game.current_side = Side::Attacker;
        let game = game.make_move(Pos::E9, Pos::E6).unwrap();
        assert_eq!(game.result, Result::AttackerWin);
        assert!(game.all_legal_moves().is_empty());
    }

    #[test]
    fn king_not_captured_without_two_flankers() {
        // Row4: E5=attacker, F5=king. Only one attacker adjacent.
        let mut game = Game::from_ten("9/9/9/9/4ak3/9/9/9/9");
        game.current_side = Side::Attacker;
        let game = game.make_move(Pos::E5, Pos::E4).unwrap();
        assert_eq!(game.result, Result::Pending);
    }

    // ===== Warning: Raichi / Tuichi =====

    #[test]
    fn raichi_on_open_board() {
        // King at E1 on empty board: paths to all 4 corners
        let game = Game::from_ten("4k4/9/9/9/9/9/9/9/9");
        let sit = game.situation();
        assert!(sit.raichi);
        assert!(!sit.tuichi);
    }

    #[test]
    fn tuichi_one_escape_path() {
        // King at E1, attacker at F1 blocks east, wall at row1 blocks south.
        // Only path: west to A1.
        let game = Game::from_ten("4ka3/aaaaaaaaa/9/9/9/9/9/9/9");
        let sit = game.situation();
        assert!(!sit.raichi);
        assert!(sit.tuichi);
    }

    #[test]
    fn no_raichi_or_tuichi_when_trapped() {
        // King at E5, attackers on all 4 sides
        let game = Game::from_ten("9/9/9/4a4/3aka3/4a4/9/9/9");
        let sit = game.situation();
        assert!(!sit.raichi);
        assert!(!sit.tuichi);
    }

    // ===== Game Over States =====

    #[test]
    fn game_over_prevents_further_moves() {
        let mut game = Game::from_ten("9/9/9/9/3ak3a/9/9/9/9");
        game.current_side = Side::Attacker;
        let game = game.make_move(Pos::E9, Pos::E6).unwrap();
        assert_eq!(game.result, Result::AttackerWin);
        assert!(game.make_move(Pos::E6, Pos::E5).is_none());
    }
}
