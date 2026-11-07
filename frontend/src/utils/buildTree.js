// Turn the flat message list returned by GET /messages?parent=<id> into a tree.
//
// The payload contains the requested message (the "view root") plus all of its
// descendants. The view root is identified by its id, never by parent_id === null:
// for a nested view its parent_id points at a message outside the payload.
//
// Runs in O(n) (plus sorting each node's children by sent_at). Messages whose
// parent is not in the list (other than the view root) are left out.
//
// Returns the root node { msg, children: [node, ...] }, or null when the view
// root is not in the list.
export function buildTree(messages, rootId) {
  const nodes = new Map();
  for (const msg of messages || []) {
    if (msg && msg.message_id) {
      nodes.set(msg.message_id, { msg, children: [] });
    }
  }

  for (const node of nodes.values()) {
    const { message_id, parent_id } = node.msg;
    if (message_id === rootId || parent_id == null) continue;
    const parent = nodes.get(parent_id);
    if (parent) parent.children.push(node);
  }

  const time = (node) => {
    const t = Date.parse(node.msg.sent_at);
    return Number.isNaN(t) ? 0 : t;
  };
  for (const node of nodes.values()) {
    if (node.children.length > 1) {
      node.children.sort((a, b) => time(a) - time(b));
    }
  }

  return nodes.get(rootId) || null;
}

// Number of messages below a node (children, grandchildren, ...).
export function countDescendants(node) {
  let count = 0;
  const stack = [...node.children];
  while (stack.length) {
    const next = stack.pop();
    count += 1;
    stack.push(...next.children);
  }
  return count;
}
