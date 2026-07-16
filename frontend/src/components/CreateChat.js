import { useState } from 'react';
import { api } from '../api/api';

function CreateChat({ currentUser, onChatCreated }) {
  const [chatName, setChatName] = useState('');
  const [participants, setParticipants] = useState(['']);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const handleAddParticipant = () => {
    setParticipants([...participants, '']);
  };

  const handleRemoveParticipant = (index) => {
    if (participants.length > 1) {
      const newParticipants = participants.filter((_, i) => i !== index);
      setParticipants(newParticipants);
    }
  };

  const handleParticipantChange = (index, value) => {
    const newParticipants = [...participants];
    newParticipants[index] = value;
    setParticipants(newParticipants);
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      // Step 1: Create the chat (posts a root message; the creator is added as a
      // participant by the backend automatically).
      const chatResult = await api.createChat({
        sender_name: currentUser.username,
        title: chatName,
      });
      if (chatResult.res !== 'success') {
        throw new Error('Failed to create chat');
      }
      const chat_id = chatResult.data.message_id;

      // Step 2: Link the other participants to the chat by username.
      const usernameList = participants
        .map((u) => u.trim())
        .filter((u) => u.length > 0);

      for (const username of usernameList) {
        const linkResult = await api.linkUserToChat(username, chat_id);
        if (linkResult.res !== 'success') {
          setError(`User "${username}" not found, but chat was created.`);
        }
      }

      // Success - reset form and notify parent
      setChatName('');
      setParticipants(['']);
      onChatCreated(chat_id);
    } catch (err) {
      setError(err.message || 'Failed to create chat');
    } finally {
      setLoading(false);
    }
  };

  const hasEmptyParticipants = participants.some((p) => p.trim().length > 0);
  const canSubmit = chatName.trim() && hasEmptyParticipants && !loading;

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
            placeholder="Enter chat name"
            required
          />
        </div>
        <div className="form-group">
          <label>Participants</label>
          <div className="participants-list">
            {participants.map((participant, index) => (
              <div key={index} className="participant-input">
                <input
                  type="text"
                  value={participant}
                  onChange={(e) => handleParticipantChange(index, e.target.value)}
                  placeholder={`Username ${index + 1}`}
                />
                {participants.length > 1 && (
                  <button
                    type="button"
                    className="remove-participant-btn"
                    onClick={() => handleRemoveParticipant(index)}
                    title="Remove participant"
                  >
                    ✕
                  </button>
                )}
              </div>
            ))}
          </div>
          <button
            type="button"
            className="add-participant-btn"
            onClick={handleAddParticipant}
          >
            + Add Another Participant
          </button>
        </div>
        {error && <p className="error">{error}</p>}
        <button type="submit" disabled={!canSubmit}>
          {loading ? 'Creating...' : 'Create Chat'}
        </button>
      </form>
    </div>
  );
}

export default CreateChat;
