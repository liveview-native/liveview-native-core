use std::{cmp::Ordering, collections::BTreeSet, fmt, mem, ops::Deref};

use smallvec::{smallvec, SmallVec};

use super::{MoveTo, Patch};
use crate::dom::*;

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

    fn node(&self) -> &NodeData {
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

    // Create a cursor with the current node that iterates over nodes descendant of the ancestor at `ancestor_depth`
    fn fork_relative(&self, ancestor_depth: usize) -> Cursor<'a> {
        Cursor {
            doc: self.doc,
            path: SmallVec::from_slice(&self.path.as_slice()[ancestor_depth..]),
            node: self.node,
        }
    }

    // Create a cursor at `node` that iterates over descendant nodes
    fn at(&self, node: NodeRef) -> Cursor<'a> {
        Cursor::new(self.doc, node)
    }

    fn parent(&self) -> Option<NodeRef> {
        self.doc.parent(self.node)
    }
    fn children(&self) -> &'a [NodeRef] {
        self.doc.children(self.node)
    }

    fn next_sibling(&self) -> Option<Cursor<'a>> {
        let parent = self.parent()?;
        let siblings = self.doc.children(parent);
        let index = siblings.iter().position(|node| self.node.eq(node))? + 1;
        let node = siblings.get(index)?;

        Some(Cursor {
            doc: self.doc,
            path: smallvec![],
            node: *node,
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
    type Target = NodeData;

    fn deref(&self) -> &Self::Target {
        self.node()
    }
}

trait CompatibleWith: Deref<Target = NodeData> {
    fn is_compatible_with<T>(&self, other: &T) -> bool
    where
        T: Deref<Target = NodeData>,
    {
        match (self.deref(), other.deref()) {
            (NodeData::NodeElement { element: from }, NodeData::NodeElement { element: to }) => {
                to.name.eq(&from.name) && to.id().eq(&from.id())
            }
            (NodeData::Leaf { value: _ }, NodeData::Leaf { value: _ }) => true,
            (NodeData::Root, NodeData::Root) => true,
            _ => false,
        }
    }
}

impl<T> CompatibleWith for T where T: Deref<Target = NodeData> {}

#[derive(Debug)]
#[derive(Default)]
enum Op<'a> {
    #[default]
    Continue,
    /// Detach node and keyed descendants then remove node
    RemoveNode {
        node: NodeRef,
        /// A forked cursor of the node to be removed that iterates over descendant nodes
        cursor: Cursor<'a>,
        to: Cursor<'a>,
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
    /// Append `to` relative to `insertion_point`
    AppendNodes {
        insertion_point: Cursor<'a>,
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

pub struct Morph<'a> {
    stack: SmallVec<[Op<'a>; 16]>,
    queue: SmallVec<[Op<'a>; 8]>,
    detached: BTreeSet<NodeRef>,
}

impl<'a> Morph<'a> {
    pub fn new(from: &'a Document, to: &'a Document) -> Self {
        (from, to).into()
    }

    fn advance(&mut self, advance: Advance, skip_children: bool) {
        let op = self.stack.last_mut().unwrap();

        if let Op::Morph(from, to) = op {
            let insertion_point = from.clone();

            let next = match advance {
                Advance::BothCursors => (from.advance(skip_children), to.advance(skip_children)),
                Advance::To => (Some(()), to.advance(skip_children)),
                Advance::From => (from.advance(skip_children), Some(())),
            };

            match next {
                (Some(_), Some(_)) => {
                    match to.depth().cmp(&from.depth()) {
                        Ordering::Less | Ordering::Equal => {
                            // If the node was detached earlier, that means it's already been reattached and can be skipped over
                            if self.detached.contains(&from.node) {
                                self.advance(Advance::From, true);
                            }
                        }
                        // New nodes added
                        Ordering::Greater => {
                            let depth = from.depth();

                            self.queue.push(Op::AppendNodes {
                                insertion_point: insertion_point.fork_relative(depth),
                                to: to.fork_relative(depth),
                            });

                            while to.depth().gt(&depth) {
                                to.move_to_parent();
                            }

                            if to.advance(true).is_none() {
                                *op = Op::RemoveNodes {
                                    from: from.to_owned(),
                                    to: to.to_owned(),
                                };
                            }
                        }
                    }
                }
                (Some(_), None) => {
                    if self.detached.contains(&from.node) {
                        self.advance(Advance::From, true);
                    } else {
                        *op = Op::RemoveNodes {
                            from: from.to_owned(),
                            to: to.to_owned(),
                        };
                    }
                }
                (None, Some(_)) => {
                    *op = Op::AppendNodes {
                        insertion_point,
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
                } => {
                    if cursor.next().is_some() {
                        if let NodeData::NodeElement { element: el } = cursor.node() {
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

                    *op = Op::Patch(Patch::Remove { node: *node });
                }
                Op::RemoveNodes { from, to } => {
                    if !self.detached.contains(&from.node) {
                        self.queue.push(Op::RemoveNode {
                            node: from.node,
                            cursor: from.fork(),
                            to: to.fork(),
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

                            *op = Op::Continue;

                            continue;
                        }
                    }

                    let node = cursor.node().to_owned();
                    if cursor.next().is_some() {
                        self.queue.extend([
                            Op::Patch(Patch::CreateAndMoveTo { node }),
                            // Attach relative to current parent on the stack
                            Op::Patch(Patch::Attach),
                            // Move to newly created node relative to parent
                            Op::Patch(Patch::Move(MoveTo::ReverseChild(0))),
                            // Set created node as parent for inner append
                            Op::Patch(Patch::PushCurrent),
                            Op::AppendSiblings {
                                from: from.clone(),
                                cursor: cursor.fork(),
                            },
                            Op::Patch(Patch::Pop),
                        ]);
                    } else {
                        self.queue.extend([
                            Op::Patch(Patch::Create { node }),
                            // Attach relative to current parent on the stack
                            Op::Patch(Patch::Attach),
                        ]);
                    }

                    *op = Op::Continue;
                }
                Op::AppendNodes {
                    insertion_point,
                    to,
                } => {
                    // Move cursor to parent for append
                    while insertion_point.depth().ge(&to.depth()) {
                        insertion_point.move_to_parent();
                    }

                    self.queue.extend([
                        Op::Patch(Patch::Push(insertion_point.node)),
                        Op::AppendSiblings {
                            from: insertion_point.clone(),
                            cursor: to.clone(),
                        },
                        Op::Patch(Patch::Pop),
                    ]);

                    loop {
                        if to.move_to_parent().is_some() {
                            if to.advance(true).is_some() {
                                break;
                            }
                        } else {
                            *op = Op::Continue;
                            break;
                        }
                    }
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
                    if to.depth().lt(&from.depth()) {
                        self.queue.push(Op::RemoveNode {
                            node: from.node,
                            cursor: from.fork(),
                            to: to.fork(),
                        });

                        self.advance(Advance::From, true);
                        continue;
                    }

                    match (from.node(), to.node()) {
                        (NodeData::Root, NodeData::Root) | (NodeData::Root, _) | (_, NodeData::Root) => {
                            self.advance(Advance::BothCursors, false);
                        }
                        (NodeData::Leaf { value: old_content }, NodeData::Leaf { value: content }) => {
                            if old_content.ne(content) {
                                self.queue.push(Op::Patch(Patch::Replace {
                                    node: from.node,
                                    replacement: NodeData::Leaf { value: content.to_owned() },
                                }));
                            }

                            self.advance(Advance::BothCursors, false);
                        }
                        (NodeData::Leaf { value: _ }, NodeData::NodeElement { element: _ }) => {
                            self.queue
                                .push(Op::Patch(Patch::Remove { node: from.node }));

                            self.advance(Advance::From, true);
                        }
                        (NodeData::NodeElement { element: _ }, NodeData::Leaf { value: content }) => {
                            self.queue.push(Op::Patch(Patch::InsertBefore {
                                before: from.node,
                                node: NodeData::Leaf { value: content.to_owned() },
                            }));

                            self.advance(Advance::To, true);
                        }
                        (NodeData::NodeElement { element: from_el }, NodeData::NodeElement { element: to_el }) => {
                            // nodes are compatible; morph attribute changes and continue
                            if to_el.name.eq(&from_el.name) && to_el.id().eq(&from_el.id()) {
                                if from_el.attributes.ne(&to_el.attributes) {
                                    self.queue.push(Op::Patch(Patch::SetAttributes {
                                        node: from.node,
                                        attributes: to.attributes().to_vec(),
                                    }));
                                }

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

                                    self.advance(Advance::To, true);

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
                                replacement: NodeData::NodeElement { element: to_el.to_owned() },
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

pub fn diff(old_document: &Document, new_document: &Document) -> Vec<Patch> {
    Vec::from_iter(Morph::new(old_document, new_document))
}
