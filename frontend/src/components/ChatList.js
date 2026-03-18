import { useEffect, useState, useCallback } from 'react';
import { api } from '../api/api';

function ChatList({ currentUser, onSelectChat, selectedChatId }) {
  const [chats, setChats] = useState([]);
  const [loading, setLoading] = useState(true);

  const loadChats = useCallback(async () => {
    setLoading(true);
    try {
      const result = await api.getUserChats(currentUser.user_id);
      if (result.status === 'success' && result.data) {
        setChats(result.data);
      }
    } catch (err) {
      console.error('Failed to load chats:', err);
    } finally {
      setLoading(false);
    }
  }, [currentUser.user_id]);

  useEffect(() => {
    loadChats();
  }, [loadChats]);

  // Expose loadChats to parent via custom event
  useEffect(() => {
    const handleRefresh = () => loadChats();
    window.addEventListener('refreshChats', handleRefresh);
    return () => window.removeEventListener('refreshChats', handleRefresh);
  }, [loadChats]);

  if (loading) {
    return <div className="chat-list loading">Loading chats...</div>;
  }

  if (chats.length === 0) {
    return <div className="chat-list empty">No chats yet. Create one!</div>;
  }

  return (
    <div className="chat-list">
      <h3>Your Chats</h3>
      <ul>
        {chats.map((chat) => (
          <li
            key={chat.chat_id}
            className={selectedChatId === chat.chat_id ? 'active' : ''}
            onClick={() => onSelectChat(chat.chat_id)}
          >
            {chat.chat_name || 'Unnamed Chat'}
          </li>
        ))}
      </ul>
    </div>
  );
}

export default ChatList;
