import { useState, useEffect } from 'react';
import SignUp from './components/SignUp';
import Login from './components/Login';
import CreateChat from './components/CreateChat';
import ChatList from './components/ChatList';
import ChatView from './components/ChatView';
import './App.css';
import { api } from './api/api';

const USER_STORAGE_KEY = 'currentUser';
const DARK_THEME_KEY = 'darkTheme';

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

function App() {
  const [currentUser, setCurrentUser] = useState(() => readStoredUser());
  const [selectedChatId, setSelectedChatId] = useState(null);
  const [view, setView] = useState(() => (readStoredUser() ? 'chat' : 'login')); // 'signup', 'login', 'chat'
  const [darkTheme, setDarkTheme] = useState(() => readStoredTheme());

  const toggleTheme = () => {
    const newTheme = !darkTheme;
    setDarkTheme(newTheme);
    localStorage.setItem(DARK_THEME_KEY, newTheme.toString());
  };

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

  const handleLogout = () => {
    api.clearToken();
    localStorage.removeItem(USER_STORAGE_KEY);
    setCurrentUser(null);
    setSelectedChatId(null);
    setView('login');
  };

  const handleChatCreated = () => {
    // Trigger chat list refresh
    window.dispatchEvent(new CustomEvent('refreshChats'));
  };

  const handleSelectChat = (chatId) => {
    setSelectedChatId(chatId);
  };

  return (
    <div className="App">
      <header className="App-header">
        <h1>Chat App</h1>
        {currentUser && (
          <div className="user-info">
            <span>Logged in as: {currentUser.username}</span>
            <button onClick={handleLogout} className="logout-btn">
              Logout
            </button>
          </div>
        )}
        <button onClick={toggleTheme} className="theme-toggle-btn">
          {darkTheme ? '☀️' : '🌙'}
        </button>
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

        {currentUser && view === 'chat' && (
          <div className="chat-container">
            <aside className="chat-sidebar">
              <CreateChat currentUser={currentUser} onChatCreated={handleChatCreated} />
              <ChatList
                currentUser={currentUser}
                onSelectChat={handleSelectChat}
                selectedChatId={selectedChatId}
              />
            </aside>
            <section className="chat-main">
              <ChatView chatId={selectedChatId} currentUser={currentUser} />
            </section>
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
