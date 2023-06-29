use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::fmt;
use std::mem;

use smallvec::{smallvec, SmallVec};
use std::ops::Deref;

use crate::dom::*;

use super::MoveTo;
use super::Patch;
#[derive(Clone)]
struct Cursor<'a> {
    doc: &'a Document,
    path: SmallVec<[u16; 6]>,
    node: NodeRef,
}

impl fmt::Debug for Cursor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cursor")
            .field("path", &self.path)
            .field("node", &self.node)
            .field("*node", &self.node())
            .finish()
    }
}

impl<'a> Cursor<'a> {
    fn new(doc: &'a Document, node: NodeRef) -> Self {
        Cursor {
            doc,
            path: smallvec![],
            node,
        }
    }

    fn node(&self) -> &Node {
        self.doc.get(self.node)
    }

    fn depth(&self) -> usize {
        self.path.len()
    }

    // Create a cursor with the current node that iterates over descendant nodes
    fn fork(&self) -> Cursor<'a> {
        Cursor {
            doc: self.doc,
            path: smallvec![],
            node: self.node,
        }
    }

    // Create a cursor at `node` that iterates over descendant nodes
    fn at(&self, node: NodeRef) -> Cursor<'a> {
        Cursor::new(&self.doc, node)
    }

    fn children(&self) -> &'a [NodeRef] {
        self.doc.children(self.node)
    }

    fn next_sibling(&self) -> Option<Cursor<'a>> {
        let index = *self.path.last()? + 1;
        let parent = self.doc.parent(self.node)?;
        let node = *self.doc.children(parent).get(index as usize)?;
        let mut path = self.path.clone();
        *path.last_mut()? = index;

        Some(Cursor {
            doc: self.doc,
            path,
            node,
        })
    }

    fn move_to_parent(&mut self) -> Option<()> {
        if !self.path.is_empty() {
            let parent = self.doc.parent(self.node)?;
            self.node = parent;
            self.path.pop();
            Some(())
        } else {
            None
        }
    }

    // Advance in depth-first order
    fn advance(&mut self, skip_children: bool) -> Option<()> {
        if !skip_children {
            if let Some(node) = self.children().first() {
                self.path.push(0);
                self.node = *node;

                return Some(());
            }
        }

        let mut path = self.path.clone();
        let mut parent = self.doc.parent(self.node)?;

        loop {
            let index = *path.last()? + 1;

            if let Some(node) = self.doc.children(parent).get(index as usize) {
                *path.last_mut()? = index;

                self.path = path;
                self.node = *node;

                return Some(());
            } else {
                if path.len() <= 1 {
                    return None;
                }

                path.pop();

                parent = self.doc.parent(parent)?;
            }
        }
    }

    fn advance_with_skip_list(
        &mut self,
        skip_children: bool,
        skip_list: &BTreeSet<NodeRef>,
    ) -> Option<()> {
        let mut cursor = self.clone();

        while cursor.advance(skip_children).is_some() {
            if !skip_list.contains(&cursor.node) {
                std::mem::swap(self, &mut cursor);
                return Some(());
            }
        }

        None
    }
}

impl<'a> Iterator for Cursor<'a> {
    type Item = ();

    fn next(&mut self) -> Option<()> {
        self.advance(false)
    }
}

impl<'a> From<&'a Document> for Cursor<'a> {
    fn from(doc: &'a Document) -> Self {
        Cursor::new(doc, doc.root())
    }
}

impl<'a> Deref for Cursor<'a> {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        self.node()
    }
}

trait CompatibleWith: Deref<Target = Node> {
    fn is_compatible_with<T>(&self, other: &T) -> bool
    where
        T: Deref<Target = Node>,
    {
        match (self.deref(), other.deref()) {
            (Node::Element(from), Node::Element(to)) => {
                to.name.eq(&from.name) && to.id().eq(&from.id())
            }
            (Node::Leaf(_), Node::Leaf(_)) => true,
            (Node::Root, Node::Root) => true,
            _ => false,
        }
    }
}

impl<T> CompatibleWith for T where T: Deref<Target = Node> {}

#[derive(Debug)]
enum Op<'a> {
    Continue,
    /// Detach node and keyed descendants then remove node
    RemoveNode {
        node: NodeRef,
        /// A forked cursor of the node to be removed that iterates over descendant nodes
        cursor: Cursor<'a>,
        to: Cursor<'a>,
        detach: bool,
    },
    /// Remove all `from` nodes
    RemoveNodes {
        from: Cursor<'a>,
        to: Cursor<'a>,
    },
    /// Append `cursor` relative to the parent node last pushed to the stack by a patch operation
    Append {
        from: Cursor<'a>,
        cursor: Cursor<'a>,
    },
    /// Append `to`
    AppendNodes {
        from: Cursor<'a>,
        to: Cursor<'a>,
    },
    /// Append sibling nodes relative to the parent node on the stack
    AppendSiblings {
        from: Cursor<'a>,
        cursor: Cursor<'a>,
    },
    /// Inserts `node` before the current node and appends descendant nodes
    InsertBefore {
        from: Cursor<'a>,
        cursor: Cursor<'a>,
    },
    /// Detach node if not already detached
    MaybeDetach {
        node: NodeRef,
    },
    Morph(Cursor<'a>, Cursor<'a>),
    Patch(Patch),
}

impl Default for Op<'_> {
    fn default() -> Self {
        Op::Continue
    }
}

impl<'a, T> From<(T, T)> for Op<'a>
where
    T: Into<Cursor<'a>>,
{
    fn from((from, to): (T, T)) -> Self {
        Op::Morph(from.into(), to.into())
    }
}

enum Advance {
    BothCursors,
    To,
    From,
}

struct Morph<'a> {
    stack: SmallVec<[Op<'a>; 16]>,
    queue: SmallVec<[Op<'a>; 8]>,
    detached: BTreeSet<NodeRef>,
}

impl<'a> Morph<'a> {
    fn new(from: &'a Document, to: &'a Document) -> Self {
        (from, to).into()
    }

    // enqueue patches to morph attributes
    fn morph_current_attr(&mut self) {
        if let Some(Op::Morph(from, to)) = self.stack.last_mut() {
            let node = from.node;

            if let (Node::Element(old), Node::Element(new)) = (from.node(), to.node()) {
                let mut current = BTreeMap::from_iter(
                    old.attributes()
                        .into_iter()
                        .map(|attr| (&attr.name, &attr.value)),
                );

                self.queue
                    .extend(new.attributes().into_iter().filter_map(|attr| {
                        match current.remove(&attr.name) {
                            Some(value) if value.ne(&attr.value) => {
                                Some(Op::Patch(Patch::UpdateAttribute {
                                    node,
                                    name: attr.name.to_owned(),
                                    value: attr.value.to_owned(),
                                }))
                            }
                            Some(_) => None,
                            None => Some(Op::Patch(Patch::AddAttributeTo {
                                node,
                                name: attr.name.to_owned(),
                                value: attr.value.to_owned(),
                            })),
                        }
                    }));

                while let Some(patch) =
                    current
                        .pop_first()
                        .map(|(name, _)| Patch::RemoveAttributeByName {
                            node,
                            name: name.to_owned(),
                        })
                {
                    self.queue.push(Op::Patch(patch));
                }
            }
        }
    }

    fn advance(&mut self, advance: Advance, skip_children: bool) {
        let op = self.stack.last_mut().unwrap();

        if let Op::Morph(from, to) = op {
            let next = match advance {
                Advance::BothCursors => (
                    from.advance_with_skip_list(skip_children, &self.detached),
                    to.advance(skip_children),
                ),
                Advance::To => (Some(()), to.advance(skip_children)),
                Advance::From => (
                    from.advance_with_skip_list(skip_children, &self.detached),
                    Some(()),
                ),
            };

            match next {
                (Some(_), Some(_)) => {}
                (Some(_), None) => {
                    *op = Op::RemoveNodes {
                        from: from.to_owned(),
                        to: to.to_owned(),
                    };
                }
                (None, Some(_)) => {
                    *op = Op::AppendNodes {
                        from: from.to_owned(),
                        to: to.to_owned(),
                    };
                }
                (None, None) => {
                    *op = Op::Continue;
                }
            }
        }
    }
}

impl<'a, T> From<T> for Morph<'a>
where
    T: Into<Op<'a>>,
{
    fn from(op: T) -> Self {
        Morph {
            stack: smallvec![op.into()],
            queue: smallvec![],
            detached: BTreeSet::new(),
        }
    }
}

impl<'a> Iterator for Morph<'a> {
    type Item = Patch;

    fn next(&mut self) -> Option<Patch> {
        loop {
            while let Some(Op::Continue) = self.stack.last() {
                self.stack.pop();
            }

            self.stack.extend(self.queue.drain(..).rev());

            let op = self.stack.last_mut()?;

            match op {
                Op::Continue => {
                    unreachable!("Op::Continue should be popped off the stack prior")
                }
                Op::RemoveNode {
                    ref node,
                    cursor,
                    ref to,
                    detach,
                } => {
                    let node = *node;

                    if *detach {
                        *detach = false;

                        if !cursor.children().is_empty() {
                            self.queue.push(Op::Patch(Patch::Detach { node }));
                            continue;
                        }
                    }

                    if cursor.next().is_some() {
                        if let Node::Element(el) = cursor.node() {
                            if let Some(id) = el.id() {
                                if to.doc.get_by_id(id).is_some() {
                                    // Only detach if not previously moved
                                    if self.detached.insert(cursor.node) {
                                        self.queue
                                            .push(Op::Patch(Patch::Detach { node: cursor.node }));
                                        continue;
                                    }
                                }
                            }
                        }
                    }

                    *op = Op::Patch(Patch::Remove { node });
                }
                Op::RemoveNodes { from, to } => {
                    if !self.detached.contains(&from.node) {
                        self.queue.push(Op::RemoveNode {
                            node: from.node,
                            cursor: from.fork(),
                            to: to.fork(),
                            detach: true,
                        });
                    }

                    if from.advance(true).is_none() {
                        *op = Op::Continue;
                    }
                }
                Op::Append { ref from, cursor } => {
                    if let Some(id) = cursor.id() {
                        if let Some(node) = from.doc.get_by_id(id) {
                            self.queue.extend([
                                Op::MaybeDetach { node },
                                // Parent will already be on the stack so only need to push child
                                Op::Patch(Patch::Push(node)),
                                // Attach will pop node off the stack and append to parent
                                Op::Patch(Patch::Attach),
                                Op::Morph(from.at(node), cursor.fork()),
                            ]);

                            if cursor.advance(true).is_none() {
                                *op = Op::Continue;
                            }

                            continue;
                        }
                    }

                    let node = cursor.node().to_owned();
                    let depth = cursor.depth();

                    if cursor.next().is_some() {
                        match cursor.depth().cmp(&depth) {
                            Ordering::Less => {
                                unreachable!(
                                    "cursor should always be forked so that it will never move up"
                                );
                            }
                            // Next node is also a sibling, so leave parent unchanged and append current
                            Ordering::Equal => {
                                self.queue.push(Op::Patch(Patch::Append { node }));
                                continue;
                            }
                            // Next node is a child node, so append and make node the new parent
                            Ordering::Greater => {
                                self.queue.extend([
                                    Op::Patch(Patch::CreateAndMoveTo { node }),
                                    // Insertion point resets to parent
                                    Op::Patch(Patch::Attach),
                                    // Move to newly created node relative to parent
                                    Op::Patch(Patch::Move(MoveTo::ReverseChild(0))),
                                    // Set created node as parent for next append
                                    Op::Patch(Patch::PushCurrent),
                                    Op::Append {
                                        from: from.clone(),
                                        cursor: cursor.fork(),
                                    },
                                    Op::Patch(Patch::Pop),
                                ]);

                                if cursor.advance(true).is_none() {
                                    *op = Op::Continue;
                                }
                            }
                        }
                    } else {
                        // self.queue.push(Op::Patch(Patch::Append { node }));

                        self.queue.extend([
                            Op::Patch(Patch::CreateAndMoveTo { node }),
                            // Insertion point resets to parent
                            Op::Patch(Patch::Attach),
                        ]);

                        *op = Op::Continue;
                    }
                }
                Op::AppendNodes { from, to } => {
                    // Move cursor to parent for append
                    while from.depth().ge(&to.depth()) {
                        from.move_to_parent();
                    }

                    self.queue.extend([
                        Op::Patch(Patch::Push(from.node)),
                        Op::AppendSiblings {
                            from: from.clone(),
                            cursor: to.clone(),
                        },
                        Op::Patch(Patch::Pop),
                    ]);

                    while to.move_to_parent().is_some() {
                        if to.advance(true).is_some() {
                            continue;
                        }
                    }

                    *op = Op::Continue;
                }
                Op::AppendSiblings {
                    ref from,
                    cursor: to,
                } => {
                    self.queue.push(Op::Append {
                        from: from.clone(),
                        cursor: to.fork(),
                    });

                    if let Some(next) = to.next_sibling() {
                        *to = next;
                    } else {
                        *op = Op::Continue;
                    }
                }
                Op::InsertBefore { ref from, cursor } => {
                    let node = cursor.node().to_owned();

                    if cursor.next().is_some() {
                        self.queue.extend([
                            Op::Patch(Patch::CreateAndMoveTo { node }),
                            Op::Patch(Patch::PrependBefore { before: from.node }),
                            // Set newly inserted node as append parent
                            Op::Patch(Patch::PushCurrent),
                            Op::Append {
                                from: from.clone(),
                                cursor: cursor.fork(),
                            },
                            Op::Patch(Patch::Pop),
                        ]);

                        *op = Op::Continue;
                    } else {
                        *op = Op::Patch(Patch::InsertBefore {
                            before: from.node,
                            node,
                        });
                    }
                }
                Op::MaybeDetach { ref node } => {
                    // If a node was previously moved, it will have already been set as detached and can be ignored
                    if self.detached.insert(*node) {
                        *op = Op::Patch(Patch::Detach { node: *node });
                    } else {
                        *op = Op::Continue;
                    }
                }
                Op::Morph(from, to) => {
                    // paths are relative; a forked cursor without siblings will have a depth of 0
                    match to.depth().cmp(&from.depth()) {
                        // node added as child
                        Ordering::Greater => {
                            self.queue.extend([
                                Op::Patch(Patch::Push(from.node)),
                                Op::Append {
                                    from: from.clone(),
                                    cursor: to.fork(),
                                },
                                Op::Patch(Patch::Pop),
                            ]);

                            self.advance(Advance::To, true);
                            continue;
                        }
                        Ordering::Equal => {}
                        // existing node deleted
                        Ordering::Less => {
                            self.queue.push(Op::RemoveNode {
                                node: from.node,
                                cursor: from.fork(),
                                to: to.fork(),
                                detach: true,
                            });

                            self.advance(Advance::From, true);
                            continue;
                        }
                    }

                    match (from.node(), to.node()) {
                        (Node::Root, Node::Root) | (Node::Root, _) | (_, Node::Root) => {
                            self.advance(Advance::BothCursors, false);
                        }
                        (Node::Leaf(old_content), Node::Leaf(content)) => {
                            if old_content.ne(content) {
                                self.queue.push(Op::Patch(Patch::Replace {
                                    node: from.node,
                                    replacement: Node::Leaf(content.to_owned()),
                                }));
                            }

                            self.advance(Advance::BothCursors, false);
                        }
                        (Node::Leaf(_), Node::Element(_)) => {
                            self.queue
                                .push(Op::Patch(Patch::Remove { node: from.node }));

                            self.advance(Advance::From, true);
                        }
                        (Node::Element(_), Node::Leaf(content)) => {
                            self.queue.push(Op::Patch(Patch::InsertBefore {
                                before: from.node,
                                node: Node::Leaf(content.to_owned()),
                            }));

                            self.advance(Advance::To, true);
                        }
                        (Node::Element(from_el), Node::Element(to_el)) => {
                            // nodes are compatible; morph attribute changes and continue
                            if to_el.name.eq(&from_el.name) && to_el.id().eq(&from_el.id()) {
                                self.morph_current_attr();

                                self.advance(Advance::BothCursors, false);
                                continue;
                            }

                            // Keyed node shouldn't be here; detach/remove and continue
                            if let Some(id) = from.id() {
                                if to.doc.get_by_id(id).is_some() {
                                    self.queue.push(Op::MaybeDetach { node: from.node });
                                } else {
                                    self.queue.push(Op::RemoveNode {
                                        node: from.node,
                                        cursor: from.fork(),
                                        to: to.fork(),
                                        detach: true,
                                    });
                                }

                                self.advance(Advance::From, true);
                                continue;
                            }

                            // If keyed el should be here, relocated or insert instead of transforming el
                            if let Some(id) = to.id() {
                                if let Some(node) = from.doc.get_by_id(id) {
                                    self.queue.extend([
                                        Op::Patch(Patch::Push(node)),
                                        Op::MaybeDetach { node },
                                        Op::Patch(Patch::PrependBefore { before: from.node }),
                                        Op::Morph(from.at(node), to.fork()),
                                    ]);

                                    continue;
                                } else {
                                    self.queue.push(Op::InsertBefore {
                                        from: from.clone(),
                                        cursor: to.fork(),
                                    });

                                    self.advance(Advance::To, true);
                                    continue;
                                }
                            }

                            // If the next existing el can be morphed into the target el, delete current instead of replacing
                            if let Some(from_next) = from.next_sibling() {
                                if from_next.is_compatible_with(to) {
                                    self.queue.push(Op::RemoveNode {
                                        node: from.node,
                                        cursor: from.fork(),
                                        to: to.fork(),
                                        detach: true,
                                    });

                                    self.advance(Advance::From, true);
                                    continue;
                                }
                            }

                            // If the next node being morphed into is compatible, insert target node before current
                            if let Some(to_next) = to.next_sibling() {
                                if to_next.is_compatible_with(from) {
                                    self.queue.push(Op::InsertBefore {
                                        from: from.clone(),
                                        cursor: to.fork(),
                                    });

                                    self.advance(Advance::To, true);
                                    continue;
                                }
                            }

                            // TODO: as an optimization, use peek to add node caching
                            self.queue.push(Op::Patch(Patch::Replace {
                                node: from.node,
                                replacement: Node::Element(to_el.to_owned()),
                            }));

                            self.advance(Advance::BothCursors, false);
                        }
                    }
                }
                Op::Patch(_) => {
                    if let Op::Patch(patch) = mem::replace(op, Op::Continue) {
                        return Some(patch);
                    } else {
                        unreachable!();
                    }
                }
            }
        }
    }
}

pub fn diff(old_document: &Document, new_document: &Document) -> VecDeque<Patch> {
    VecDeque::from_iter(Morph::new(old_document, new_document))
}
