import { useState, useRef, useEffect } from 'react';
import { api } from '../api/api';

// ---------------------------------------------------------------------------
// Connect4 sub-form shown inside the attach modal
// ---------------------------------------------------------------------------
function Connect4Form({ senderName, onSubmit, onCancel, loading, serverError }) {
  const [boardName, setBoardName] = useState('');
  const [startCol, setStartCol] = useState('4');
  const [opponent, setOpponent] = useState('');
  const [validationError, setValidationError] = useState('');

  const error = serverError || validationError;

  const handleSubmit = (e) => {
    e.preventDefault();
    const col = parseInt(startCol, 10);
    if (!boardName.trim()) { setValidationError('Board name is required'); return; }
    if (!opponent.trim()) { setValidationError('Opponent username is required'); return; }
    if (isNaN(col) || col < 1 || col > 7) { setValidationError('Starting column must be 1–7'); return; }
    setValidationError('');
    onSubmit({ boardName: boardName.trim(), startCol: col, opponent: opponent.trim() });
  };

  return (
    <form className="attach-modal-form" onSubmit={handleSubmit}>
      <h4 className="attach-modal-section-title">🟡 New Connect 4 Board</h4>

      <label className="attach-modal-label" htmlFor="c4-board-name">Board name</label>
      <input
        id="c4-board-name"
        className="attach-modal-input"
        type="text"
        placeholder="e.g. Game vs Alice"
        value={boardName}
        onChange={(e) => setBoardName(e.target.value)}
        disabled={loading}
        autoFocus
      />

      <label className="attach-modal-label" htmlFor="c4-opponent">Opponent username</label>
      <input
        id="c4-opponent"
        className="attach-modal-input"
        type="text"
        placeholder="their username"
        value={opponent}
        onChange={(e) => setOpponent(e.target.value)}
        disabled={loading}
      />

      <label className="attach-modal-label" htmlFor="c4-start-col">
        Starting column (1–7) — Yellow's first chip
      </label>
      <input
        id="c4-start-col"
        className="attach-modal-input"
        type="number"
        min="1"
        max="7"
        value={startCol}
        onChange={(e) => setStartCol(e.target.value)}
        disabled={loading}
      />

      {error && <p className="attach-modal-error">{error}</p>}

      <div className="attach-modal-actions">
        <button type="button" className="attach-modal-cancel-btn" onClick={onCancel} disabled={loading}>
          Cancel
        </button>
        <button type="submit" className="attach-modal-submit-btn" disabled={loading}>
          {loading ? 'Creating…' : 'Create Board'}
        </button>
      </div>
    </form>
  );
}

// ---------------------------------------------------------------------------
// Main SendMessage component
// ---------------------------------------------------------------------------
function SendMessage({ chatId, senderName, onMessageSent }) {
  const [content, setContent] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  // Image state
  const [imageFile, setImageFile] = useState(null);
  const [imagePreview, setImagePreview] = useState(null);
  const fileInputRef = useRef(null);

  // Attach modal state: null | 'menu' | 'connect4'
  const [attachView, setAttachView] = useState(null);
  const [connect4Error, setConnect4Error] = useState('');
  const modalRef = useRef(null);

  // Close modal on outside click
  useEffect(() => {
    if (!attachView) return;
    const handleClick = (e) => {
      if (modalRef.current && !modalRef.current.contains(e.target)) {
        setAttachView(null);
      }
    };
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, [attachView]);

  // Close modal on Escape
  useEffect(() => {
    if (!attachView) return;
    const handleKey = (e) => { if (e.key === 'Escape') setAttachView(null); };
    document.addEventListener('keydown', handleKey);
    return () => document.removeEventListener('keydown', handleKey);
  }, [attachView]);

  const handleFileChange = (e) => {
    const file = e.target.files[0];
    if (!file) return;
    setImageFile(file);
    setImagePreview(URL.createObjectURL(file));
    setAttachView(null);
  };

  const clearImage = () => {
    setImageFile(null);
    setImagePreview(null);
    if (fileInputRef.current) fileInputRef.current.value = '';
  };

  // ---- Send regular message / image ----
  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!content.trim() && !imageFile) return;

    setError('');
    setLoading(true);

    try {
      let minioKey = null;

      if (imageFile) {
        const fileExtension = imageFile.name.split('.').pop();
        const { upload_url, object_key } = await api.getUploadUrl(chatId, fileExtension);
        await api.uploadFileToMinio(upload_url, imageFile);
        minioKey = object_key;
      }

      const messageContent = { text: content.trim() || '' };
      if (minioKey) messageContent.url = minioKey;

      const result = await api.sendMessage({
        sender_name: senderName,
        parent: chatId,
        content: messageContent,
      });

      if (result.res === 'success') {
        setContent('');
        clearImage();
        onMessageSent();
      } else {
        setError(result.res || 'Failed to send message');
      }
    } catch (err) {
      setError(err.message || 'Failed to send message');
    } finally {
      setLoading(false);
    }
  };

  // ---- Create Connect4 board ----
  const handleConnect4Submit = async ({ boardName, startCol, opponent }) => {
    setLoading(true);
    setConnect4Error('');
    try {
      const result = await api.createConnect4({
        senderName,
        chatId,
        boardName,
        startCol,
        opponent,
      });
      if (result.res === 'success') {
        setAttachView(null);
        setConnect4Error('');
        onMessageSent();
      } else {
        setConnect4Error(result.res || result.error || 'Failed to create board');
      }
    } catch (err) {
      setConnect4Error(err.message || 'Failed to create board');
    } finally {
      setLoading(false);
    }
  };

  const canSend = !loading && (content.trim() || imageFile);

  return (
    <div className="send-message">
      {imagePreview && (
        <div className="image-preview">
          <img src={imagePreview} alt="Preview" />
          <button
            type="button"
            className="remove-image-btn"
            onClick={clearImage}
            title="Remove image"
          >
            ✕
          </button>
        </div>
      )}

      {/* Hidden file input */}
      <input
        type="file"
        accept="image/*"
        ref={fileInputRef}
        onChange={handleFileChange}
        style={{ display: 'none' }}
      />

      {/* Row: attach button + text input + send button.
          The attach-wrapper is intentionally outside the <form> so that
          the Connect4Form (which has its own <form>) is never nested inside
          this form — nested forms cause the browser to reload the page. */}
      <div className="send-message-row">
        <div className="attach-wrapper" ref={modalRef}>
          <button
            type="button"
            className={`attach-btn${attachView ? ' attach-btn--active' : ''}`}
            onClick={() => { setAttachView(attachView ? null : 'menu'); setConnect4Error(''); }}
            disabled={loading}
            title="Attach or create"
            aria-haspopup="true"
            aria-expanded={!!attachView}
          >
            ＋
          </button>

          {attachView && (
            <div className="attach-modal" role="dialog" aria-label="Attach options">
              {attachView === 'menu' && (
                <ul className="attach-menu">
                  <li>
                    <button
                      type="button"
                      className="attach-menu-item"
                      onClick={() => fileInputRef.current?.click()}
                    >
                      <span className="attach-menu-icon">🖼️</span>
                      <span>Upload Image</span>
                    </button>
                  </li>
                  <li>
                    <button
                      type="button"
                      className="attach-menu-item"
                      onClick={() => setAttachView('connect4')}
                    >
                      <span className="attach-menu-icon">🔴</span>
                      <span>New Connect 4 Board</span>
                    </button>
                  </li>
                </ul>
              )}

              {attachView === 'connect4' && (
                <Connect4Form
                  senderName={senderName}
                  onSubmit={handleConnect4Submit}
                  onCancel={() => { setAttachView('menu'); setConnect4Error(''); }}
                  loading={loading}
                  serverError={connect4Error}
                />
              )}
            </div>
          )}
        </div>

        <form onSubmit={handleSubmit}>
          <input
            type="text"
            className="send-message-input"
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder="Type a message..."
            disabled={loading}
          />
          <button type="submit" disabled={!canSend}>
            {loading ? 'Sending...' : 'Send'}
          </button>
        </form>
      </div>

      {error && <p className="error">{error}</p>}
    </div>
  );
}

export default SendMessage;
