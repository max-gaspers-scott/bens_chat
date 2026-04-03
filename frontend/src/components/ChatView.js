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
  const messagesEndRef = useRef(null);

  const loadMessages = useCallback(async () => {
    try {
      const result = await api.getMessages(chatId);
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
  }, [chatId]);

  useEffect(() => {
    if (!chatId) {
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
  }, [chatId, loadMessages]);

  const handleRefresh = () => {
    loadMessages();
  };

  // Scroll to bottom when messages change
  useEffect(() => {
    if (messagesEndRef.current) {
      messagesEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages]);

  if (!chatId) {
    return <div className="chat-view empty">Select a chat to view messages</div>;
  }

  return (
    <div className="chat-view">
      <div className="chat-header">
        <h3>Chat</h3>
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
              className={`message ${msg.sender_id === currentUser.user_id ? 'own' : 'other'}`}
            >
              {msg.content && msg.content.trim() && (
                <div className="message-content">{msg.content}</div>
              )}
              {msg.minio_url && <ChatImage objectKey={msg.minio_url} />}
              <div className="message-meta">
                <span className="message-sender">
                  {msg.sender_id === currentUser.user_id ? 'You' : 'Other'}
                </span>
                <span className="message-time">
                  {msg.sent_at ? new Date(msg.sent_at).toLocaleString() : ''}
                </span>
              </div>
            </div>
          ))
        )}
        <div ref={messagesEndRef} />
      </div>
      <SendMessage chatId={chatId} senderId={currentUser.user_id} onMessageSent={handleRefresh} />
    </div>
  );
}

export default ChatView;
