import { useState } from 'react';
import { api } from '../api/api';

function CreateChat({ currentUser, onChatCreated }) {
  const [chatName, setChatName] = useState('');
  const [usernames, setUsernames] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      // Step 1: Create the chat
      const chatResult = await api.createChat(chatName);
      if (chatResult.res !== 'success') {
        throw new Error('Failed to create chat');
      }
      const chat_id = chatResult.data.chat_id;

      // Step 2: Link current user to the chat
      const linkResult = await api.linkUserToChat(currentUser.user_id, chat_id);
      if (linkResult.res !== 'success') {
        throw new Error('Failed to join chat');
      }

      // Step 3: Look up other users by username and link them
      const usernameList = usernames
        .split(',')
        .map((u) => u.trim())
        .filter((u) => u.length > 0);

      for (const username of usernameList) {
        const userResult = await api.getUserByUsername(username);
        if (userResult.status === 'success' && userResult.payload) {
          await api.linkUserToChat(userResult.payload.user_id, chat_id);
        } else {
          setError(`User "${username}" not found, but chat was created.`);
        }
      }

      // Success - reset form and notify parent
      setChatName('');
      setUsernames('');
      onChatCreated();
    } catch (err) {
      setError(err.message || 'Failed to create chat');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="create-chat">
      <h3>Create New Chat</h3>
      <form onSubmit={handleSubmit}>
        <div className="form-group">
          <label>Chat Name</label>
          <input
            type="text"
            value={chatName}
            onChange={(e) => setChatName(e.target.value)}
            required
          />
        </div>
        <div className="form-group">
          <label>Add Users (comma-separated usernames)</label>
          <input
            type="text"
            value={usernames}
            onChange={(e) => setUsernames(e.target.value)}
            placeholder="user1, user2, user3"
          />
        </div>
        {error && <p className="error">{error}</p>}
        <button type="submit" disabled={loading}>
          {loading ? 'Creating...' : 'Create Chat'}
        </button>
      </form>
    </div>
  );
}

export default CreateChat;
