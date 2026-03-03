import { useState } from 'react';

const API_URL = process.env.REACT_APP_API_URL || '';

async function hashPassword(password) {
  const encoder = new TextEncoder();
  const data = encoder.encode(password);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  return Array.from(new Uint8Array(hashBuffer))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

function Login({ onLogin, onGoToRegister }) {
  const [form, setForm] = useState({ username: '', password: '' });
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const handleChange = e => setForm({ ...form, [e.target.name]: e.target.value });

  const handleSubmit = async e => {
    e.preventDefault();
    setError('');
    if (!form.username || !form.password) {
      setError('Username and password are required.');
      return;
    }
    setLoading(true);
    try {
      const res = await fetch(`${API_URL}/get_user_id?username=${encodeURIComponent(form.username)}`);
      const users = await res.json();

      if (!users || users.length === 0) {
        setError('User not found.');
        setLoading(false);
        return;
      }

      const user = users[0];
      const inputHash = await hashPassword(form.password);

      if (inputHash !== user.password_hash) {
        setError('Incorrect password.');
        setLoading(false);
        return;
      }

      onLogin({ id: user.id, username: user.username, email: user.email });
    } catch (err) {
      setError('Could not connect to server.');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="auth-container">
      <div className="auth-card">
        <h2>Welcome Back</h2>
        <form onSubmit={handleSubmit}>
          <label>Username</label>
          <input
            name="username"
            value={form.username}
            onChange={handleChange}
            placeholder="Your username"
            autoComplete="username"
          />
          <label>Password</label>
          <input
            name="password"
            type="password"
            value={form.password}
            onChange={handleChange}
            placeholder="Password"
            autoComplete="current-password"
          />
          {error && <p className="auth-error">{error}</p>}
          <button className="auth-btn" type="submit" disabled={loading}>
            {loading ? 'Logging in...' : 'Log In'}
          </button>
        </form>
        <p className="auth-switch">
          No account yet?{' '}
          <span onClick={onGoToRegister}>Register</span>
        </p>
      </div>
    </div>
  );
}

export default Login;

