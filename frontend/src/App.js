import { io } from 'socket.io-client';
import { useState, useEffect, useCallback, useRef } from 'react';
import SignUp from './components/SignUp';
import Login from './components/Login';
import CreateChat from './components/CreateChat';
import ChatList from './components/ChatList';
import ChatView from './components/ChatView';
import ResetPassword from './components/ResetPassword';
import './App.css';
import { api, getSocketUrl } from './api/api';

const USER_STORAGE_KEY = 'currentUser';
const DARK_THEME_KEY = 'darkTheme';
const NOTIFICATIONS_ENABLED_KEY = 'notificationsEnabled';

function readStoredUser() {
  const token = api.getToken();
  const storedUser = localStorage.getItem(USER_STORAGE_KEY);

  if (!token || !storedUser) {
    return null;
  }

  try {
    return JSON.parse(storedUser);
  } catch (err) {
    localStorage.removeItem(USER_STORAGE_KEY);
    api.clearToken();
    return null;
  }
}

function readStoredTheme() {
  const stored = localStorage.getItem(DARK_THEME_KEY);
  return stored === 'true';
}

function readStoredNotificationsEnabled() {
  if (typeof window === 'undefined' || !('Notification' in window)) {
    return false;
  }
  return (
    localStorage.getItem(NOTIFICATIONS_ENABLED_KEY) === 'true' &&
    Notification.permission === 'granted'
  );
}

function App() {
  const [currentUser, setCurrentUser] = useState(() => readStoredUser());
  const [selectedChatId, setSelectedChatId] = useState(null);
  const [view, setView] = useState(() => (readStoredUser() ? 'chat' : 'login')); // 'signup', 'login', 'chat', 'reset-password'
  const [darkTheme, setDarkTheme] = useState(() => readStoredTheme());
  const [notificationsEnabled, setNotificationsEnabled] = useState(() =>
    readStoredNotificationsEnabled()
  );
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);
  const notificationsEnabledRef = useRef(notificationsEnabled);
  const mobileMenuRef = useRef(null);

  // Close mobile menu when clicking outside
  useEffect(() => {
    if (!mobileMenuOpen) return;
    const handleClickOutside = (e) => {
      if (mobileMenuRef.current && !mobileMenuRef.current.contains(e.target)) {
        setMobileMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('touchstart', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('touchstart', handleClickOutside);
    };
  }, [mobileMenuOpen]);

  const toggleTheme = () => {
    const newTheme = !darkTheme;
    setDarkTheme(newTheme);
    localStorage.setItem(DARK_THEME_KEY, newTheme.toString());
  };

  // Keep a ref in sync so the socket handler always sees the latest preference
  useEffect(() => {
    notificationsEnabledRef.current = notificationsEnabled;
  }, [notificationsEnabled]);

  const toggleNotifications = useCallback(async () => {
    if (notificationsEnabled) {
      setNotificationsEnabled(false);
      localStorage.setItem(NOTIFICATIONS_ENABLED_KEY, 'false');
      return;
    }

    if (!('Notification' in window)) {
      alert('This browser does not support desktop notifications.');
      return;
    }

    let permission = Notification.permission;
    if (permission !== 'granted') {
      permission = await Notification.requestPermission();
    }

    if (permission === 'granted') {
      setNotificationsEnabled(true);
      localStorage.setItem(NOTIFICATIONS_ENABLED_KEY, 'true');
    } else {
      alert('Notification permission was denied. Please enable it in your browser settings.');
    }
  }, [notificationsEnabled]);

  // Apply dark theme class to body
  useEffect(() => {
    if (darkTheme) {
      document.body.classList.add('dark-theme');
    } else {
      document.body.classList.remove('dark-theme');
    }
  }, [darkTheme]);

  const handleSignUpSuccess = () => {
    setView('login');
  };

  const handleLoginSuccess = (user) => {
    setCurrentUser(user);
    localStorage.setItem(USER_STORAGE_KEY, JSON.stringify(user));
    setView('chat');
  };

  const handleGoHome = () => {
    setSelectedChatId(null);
    setMobileSidebarOpen(false);
    setMobileMenuOpen(false);
    if (currentUser) {
      setView('chat');
    } else {
      setView('login');
    }
  };

  const handleLogout = useCallback(() => {
    api.clearToken();
    localStorage.removeItem(USER_STORAGE_KEY);
    setCurrentUser(null);
    setSelectedChatId(null);
    setMobileSidebarOpen(false);
    setMobileMenuOpen(false);
    setView('login');
  }, []);


  // Connect to Socket.io and join user chat rooms for real-time messages & notifications
  useEffect(() => {
    if (!currentUser) return;

    const socketUrl = getSocketUrl();

    const socket = io(socketUrl, {
      path: '/socket.io',
      transports: ['websocket', 'polling'],
    });

    socket.on('connect', () => {
      console.log('Socket connected:', socket.id);
      socket.emit('join_chats', currentUser.username);
    });

    socket.on('new_message', (data) => {
      console.log('New message received via socket:', data);
      window.dispatchEvent(new CustomEvent('refreshChats'));
      window.dispatchEvent(new CustomEvent('refreshMessages', { detail: data }));

      const message = data && data.data;
      if (
        notificationsEnabledRef.current &&
        'Notification' in window &&
        Notification.permission === 'granted' &&
        message &&
        message.sender_name !== currentUser.username
      ) {
        const content = message.content || {};
        const body = content.text || (content.url ? 'Sent an image' : 'You have a new message');
        new Notification(`New message from ${message.sender_name}`, {
          body,
          icon: '/favicon.ico',
        });
      }
    });

    return () => {
      socket.disconnect();
    };
  }, [currentUser]);

  useEffect(() => {
    api.registerUnauthorizedHandler(handleLogout);
    return () => {
      api.registerUnauthorizedHandler(null);
    };
  }, [handleLogout]);

  const handleChatCreated = (newChatId) => {
    // Trigger chat list refresh
    window.dispatchEvent(new CustomEvent('refreshChats'));
    if (newChatId) {
      setSelectedChatId(newChatId);
      setMobileSidebarOpen(false);
    }
  };

  const handleSelectChat = (chatId) => {
    setSelectedChatId(chatId);
    setMobileSidebarOpen(false);
  };

  return (
    <div className="App">
      <header className="App-header">
        <h1
          className="app-title"
          onClick={handleGoHome}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              handleGoHome();
            }
          }}
          title="Bens Chat"
        >
          Bens Chat
        </h1>
        <div className="header-actions" ref={mobileMenuRef}>
          {currentUser && (
            <div className={`user-info ${mobileMenuOpen ? 'mobile-menu-open' : ''}`}>
              <span className="user-greeting">
                Logged in as: <strong>{currentUser.username}</strong>
              </span>
              <button
                onClick={() => {
                  toggleNotifications();
                  setMobileMenuOpen(false);
                }}
                className="settings-btn"
                title={
                  notificationsEnabled
                    ? 'Disable push notifications'
                    : 'Enable push notifications'
                }
              >
                {notificationsEnabled ? '🔔 Notifications On' : '🔕 Notifications Off'}
              </button>
              <button
                onClick={() => {
                  setView('reset-password');
                  setMobileMenuOpen(false);
                }}
                className="settings-btn"
              >
                Change Password
              </button>
              <button
                onClick={() => {
                  handleLogout();
                  setMobileMenuOpen(false);
                }}
                className="logout-btn"
              >
                Logout
              </button>
            </div>
          )}
          <button
            onClick={toggleTheme}
            className="theme-toggle-btn"
            title="Toggle theme"
            aria-label="Toggle theme"
          >
            {darkTheme ? '☀️' : '🌙'}
          </button>
          {currentUser && (
            <button
              type="button"
              className="mobile-menu-toggle-btn"
              onClick={() => setMobileMenuOpen((prev) => !prev)}
              aria-label="Toggle user menu"
              title="User menu"
            >
              {mobileMenuOpen ? '✕' : '☰'}
            </button>
          )}
        </div>
      </header>

      <main className="App-main">
        {!currentUser && view === 'signup' && (
          <SignUp onSignUpSuccess={handleSignUpSuccess} />
        )}

        {!currentUser && view === 'login' && (
          <>
            <Login onLoginSuccess={handleLoginSuccess} />
            <p className="switch-view">
              Don't have an account?{' '}
              <button onClick={() => setView('signup')} className="link-btn">
                Sign Up
              </button>
            </p>
          </>
        )}

        {currentUser && view === 'reset-password' && (
          <ResetPassword onBack={() => setView('chat')} />
        )}

        {currentUser && view === 'chat' && (
          <div
            className={`chat-container ${!selectedChatId ? 'no-chat-selected' : ''} ${
              mobileSidebarOpen ? 'mobile-sidebar-open' : ''
            }`}
          >
            {mobileSidebarOpen && selectedChatId && (
              <div
                className="mobile-sidebar-backdrop"
                onClick={() => setMobileSidebarOpen(false)}
                aria-hidden="true"
              />
            )}
            <aside className={`chat-sidebar ${mobileSidebarOpen ? 'show-mobile' : ''}`}>
              {selectedChatId && (
                <div className="mobile-sidebar-header">
                  <span className="mobile-sidebar-title">Chats & Groups</span>
                  <button
                    type="button"
                    className="mobile-sidebar-close-btn"
                    onClick={() => setMobileSidebarOpen(false)}
                    aria-label="Close chat list"
                  >
                    ✕
                  </button>
                </div>
              )}
              <CreateChat currentUser={currentUser} onChatCreated={handleChatCreated} />
              <ChatList
                currentUser={currentUser}
                onSelectChat={handleSelectChat}
                selectedChatId={selectedChatId}
              />
            </aside>
            <section className="chat-main">
              <ChatView
                chatId={selectedChatId}
                currentUser={currentUser}
                onSelectChat={handleSelectChat}
                onToggleSidebar={() => setMobileSidebarOpen((prev) => !prev)}
                sidebarOpen={mobileSidebarOpen}
              />
            </section>
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
