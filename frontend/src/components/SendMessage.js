import { useState } from 'react';
import { api } from '../api/api';

function SendMessage({ chatId, senderId, onMessageSent }) {
  const [content, setContent] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!content.trim()) return;

    setError('');
    setLoading(true);

    try {
      const result = await api.sendMessage({
        chat_id: chatId,
        sender_id: senderId,
        content: content.trim(),
      });

      if (result.res === 'success') {
        setContent('');
        onMessageSent();
      } else {
        setError(result.res || 'Failed to send message');
      }
    } catch (err) {
      setError('Failed to connect to server');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="send-message">
      <form onSubmit={handleSubmit}>
        <input
          type="text"
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder="Type a message..."
          disabled={loading}
        />
        <button type="submit" disabled={loading || !content.trim()}>
          {loading ? 'Sending...' : 'Send'}
        </button>
      </form>
      {error && <p className="error">{error}</p>}
    </div>
  );
}

export default SendMessage;
