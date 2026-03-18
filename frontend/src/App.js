import { useState } from 'react';
import SignUp from './components/SignUp';
import Login from './components/Login';
import CreateChat from './components/CreateChat';
import ChatList from './components/ChatList';
import ChatView from './components/ChatView';
import './App.css';

function App() {
  const [currentUser, setCurrentUser] = useState(null);
  const [selectedChatId, setSelectedChatId] = useState(null);
  const [view, setView] = useState('login'); // 'signup', 'login', 'chat'

  const handleSignUpSuccess = () => {
    setView('login');
  };

  const handleLoginSuccess = (user) => {
    setCurrentUser(user);
    setView('chat');
  };

  const handleLogout = () => {
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
