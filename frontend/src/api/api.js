// MinIO public URL (browser-accessible). Override with REACT_APP_MINIO_PUBLIC_URL if needed.
const MINIO_PUBLIC_URL =
  process.env.REACT_APP_MINIO_PUBLIC_URL || 'http://localhost:9000';

// Presigned PUT URLs from the backend contain the internal docker hostname "minio:9000".
// Rewrite them so the browser can reach MinIO directly on the mapped port.
const rewriteMinioHost = (url) =>
  url.replace(/^https?:\/\/minio:[0-9]+/, MINIO_PUBLIC_URL);

const resolveApiBaseUrl = () => {
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

const API_BASE_URL = resolveApiBaseUrl();

const TOKEN_STORAGE_KEY = 'authToken';

const jsonHeaders = () => ({ 'Content-Type': 'application/json' });

const authHeaders = () => {
  const token = localStorage.getItem(TOKEN_STORAGE_KEY);
  const headers = token
    ? { ...jsonHeaders(), Authorization: `Bearer ${token}` }
    : jsonHeaders();

  return headers;
};

export const api = {
  setToken(token) {
    localStorage.setItem(TOKEN_STORAGE_KEY, token);
  },

  getToken() {
    return localStorage.getItem(TOKEN_STORAGE_KEY);
  },

  clearToken() {
    localStorage.removeItem(TOKEN_STORAGE_KEY);
  },

  // Health check
  async health() {
    const response = await fetch(`${API_BASE_URL}/health`);
    return response.text();
  },

  // Sign up - create a new user
  async signUp({ username, email, password, phone_number }) {
    const response = await fetch(`${API_BASE_URL}/users`, {
      method: 'POST',
      headers: jsonHeaders(),
      body: JSON.stringify({
        username,
        email,
        password,
        phone_number: phone_number || null,
      }),
    });
    return response.json();
  },

  // Login - verify username/password and return a JWT
  async login(username, password) {
    const response = await fetch(`${API_BASE_URL}/auth/login`, {
      method: 'POST',
      headers: jsonHeaders(),
      body: JSON.stringify({ username, password }),
    });
    return response.json();
  },

  // Create a new chat
  async createChat(chat_name) {
    const response = await fetch(`${API_BASE_URL}/chats`, {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        chat_name,
      }),
    });
    return response.json();
  },

  // Link a user to a chat
  async linkUserToChat(user_id, chat_id) {
    const response = await fetch(`${API_BASE_URL}/user-chats`, {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({ user_id, chat_id }),
    });
    return response.json();
  },

  // Get user's chats
  async getUserChats(user_id) {
    const response = await fetch(`${API_BASE_URL}/user-chats?user_id=${user_id}`, {
      headers: authHeaders(),
    });
    return response.json();
  },

  // Get messages for a chat
  async getMessages(chat_id) {
    const response = await fetch(`${API_BASE_URL}/messages?chat_id=${chat_id}`, {
      headers: authHeaders(),
    });
    return response.json();
  },

  // Send a message (minio_url is optional — pass the object key if an image was uploaded)
  async sendMessage({ chat_id, content, minio_url }) {
    const response = await fetch(`${API_BASE_URL}/messages`, {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        chat_id,
        content,
        minio_url: minio_url || null,
      }),
    });
    return response.json();
  },

  // Get a presigned PUT URL + server-generated object key for uploading an image to MinIO
  async getUploadUrl(chatId, fileExtension) {
    const response = await fetch(
      `${API_BASE_URL}/minio-post?chat_id=${encodeURIComponent(chatId)}&file_extension=${encodeURIComponent(fileExtension)}`,
      { headers: authHeaders() }
    );
    return response.json();
  },

  // PUT a file directly to MinIO using a presigned URL
  async uploadFileToMinio(presignedUrl, file) {
    const url = rewriteMinioHost(presignedUrl);
    const response = await fetch(url, {
      method: 'PUT',
      body: file,
      headers: { 'Content-Type': file.type || 'application/octet-stream' },
    });
    if (!response.ok) throw new Error(`MinIO upload failed: ${response.status}`);
  },

  // Get a presigned GET URL for displaying a stored image
  async getImageUrl(objectKey) {
    const response = await fetch(
      `${API_BASE_URL}/minio-fetch?object_key=${encodeURIComponent(objectKey)}`,
      { headers: authHeaders() }
    );
    const data = await response.json();
    return rewriteMinioHost(data.url);
  },

  // Get user by username (for looking up users to add to chat)
  async getUserByUsername(username) {
    const response = await fetch(`${API_BASE_URL}/users?username=${username}`, {
      headers: authHeaders(),
    });
    return response.json();
  },
};
