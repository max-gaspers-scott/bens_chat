import { useEffect, useState, useCallback, memo, useRef } from 'react';
import { api } from '../api/api';
import SendMessage from './SendMessage';

// Shows a placeholder shimmer while the signed URL is fetched and the image loads.
const ChatImage = memo(function ChatImage({ objectKey }) {
  const [src, setSrc] = useState(null);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    api.getImageUrl(objectKey).then(setSrc).catch(console.error);
  }, [objectKey]);

  return (
    <div className="message-image-wrapper">
      {!loaded && <div className="image-placeholder" />}
      {src && (
        <img
          src={src}
          alt="attachment"
          className={`message-image${loaded ? ' loaded' : ''}`}
          onLoad={() => setLoaded(true)}
          onError={() => setLoaded(true)}
        />
      )}
    </div>
  );
});

function ChatView({ chatId, currentUser }) {
  const [messages, setMessages] = useState([]);
  const [loading, setLoading] = useState(true);
  // Navigation stack of parents. The first entry is the chat's root message;
  // each additional entry is a message the user opened as a sub-chat.
  const [parentStack, setParentStack] = useState(() =>
    chatId ? [{ id: chatId, label: 'Chat' }] : []
  );
  const messagesEndRef = useRef(null);

  const currentParent = parentStack.length ? parentStack[parentStack.length - 1] : null;
  const currentParentId = currentParent ? currentParent.id : null;

  // Reset the navigation stack whenever the selected chat changes.
  useEffect(() => {
    setParentStack(chatId ? [{ id: chatId, label: 'Chat' }] : []);
  }, [chatId]);

  const loadMessages = useCallback(async () => {
    if (!currentParentId) {
      return;
    }
    try {
      const result = await api.getMessages(currentParentId);
      if (result.status === 'success' && result.payload) {
        setMessages((prevMessages) => {
          // Only update if messages have actually changed
          const prevIds = prevMessages.map((m) => m.message_id).join(',');
          const newIds = result.payload.map((m) => m.message_id).join(',');
          if (prevIds === newIds) {
            return prevMessages;
          }
          return result.payload;
        });
      } else {
        setMessages([]);
      }
    } catch (err) {
      console.error('Failed to load messages:', err);
      setMessages([]);
    } finally {
      setLoading(false);
    }
  }, [currentParentId]);

  // Clear messages and show the loading state when navigating to a new parent.
  useEffect(() => {
    setMessages([]);
    setLoading(true);
  }, [currentParentId]);

  useEffect(() => {
    if (!currentParentId) {
      return;
    }

    loadMessages();

    const intervalId = setInterval(() => {
      loadMessages();
    }, 3000);

    return () => {
      if (intervalId) {
        clearInterval(intervalId);
      }
    };
  }, [currentParentId, loadMessages]);

  const handleRefresh = () => {
    loadMessages();
  };

  // Open the children of a message as a nested sub-chat.
  const openSubChat = (msg) => {
    const text = msg.content && msg.content.text ? msg.content.text.trim() : '';
    const label = text ? text.slice(0, 30) : 'Sub-chat';
    setParentStack((stack) => [...stack, { id: msg.message_id, label }]);
  };

  // Navigate back up the sub-chat stack to the given depth.
  const goToParent = (index) => {
    setParentStack((stack) => stack.slice(0, index + 1));
  };

  // Scroll to bottom when messages change
  useEffect(() => {
    if (messagesEndRef.current) {
      messagesEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages]);

  if (!chatId) {
    return (
      <div className="chat-view empty">
        <p>Select a chat to view messages</p>
        <p className="hint">You can create a chat by typing participants' usernames and a chat name above.</p>
      </div>
    );
  }

  return (
    <div className="chat-view">
      <div className="chat-header">
        {parentStack.length > 1 ? (
          <div className="subchat-nav">
            <button
              type="button"
              className="link-btn"
              onClick={() => goToParent(parentStack.length - 2)}
            >
              ← Back
            </button>
            <h3>{currentParent.label}</h3>
          </div>
        ) : (
          <h3>Chat</h3>
        )}
      </div>
      <div className="messages-container">
        {loading ? (
          <p>Loading messages...</p>
        ) : messages.length === 0 ? (
          <p className="no-messages">No messages yet. Be the first to send one!</p>
        ) : (
          messages.map((msg, index) => (
            <div
              key={msg.message_id || index}
              className={`message ${msg.sender_name === currentUser.username ? 'own' : 'other'}`}
            >
              {msg.content && msg.content.text && msg.content.text.trim() && (
                <div className="message-content">{msg.content.text}</div>
              )}
              {msg.content && msg.content.url && <ChatImage objectKey={msg.content.url} />}
              <div className="message-meta">
                <span className="message-sender">
                  {msg.sender_name === currentUser.username ? 'You' : msg.sender_name}
                </span>
                <span className="message-time">
                  {msg.sent_at ? new Date(msg.sent_at).toLocaleString() : ''}
                </span>
              </div>
              {msg.message_id && (
                <button
                  type="button"
                  className="link-btn subchat-btn"
                  onClick={() => openSubChat(msg)}
                  title="Open a sub-chat from this message"
                >
                  💬 Open sub-chat
                </button>
              )}
            </div>
          ))
        )}
        <div ref={messagesEndRef} />
      </div>
      <SendMessage chatId={currentParentId} senderName={currentUser.username} onMessageSent={handleRefresh} />
    </div>
  );
}

export default ChatView;
