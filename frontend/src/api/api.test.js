import { api } from './api';

describe('api auth helpers', () => {
  beforeEach(() => {
    localStorage.clear();
    global.fetch = jest.fn().mockResolvedValue({
      json: jest.fn().mockResolvedValue({ ok: true }),
      text: jest.fn().mockResolvedValue('healthy'),
    });
  });

  afterEach(() => {
    jest.resetAllMocks();
  });

  test('login posts username and password to the auth endpoint', async () => {
    await api.login('alice', 'secret');

    expect(fetch).toHaveBeenCalledWith(
      '/auth/login',
      expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username: 'alice', password: 'secret' }),
      }),
    );
  });

  test('signUp sends password instead of password_hash', async () => {
    await api.signUp({
      username: 'alice',
      email: 'alice@example.com',
      password: 'secret',
      phone_number: '',
    });

    expect(fetch).toHaveBeenCalledWith(
      '/users',
      expect.objectContaining({
        body: JSON.stringify({
          username: 'alice',
          email: 'alice@example.com',
          password: 'secret',
          phone_number: null,
        }),
      }),
    );
  });

  test('protected requests include bearer token when stored', async () => {
    api.setToken('token-123');

    await api.getMessages('chat-1');

    expect(fetch).toHaveBeenCalledWith(
      '/messages?chat_id=chat-1',
      expect.objectContaining({
        headers: expect.objectContaining({
          Authorization: 'Bearer token-123',
        }),
      }),
    );
  });
});