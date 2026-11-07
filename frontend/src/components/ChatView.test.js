import { render, screen, waitFor, fireEvent, act } from '@testing-library/react';
import ChatView, { MAX_DEPTH } from './ChatView';
import { api } from '../api/api';

jest.mock('../api/api', () => ({
  api: {
    getMessages: jest.fn(),
  },
}));

jest.mock('./SendMessage', () => (props) => (
  <div data-testid="send-message" data-parent-id={props.parentId} data-chat-id={props.chatId} />
));

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
        message_id: 'chat-1',
        parent_id: null,
        sender_name: 'user-1',
        content: { title: 'My Chat' },
        sent_at: '2023-12-31T00:00:00Z',
      },
      {
        message_id: 'message-1',
        parent_id: 'chat-1',
        sender_name: 'user-1',
        content: { text: 'Hello there' },
        sent_at: '2024-01-01T00:00:00Z',
      },
      {
        message_id: 'message-2',
        parent_id: 'chat-1',
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
// ---------------------------------------------------------------------------
// Threaded view
// ---------------------------------------------------------------------------
const m = (id, parent_id, minute, text = id, sender = 'user-2') => ({
  message_id: id,
  parent_id,
  sender_name: sender,
  content: parent_id === null ? { title: text } : { text },
  sent_at: `2024-01-01T00:${String(minute).padStart(2, '0')}:00Z`,
});

test('nests replies under their parents and shows the chat title as header', async () => {
  api.getMessages.mockResolvedValue({
    status: 'success',
    payload: [
      m('chat-1', null, 0, 'Family'),
      m('reply', 'top', 2, 'a reply'),
      m('top', 'chat-1', 1, 'top level'),
    ],
  });

  render(<ChatView chatId="chat-1" currentUser={{ username: 'user-1' }} />);

  const reply = await screen.findByText('a reply');
  expect(screen.getByRole('heading', { name: 'Family' })).toBeInTheDocument();
  // The chat root itself is not repeated in the list.
  expect(screen.queryByText('Family', { selector: '.message-content' })).not.toBeInTheDocument();
  const topNode = screen.getByText('top level').closest('.thread-node');
  expect(topNode).toContainElement(reply);
  expect(screen.queryByText(/up one level/i)).not.toBeInTheDocument();
});

test('stops at MAX_DEPTH and offers "continue this thread"', async () => {
  const payload = [m('chat-1', null, 0, 'Deep')];
  let parent = 'chat-1';
  for (let i = 1; i <= MAX_DEPTH + 2; i += 1) {
    payload.push(m(`d${i}`, parent, i, `depth ${i}`));
    parent = `d${i}`;
  }
  api.getMessages.mockResolvedValue({ status: 'success', payload });

  render(<ChatView chatId="chat-1" currentUser={{ username: 'user-1' }} />);

  expect(await screen.findByText(`depth ${MAX_DEPTH}`)).toBeInTheDocument();
  expect(screen.queryByText(`depth ${MAX_DEPTH + 1}`)).not.toBeInTheDocument();

  const continueBtn = screen.getByRole('button', { name: /continue this thread/i });
  expect(continueBtn).toHaveTextContent('2 more replies');

  fireEvent.click(continueBtn);
  await waitFor(() => expect(api.getMessages).toHaveBeenLastCalledWith(`d${MAX_DEPTH}`));
});

test('reply button targets the composer and cancel resets it to the view root', async () => {
  api.getMessages.mockResolvedValue({
    status: 'success',
    payload: [m('chat-1', null, 0, 'Chat'), m('top', 'chat-1', 1, 'hello world', 'bob')],
  });

  render(<ChatView chatId="chat-1" currentUser={{ username: 'user-1' }} />);
  await screen.findByText('hello world');

  expect(screen.getByTestId('send-message')).toHaveAttribute('data-parent-id', 'chat-1');
  fireEvent.click(screen.getByRole('button', { name: 'Reply' }));
  expect(screen.getByText(/Replying to/i)).toHaveTextContent('Replying to bob: hello world');
  expect(screen.getByTestId('send-message')).toHaveAttribute('data-parent-id', 'top');

  fireEvent.click(screen.getByRole('button', { name: /cancel reply/i }));
  expect(screen.queryByText(/Replying to/i)).not.toBeInTheDocument();
  expect(screen.getByTestId('send-message')).toHaveAttribute('data-parent-id', 'chat-1');
});

test('opening a thread shows the message as header with up / back navigation', async () => {
  const full = [m('chat-1', null, 0, 'Chat'), m('top', 'chat-1', 1, 'top level'), m('reply', 'top', 2, 'a reply')];
  const nested = [m('reply', 'top', 2, 'a reply'), m('deeper', 'reply', 3, 'deeper reply')];
  api.getMessages.mockImplementation(async (parent) => ({
    status: 'success',
    payload: parent === 'reply' ? nested : full,
  }));

  render(<ChatView chatId="chat-1" currentUser={{ username: 'user-1' }} />);
  await screen.findByText('a reply');

  const replyNode = screen.getByText('a reply').closest('.thread-node');
  fireEvent.click(replyNode.querySelector('.message-actions button:last-child'));

  expect(await screen.findByText('deeper reply')).toBeInTheDocument();
  expect(api.getMessages).toHaveBeenLastCalledWith('reply');
  expect(screen.getByText('a reply').closest('.thread-root')).not.toBeNull();
  expect(screen.getByTestId('send-message')).toHaveAttribute('data-parent-id', 'reply');
  expect(screen.getByTestId('send-message')).toHaveAttribute('data-chat-id', 'chat-1');

  fireEvent.click(screen.getByRole('button', { name: /up one level/i }));
  await waitFor(() => expect(api.getMessages).toHaveBeenLastCalledWith('top'));

  fireEvent.click(screen.getByRole('button', { name: /back to full chat/i }));
  await waitFor(() => expect(api.getMessages).toHaveBeenLastCalledWith('chat-1'));
  expect(await screen.findByText('top level')).toBeInTheDocument();
});

test('live messages attach under a parent in view and are ignored otherwise', async () => {
  api.getMessages.mockResolvedValue({
    status: 'success',
    payload: [m('chat-1', null, 0, 'Chat'), m('top', 'chat-1', 1, 'top level')],
  });

  render(<ChatView chatId="chat-1" currentUser={{ username: 'user-1' }} />);
  await screen.findByText('top level');

  act(() => {
    window.dispatchEvent(new CustomEvent('refreshMessages', {
      detail: { res: 'success', data: m('live', 'top', 5, 'live reply') },
    }));
    window.dispatchEvent(new CustomEvent('refreshMessages', {
      detail: { res: 'success', data: m('elsewhere', 'other-chat', 6, 'not here') },
    }));
  });

  const live = await screen.findByText('live reply');
  expect(screen.getByText('top level').closest('.thread-node')).toContainElement(live);
  expect(screen.queryByText('not here')).not.toBeInTheDocument();
});
