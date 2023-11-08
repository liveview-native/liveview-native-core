use std::fmt;

use super::{Document, Node, NodeRef};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PrintOptions {
    /// Prints a document/fragment without any extra whitespace (indentation/whitespace)
    Minified,
    /// Prints a document/fragment with each element open/closed on it's own line,
    /// and indented based on the level of nesting in the document.
    Pretty,
}
impl PrintOptions {
    #[inline(always)]
    pub fn pretty(&self) -> bool {
        self == &Self::Pretty
    }
}

pub struct Printer<'a> {
    doc: &'a Document,
    root: NodeRef,
    options: PrintOptions,
    indent: usize,
}
impl<'a> Printer<'a> {
    pub fn new(doc: &'a Document, root: NodeRef, options: PrintOptions) -> Self {
        Self {
            doc,
            root,
            options,
            indent: 0,
        }
    }

    pub fn print(mut self, writer: &mut dyn fmt::Write) -> fmt::Result {
        use petgraph::visit::{depth_first_search, DfsEvent};

        let mut first = true;
        depth_first_search(self.doc, Some(self.root), |event| {
            match event {
                DfsEvent::Discover(node, _) => {
                    // We're encountering `node` for the first time
                    match &self.doc.nodes[node] {
                        Node::Element { element: elem } => {
                            let pretty = self.options.pretty();
                            let self_closing = self.doc.children[node].is_empty();
                            if pretty {
                                if !first {
                                    writer.write_char('\n')?;
                                } else {
                                    first = false;
                                }
                                indent(self.indent, writer)?;
                            }
                            write!(writer, "<{}", &elem.name)?;
                            let attrs = elem.attributes();
                            if !attrs.is_empty() {
                                for attr in attrs.iter() {
                                    write!(writer, " {}=\"{}\"", &attr.name, &attr.value.clone().unwrap_or_default())?
                                }
                            }
                            if self_closing {
                                writer.write_str(" />")
                            } else {
                                if pretty {
                                    self.indent += 1;
                                }
                                writer.write_str(">")
                            }
                        }
                        Node::Leaf { leaf: content } => {
                            if self.options.pretty() {
                                if !first {
                                    writer.write_char('\n')?;
                                } else {
                                    first = false;
                                }
                                indent(self.indent, writer)?;
                            }
                            writer.write_str(content.as_str())
                        }
                        Node::Root => Ok(()),
                    }
                }
                DfsEvent::Finish(node, _) => {
                    // We've visited all the children of `node`
                    if let Node::Element { element: elem } = &self.doc.nodes[node] {
                        let self_closing = self.doc.children[node].is_empty();
                        if self_closing {
                            return Ok(());
                        }
                        if self.options.pretty() {
                            writer.write_char('\n')?;
                            self.indent -= 1;
                            indent(self.indent, writer)?;
                        }
                        write!(writer, "</{}>", &elem.name)?
                    }
                    Ok(())
                }
                _ => Ok(()),
            }
        })
    }
}

fn indent(mut n: usize, writer: &mut dyn fmt::Write) -> fmt::Result {
    const INDENT: &'static str = "    ";
    while n > 0 {
        writer.write_str(INDENT)?;
        n -= 1;
    }
    Ok(())
}
