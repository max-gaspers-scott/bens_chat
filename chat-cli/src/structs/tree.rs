use std::collections::HashMap;
use uuid::Uuid;

/// Minimal view of a message needed to build the reply tree.
/// Implemented for the shared `Message<C>` so tests can use any content type.
pub trait TreeItem {
    fn id(&self) -> Uuid;
    fn parent(&self) -> Option<Uuid>;
    fn sent_at(&self) -> chrono::DateTime<chrono::Utc>;
}

impl<C> TreeItem for bens_chat_shared::Message<C> {
    fn id(&self) -> Uuid {
        self.message_id
    }
    fn parent(&self) -> Option<Uuid> {
        self.parent_id
    }
    fn sent_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.sent_at
    }
}

/// The flat /messages payload rebuilt as a tree rooted at `root_id`.
pub struct MsgTree<'a, T> {
    /// The message the view was opened on (None if the backend did not include it).
    pub root: Option<&'a T>,
    /// Every descendant of the root in display order, with its depth (direct replies = 1).
    pub rows: Vec<(usize, &'a T)>,
}

/// Build the reply tree in O(n): one pass to index children by parent_id,
/// then a depth first walk from `root_id`. Siblings are ordered by sent_at.
/// Messages not reachable from `root_id` are ignored.
pub fn build_tree<T: TreeItem>(messages: &[T], root_id: Uuid) -> MsgTree<'_, T> {
    let mut root = None;
    let mut children: HashMap<Uuid, Vec<&T>> = HashMap::new();
    for m in messages {
        if m.id() == root_id {
            root = Some(m);
            continue;
        }
        if let Some(p) = m.parent() {
            children.entry(p).or_default().push(m);
        }
    }
    // payload is already sorted by sent_at, but don't rely on it
    for kids in children.values_mut() {
        kids.sort_by_key(|m| m.sent_at());
    }

    let mut rows = Vec::with_capacity(messages.len());
    // iterative DFS so very deep threads can't blow the stack
    let mut stack: Vec<(usize, &T)> = Vec::new();
    if let Some(kids) = children.get(&root_id) {
        stack.extend(kids.iter().rev().map(|m| (1, *m)));
    }
    while let Some((depth, m)) = stack.pop() {
        rows.push((depth, m));
        if let Some(kids) = children.get(&m.id()) {
            stack.extend(kids.iter().rev().map(|k| (depth + 1, *k)));
        }
    }

    MsgTree { root, rows }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bens_chat_shared::Message;
    use chrono::{TimeZone, Utc};

    fn msg(id: u128, parent: Option<u128>, t: i64, text: &str) -> Message<String> {
        Message {
            message_id: Uuid::from_u128(id),
            sender_name: "a".to_string(),
            parent_id: parent.map(Uuid::from_u128),
            content: text.to_string(),
            sent_at: Utc.timestamp_opt(t, 0).unwrap(),
        }
    }

    fn shape<'a>(tree: &MsgTree<'a, Message<String>>) -> Vec<(usize, &'a str)> {
        tree.rows
            .iter()
            .map(|(d, m)| (*d, m.content.as_str()))
            .collect()
    }

    #[test]
    fn builds_tree_from_chat_root() {
        // deliberately out of order to make sure structure comes from parent_id
        let msgs = vec![
            msg(4, Some(2), 40, "grandchild"),
            msg(3, Some(1), 30, "child b"),
            msg(1, None, 10, "title"),
            msg(2, Some(1), 20, "child a"),
        ];
        let tree = build_tree(&msgs, Uuid::from_u128(1));
        assert_eq!(tree.root.unwrap().content, "title");
        assert_eq!(
            shape(&tree),
            vec![(1, "child a"), (2, "grandchild"), (1, "child b")]
        );
    }

    #[test]
    fn builds_tree_from_nested_root() {
        // root (id 2) has a parent (id 1) that is outside the payload
        let msgs = vec![
            msg(2, Some(1), 20, "sub root"),
            msg(3, Some(2), 30, "reply"),
            msg(4, Some(3), 40, "reply to reply"),
            msg(5, Some(2), 50, "second reply"),
        ];
        let tree = build_tree(&msgs, Uuid::from_u128(2));
        assert_eq!(tree.root.unwrap().content, "sub root");
        assert_eq!(
            shape(&tree),
            vec![(1, "reply"), (2, "reply to reply"), (1, "second reply")]
        );
    }

    #[test]
    fn works_without_root_in_payload() {
        // old backend shape: only direct children
        let msgs = vec![msg(2, Some(1), 20, "a"), msg(3, Some(1), 30, "b")];
        let tree = build_tree(&msgs, Uuid::from_u128(1));
        assert!(tree.root.is_none());
        assert_eq!(shape(&tree), vec![(1, "a"), (1, "b")]);
    }
}
