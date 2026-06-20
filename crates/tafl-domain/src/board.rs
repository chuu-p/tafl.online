use serde::{Deserialize, Serialize};

pub const SIZE: usize = 9;
pub const CASTLE: usize = 4; // center index (4,4)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "u8", from = "u8")]
pub enum Piece {
    #[serde(rename = "0")]
    Empty,
    #[serde(rename = "1")]
    King,
    #[serde(rename = "2")]
    Defender,
    #[serde(rename = "3")]
    Attacker,
}

impl From<Piece> for u8 {
    fn from(p: Piece) -> u8 {
        match p {
            Piece::Empty => 0,
            Piece::King => 3,
            Piece::Defender => 2,
            Piece::Attacker => 1,
        }
    }
}

impl From<u8> for Piece {
    fn from(v: u8) -> Piece {
        match v {
            0 => Piece::Empty,
            1 => Piece::Attacker,
            2 => Piece::Defender,
            3 => Piece::King,
            _ => Piece::Empty,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    Swedes,
    Muscovites,
}

impl Side {
    pub fn own(&self) -> Piece {
        match self {
            Side::Swedes => Piece::Defender,
            Side::Muscovites => Piece::Attacker,
        }
    }
    pub fn enemy(&self) -> Piece {
        match self {
            Side::Swedes => Piece::Attacker,
            Side::Muscovites => Piece::Defender,
        }
    }
    pub fn opposite(&self) -> Side {
        match self {
            Side::Swedes => Side::Muscovites,
            Side::Muscovites => Side::Swedes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    squares: [Piece; SIZE * SIZE],
}

impl Board {
    pub fn new() -> Self {
        let mut squares = [Piece::Empty; SIZE * SIZE];

        // King at center
        squares[IDX(4, 4)] = Piece::King;

        // Defenders (Swedes) - 8 pieces in a cross around king
        squares[IDX(2, 4)] = Piece::Defender;
        squares[IDX(3, 4)] = Piece::Defender;
        squares[IDX(4, 2)] = Piece::Defender;
        squares[IDX(4, 3)] = Piece::Defender;
        squares[IDX(4, 5)] = Piece::Defender;
        squares[IDX(4, 6)] = Piece::Defender;
        squares[IDX(5, 4)] = Piece::Defender;
        squares[IDX(6, 4)] = Piece::Defender;

        // Attackers (Muscovites) - 16 pieces on edges
        // Top edge
        squares[IDX(0, 3)] = Piece::Attacker;
        squares[IDX(0, 4)] = Piece::Attacker;
        squares[IDX(0, 5)] = Piece::Attacker;
        squares[IDX(1, 4)] = Piece::Attacker;
        // Bottom edge
        squares[IDX(8, 3)] = Piece::Attacker;
        squares[IDX(8, 4)] = Piece::Attacker;
        squares[IDX(8, 5)] = Piece::Attacker;
        squares[IDX(7, 4)] = Piece::Attacker;
        // Left edge
        squares[IDX(3, 0)] = Piece::Attacker;
        squares[IDX(4, 0)] = Piece::Attacker;
        squares[IDX(5, 0)] = Piece::Attacker;
        squares[IDX(4, 1)] = Piece::Attacker;
        // Right edge
        squares[IDX(3, 8)] = Piece::Attacker;
        squares[IDX(4, 8)] = Piece::Attacker;
        squares[IDX(5, 8)] = Piece::Attacker;
        squares[IDX(4, 7)] = Piece::Attacker;

        Board { squares }
    }

    pub fn get(&self, r: usize, c: usize) -> Piece {
        self.squares[IDX(r, c)]
    }

    pub fn set(&mut self, r: usize, c: usize, piece: Piece) {
        self.squares[IDX(r, c)] = piece;
    }

    pub fn remove(&mut self, r: usize, c: usize) {
        self.squares[IDX(r, c)] = Piece::Empty;
    }

    pub fn is_empty(&self, r: usize, c: usize) -> bool {
        self.squares[IDX(r, c)] == Piece::Empty
    }

    pub fn is_edge(&self, r: usize, c: usize) -> bool {
        r == 0 || r == SIZE - 1 || c == 0 || c == SIZE - 1
    }

    pub fn is_castle(&self, r: usize, c: usize) -> bool {
        r == CASTLE && c == CASTLE
    }

    pub fn squares(&self) -> &[Piece; SIZE * SIZE] {
        &self.squares
    }

    /// Returns all valid destination squares for a piece at (r, c).
    pub fn valid_moves(&self, r: usize, c: usize) -> Vec<(usize, usize)> {
        let piece = self.get(r, c);
        if piece == Piece::Empty {
            return vec![];
        }
        let mut moves = vec![];
        // 4 directions: up, down, left, right
        let dirs: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for (dr, dc) in dirs {
            let mut nr = r as isize + dr;
            let mut nc = c as isize + dc;
            while nr >= 0 && nr < SIZE as isize && nc >= 0 && nc < SIZE as isize {
                let (ur, uc) = (nr as usize, nc as usize);
                if self.get(ur, uc) == Piece::Empty {
                    moves.push((ur, uc));
                } else {
                    break; // blocked
                }
                nr += dr;
                nc += dc;
            }
        }
        moves
    }

    /// Check if a move from (fr,fc) to (tr,tc) is valid.
    pub fn is_valid_move(&self, fr: usize, fc: usize, tr: usize, tc: usize) -> bool {
        if self.get(fr, fc) == Piece::Empty {
            return false;
        }
        if !self.is_empty(tr, tc) {
            return false;
        }
        // Must be same row or same column
        if fr != tr && fc != tc {
            return false;
        }
        // Must move at least 1 square
        if fr == tr && fc == tc {
            return false;
        }
        // Path must be clear
        if fr == tr {
            // Horizontal move
            let min_c = fc.min(tc);
            let max_c = fc.max(tc);
            for c in (min_c + 1)..max_c {
                if !self.is_empty(fr, c) {
                    return false;
                }
            }
        } else {
            // Vertical move
            let min_r = fr.min(tr);
            let max_r = fr.max(tr);
            for r in (min_r + 1)..max_r {
                if !self.is_empty(r, fc) {
                    return false;
                }
            }
        }
        true
    }

    /// Execute a move (does not check validity — caller must ensure).
    pub fn make_move(&mut self, fr: usize, fc: usize, tr: usize, tc: usize) {
        let piece = self.get(fr, fc);
        self.remove(fr, fc);
        self.set(tr, tc, piece);
    }

    /// After a piece moves to (tr, tc), find all enemy pieces captured.
    /// Standard capture: enemy sandwiched between two friendly pieces (horizontally or vertically).
    /// Does NOT handle castle rules — call `find_captures_with_castle` for that.
    pub fn find_captures(&self, tr: usize, tc: usize, mover: Side) -> Vec<(usize, usize)> {
        let enemy = mover.enemy();
        let own = mover.own();
        let mut captured = vec![];

        let dirs: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for (dr, dc) in &dirs {
            // Check in both directions along this axis
            for sign in [1isize, -1] {
                let mut r = tr as isize + dr * sign;
                let mut c = tc as isize + dc * sign;
                let mut candidates = vec![];
                let mut found_friendly = false;

                while r >= 0 && r < SIZE as isize && c >= 0 && c < SIZE as isize {
                    let (ur, uc) = (r as usize, c as usize);
                    let p = self.get(ur, uc);
                    if p == enemy {
                        candidates.push((ur, uc));
                    } else if p == own || (p == Piece::King && mover == Side::Muscovites) {
                        found_friendly = true;
                        break;
                    } else {
                        // Empty or friendly king (defenders) — stop
                        break;
                    }
                    r += dr * sign;
                    c += dc * sign;
                }

                if found_friendly {
                    captured.extend(candidates);
                }
            }
        }

        captured
    }

    /// Remove captured pieces from the board.
    pub fn remove_captures(&mut self, captures: &[(usize, usize)]) {
        for &(r, c) in captures {
            self.remove(r, c);
        }
    }

    /// Find captures including castle rules.
    /// This wraps find_captures with special logic for the castle and king.
    pub fn find_captures_with_castle(&self, tr: usize, tc: usize, mover: Side) -> Vec<(usize, usize)> {
        let mut captured = self.find_captures(tr, tc, mover);

        // Castle rules only matter when Muscovites (attackers) are capturing
        if mover != Side::Muscovites {
            return captured;
        }

        let king_pos = self.find_king();
        if let Some((kr, kc)) = king_pos {
            let king_on_castle = kr == CASTLE && kc == CASTLE;
            let king_adjacent = !king_on_castle
                && ((kr == CASTLE && (kc as isize - CASTLE as isize).abs() == 1)
                    || (kc == CASTLE && (kr as isize - CASTLE as isize).abs() == 1));

            if king_on_castle {
                // King inside castle: must be surrounded on all 4 sides to capture
                let surrounding = self.count_surrounding(kr, kc, Side::Muscovites);
                if surrounding >= 4 && !captured.contains(&(kr, kc)) {
                    captured.push((kr, kc));
                }
                // Special rule: king in castle, 3 sides surrounded, defender on 4th
                // Attackers can capture the defender by pinning it between attacker and castle
                if surrounding == 3 {
                    // Find the defender protecting the 4th side
                    let dirs: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                    for (dr, dc) in &dirs {
                        let nr = kr as isize + dr;
                        let nc = kc as isize + dc;
                        if nr >= 0 && nr < SIZE as isize && nc >= 0 && nc < SIZE as isize {
                            let (ur, uc) = (nr as usize, nc as usize);
                            if self.get(ur, uc) == Piece::Defender {
                                // Check if this defender is between an attacker and the castle
                                let opp_r = kr as isize - dr;
                                let opp_c = kc as isize - dc;
                                if opp_r >= 0 && opp_r < SIZE as isize
                                    && opp_c >= 0 && opp_c < SIZE as isize
                                    && self.get(opp_r as usize, opp_c as usize) == Piece::Attacker
                                {
                                    if !captured.contains(&(ur, uc)) {
                                        captured.push((ur, uc));
                                    }
                                }
                            }
                        }
                    }
                }
            } else if king_adjacent {
                // King adjacent to castle: must be surrounded on 3 sides to capture
                let surrounding = self.count_surrounding(kr, kc, Side::Muscovites);
                if surrounding >= 3 && !captured.contains(&(kr, kc)) {
                    captured.push((kr, kc));
                }
            }
        }

        captured
    }

    /// Find the king's position.
    pub fn find_king(&self) -> Option<(usize, usize)> {
        for r in 0..SIZE {
            for c in 0..SIZE {
                if self.get(r, c) == Piece::King {
                    return Some((r, c));
                }
            }
        }
        None
    }

    /// Count how many squares around (r,c) are occupied by attacker pieces.
    fn count_surrounding(&self, r: usize, c: usize, attacker: Side) -> usize {
        let piece = attacker.own();
        let mut count = 0;
        let dirs: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for (dr, dc) in &dirs {
            let nr = r as isize + dr;
            let nc = c as isize + dc;
            if nr >= 0 && nr < SIZE as isize && nc >= 0 && nc < SIZE as isize {
                let p = self.get(nr as usize, nc as usize);
                if p == piece {
                    count += 1;
                }
            }
        }
        count
    }

    /// Check if the king has escaped to an edge square.
    pub fn king_escaped(&self) -> bool {
        match self.find_king() {
            Some((r, c)) => self.is_edge(r, c),
            None => false,
        }
    }

    /// Check if the king has been captured (no king on board).
    pub fn king_captured(&self) -> bool {
        self.find_king().is_none()
    }

    /// Check win conditions. Returns Some(winner) if game is over.
    pub fn check_win(&self) -> Option<Side> {
        if self.king_escaped() {
            Some(Side::Swedes)
        } else if self.king_captured() {
            Some(Side::Muscovites)
        } else {
            None
        }
    }

    /// Count distinct paths from king to any edge square.
    /// Uses BFS. A path is clear if all squares along it are empty.
    /// Returns the number of distinct edge squares reachable.
    pub fn king_escape_paths(&self) -> usize {
        let (kr, kc) = match self.find_king() {
            Some(pos) => pos,
            None => return 0,
        };

        let mut visited = [[false; SIZE]; SIZE];
        let mut queue = std::collections::VecDeque::new();
        let mut edge_count = 0;

        visited[kr][kc] = true;
        queue.push_back((kr, kc));

        let dirs: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

        while let Some((r, c)) = queue.pop_front() {
            if self.is_edge(r, c) && (r, c) != (kr, kc) {
                edge_count += 1;
                continue; // don't expand from edge
            }

            for (dr, dc) in &dirs {
                let nr = r as isize + dr;
                let nc = c as isize + dc;
                if nr >= 0 && nr < SIZE as isize && nc >= 0 && nc < SIZE as isize {
                    let (ur, uc) = (nr as usize, nc as usize);
                    if !visited[ur][uc] && self.is_empty(ur, uc) {
                        visited[ur][uc] = true;
                        queue.push_back((ur, uc));
                    }
                }
            }
        }

        edge_count
    }

    /// Detect raichi (1 escape path) or tuichu (2+ escape paths).
    pub fn detect_warning(&self) -> Option<&'static str> {
        let paths = self.king_escape_paths();
        if paths >= 2 {
            Some("tuichu")
        } else if paths == 1 {
            Some("raichi")
        } else {
            None
        }
    }
}

#[inline]
pub fn idx(r: usize, c: usize) -> usize {
    r * SIZE + c
}

#[inline]
pub fn IDX(r: usize, c: usize) -> usize {
    r * SIZE + c
}

pub fn rc(idx: usize) -> (usize, usize) {
    (idx / SIZE, idx % SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_setup_piece_count() {
        let b = Board::new();
        let mut kings = 0;
        let mut defenders = 0;
        let mut attackers = 0;
        for &p in b.squares() {
            match p {
                Piece::Empty => {}
                Piece::King => kings += 1,
                Piece::Defender => defenders += 1,
                Piece::Attacker => attackers += 1,
            }
        }
        assert_eq!(kings, 1);
        assert_eq!(defenders, 8);
        assert_eq!(attackers, 16);
    }

    #[test]
    fn king_at_center() {
        let b = Board::new();
        assert_eq!(b.get(4, 4), Piece::King);
    }

    #[test]
    fn edge_detection() {
        let b = Board::new();
        assert!(b.is_edge(0, 0));
        assert!(b.is_edge(0, 4));
        assert!(b.is_edge(8, 8));
        assert!(b.is_edge(4, 0));
        assert!(!b.is_edge(1, 1));
        assert!(!b.is_edge(4, 4));
    }

    #[test]
    fn castle_detection() {
        let b = Board::new();
        assert!(b.is_castle(4, 4));
        assert!(!b.is_castle(3, 4));
        assert!(!b.is_castle(4, 3));
    }

    #[test]
    fn side_ownership() {
        assert_eq!(Side::Swedes.own(), Piece::Defender);
        assert_eq!(Side::Muscovites.own(), Piece::Attacker);
        assert_eq!(Side::Swedes.enemy(), Piece::Attacker);
        assert_eq!(Side::Muscovites.enemy(), Piece::Defender);
    }

    #[test]
    fn side_opposite() {
        assert_eq!(Side::Swedes.opposite(), Side::Muscovites);
        assert_eq!(Side::Muscovites.opposite(), Side::Swedes);
    }

    #[test]
    fn is_valid_move_diagonal_rejected() {
        let b = Board::new();
        // Defender at (4,3) — diagonal to (3,2) should be invalid
        assert!(!b.is_valid_move(4, 3, 3, 2));
    }

    #[test]
    fn make_move_moves_piece() {
        let mut b = Board::new();
        // Attacker at (0,3) moves to (1,3)
        b.make_move(0, 3, 1, 3);
        assert!(b.is_empty(0, 3));
        assert_eq!(b.get(1, 3), Piece::Attacker);
    }

    #[test]
    fn empty_square_has_no_moves() {
        let b = Board::new();
        assert!(b.valid_moves(0, 0).is_empty());
    }

    #[test]
    fn basic_horizontal_capture() {
        let mut b = Board::new();
        // Set up: A D A — defender sandwiched between two attackers
        // Row 7, cols 3-5
        b.set(7, 3, Piece::Attacker);
        b.set(7, 4, Piece::Defender);
        b.set(7, 5, Piece::Attacker);

        // Move attacker from (7,5) to (7,6) — doesn't create sandwich
        // Instead, move attacker from (7,5) left to complete sandwich:
        // Actually, let's just test find_captures directly
        // If an attacker moves to (7,2), the defender at (7,4) is sandwiched
        // between attacker at (7,3) and attacker at (7,2)... no that's not right.

        // Better: place attacker moving to create sandwich
        // Setup: . A D A .  at row 7, cols 2-5
        b.set(7, 2, Piece::Attacker);
        b.set(7, 3, Piece::Attacker);
        b.set(7, 4, Piece::Defender);
        b.set(7, 5, Piece::Attacker);

        // Attacker at (7,5) just moved there. Check captures along row 7.
        // Left from (7,5): (7,4)=Defender, (7,3)=Attacker(own) → captured!
        let captures = b.find_captures(7, 5, Side::Muscovites);
        assert!(captures.contains(&(7, 4)));
    }

    #[test]
    fn basic_vertical_capture() {
        let mut b = Board::new();
        // Setup: column 7
        // (5,7) = Attacker (just moved here)
        // (6,7) = Defender
        // (7,7) = Attacker
        b.set(5, 7, Piece::Attacker);
        b.set(6, 7, Piece::Defender);
        b.set(7, 7, Piece::Attacker);

        let captures = b.find_captures(5, 7, Side::Muscovites);
        assert!(captures.contains(&(6, 7)));
    }

    #[test]
    fn no_capture_without_sandwich() {
        let mut b = Board::new();
        // Defender alone between empty squares
        b.set(7, 4, Piece::Defender);
        b.set(7, 5, Piece::Attacker);

        let captures = b.find_captures(7, 5, Side::Muscovites);
        assert!(captures.is_empty());
    }

    #[test]
    fn capture_multiple() {
        let mut b = Board::new();
        // Row 7: A D D A
        b.set(7, 2, Piece::Attacker);
        b.set(7, 3, Piece::Defender);
        b.set(7, 4, Piece::Defender);
        b.set(7, 5, Piece::Attacker);

        // Attacker at (7,5) moved there
        let captures = b.find_captures(7, 5, Side::Muscovites);
        assert!(captures.contains(&(7, 3)));
        assert!(captures.contains(&(7, 4)));
    }

    #[test]
    fn remove_captures_clears_board() {
        let mut b = Board::new();
        b.set(7, 3, Piece::Defender);
        b.set(7, 4, Piece::Defender);
        let captures = vec![(7, 3), (7, 4)];
        b.remove_captures(&captures);
        assert!(b.is_empty(7, 3));
        assert!(b.is_empty(7, 4));
    }

    #[test]
    fn king_adjacent_to_castle_needs_3_surroundings() {
        let mut b = Board::new();
        // King at (4,5), adjacent to castle at (4,4)
        b.remove(4, 4); // remove king from castle
        b.set(4, 5, Piece::King);
        b.set(4, 4, Piece::Empty); // castle is empty (hostile)

        // Surround king on 2 sides — not enough
        b.set(3, 5, Piece::Attacker);
        b.set(5, 5, Piece::Attacker);
        let captures = b.find_captures_with_castle(3, 5, Side::Muscovites);
        assert!(!captures.contains(&(4, 5)));

        // Add 3rd attacker
        b.set(4, 6, Piece::Attacker);
        let captures = b.find_captures_with_castle(4, 6, Side::Muscovites);
        assert!(captures.contains(&(4, 5)));
    }

    #[test]
    fn king_inside_castle_needs_4_surroundings() {
        let mut b = Board::new();
        // King at (4,4) — castle is occupied
        // Surround on 3 sides — not enough
        b.remove(3, 4); // remove defender
        b.set(3, 4, Piece::Attacker);
        b.remove(5, 4);
        b.set(5, 4, Piece::Attacker);
        b.remove(4, 3);
        b.set(4, 3, Piece::Attacker);

        let captures = b.find_captures_with_castle(4, 3, Side::Muscovites);
        assert!(!captures.contains(&(4, 4))); // king not captured with 3

        // Add 4th attacker
        b.remove(4, 5);
        b.set(4, 5, Piece::Attacker);
        let captures = b.find_captures_with_castle(4, 5, Side::Muscovites);
        assert!(captures.contains(&(4, 4))); // king captured with 4
    }

    #[test]
    fn castle_empty_acts_as_hostile() {
        let mut b = Board::new();
        // Remove king from castle
        b.remove(4, 4);
        // Place: Attacker - Castle(empty) - Defender
        // A . D  at row 4, cols 3-5 (castle at 4,4 is empty/hostile)
        b.set(4, 3, Piece::Attacker);
        b.set(4, 4, Piece::Empty); // castle empty = hostile
        b.set(4, 5, Piece::Defender);

        // Defender at (4,5) is between attacker at (4,3) and empty castle
        // The castle acts as hostile, so defender is NOT captured by standard rules
        // (castle is not a piece)
        // But standard capture requires two friendly pieces, castle doesn't count
        let captures = b.find_captures(4, 5, Side::Muscovites);
        assert!(!captures.contains(&(4, 5))); // standard capture: no sandwich
    }

    #[test]
    fn find_king_works() {
        let b = Board::new();
        assert_eq!(b.find_king(), Some((4, 4)));
    }

    #[test]
    fn king_not_escaped_at_start() {
        let b = Board::new();
        assert!(!b.king_escaped());
        assert_eq!(b.check_win(), None);
    }

    #[test]
    fn king_escaped_to_edge() {
        let mut b = Board::new();
        b.remove(4, 4);
        b.set(0, 4, Piece::King); // top edge
        assert!(b.king_escaped());
        assert_eq!(b.check_win(), Some(Side::Swedes));
    }

    #[test]
    fn king_captured() {
        let mut b = Board::new();
        b.remove(4, 4); // remove king
        assert!(b.king_captured());
        assert_eq!(b.check_win(), Some(Side::Muscovites));
    }

    #[test]
    fn king_not_on_edge() {
        let mut b = Board::new();
        b.remove(4, 4);
        b.set(3, 3, Piece::King); // not on edge
        assert!(!b.king_escaped());
        assert_eq!(b.check_win(), None);
    }

    #[test]
    fn some_escape_paths_when_path_cleared() {
        let mut b = Board::new();
        // Clear a path from king to top edge
        b.remove(3, 4); // defender
        b.remove(2, 4); // defender
        // King can now reach some edge squares
        let paths = b.king_escape_paths();
        assert!(paths > 0);
        assert!(b.detect_warning().is_some());
    }

    #[test]
    fn more_escape_paths_with_two_directions() {
        let mut b = Board::new();
        // Clear paths to top and left
        b.remove(3, 4); // defender
        b.remove(2, 4); // defender
        b.remove(4, 3); // defender
        b.remove(4, 2); // defender
        let paths = b.king_escape_paths();
        assert!(paths >= 2);
        assert_eq!(b.detect_warning(), Some("tuichu"));
    }
}
