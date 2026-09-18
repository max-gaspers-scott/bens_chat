import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import App from './App';
import { api } from './api/api';

jest.mock('socket.io-client', () => ({
  io: () => ({
    on: jest.fn(),
    emit: jest.fn(),
    disconnect: jest.fn(),
  }),
}));

jest.mock('./api/api', () => ({
  getSocketUrl: jest.fn(() => 'http://localhost:9821'),
  api: {
    getToken: jest.fn(),
    clearToken: jest.fn(),
    setToken: jest.fn(),
    registerUnauthorizedHandler: jest.fn(),
    getUserChats: jest.fn().mockResolvedValue({
      status: 'success',
      payload: [
        {
          message_id: 'chat-123',
          content: { title: 'Test Chat Room' },
        },
      ],
    }),
    getMessages: jest.fn().mockResolvedValue({
      status: 'success',
      payload: [],
    }),
  },
}));

beforeEach(() => {
  localStorage.clear();
  jest.clearAllMocks();
  api.getToken.mockReturnValue(null);
  api.getUserChats.mockResolvedValue({
    status: 'success',
    payload: [
      {
        message_id: 'chat-123',
        content: { title: 'Test Chat Room' },
      },
    ],
  });
  api.getMessages.mockResolvedValue({
    status: 'success',
    payload: [],
  });
});

test('renders the login form by default', () => {
  render(<App />);
  expect(screen.getByText(/chat app/i)).toBeInTheDocument();
  expect(screen.getByRole('heading', { name: /login/i })).toBeInTheDocument();
  expect(screen.getByRole('button', { name: /^login$/i })).toBeInTheDocument();
  expect(screen.getByText(/don't have an account\?/i)).toBeInTheDocument();
});

test('clicking "Chat App" takes user home from sign up view', () => {
  render(<App />);

  // Go to Sign Up
  fireEvent.click(screen.getByRole('button', { name: /^sign up$/i }));
  expect(screen.getByRole('heading', { name: /^sign up$/i })).toBeInTheDocument();

  // Click "Chat App"
  fireEvent.click(screen.getByText('Chat App'));
  expect(screen.getByRole('heading', { name: /login/i })).toBeInTheDocument();
});

test('when logged in, clicking "Chat App" takes user home and clears selected chat', async () => {
  api.getToken.mockReturnValue('valid-token');
  localStorage.setItem('currentUser', JSON.stringify({ username: 'testuser' }));

  const { container } = render(<App />);

  // Initially at home with no chat selected
  const chatContainer = container.querySelector('.chat-container');
  expect(chatContainer).toHaveClass('no-chat-selected');

  // Wait for chats to load and select a chat
  const chatItem = await screen.findByText('Test Chat Room');
  expect(chatItem).toBeInTheDocument();
  fireEvent.click(chatItem);

  // Once a chat is selected, no-chat-selected is removed (hiding sidebar on mobile by default)
  await waitFor(() => {
    expect(chatContainer).not.toHaveClass('no-chat-selected');
  });

  // Now click "Chat App" in the top left
  fireEvent.click(screen.getByText('Chat App'));

  // Chat container should be back in no-chat-selected state (home)
  await waitFor(() => {
    expect(chatContainer).toHaveClass('no-chat-selected');
  });
});

test('toggles mobile user menu when mobile menu button is clicked', async () => {
  api.getToken.mockReturnValue('valid-token');
  localStorage.setItem('currentUser', JSON.stringify({ username: 'testuser' }));

  const { container } = render(<App />);

  expect(await screen.findByText('Test Chat Room')).toBeInTheDocument();

  const menuButton = screen.getByRole('button', { name: /toggle user menu/i });
  const userInfo = container.querySelector('.user-info');

  expect(userInfo).not.toHaveClass('mobile-menu-open');
  fireEvent.click(menuButton);
  expect(userInfo).toHaveClass('mobile-menu-open');

  fireEvent.click(menuButton);
  expect(userInfo).not.toHaveClass('mobile-menu-open');
});

test('allows toggling mobile sidebar when in a chat and closing it', async () => {
  api.getToken.mockReturnValue('valid-token');
  localStorage.setItem('currentUser', JSON.stringify({ username: 'testuser' }));

  const { container } = render(<App />);

  const chatItem = await screen.findByText('Test Chat Room');
  fireEvent.click(chatItem);

  const chatContainer = container.querySelector('.chat-container');
  await waitFor(() => {
    expect(chatContainer).not.toHaveClass('mobile-sidebar-open');
  });

  // Open sidebar drawer via toggle button in chat header
  const toggleBtn = await screen.findByRole('button', { name: /toggle chats list/i });
  fireEvent.click(toggleBtn);
  expect(chatContainer).toHaveClass('mobile-sidebar-open');

  // Close via close button in mobile sidebar header
  const closeBtn = screen.getByRole('button', { name: /close chat list/i });
  fireEvent.click(closeBtn);
  expect(chatContainer).not.toHaveClass('mobile-sidebar-open');
});
