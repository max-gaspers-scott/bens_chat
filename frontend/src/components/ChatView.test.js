import { render, screen, waitFor } from '@testing-library/react';
import ChatView from './ChatView';
import { api } from '../api/api';

jest.mock('../api/api', () => ({
  api: {
    getMessages: jest.fn(),
  },
}));

jest.mock('./SendMessage', () => () => <div data-testid="send-message" />);

test('renders all messages returned for a chat', async () => {
  api.getMessages.mockResolvedValue({
    status: 'success',
    payload: [
      {
        message_id: 'message-1',
        sender_id: 'user-1',
        content: 'Hello there',
        sent_at: '2024-01-01T00:00:00Z',
      },
      {
        message_id: 'message-2',
        sender_id: 'user-2',
        content: 'General Kenobi',
        sent_at: '2024-01-01T00:01:00Z',
      },
    ],
  });

  render(<ChatView chatId="chat-1" currentUser={{ user_id: 'user-1' }} />);

  expect(screen.getByText(/loading messages/i)).toBeInTheDocument();
  expect(await screen.findByText('Hello there')).toBeInTheDocument();
  expect(screen.getByText('General Kenobi')).toBeInTheDocument();

  await waitFor(() => {
    expect(api.getMessages).toHaveBeenCalledWith('chat-1');
  });
});