# Tablut Game Implementation Plan

## Overview

Real-time 2-player Tablut (a Viking chess variant) on a 9×9 board. Players connect via WebSocket; one plays Swedes (defenders), the other Muscovites (attackers).

---

## Data Model

### Board

9×9 grid. Use a flat `[Piece; 81]` array indexed by `row * 9 + col`.

```rust
#[derive(Clone, Copy, PartialEq)]
enum Piece {
    Empty,
    King,
    Defender,  // Swede
    Attacker,  // Muscovite
}

#[derive(Clone, Copy, PartialEq)]
enum Square {
    Normal,
    Castle,  // center (4,4)
    Edge,    // any square on row 0, row 8, col 0, col 8
}

struct Board {
    squares: [Piece; 81],
}
```

### Initial Setup

```
. A . A . A . A .     A = Attacker
. . . . . . . . .
A . D D D D D . A
. . D D D D D . .
A D D D K D D D A     K = King (center, square 4,4)
. . D D D D D . .
A . D D D D D . A
. . . . . . . . .
. A . A . A . A .
```

- King at (4,4)
- 8 Defenders: cross around king — (3,4),(5,4),(4,3),(4,5) + (2,4),(6,4),(4,2),(4,6)
- 16 Attackers: 4 on center of each edge — top (0,3),(0,4),(0,5),(0,6)... etc.

### Game State

```rust
struct Game {
    board: Board,
    turn: Side,        // whose turn
    phase: Phase,      // Playing | SwedishWin | MuscoviteWin
    raichi_called: bool,
    tuichu_called: bool,
}

enum Side { Swedes, Muscovites }
enum Phase { Playing, SwedishWin, MuscoviteWin }
```

---

## Movement Rules

1. Piece moves in a straight line (up/down/left/right), 1+ squares
2. Cannot jump over any piece (friendly or enemy)
3. Cannot land on a square occupied by another piece
4. King cannot re-enter the castle once it has left (simpler variant)

**Implementation:** From `(r,c)`, walk in each of 4 directions until hitting a piece or edge. All squares along the way (excluding start, including end if empty) are valid moves.

---

## Capture Rules

### Standard Capture

A piece is captured when sandwiched between **two** enemy pieces on opposite sides (horizontally or vertically) **in the same move**. The capturing player must be the one who moved a piece to create the sandwich.

```
Before: . A D .    After moving A left:  . A D A
                         → D is captured (sandwiched between two A's)
```

**Implementation:** After each move, check the moved piece's row and column for enemy pieces sandwiched between the mover and another friendly piece (or the castle, per castle rules).

### Castle Capture Rules

The castle at (4,4) is special:

1. **Castle unoccupied** — acts like a hostile piece (can participate in captures like a normal piece for either side)
2. **King adjacent to castle (not inside)** — king must be surrounded on 3 sides to be captured
3. **King inside castle (4,4)** — king must be surrounded on all 4 sides to be captured
4. **King in castle, 3 sides surrounded, defender on 4th side** — attackers can capture the defender by pinning it between an attacker and the occupied castle

**Implementation:** After a move targeting the king, apply special capture logic based on king's position relative to castle.

---

## Win Conditions

- **Swedes win** — King reaches any edge square (row 0, row 8, col 0, col 8)
- **Muscovites win** — King is captured

**Implementation:** After each move, check:
1. Is the king on an edge square? → Swedish win
2. Was the king just captured? → Muscovite win

---

## Raichi / Tuichu Warnings

- **raichi** — King has a clear path to the edge. Must be declared.
- **tuichu** — King has two clear paths to the edge. Must be declared.

**Implementation:** After each defender move, check if the king has 0, 1, or 2 unobstructed paths to any edge square. Auto-declare or require player to declare.

---

## Multiplayer Architecture

### Server (Rust / Axum)

- WebSocket endpoint at `/ws`
- Room/lobby system: create game → get room code → other player joins
- Server is authoritative: validates all moves, broadcasts state
- Game state stored in memory per room

### Protocol (JSON over WebSocket)

```rust
// Client → Server
enum ClientMsg {
    CreateGame { variant: String, minutes: u32, increment: u32 },
    JoinGame { room_id: String },
    Move { from: (u8, u8), to: (u8, u8) },
    Resign,
}

// Server → Client
enum ServerMsg {
    GameCreated { room_id: String, side: Side },
    GameJoined { side: Side },
    GameStart { board: [[Piece; 9]; 9], turn: Side },
    MoveMade { from: (u8, u8), to: (u8, u8), captures: Vec<(u8, u8)>, board: [[Piece; 9]; 9], turn: Side },
    Raichi,
    Tuichu,
    GameOver { winner: Side },
    Error(String),
}
```

### Move Validation Flow

1. Client sends `Move { from, to }`
2. Server validates: correct turn, piece belongs to player, path is clear, destination is empty
3. Server executes move on board
4. Server checks for captures (standard + castle rules)
5. Server checks win conditions
6. Server broadcasts `MoveMade` to both players
7. Server switches turn

---

## Frontend

### Board Rendering

- 9×9 CSS grid
- Pieces as colored circles/symbols on squares
- Highlight last move
- Show valid moves on piece selection (click piece → highlight reachable squares)

### Interaction Flow

1. Player clicks a piece → highlight valid destination squares
2. Player clicks a destination → send `Move` to server
3. Server validates and broadcasts → board updates for both players
4. If game over → show result

### Timer

- Each player has a clock (minutes per side + increment)
- Server tracks time, sends clock updates
- Clock ticks down on your turn

---

## Implementation Steps

### Phase 1: Core Game Logic
1. Board representation and initial setup
2. Movement validation (path finding)
3. Standard capture detection
4. Castle capture rules
5. Win condition checks
6. Raichi/tuichu detection

### Phase 2: Server
7. WebSocket endpoint with axum
8. Room/lobby management
9. Game state machine (create → join → playing → over)
10. Move validation on server
11. Timer management

### Phase 3: Frontend
12. Board UI component (9×9 grid with pieces)
13. Piece selection and move highlighting
14. WebSocket client for game communication
15. Lobby UI (create/join game)
16. Timer display

### Phase 4: Polish
17. Captured pieces display
18. Game history / move list
19. Rematch functionality
20. Spectator mode
