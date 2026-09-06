import { memo, useState } from 'react';
import { api } from '../api/api';

const ROWS = 6;
const COLS = 7;

/**
 * Renders a Connect4 board from the content of a Connect4 message.
 *
 * Props:
 *   msg          – the full message object (needs msg.message_id, msg.content)
 *   currentUser  – the logged-in user object
 *   onMoveSent   – called after a move is successfully posted so the parent can refresh
 */
const Connect4Board = memo(function Connect4Board({ msg, currentUser, onMoveSent }) {
  const [error, setError] = useState(null);
  const [sending, setSending] = useState(false);

  const { grid, turn, name } = msg.content;

  // Build a 2D array [row][col] where row 0 is the TOP of the board.
  // col.row[0] is the bottom-most chip (index 0 = bottom, highest index = top).
  // Display row r maps to col.row[ROWS - 1 - r]:
  //   r=0 (top)    → col.row[5] (highest chip)
  //   r=5 (bottom) → col.row[0] (lowest chip)
  const cells = [];
  for (let r = 0; r < ROWS; r++) {
    cells[r] = [];
    for (let c = 0; c < COLS; c++) {
      const col = grid[c];
      const chipIndex = ROWS - 1 - r;
      cells[r][c] = col && chipIndex < col.row.length ? col.row[chipIndex] : null;
    }
  }

  const handleColumnClick = async (colIndex) => {
    if (sending) return;

    // Check if the column is full
    const col = grid[colIndex];
    if (col && col.row.length >= ROWS) return;

    setSending(true);
    setError(null);

    try {
      // PATCH /messages?name=<game_name> with body { content: colIndex }
      const result = await api.updateConnect4(name, colIndex);

      if (result.status === 'success') {
        if (onMoveSent) onMoveSent();
      } else {
        setError(result.status || 'Failed to send move');
      }
    } catch (err) {
      setError(err.message || 'Failed to send move');
    } finally {
      setSending(false);
    }
  };

  const turnLabel = turn === 'Red' ? '🔴 Red' : '🟡 Yellow';
  // Turn enforcement is handled server-side; we just display whose turn it is.

  return (
    <div className="connect4-wrapper">
      <div className="connect4-header">
        <span className="connect4-name">{name}</span>
        <span className="connect4-turn">{turnLabel}'s turn</span>
      </div>

      <div className="connect4-board" role="grid" aria-label={`Connect4 board: ${name}`}>
        {/* Column drop buttons */}
        <div className="connect4-col-buttons" aria-label="Choose a column">
          {Array.from({ length: COLS }, (_, c) => {
            const full = grid[c] && grid[c].row.length >= ROWS;
            return (
              <button
                key={c}
                className="connect4-col-btn"
                onClick={() => handleColumnClick(c)}
                disabled={sending || full}
                aria-label={`Drop in column ${c + 1}${full ? ' (full)' : ''}`}
                title={full ? 'Column full' : `Drop in column ${c + 1}`}
              >
                ▼
              </button>
            );
          })}
        </div>

        {/* Board grid */}
        {cells.map((row, r) => (
          <div key={r} className="connect4-row" role="row">
            {row.map((chip, c) => (
              <div
                key={c}
                className={`connect4-cell${chip === 'Red' ? ' red' : chip === 'Yellow' ? ' yellow' : ''}`}
                role="gridcell"
                aria-label={chip ? `${chip} chip` : 'Empty'}
              >
                <div className="connect4-chip" />
              </div>
            ))}
          </div>
        ))}
      </div>

      {error && <p className="connect4-error">{error}</p>}
    </div>
  );
});

export default Connect4Board;
