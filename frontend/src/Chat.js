import { useState, useEffect, useRef } from 'react';

const API_URL = process.env.REACT_APP_API_URL || '';

function Chat({ currentUser, onLogout }) {
  // In-memory array of all users: [{ id, username }, ...]
  const [users, setUsers] = useState([]);
  const [selectedUser, setSelectedUser] = useState(null);
  const [messages, setMessages] = useState([]);
  const [messageText, setMessageText] = useState('');
  const [loadingUsers, setLoadingUsers] = useState(true);
  const [loadingMsgs, setLoadingMsgs] = useState(false);
  const [sendError, setSendError] = useState('');
  const [uploading, setUploading] = useState(false);
  const messagesEndRef = useRef(null);
  const fileInputRef = useRef(null);

  // Returns 'image', 'video', or null based on URL extension
  function getMediaType(content) {
    if (!content.startsWith('http')) return null;
    const lower = content.toLowerCase().split('?')[0];
    if (/\.(jpg|jpeg|png|gif|webp|bmp|svg)$/.test(lower)) return 'image';
    if (/\.(mp4|webm|mov|avi|mkv|ogg)$/.test(lower)) return 'video';
    return null;
  }

  // Fetch all users into RAM on mount
  useEffect(() => {
    fetch(`${API_URL}/all-users`)
      .then(r => r.json())
      .then(data => {
        // Filter out the current user from the list
        const others = data.filter(u => u.id !== currentUser.id);
        setUsers(others);
        setLoadingUsers(false);
      })
      .catch(() => setLoadingUsers(false));
  }, [currentUser.id]);

  // Fetch messages whenever selected user changes
  useEffect(() => {
    if (!selectedUser) return;
    loadMessages(selectedUser);
    // Poll every 3 seconds for new messages
    const interval = setInterval(() => loadMessages(selectedUser), 3000);
    return () => clearInterval(interval);
  }, [selectedUser]);

  // Scroll to bottom when messages update
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  async function loadMessages(otherUser) {
    setLoadingMsgs(true);
    try {
      // Fetch both directions and merge
      const [sent, received] = await Promise.all([
        fetch(`${API_URL}/get-messages?sender_id=${currentUser.id}&receiver_id=${otherUser.id}`).then(r => r.json()),
        fetch(`${API_URL}/get-messages?sender_id=${otherUser.id}&receiver_id=${currentUser.id}`).then(r => r.json()),
      ]);
      const all = [...(sent || []), ...(received || [])];
      all.sort((a, b) => new Date(a.sent_at) - new Date(b.sent_at));
      setMessages(all);
    } catch (e) {
      setMessages([]);
    } finally {
      setLoadingMsgs(false);
    }
  }

  async function handleSend(e) {
    e.preventDefault();
    if (!messageText.trim() || !selectedUser) return;
    setSendError('');
    try {
      const res = await fetch(`${API_URL}/messages`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          sender_id: currentUser.id,
          receiver_id: selectedUser.id,
          content: messageText.trim(),
        }),
      });
      const data = await res.json();
      if (data.res === 'success') {
        setMessageText('');
        loadMessages(selectedUser);
      } else {
        setSendError('Failed to send message.');
      }
    } catch {
      setSendError('Could not connect to server.');
    }
  }

  async function handleFileUpload(e) {
    const file = e.target.files[0];
    if (!file || !selectedUser) return;
    setUploading(true);
    setSendError('');
    try {
      const formData = new FormData();
      formData.append('file', file);
      const res = await fetch(`${API_URL}/upload`, { method: 'POST', body: formData });
      const data = await res.json();
      if (data.status && data.url) {
        const msgRes = await fetch(`${API_URL}/messages`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            sender_id: currentUser.id,
            receiver_id: selectedUser.id,
            content: data.url,
          }),
        });
        const msgData = await msgRes.json();
        if (msgData.res === 'success') {
          loadMessages(selectedUser);
        } else {
          setSendError('Failed to send media message.');
        }
      } else {
        setSendError('Upload failed.');
      }
    } catch {
      setSendError('Could not upload file.');
    } finally {
      setUploading(false);
      if (fileInputRef.current) fileInputRef.current.value = '';
    }
  }

  return (
    <div className="chat-layout">
      <div className="chat-sidebar">
        <div className="sidebar-header">
          <span className="sidebar-title">BensChat</span>
          <button className="logout-btn" onClick={onLogout}>Logout</button>
        </div>
        <div className="current-user-tag">Logged in as <strong>{currentUser.username}</strong></div>
        <div className="users-label">Users</div>
        {loadingUsers ? (
          <div className="sidebar-loading">Loading users...</div>
        ) : users.length === 0 ? (
          <div className="sidebar-loading">No other users yet.</div>
        ) : (
          <ul className="user-list">
            {users.map(u => (
              <li
                key={u.id}
                className={`user-item${selectedUser?.id === u.id ? ' selected' : ''}`}
                onClick={() => { setSelectedUser(u); setMessages([]); }}
              >
                <div className="user-avatar">{u.username[0].toUpperCase()}</div>
                <span>{u.username}</span>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="chat-main">
        {!selectedUser ? (
          <div className="chat-placeholder">
            <p>Select a user to start chatting</p>
          </div>
        ) : (
          <>
            <div className="chat-header">
              <div className="user-avatar">{selectedUser.username[0].toUpperCase()}</div>
              <span>{selectedUser.username}</span>
            </div>
            <div className="messages-area">
              {loadingMsgs && messages.length === 0 && <div className="msg-loading">Loading messages...</div>}
              {messages.map(msg => {
                const mediaType = getMediaType(msg.content);
                return (
                  <div
                    key={msg.id}
                    className={`message-bubble ${msg.sender_id === currentUser.id ? 'sent' : 'received'}`}
                  >
                    {mediaType === 'image' ? (
                      <img className="msg-media" src={msg.content} alt="media" />
                    ) : mediaType === 'video' ? (
                      <video className="msg-media" src={msg.content} controls />
                    ) : (
                      <span className="msg-content">{msg.content}</span>
                    )}
                    <span className="msg-time">{msg.sent_at ? new Date(msg.sent_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : ''}</span>
                  </div>
                );
              })}
              <div ref={messagesEndRef} />
            </div>
            <form className="message-input-row" onSubmit={handleSend}>
              <input
                ref={fileInputRef}
                type="file"
                accept="image/*,video/*"
                style={{ display: 'none' }}
                onChange={handleFileUpload}
              />
              <button
                type="button"
                className="upload-btn"
                onClick={() => fileInputRef.current?.click()}
                disabled={uploading}
                title="Upload image or video"
              >
                {uploading ? '⏳' : '📎'}
              </button>
              <input
                className="message-input"
                value={messageText}
                onChange={e => setMessageText(e.target.value)}
                placeholder={`Message ${selectedUser.username}...`}
                autoComplete="off"
                disabled={uploading}
              />
              <button className="send-btn" type="submit" disabled={uploading}>Send</button>
            </form>
            {sendError && <p className="auth-error" style={{ padding: '0 1rem' }}>{sendError}</p>}
          </>
        )}
      </div>
    </div>
  );
}

export default Chat;

