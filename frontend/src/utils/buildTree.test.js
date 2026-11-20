import { buildTree, countDescendants } from './buildTree';

const msg = (id, parent_id, minute) => ({
  message_id: id,
  parent_id,
  sender_name: 'user',
  content: { text: id },
  sent_at: `2024-01-01T00:${String(minute).padStart(2, '0')}:00Z`,
});

const ids = (node) => node.children.map((c) => c.msg.message_id);

test('builds a tree from shuffled input and sorts children by sent_at', () => {
  const messages = [
    msg('c', 'root', 3),
    msg('a', 'root', 1),
    msg('root', null, 0),
    msg('b', 'root', 2),
  ];

  const root = buildTree(messages, 'root');
  expect(root.msg.message_id).toBe('root');
  expect(ids(root)).toEqual(['a', 'b', 'c']);
});

test('attaches grandchildren under their parents', () => {
  const messages = [
    msg('grandchild-2', 'child', 4),
    msg('root', null, 0),
    msg('grandchild-1', 'child', 3),
    msg('child', 'root', 1),
    msg('great-grandchild', 'grandchild-1', 5),
    msg('sibling', 'root', 2),
  ];

  const root = buildTree(messages, 'root');
  expect(ids(root)).toEqual(['child', 'sibling']);
  const child = root.children[0];
  expect(ids(child)).toEqual(['grandchild-1', 'grandchild-2']);
  expect(ids(child.children[0])).toEqual(['great-grandchild']);
  expect(root.children[1].children).toEqual([]);
  expect(countDescendants(root)).toBe(5);
  expect(countDescendants(child)).toBe(3);
});

test('handles a nested view root whose parent_id is outside the list', () => {
  const messages = [
    msg('reply-2', 'nested', 7),
    msg('nested', 'outside-parent', 5),
    msg('reply-1', 'nested', 6),
    msg('reply-1-1', 'reply-1', 8),
  ];

  const root = buildTree(messages, 'nested');
  expect(root).not.toBeNull();
  expect(root.msg.message_id).toBe('nested');
  expect(ids(root)).toEqual(['reply-1', 'reply-2']);
  expect(ids(root.children[0])).toEqual(['reply-1-1']);
});

test('returns null when the view root is missing', () => {
  expect(buildTree([msg('a', 'root', 1)], 'root')).toBeNull();
  expect(buildTree([], 'root')).toBeNull();
  expect(buildTree(undefined, 'root')).toBeNull();
});
