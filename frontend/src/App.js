import './App.css';
import { useState } from 'react';
import Register from './Register';
import Login from './Login';
import Chat from './Chat';

function App() {
  // 'register' | 'login' | 'chat'
  const [view, setView] = useState('register');
  const [currentUser, setCurrentUser] = useState(null);

  const handleLogin = (user) => {
    setCurrentUser(user);
    setView('chat');
  };

  const handleLogout = () => {
    setCurrentUser(null);
    setView('login');
  };

  if (view === 'register') {
    return <Register onGoToLogin={() => setView('login')} />;
  }

  if (view === 'login') {
    return <Login onLogin={handleLogin} onGoToRegister={() => setView('register')} />;
  }

  return <Chat currentUser={currentUser} onLogout={handleLogout} />;
}

export default App;
