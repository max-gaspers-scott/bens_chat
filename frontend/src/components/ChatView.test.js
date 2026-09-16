import { render, screen, waitFor } from '@testing-library/react';
import ChatView from './ChatView';
import { api } from '../api/api';

jest.mock('../api/api', () => ({
  api: {
    getMessages: jest.fn(),
  },
}));

jest.mock('./SendMessage', () => () => <div data-testid="send-message" />);

// Mock scrollIntoView
window.HTMLElement.prototype.scrollIntoView = jest.fn();

test('renders empty state when no chatId is provided', () => {
  render(<ChatView chatId={null} currentUser={{ username: 'user-1' }} />);
  expect(screen.getByText(/Select a chat to view messages/i)).toBeInTheDocument();
  expect(screen.getByText(/You can create a chat/i)).toBeInTheDocument();
});

test('renders all messages returned for a chat', async () => {
  api.getMessages.mockResolvedValue({
    status: 'success',
    payload: [
      {
        message_id: 'message-1',
        sender_name: 'user-1',
        content: { text: 'Hello there' },
        sent_at: '2024-01-01T00:00:00Z',
      },
      {
        message_id: 'message-2',
        sender_name: 'user-2',
        content: { text: 'General Kenobi' },
        sent_at: '2024-01-01T00:01:00Z',
      },
    ],
  });

  render(<ChatView chatId="chat-1" currentUser={{ username: 'user-1' }} />);

  expect(screen.getByText(/loading messages/i)).toBeInTheDocument();
  expect(await screen.findByText('Hello there')).toBeInTheDocument();
  expect(screen.getByText('General Kenobi')).toBeInTheDocument();

  await waitFor(() => {
    expect(api.getMessages).toHaveBeenCalledWith('chat-1');
  });
});

test('renders back to chats button and calls onSelectChat with null when clicked', async () => {
  api.getMessages.mockResolvedValue({
    status: 'success',
    payload: [],
  });
  const mockSelectChat = jest.fn();

  render(
    <ChatView
      chatId="chat-1"
      currentUser={{ username: 'user-1' }}
      onSelectChat={mockSelectChat}
    />
  );

  expect(await screen.findByText(/No messages yet/i)).toBeInTheDocument();
  const backButton = screen.getByRole('button', { name: /back to chats/i });
  expect(backButton).toBeInTheDocument();
  backButton.click();
  expect(mockSelectChat).toHaveBeenCalledWith(null);
});

test('renders toggle sidebar button and calls onToggleSidebar when clicked', async () => {
  api.getMessages.mockResolvedValue({
    status: 'success',
    payload: [],
  });
  const mockToggleSidebar = jest.fn();

  const { rerender } = render(
    <ChatView
      chatId="chat-1"
      currentUser={{ username: 'user-1' }}
      onToggleSidebar={mockToggleSidebar}
      sidebarOpen={false}
    />
  );

  expect(await screen.findByText(/No messages yet/i)).toBeInTheDocument();
  const toggleButton = screen.getByRole('button', { name: /toggle chats list/i });
  expect(toggleButton).toHaveTextContent(/📋 Chats/i);
  toggleButton.click();
  expect(mockToggleSidebar).toHaveBeenCalledTimes(1);

  rerender(
    <ChatView
      chatId="chat-1"
      currentUser={{ username: 'user-1' }}
      onToggleSidebar={mockToggleSidebar}
      sidebarOpen={true}
    />
  );
  expect(screen.getByRole('button', { name: /toggle chats list/i })).toHaveTextContent(/📋 Hide Chats/i);
});