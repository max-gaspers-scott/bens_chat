const API_BASE_URL = process.env.REACT_APP_API_URL || 'http://localhost:9821';

export const api = {
  // Health check
  async health() {
    const response = await fetch(`${API_BASE_URL}/health`);
    return response.text();
  },

  // Sign up - create a new user
  async signUp({ username, email, password_hash, phone_number }) {
    const response = await fetch(`${API_BASE_URL}/users`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        user_id: crypto.randomUUID(),
        username,
        email,
        password_hash,
        phone_number: phone_number || null,
        created_at: new Date().toISOString(),
      }),
    });
    return response.json();
  },

  // Login - get user by username
  async login(username) {
    const response = await fetch(`${API_BASE_URL}/users?username=${username}`);
    return response.json();
  },

  // Create a new chat
  async createChat(chat_name) {
    const response = await fetch(`${API_BASE_URL}/chats`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        chat_id: crypto.randomUUID(),
        chat_name,
        created_at: new Date().toISOString(),
      }),
    });
    return response.json();
  },

  // Link a user to a chat
  async linkUserToChat(user_id, chat_id) {
    const response = await fetch(`${API_BASE_URL}/user-chats`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ user_id, chat_id }),
    });
    return response.json();
  },

  // Get user's chats
  async getUserChats(user_id) {
    const response = await fetch(`${API_BASE_URL}/user-chats?user_id=${user_id}`);
    return response.json();
  },

  // Get messages for a chat
  async getMessages(chat_id) {
    const response = await fetch(`${API_BASE_URL}/messages?chat_id=${chat_id}`);
    return response.json();
  },

  // Send a message
  async sendMessage({ chat_id, sender_id, content }) {
    const response = await fetch(`${API_BASE_URL}/messages`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        message_id: crypto.randomUUID(),
        chat_id,
        sender_id,
        content,
        sent_at: new Date().toISOString(),
      }),
    });
    return response.json();
  },

  // Get user by username (for looking up users to add to chat)
  async getUserByUsername(username) {
    const response = await fetch(`${API_BASE_URL}/users?username=${username}`);
    return response.json();
  },
};
