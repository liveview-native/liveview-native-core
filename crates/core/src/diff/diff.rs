use std::collections::{BTreeMap, VecDeque};

use itertools::unfold;
use smallvec::{smallvec, SmallVec};

use crate::dom::*;

use super::patch::Patch;
use super::traversal::MoveTo;

#[derive(Debug, Clone)]
struct Cursor {
    path: SmallVec<[u16; 6]>,
    node: NodeRef,
}
impl Cursor {
    #[inline(always)]
    fn node<'a>(&self, doc: &'a Document) -> &'a Node {
        doc.get(self.node)
    }

    #[inline]
    fn depth(&self) -> usize {
        self.path.len()
    }

    /// Moves the cursor up the tree to its parent, if it has one
    fn up(&mut self, doc: &Document) {
        if let Some(node) = doc.parent(self.node) {
            self.path.pop();
            self.node = node;
        } else {
            panic!("expected parent");
        }
    }

    /// Moves the cursor to the next sibling of the cursor's node, if possible
    #[allow(unused)]
    fn forward(&mut self, n: usize, doc: &Document) {
        if let Some(node) = doc.parent(self.node) {
            let index = self.path.last().copied().unwrap() as usize;
            let pos = index + n;
            let children = doc.children(node);
            if let Some(node) = children.get(pos) {
                self.path.pop();
                self.path.push(pos as u16);
                self.node = *node;
            } else {
                panic!("expected child");
            }
        } else {
            panic!("expected parent");
        }
    }
}
impl Eq for Cursor {}
impl PartialEq for Cursor {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.path.eq(&other.path)
    }
}
impl PartialOrd for Cursor {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Cursor {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        let mut anext = self.path.iter();
        let mut bnext = other.path.iter();

        loop {
            match (anext.next(), bnext.next()) {
                (None, None) => break,
                (None, _) => return Ordering::Less,
                (_, None) => return Ordering::Greater,
                (a, b) => match a.cmp(&b) {
                    Ordering::Equal => continue,
                    other => return other,
                },
            }
        }

        Ordering::Equal
    }
}

pub fn diff(old_document: &Document, new_document: &Document) -> VecDeque<Patch> {
    let mut patches = VecDeque::new();
    let mut old_next = VecDeque::from([Cursor {
        path: smallvec![],
        node: old_document.root(),
    }]);
    let mut new_next = VecDeque::from([Cursor {
        path: smallvec![],
        node: new_document.root(),
    }]);
    loop {
        match (old_next.pop_front(), new_next.pop_front()) {
            // We're at the same position in both trees, so examine the details of the node under the cursor
            (Some(old_cursor), Some(new_cursor)) if old_cursor == new_cursor => {
                match (old_cursor.node(old_document), new_cursor.node(new_document)) {
                    // This was the root node, so move the cursor down a level and start walking the children
                    (Node::Root, Node::Root) => {
                        let old_children = old_document.children(old_cursor.node);
                        let new_children = new_document.children(new_cursor.node);
                        old_next.extend(old_children.iter().copied().enumerate().map(
                            |(i, node)| {
                                let mut path = old_cursor.path.clone();
                                path.push(i as u16);
                                Cursor { path, node }
                            },
                        ));
                        new_next.extend(new_children.iter().copied().enumerate().map(
                            |(i, node)| {
                                let mut path = new_cursor.path.clone();
                                path.push(i as u16);
                                Cursor { path, node }
                            },
                        ));
                    }
                    // Both nodes are leaf nodes, compare their content for equality
                    (Node::Leaf(ref ol), Node::Leaf(ref nl)) => {
                        // If both nodes are identical, keep moving
                        // Otherwise, emit a patch to update the text of this node
                        if ol == nl {
                            continue;
                        } else {
                            patches.push_back(Patch::Replace {
                                node: old_cursor.node,
                                replacement: Node::Leaf(nl.clone()),
                            });
                        }
                    }
                    // Both nodes are elements, compare their tag and attributes for equality, then append their children to the stack
                    (Node::Element(ref old), Node::Element(ref new)) => {
                        // If the names are different, replace the old element with the new
                        if old.name != new.name {
                            patches.push_back(Patch::Replace {
                                node: old_cursor.node,
                                replacement: Node::Element(new.clone()),
                            });
                        } else {
                            // Check for changes to the attributes
                            patches.extend(diff_attributes(old_cursor.node, old, new));
                        }
                        // Add the children of both nodes to the worklist
                        let old_children = old_document.children(old_cursor.node);
                        let new_children = new_document.children(new_cursor.node);
                        old_next.extend(old_children.iter().copied().enumerate().map(
                            |(i, node)| {
                                let mut path = old_cursor.path.clone();
                                path.push(i as u16);
                                Cursor { path, node }
                            },
                        ));
                        new_next.extend(new_children.iter().copied().enumerate().map(
                            |(i, node)| {
                                let mut path = new_cursor.path.clone();
                                path.push(i as u16);
                                Cursor { path, node }
                            },
                        ));
                    }
                    // The old node was an element and the new node is a leaf; determine if this is a simple swap, addition, or removal
                    // by looking forward in the stack to future cursors which are at the same depth.
                    (Node::Element(_), Node::Leaf(ref new)) => {
                        patches.push_back(Patch::Replace {
                            node: old_cursor.node,
                            replacement: Node::Leaf(new.clone()),
                        });
                        let old_children = old_document.children(old_cursor.node);
                        old_next.extend(old_children.iter().copied().enumerate().map(
                            |(i, node)| {
                                let mut path = old_cursor.path.clone();
                                path.push(i as u16);
                                Cursor { path, node }
                            },
                        ));
                    }
                    // The old node was a leaf and the new node is an element; determine if this is a simple swap, addition, or removal
                    // by looking forward in the stack to future cursors which are at the same depth.
                    (Node::Leaf(_), Node::Element(ref new)) => {
                        patches.push_back(Patch::Replace {
                            node: old_cursor.node,
                            replacement: Node::Element(new.clone()),
                        });
                        let new_children = new_document.children(new_cursor.node);
                        new_next.extend(new_children.iter().copied().enumerate().map(
                            |(i, node)| {
                                let mut path = new_cursor.path.clone();
                                path.push(i as u16);
                                Cursor { path, node }
                            },
                        ));
                    }
                    _ => unreachable!(),
                }
            }
            // This occurs in the following scenario (where `^` indicates the cursor location in each tree):
            //
            //       old     new                  old worklist  new worklist
            //        |       |   <- depth 0      a             a
            //       / \     / \                  b             b
            //      a   b   a   b <- depth 1      c             c
            //     / \  |   |   |                 d <           e <
            //    c  d  e   c   e <- depth 2      e
            //       ^          ^
            //
            // In this scenario, the new tree has had `d` removed from it, and as nodes are added to the worklist
            // in breadth-first order, this means that when we move to `d` in the old tree, we'll move to `e` in the new.
            //
            // When this occurs, it means that the old parent node (e.g. `a` in the example above) has extra nodes which need
            // to be removed. To bring the old cursor up to par with the new cursor, we remove all nodes in the old tree until
            // we catch up to it
            (Some(old_cursor), Some(new_cursor))
                if old_cursor < new_cursor && old_cursor.depth() == new_cursor.depth() =>
            {
                patches.push_back(Patch::Remove {
                    node: old_cursor.node,
                });
                while let Some(old_cursor) = old_next.front() {
                    if old_cursor >= &new_cursor {
                        // We've caught up to the new cursor
                        break;
                    }
                    let old_cursor = old_next.pop_front().unwrap();
                    patches.push_back(Patch::Remove {
                        node: old_cursor.node,
                    });
                }
                // We need to revisit the new_cursor now that we've adjusted the old cursor
                new_next.push_front(new_cursor);
            }
            // This occurs in the following scenario (where `^` indicates the cursor location in each tree):
            //
            //       old      new                  old worklist  new worklist
            //        |        |   <- depth 0      a             a
            //       / \      / \                  b             b
            //      a   b    a   b <- depth 1      c             c
            //      |   |   / \  |                 d <           e <
            //      c   d  c   e d <- depth 2                    d
            //          ^      ^
            //
            // Similar to the previous scenario, this one occurs when the new tree has had `e` added to `a` as a new
            // child element. This means that when we move to `d` in the old tree, we'll move to `e` in the new.
            //
            // As implied, this means that the old parent node (e.g. `a` in the example above) has nodes which need
            // to be added to it. To bring the new cursor up to par with the old cursor, we add every node in the new tree
            // until we've caught up to it
            (Some(old_cursor), Some(new_cursor))
                if old_cursor > new_cursor && old_cursor.depth() == new_cursor.depth() =>
            {
                patches.push_back(Patch::Move(MoveTo::Node(old_document.root())));
                // Traverse the old tree based on the new_cursor path until we get to its immediate parent
                {
                    let (_, parent_path) = new_cursor.path.split_last().unwrap();
                    for index in parent_path.iter().copied() {
                        patches.push_back(Patch::Move(MoveTo::Child(index as u32)));
                    }
                }
                patches.push_back(Patch::PushCurrent);
                let mut subtree = recursively_append(new_cursor.node, new_document);
                patches.append(&mut subtree);
                old_next.push_front(old_cursor);
            }
            // This occurs in the following scenario (where `^` indicates the cursor location in each tree):
            //
            //       old      new                  old worklist  new worklist
            //        |        |     <- depth 0      a             a
            //       / \      / \                    b             b
            //      a   b    a   b   <- depth 1      c             c
            //     /        /   / \                  d <           f <
            //    c        c   f   g <- depth 2      e             g
            //    |        |   ^                                   d
            //    d<       d   j                                   j
            //    |        |                                       e
            //    e        e
            //
            //
            // In this scenario, a new child element was added in the new tree which will be visited first before we can catch up
            // with the old cursor. To catch up, we simply add all nodes in the new tree until we reach the old cursor.
            // This is like the previous scenario, but inverted. The new tree has been modified in such a way that when we move from
            // `c` in both trees, we'll end up at a greater depth in the old tree.
            //
            // In order to proceed, we must add all nodes from the new tree with the same depth and parent as the initial new cursor,
            // until we either reach the end of the new tree, or we catch up to the old cursor
            (Some(old_cursor), Some(new_cursor))
                if new_cursor > old_cursor && old_cursor.depth() > new_cursor.depth() =>
            {
                // Create a cursor to traverse back up the tree to the last common parent
                let mut tmp_cursor = old_cursor.clone();
                let hops = old_cursor.depth() - new_cursor.depth();
                for _ in 0..hops {
                    tmp_cursor.up(old_document);
                }
                debug_assert_eq!(tmp_cursor.depth(), new_cursor.depth());
                // Insert the missing node after the current node in the old tree, as it must be an immediate sibling
                let mut subtree =
                    recursively_append_after(tmp_cursor.node, new_cursor.node, new_document);
                patches.append(&mut subtree);
                // Retry with the next item in the worklist
                old_next.push_front(old_cursor);
            }
            // This is the exact inverse of the scenario above, where rather than a new child being introduced, a previously
            // existing node has been removed, resulting in the old cursor moving ahead in the tree compared to the new cursor.
            //
            //       old        new                  old worklist  new worklist
            //        |          |     <- depth 0      a             a
            //       / \        / \                    b             b
            //      a   b      a   b   <- depth 1      c             c
            //     /   / \    /                        f <           d <
            //    c   f   g  c         <- depth 2      g             e
            //    |   ^      |                         d
            //    d   j      d<                        j
            //    |          |                         e
            //    e          e
            //
            //           //
            (Some(old_cursor), Some(new_cursor))
                if old_cursor > new_cursor && new_cursor.depth() > old_cursor.depth() =>
            {
                patches.push_back(Patch::Remove {
                    node: old_cursor.node,
                });
                // Retry with the next item in the worklist
                new_next.push_front(new_cursor);
            }
            (Some(old_cursor), Some(new_cursor)) => {
                panic!(
                    "unexpected edge case in diff calculation: {:#?} vs {:#?}",
                    old_cursor, new_cursor
                );
            }
            // We've reached the end of the worklist for the new tree, but not the old; this means
            // that all remaining nodes in the old tree were removed, since we can't have visited them yet,
            // so issue removals for all remaining old tree nodes
            (Some(old_cursor), None) => {
                patches.push_back(Patch::Remove {
                    node: old_cursor.node,
                });
                while let Some(old_cursor) = old_next.pop_front() {
                    patches.push_back(Patch::Remove {
                        node: old_cursor.node,
                    });
                }
            }
            // We've reached the end of the worklist for the old tree, which means that all remaining nodes in the
            // new tree were added.
            //
            //      old          new         old worklist  new worklist
            //       |            |            a             a
            //      / \          / \           b             b
            //     a   b        a   b          *<            c<
            //                  |   |                        d
            //                  c   d
            //
            // As shown above, additions must necessarily be rooted under nodes of the tree that already exist, but
            // they might be under different parents. We use the cursor path to traverse to the appropriate node of
            // the tree in which to append the new children.
            (None, Some(new_cursor)) => {
                patches.push_back(Patch::Move(MoveTo::Node(old_document.root())));
                // Traverse the old tree based on the new_cursor path until we get to its immediate parent
                // Skip the last index in the new_cursor path, because that's the index of the
                // new_cursor node (c in the diagram above), which does not yet exist in the tree.
                for index in new_cursor.path[..new_cursor.path.len() - 1].iter().copied() {
                    patches.push_back(Patch::Move(MoveTo::Child(index as u32)));
                }
                patches.push_back(Patch::PushCurrent);
                let mut subtree = recursively_append(new_cursor.node, new_document);
                patches.append(&mut subtree);
            }
            // We've reached the end of the worklist, at the same time, we're done
            (None, None) => break,
        }
    }

    patches
}

pub fn diff_attributes<'a>(
    node: NodeRef,
    old: &'a Element,
    new: &'a Element,
) -> impl Iterator<Item = Patch> + 'a {
    let current: BTreeMap<&'a AttributeName, &'a AttributeValue> = BTreeMap::from_iter(
        old.attributes()
            .into_iter()
            .map(|attr| (&attr.name, &attr.value)),
    );

    unfold(
        (current, new.attributes().into_iter()),
        move |(current, new)| {
            while let Some(attr) = new.next() {
                match current.remove(&attr.name) {
                    Some(value) if value.ne(&attr.value) => {
                        return Some(Patch::UpdateAttribute {
                            node,
                            name: attr.name.to_owned(),
                            value: attr.value.to_owned(),
                        });
                    }
                    Some(_) => {}
                    None => {
                        return Some(Patch::AddAttributeTo {
                            node,
                            name: attr.name.to_owned(),
                            value: attr.value.to_owned(),
                        });
                    }
                }
            }

            current
                .pop_first()
                .map(|(name, _)| Patch::RemoveAttributeByName {
                    node,
                    name: name.to_owned(),
                })
        },
    )
}

fn recursively_append_after(
    after: NodeRef,
    src: NodeRef,
    src_document: &Document,
) -> VecDeque<Patch> {
    use petgraph::visit::{depth_first_search, DfsEvent};

    let mut patches = VecDeque::new();

    depth_first_search(src_document, Some(src), |event| {
        match event {
            DfsEvent::Discover(node, _) => {
                match src_document.get(node) {
                    Node::Leaf(ref s) => patches.push_back(Patch::CreateAndMoveTo {
                        node: Node::Leaf(s.clone()),
                    }),
                    Node::Element(ref elem) => {
                        patches.push_back(Patch::CreateAndMoveTo {
                            node: Node::Element(elem.clone()),
                        });
                    }
                    // Ignore the root
                    Node::Root => (),
                };
            }
            DfsEvent::TreeEdge(_from, _to) => {
                // Moving down the tree one level, ignored because Discover takes care of traversing for us
                ()
            }
            DfsEvent::Finish(node, _) if node == src => {
                // All children of `node` have been visited, and we're returning up the tree
                patches.push_back(Patch::AppendAfter { after });
            }
            DfsEvent::Finish(_node, _) => {
                // All children of `node` have been visited, and we're returning up the tree
                patches.push_back(Patch::Attach);
            }
            // This is a tree, these types of edges aren't allowed
            DfsEvent::BackEdge(_, _) | DfsEvent::CrossForwardEdge(_, _) => unreachable!(),
        }
    });

    patches
}

fn recursively_append(src: NodeRef, src_document: &Document) -> VecDeque<Patch> {
    use petgraph::visit::{depth_first_search, DfsEvent};

    let mut patches = VecDeque::new();

    depth_first_search(src_document, Some(src), |event| {
        match event {
            DfsEvent::Discover(node, _) => {
                match src_document.get(node) {
                    Node::Leaf(ref s) => patches.push_back(Patch::CreateAndMoveTo {
                        node: Node::Leaf(s.clone()),
                    }),
                    Node::Element(ref elem) => {
                        patches.push_back(Patch::CreateAndMoveTo {
                            node: Node::Element(elem.clone()),
                        });
                    }
                    // Ignore the root
                    Node::Root => (),
                };
            }
            DfsEvent::TreeEdge(_from, _to) => {
                // Moving down the tree one level, ignored because Discover takes care of traversing for us
                ()
            }
            DfsEvent::Finish(_node, _) => {
                // All children of `node` have been visited, and we're returning up the tree
                patches.push_back(Patch::Attach);
            }
            // This is a tree, these types of edges aren't allowed
            DfsEvent::BackEdge(_, _) | DfsEvent::CrossForwardEdge(_, _) => unreachable!(),
        }
    });

    patches
}
