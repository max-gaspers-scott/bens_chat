import { useState } from 'react';
import { sha256 } from 'js-sha256';

const API_URL = process.env.REACT_APP_API_URL || '';

function hashPassword(password) {
  return sha256(password);
}

function Register({ onGoToLogin }) {
  const [form, setForm] = useState({ username: '', email: '', password: '' });
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const [success, setSuccess] = useState(false);

  const handleChange = e => setForm({ ...form, [e.target.name]: e.target.value });

  const handleSubmit = async e => {
    e.preventDefault();
    setError('');
    if (!form.username || !form.email || !form.password) {
      setError('All fields are required.');
      return;
    }
    setLoading(true);
    try {
      const password_hash = await hashPassword(form.password);
      const res = await fetch(`${API_URL}/users`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          username: form.username,
          email: form.email,
          password_hash,
        }),
      });
      const data = await res.json();
      console.log(data);
      if (data.res === 'success') {
        setSuccess(true);
      } else {
        setError(data.res || 'Registration failed.');
      }
    } catch (err) {
      setError('Could not connect to server.');
    } finally {
      setLoading(false);
    }
  };

  if (success) {
    return (
      <div className="auth-container">
        <div className="auth-card">
          <h2>Account created!</h2>
          <p>You can now log in.</p>
          <button className="auth-btn" onClick={onGoToLogin}>Go to Login</button>
        </div>
      </div>
    );
  }

  return (
    <div className="auth-container">
      <div className="auth-card">
        <h2>Create Account</h2>
        <form onSubmit={handleSubmit}>
          <label>Username</label>
          <input
            name="username"
            value={form.username}
            onChange={handleChange}
            placeholder="Choose a username"
            autoComplete="username"
          />
          <label>Email</label>
          <input
            name="email"
            type="email"
            value={form.email}
            onChange={handleChange}
            placeholder="your@email.com"
            autoComplete="email"
          />
          <label>Password</label>
          <input
            name="password"
            type="password"
            value={form.password}
            onChange={handleChange}
            placeholder="Password"
            autoComplete="new-password"
          />
          {error && <p className="auth-error">{error}</p>}
          <button className="auth-btn" type="submit" disabled={loading}>
            {loading ? 'Creating account...' : 'Register'}
          </button>
        </form>
        <p className="auth-switch">
          Already have an account?{' '}
          <span onClick={onGoToLogin}>Log in</span>
        </p>
      </div>
    </div>
  );
}

export default Register;

