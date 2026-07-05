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

let unauthorizedHandler = null;

const apiFetch = async (url, options = {}) => {
  const response = await fetch(url, options);
  if (response.status === 401) {
    localStorage.removeItem(TOKEN_STORAGE_KEY);
    if (unauthorizedHandler) {
      unauthorizedHandler();
    }
    throw new Error('Unauthorized');
  }
  return response;
};

export const api = {
  registerUnauthorizedHandler(handler) {
    unauthorizedHandler = handler;
  },

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
    const response = await apiFetch(`${API_BASE_URL}/health`);
    return response.text();
  },

  // Sign up - create a new user.
  // The backend `users` table keys on `name`, and the create-user handler hashes
  // whatever it receives in the `password_hash` field, so the plaintext password
  // is sent there.
  async signUp({ username, email, password, phone_number }) {
    const response = await apiFetch(`${API_BASE_URL}/users`, {
      method: 'POST',
      headers: jsonHeaders(),
      body: JSON.stringify({
        name: username,
        phone_number: phone_number || null,
        email,
        password_hash: password,
      }),
    });
    return response.json();
  },

  // Login - verify username/password and return a JWT
  async login(username, password) {
    const response = await apiFetch(`${API_BASE_URL}/auth/login`, {
      method: 'POST',
      headers: jsonHeaders(),
      body: JSON.stringify({ username, password }),
    });
    return response.json();
  },

  // Create a new chat by posting a root message (parent = null). The backend
  // also registers the sender as a participant of the new chat.
  async createChat({ sender_name, title }) {
    const response = await apiFetch(`${API_BASE_URL}/messages`, {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        sender_name,
        parent: null,
        content: { text: title },
      }),
    });
    return response.json();
  },

  // Link a user to a chat (add a participant). chat_id is the chat's root message id.
  async linkUserToChat(user_name, chat_id) {
    const response = await apiFetch(`${API_BASE_URL}/user-chats`, {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({
        chat_participant_id: crypto.randomUUID(),
        chat_id,
        user_name,
      }),
    });
    return response.json();
  },

  // Get the chats (root messages) a user participates in
  async getUserChats(username) {
    const response = await apiFetch(
      `${API_BASE_URL}/user-chats?username=${encodeURIComponent(username)}`,
      { headers: authHeaders() }
    );
    return response.json();
  },

  // Get messages for a chat (children of the chat's root message)
  async getMessages(parent) {
    const response = await apiFetch(
      `${API_BASE_URL}/messages?parent=${encodeURIComponent(parent)}`,
      { headers: authHeaders() }
    );
    return response.json();
  },

  // Send a message. content is the message body object ({ text, url? }) stored as JSONB.
  async sendMessage({ sender_name, parent, content }) {
    const response = await apiFetch(`${API_BASE_URL}/messages`, {
      method: 'POST',
      headers: authHeaders(),
      body: JSON.stringify({ sender_name, parent, content }),
    });
    return response.json();
  },

  // Get a presigned PUT URL + server-generated object key for uploading an image to MinIO
  async getUploadUrl(chatId, fileExtension) {
    const response = await apiFetch(
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
    const response = await apiFetch(
      `${API_BASE_URL}/minio-fetch?object_key=${encodeURIComponent(objectKey)}`,
      { headers: authHeaders() }
    );
    const data = await response.json();
    return rewriteMinioHost(data.url);
  },

  // Set/Change password
  async setPassword(new_password) {
    const response = await apiFetch(
      `${API_BASE_URL}/password-set?new_password=${encodeURIComponent(new_password)}`,
      {
        method: 'POST',
        headers: authHeaders(),
      }
    );
    return response.json();
  },
};

