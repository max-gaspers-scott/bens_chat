import { useState, useEffect } from 'react';
import { io } from 'socket.io-client';
import './WebSocketTest.css';

const resolveWsUrl = () => {
  if (process.env.REACT_APP_API_URL) {
    return process.env.REACT_APP_API_URL;
  }
  if (typeof window !== 'undefined') {
    const isLocalCraDevServer =
      (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1') &&
      window.location.port === '3000';

    if (isLocalCraDevServer) {
      return 'http://localhost:9821';
    }
  }
  return '';
};

export default function WebSocketTest() {
  const [socket, setSocket] = useState(null);
  const [connected, setConnected] = useState(false);
  const [inputMessage, setInputMessage] = useState('');
  const [logs, setLogs] = useState([]);
  const [isOpen, setIsOpen] = useState(false);
  const [notificationsEnabled, setNotificationsEnabled] = useState(false);

  useEffect(() => {
    if ('Notification' in window) {
      if (Notification.permission === 'granted') {
        setNotificationsEnabled(true);
      }
    }
  }, []);

  const requestNotificationPermission = async () => {
    if (!('Notification' in window)) {
      alert('This browser does not support desktop notifications');
      return;
    }
    const permission = await Notification.requestPermission();
    if (permission === 'granted') {
      setNotificationsEnabled(true);
      new Notification('WebSocket Test', {
        body: 'Push notifications are now enabled!',
      });
    }
  };

  useEffect(() => {
    const wsUrl = resolveWsUrl();
    // Connect to namespace "/"
    const s = io(wsUrl || undefined, {
      path: '/socket.io',
      transports: ['websocket', 'polling'],
    });

    s.on('connect', () => {
      setConnected(true);
      setLogs((prev) => [...prev, { type: 'system', text: `Connected to socket id: ${s.id}` }]);
    });

    s.on('disconnect', (reason) => {
      setConnected(false);
      setLogs((prev) => [...prev, { type: 'system', text: `Disconnected: ${reason}` }]);
    });

    s.on('connect_error', (err) => {
      setConnected(false);
      setLogs((prev) => [...prev, { type: 'error', text: `Connection error: ${err.message}` }]);
    });

    // Listen to "message back" event as requested
    s.on('message back', (data) => {
      const displayData = typeof data === 'object' ? JSON.stringify(data) : data;
      setLogs((prev) => [...prev, { type: 'received', text: displayData }]);

      if ('Notification' in window && Notification.permission === 'granted') {
        new Notification('WebSocket Message Received', {
          body: displayData,
          icon: '/favicon.ico',
        });
      }
    });

    setSocket(s);

    return () => {
      s.disconnect();
    };
  }, []);

  const handleSendMessage = (e) => {
    e.preventDefault();
    if (!socket || !connected || !inputMessage.trim()) return;

    const payload = inputMessage.trim();
    // Send event "message" with payload
    socket.emit('message', payload);
    setLogs((prev) => [...prev, { type: 'sent', text: payload }]);
    setInputMessage('');
  };

  return (
    <div className="websocket-test-widget">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className={`ws-toggle-btn ${connected ? 'connected' : 'disconnected'}`}
      >
        🔌 WebSocket Test {connected ? '🟢' : '🔴'}
      </button>

      {isOpen && (
        <div className="ws-modal">
          <div className="ws-header">
            <h3>WebSocket Test Panel (Socket.io)</h3>
            <button onClick={() => setIsOpen(false)} className="ws-close-btn">×</button>
          </div>
          <div className="ws-status-bar">
            Status: <span className={connected ? 'text-success' : 'text-danger'}>
              {connected ? 'Connected' : 'Disconnected'}
            </span>
            {!notificationsEnabled && (
              <button onClick={requestNotificationPermission} className="ws-notif-btn">
                🔔 Enable Notifications
              </button>
            )}
          </div>

          <div className="ws-logs">
            {logs.length === 0 ? (
              <p className="ws-no-logs">No messages yet. Send a message to test!</p>
            ) : (
              logs.map((log, idx) => (
                <div key={idx} className={`ws-log-item ${log.type}`}>
                  <span className="ws-log-badge">
                    {log.type === 'sent' && 'Sent:'}
                    {log.type === 'received' && 'Received ("message back"):'}
                    {log.type === 'system' && 'System:'}
                    {log.type === 'error' && 'Error:'}
                  </span>
                  <span className="ws-log-text">{log.text}</span>
                </div>
              ))
            )}
          </div>

          <form onSubmit={handleSendMessage} className="ws-form">
            <input
              type="text"
              value={inputMessage}
              onChange={(e) => setInputMessage(e.target.value)}
              placeholder="Type message to send ('message' event)..."
              disabled={!connected}
            />
            <button type="submit" disabled={!connected || !inputMessage.trim()}>
              Send Message
            </button>
          </form>
        </div>
      )}
    </div>
  );
}
