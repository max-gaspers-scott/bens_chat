import { useEffect, useState, useCallback, useMemo, memo, useRef } from 'react';
import { api } from '../api/api';
import { buildTree, countDescendants } from '../utils/buildTree';
import SendMessage from './SendMessage';
import Connect4Board from './Connect4Board';

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

function StartDirectChatForm({ currentUser, onSelectChat }) {
  const [username, setUsername] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e) => {
    e.preventDefault();
    const otherUsername = username.trim();
    if (!otherUsername) return;

    if (otherUsername.toLowerCase() === currentUser.username.toLowerCase()) {
      setError("You cannot create a 1-on-1 chat with yourself.");
      return;
    }

    setError('');
    setLoading(true);

    try {
      // Auto-generate chat title
      const chatTitle = `${currentUser.username} & ${otherUsername}`;

      // Create Chat
      const chatResult = await api.createChat({
        sender_name: currentUser.username,
        title: chatTitle,
      });

      if (chatResult.res !== 'success') {
        throw new Error('Failed to create chat');
      }
      const chat_id = chatResult.data.message_id;

      // Link the other participant
      const linkResult = await api.linkUserToChat(otherUsername, chat_id);
      if (linkResult.res !== 'success') {
        throw new Error(`User "${otherUsername}" not found.`);
      }

      // Success
      setUsername('');
      // Trigger chat list refresh
      window.dispatchEvent(new CustomEvent('refreshChats'));
      if (onSelectChat) {
        onSelectChat(chat_id);
      }
    } catch (err) {
      setError(err.message || 'Failed to start chat');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="quick-chat-container">
      <h4>Or Start a 1-on-1 Chat</h4>
      <form onSubmit={handleSubmit} className="quick-chat-form">
        <div className="quick-chat-input-group">
          <input
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            placeholder="Enter friend's username"
            disabled={loading}
            required
          />
          <button type="submit" disabled={loading || !username.trim()}>
            {loading ? 'Starting...' : 'Start Chat'}
          </button>
        </div>
        {error && <p className="error quick-chat-error">{error}</p>}
      </form>
    </div>
  );
}

// How many levels of replies are shown inline, relative to the current view root.
// Replies deeper than this are reached through "continue this thread".
export const MAX_DEPTH = 5;

// Short plain-text description of a message, for headers and the reply indicator.
export function messageSnippet(msg, maxLength = 50) {
  const content = (msg && msg.content) || {};
  let text = (content.title || content.text || '').trim();
  if (!text) {
    if (Array.isArray(content.grid)) text = `Connect 4: ${content.name || 'game'}`;
    else if (content.url) text = 'Image';
  }
  return text.length > maxLength ? `${text.slice(0, maxLength)}…` : text;
}

// The body of a single message: Connect4 board, text/title and/or image.
function MessageBody({ msg, currentUser, onMoveSent }) {
  const content = msg.content || {};
  if (Array.isArray(content.grid)) {
    return <Connect4Board msg={msg} currentUser={currentUser} onMoveSent={onMoveSent} />;
  }
  const text = content.title || content.text;
  return (
    <>
      {text && text.trim() && <div className="message-content">{text}</div>}
      {content.url && <ChatImage objectKey={content.url} />}
    </>
  );
}

function MessageMeta({ msg, currentUser }) {
  return (
    <div className="message-meta">
      <span className="message-sender">
        {msg.sender_name === currentUser.username ? 'You' : msg.sender_name}
      </span>
      <span className="message-time">
        {msg.sent_at ? new Date(msg.sent_at).toLocaleString() : ''}
      </span>
    </div>
  );
}

// One message plus its replies, rendered recursively up to MAX_DEPTH.
const ThreadNode = memo(function ThreadNode({
  node,
  depth,
  currentUser,
  replyToId,
  onReply,
  onOpenThread,
  onMoveSent,
}) {
  const { msg, children } = node;
  const isConnect4 = msg.content && Array.isArray(msg.content.grid);
  const atMaxDepth = depth >= MAX_DEPTH;
  const hiddenReplies = atMaxDepth ? countDescendants(node) : 0;

  return (
    <div className="thread-node" data-message-id={msg.message_id}>
      <div
        className={`message ${msg.sender_name === currentUser.username ? 'own' : 'other'}${
          isConnect4 ? ' connect4-message' : ''
        }${replyToId === msg.message_id ? ' reply-target' : ''}`}
      >
        <MessageBody msg={msg} currentUser={currentUser} onMoveSent={onMoveSent} />
        <MessageMeta msg={msg} currentUser={currentUser} />
        <div className="message-actions">
          <button type="button" className="link-btn message-action-btn" onClick={() => onReply(msg)}>
            Reply
          </button>
          {hiddenReplies > 0 ? (
            <button
              type="button"
              className="link-btn message-action-btn continue-thread-btn"
              onClick={() => onOpenThread(msg.message_id)}
            >
              continue this thread → ({hiddenReplies} more {hiddenReplies === 1 ? 'reply' : 'replies'})
            </button>
          ) : (
            <button
              type="button"
              className="link-btn message-action-btn"
              onClick={() => onOpenThread(msg.message_id)}
              title="Show this message and its replies on their own"
            >
              Open thread
            </button>
          )}
        </div>
      </div>
      {!atMaxDepth && children.length > 0 && (
        <div className="thread-children">
          {children.map((child) => (
            <ThreadNode
              key={child.msg.message_id}
              node={child}
              depth={depth + 1}
              currentUser={currentUser}
              replyToId={replyToId}
              onReply={onReply}
              onOpenThread={onOpenThread}
              onMoveSent={onMoveSent}
            />
          ))}
        </div>
      )}
    </div>
  );
});

function ChatView({ chatId, currentUser, onSelectChat, onToggleSidebar, sidebarOpen }) {
  // Flat list from the backend: the view root plus all of its descendants.
  const [messages, setMessages] = useState([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState('');
  // The message shown at the top of the view. Starts at the chat's true root
  // (chatId, as opened from the chat list) and changes as the user navigates.
  const [viewRootId, setViewRootId] = useState(chatId);
  // Message the composer replies to; null means "reply to the view root".
  const [replyTo, setReplyTo] = useState(null);
  const containerRef = useRef(null);
  const prevIdsRef = useRef(null);
  // Lets a slow response for a previous view be discarded.
  const viewRootIdRef = useRef(viewRootId);
  viewRootIdRef.current = viewRootId;

  // Reset to the chat root whenever the selected chat changes.
  useEffect(() => {
    setViewRootId(chatId);
  }, [chatId]);

  const loadMessages = useCallback(async () => {
    if (!viewRootId) {
      return;
    }
    const requestedId = viewRootId;
    try {
      const result = await api.getMessages(requestedId);
      if (viewRootIdRef.current !== requestedId) return;
      if (result.status === 'success' && result.payload) {
        setLoadError('');
        setMessages((prevMessages) => {
          // Re-render if anything changed — IDs, order, or content (e.g. a Connect4 move).
          if (JSON.stringify(prevMessages) === JSON.stringify(result.payload)) {
            return prevMessages;
          }
          return result.payload;
        });
      } else {
        setLoadError(result.error || 'Failed to load messages');
        setMessages([]);
      }
    } catch (err) {
      if (viewRootIdRef.current !== requestedId) return;
      console.error('Failed to load messages:', err);
      setMessages([]);
    } finally {
      if (viewRootIdRef.current === requestedId) setLoading(false);
    }
  }, [viewRootId]);

  // Clear messages, the reply target and show the loading state when the view root changes.
  useEffect(() => {
    setMessages([]);
    setLoadError('');
    setReplyTo(null);
    setLoading(true);
    prevIdsRef.current = null;
  }, [viewRootId]);

  useEffect(() => {
    if (!viewRootId) {
      return;
    }

    loadMessages();

    // Live updates: the socket payload is { res, data: message }. Attach the new
    // message under its parent if the parent is in this view; ignore it otherwise.
    const handleRefreshMessages = (event) => {
      const incoming = event && event.detail && event.detail.data;
      if (!incoming || !incoming.message_id || incoming.parent_id === undefined) {
        loadMessages();
        return;
      }
      setMessages((prev) => {
        if (prev.some((m) => m.message_id === incoming.message_id)) return prev;
        if (!prev.some((m) => m.message_id === incoming.parent_id)) return prev;
        return [...prev, incoming];
      });
    };
    window.addEventListener('refreshMessages', handleRefreshMessages);

    const intervalId = setInterval(() => {
      loadMessages();
    }, 3000);

    return () => {
      window.removeEventListener('refreshMessages', handleRefreshMessages);
      if (intervalId) {
        clearInterval(intervalId);
      }
    };
  }, [viewRootId, loadMessages]);

  const handleRefresh = useCallback(() => {
    loadMessages();
  }, [loadMessages]);

  const tree = useMemo(() => buildTree(messages, viewRootId), [messages, viewRootId]);
  const rootMsg = tree ? tree.msg : null;
  const isChatRoot = viewRootId === chatId || (rootMsg && rootMsg.parent_id == null);

  const openThread = useCallback((messageId) => {
    if (messageId) setViewRootId(messageId);
  }, []);

  const handleReply = useCallback((msg) => {
    setReplyTo(msg);
  }, []);

  const handleMessageSent = () => {
    setReplyTo(null);
    loadMessages();
  };

  // Scroll to the bottom when a view first loads, and bring newly arrived
  // messages into view afterwards (not when existing messages update, e.g. Connect4 moves).
  useEffect(() => {
    const container = containerRef.current;
    if (!container || loading) return;

    const ids = new Set(messages.map((m) => m.message_id));
    const prevIds = prevIdsRef.current;
    prevIdsRef.current = ids;

    if (prevIds === null) {
      if (typeof container.scrollTo === 'function') {
        container.scrollTo({ top: container.scrollHeight });
      }
      return;
    }

    const added = messages.filter((m) => !prevIds.has(m.message_id));
    if (added.length === 0) return;
    const newest = added[added.length - 1];
    const el = container.querySelector(`[data-message-id="${newest.message_id}"]`);
    if (el) el.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }, [messages, loading]);

  if (!chatId) {
    return (
      <div className="chat-view empty">
        <p>Select a chat to view messages</p>
        <p className="hint">You can create a chat by typing participants' usernames and a chat name above.</p>
        <StartDirectChatForm currentUser={currentUser} onSelectChat={onSelectChat} />
      </div>
    );
  }

  const title = isChatRoot
    ? (rootMsg && rootMsg.content && rootMsg.content.title) || 'Chat'
    : 'Thread';
  const replies = tree ? tree.children : [];

  return (
    <div className="chat-view">
      <div className="chat-header">
        <div className="chat-header-main">
          <h3>{title}</h3>
          {onToggleSidebar && (
            <button
              type="button"
              className="toggle-sidebar-btn"
              onClick={onToggleSidebar}
              title="Toggle chats list"
              aria-label="Toggle chats list"
            >
              📋 {sidebarOpen ? 'Hide Chats' : 'Chats'}
            </button>
          )}
        </div>
        {!isChatRoot && (
          <div className="subchat-nav">
            {rootMsg && rootMsg.parent_id && (
              <button
                type="button"
                className="link-btn back-btn"
                onClick={() => openThread(rootMsg.parent_id)}
              >
                ↑ up one level
              </button>
            )}
            <button type="button" className="link-btn back-btn" onClick={() => openThread(chatId)}>
              back to full chat
            </button>
          </div>
        )}
      </div>
      <div className="messages-container" ref={containerRef}>
        {loading ? (
          <p>Loading messages...</p>
        ) : loadError ? (
          <p className="error">{loadError}</p>
        ) : (
          <>
            {!isChatRoot && rootMsg && (
              <div className="thread-root" data-message-id={rootMsg.message_id}>
                <div className="message other">
                  <MessageBody msg={rootMsg} currentUser={currentUser} onMoveSent={handleRefresh} />
                  <MessageMeta msg={rootMsg} currentUser={currentUser} />
                </div>
              </div>
            )}
            {replies.length === 0 ? (
              <p className="no-messages">No messages yet. Be the first to send one!</p>
            ) : (
              <div className="thread-list">
                {replies.map((child) => (
                  <ThreadNode
                    key={child.msg.message_id}
                    node={child}
                    depth={1}
                    currentUser={currentUser}
                    replyToId={replyTo ? replyTo.message_id : null}
                    onReply={handleReply}
                    onOpenThread={openThread}
                    onMoveSent={handleRefresh}
                  />
                ))}
              </div>
            )}
          </>
        )}
      </div>
      {replyTo && (
        <div className="reply-indicator">
          <span className="reply-indicator-text">
            Replying to <strong>{replyTo.sender_name === currentUser.username ? 'yourself' : replyTo.sender_name}</strong>
            {messageSnippet(replyTo) && `: ${messageSnippet(replyTo)}`}
          </span>
          <button
            type="button"
            className="reply-indicator-cancel"
            onClick={() => setReplyTo(null)}
            aria-label="Cancel reply"
            title="Cancel reply"
          >
            ×
          </button>
        </div>
      )}
      <SendMessage
        chatId={chatId}
        parentId={replyTo ? replyTo.message_id : viewRootId}
        senderName={currentUser.username}
        onMessageSent={handleMessageSent}
      />
    </div>
  );
}

export default ChatView;
