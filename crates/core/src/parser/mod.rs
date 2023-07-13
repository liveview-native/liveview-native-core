use std::{
    assert_matches::assert_matches, borrow::Cow, collections::VecDeque, convert::Infallible, fmt,
    mem,
};

use html5gum::{Emitter, Error, Readable, Reader, State, Tokenizer};
use smallstr::SmallString;
use smallvec::SmallVec;

use crate::{dom::*, symbols, InternedString};

/// Parses a `Document` from the given input
pub fn parse<'a, R>(input: R) -> Result<Document, ParseError>
where
    R: Readable<'a>,
    ParseError: From<<<R as Readable<'a>>::Reader as Reader>::Error>,
{
    let mut document = Document::empty();
    let emitter = DocumentEmitter::new();
    let mut current_node = document.root();
    for token in Tokenizer::new_with_emitter(input, emitter) {
        match token? {
            Token::Start(StartToken {
                mut ids, element, ..
            }) => {
                let node = document.push_node(element);
                document.append_child(current_node, node);
                current_node = node;
                for id in ids.drain(..) {
                    document.register_id(node, id);
                }
            }
            Token::End(_) => {
                current_node = document.parent(current_node).unwrap();
            }
            Token::String(content) => {
                let node = document.push_node(content);
                document.append_child(current_node, node);
            }
            Token::Error(err) => {
                return Err(ParseError::Tokenizer(err.into()));
            }
            Token::Doctype(_) => {}
            Token::Comment => {}
        }
    }

    Ok(document)
}

/// Represents the possible types of failure that can occur while parsing a `Document`
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("could not read document from input: {0}")]
    Reader(#[from] std::io::Error),
    #[error("encountered an error while tokenizing input: {0}")]
    Tokenizer(#[from] TokenizerError),
}
impl From<Infallible> for ParseError {
    #[inline(always)]
    fn from(_err: Infallible) -> Self {
        unreachable!()
    }
}
impl From<html5gum::Error> for ParseError {
    #[inline(always)]
    fn from(err: html5gum::Error) -> Self {
        Self::Tokenizer(err.into())
    }
}

/// Wraps `html5gum::Error` to implement `std::error::Error`
#[derive(Debug)]
#[repr(transparent)]
pub struct TokenizerError(html5gum::Error);
impl fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.as_str())
    }
}
impl From<html5gum::Error> for TokenizerError {
    #[inline(always)]
    fn from(err: html5gum::Error) -> Self {
        Self(err)
    }
}
impl std::error::Error for TokenizerError {}

#[derive(Debug)]
struct StartToken {
    ids: Vec<SmallString<[u8; 16]>>,
    element: Element,
    self_closing: bool,
}

#[derive(Debug)]
enum Token {
    /// When an element is first encountered, we create its Node, and implicitly move down a level in the element tree
    Start(StartToken),
    /// When an element is closed, we know that we're moving back up the element tree
    End(ElementName),
    /// Like `Start`, but for leaf nodes containing plain text
    String(SmallString<[u8; 16]>),
    /// Comments are ignored
    Comment,
    /// Doctype is used to determine what kind of document is being created
    Doctype(InternedString),
    Error(Error),
}
impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Start(x), Self::Start(y)) => x.element.name == y.element.name,
            (Self::End(x), Self::End(y)) => x == y,
            (Self::String(x), Self::String(y)) => x == y,
            (Self::Doctype(x), Self::Doctype(y)) => x == y,
            (Self::Error(x), Self::Error(y)) => x == y,
            (Self::Comment, Self::Comment) => true,
            _ => false,
        }
    }
}

/// This is an emitter tailored for our use case and desired parsing behavior
///
/// Compared to the default emitter provided by `html5gum`, ours makes the following changes
///
/// * Duplicate attributes are not ignored, but respected
/// * Comments are dropped
/// * All strings are interned
/// * We allocate all nodes/attributes/etc via a Document during tokenization, then use
/// the emitted tokens to construct the actual element tree (i.e. connect )
/// construct the
struct DocumentEmitter {
    current_characters: SmallVec<[u8; 16]>,
    current_token: Option<Token>,
    current_tag: SmallVec<[u8; 16]>,
    current_attribute: Option<(SmallVec<[u8; 16]>, SmallVec<[u8; 16]>)>,
    current_doctype: SmallVec<[u8; 16]>,
    last_start_tag: InternedString,
    emitted_tokens: VecDeque<Token>,
}
impl DocumentEmitter {
    pub fn new() -> Self {
        Self {
            current_characters: Default::default(),
            current_token: None,
            current_tag: Default::default(),
            current_attribute: None,
            current_doctype: Default::default(),
            last_start_tag: symbols::Empty.into(),
            emitted_tokens: VecDeque::new(),
        }
    }

    fn emit_token(&mut self, token: Token) {
        self.flush_current_characters();
        self.emitted_tokens.push_back(token);
    }

    fn flush_current_attribute(&mut self) {
        if let Some((k, v)) = self.current_attribute.take() {
            match self.current_token.as_mut().unwrap() {
                Token::Start(StartToken {
                    ref mut ids,
                    ref mut element,
                    ..
                }) => {
                    let k = smallvec_to_smallstr(k);
                    let v = smallvec_to_smallstr(v);
                    if k.as_str() == "id" {
                        ids.push(v.clone());
                    }
                    element.set_attribute(k.as_str().into(), v.into());
                }
                other => invalid_state("invalid state in which to flush a token", Some(other)),
            }
        }
    }

    fn flush_current_characters(&mut self) {
        if self.current_characters.is_empty() {
            return;
        }
        let s = mem::take(&mut self.current_characters);
        let string = smallvec_to_smallstr_trimmed(s);
        if string.is_empty() {
            return;
        }
        self.emit_token(Token::String(string));
    }
}

impl Emitter for DocumentEmitter {
    type Token = Token;

    fn set_last_start_tag(&mut self, last_start_tag: Option<&[u8]>) {
        match last_start_tag {
            None => {
                self.last_start_tag = symbols::Empty.into();
            }
            Some(bytes) => {
                self.last_start_tag = String::from_utf8_lossy(bytes).into();
            }
        }
    }

    #[inline(never)]
    fn emit_eof(&mut self) {
        self.flush_current_characters();
    }

    #[inline(never)]
    fn emit_error(&mut self, error: Error) {
        self.emitted_tokens.push_back(Token::Error(error));
    }

    #[inline]
    fn pop_token(&mut self) -> Option<Self::Token> {
        self.emitted_tokens.pop_front()
    }

    #[inline]
    fn emit_string(&mut self, s: &[u8]) {
        self.current_characters.extend_from_slice(s);
    }

    #[inline]
    fn init_start_tag(&mut self) {
        self.current_token = Some(Token::Start(StartToken {
            ids: vec![],
            element: Element::new(symbols::Empty.into()),
            self_closing: false,
        }));
    }

    #[inline]
    fn init_end_tag(&mut self) {
        self.current_token = Some(Token::End(symbols::Empty.into()));
    }

    #[inline(always)]
    fn init_comment(&mut self) {
        self.current_token = Some(Token::Comment);
    }

    fn emit_current_tag(&mut self) -> Option<State> {
        self.flush_current_attribute();
        match self.current_token.take().unwrap() {
            Token::Start(StartToken {
                ids,
                mut element,
                self_closing,
            }) => {
                assert!(!self.current_tag.is_empty());
                let tag = smallvec_to_smallstr(mem::take(&mut self.current_tag));
                element.name = tag.as_str().into();
                if self_closing {
                    let end_tag = element.name;
                    self.emit_token(Token::Start(StartToken {
                        ids,
                        element,
                        self_closing,
                    }));
                    self.emit_token(Token::End(end_tag));
                    None
                } else {
                    self.last_start_tag = element.name.into();
                    self.emit_token(Token::Start(StartToken {
                        ids,
                        element,
                        self_closing,
                    }));
                    html5gum::naive_next_state(self.last_start_tag.as_str().as_bytes())
                }
            }
            Token::End(_) => {
                assert!(!self.current_tag.is_empty());
                let t = smallvec_to_smallstr(mem::take(&mut self.current_tag));
                self.emit_token(Token::End(t.as_str().into()));
                html5gum::naive_next_state(self.last_start_tag.as_str().as_bytes())
            }
            other => invalid_state("invalid state in which to emit tag", Some(&other)),
        }
    }

    fn emit_current_comment(&mut self) {
        assert_eq!(self.current_token.take().unwrap(), Token::Comment);
        self.emit_token(Token::Comment);
    }

    fn emit_current_doctype(&mut self) {
        assert_matches!(self.current_token.take().unwrap(), Token::Doctype(_));
        let doctype = smallvec_to_smallstr(mem::take(&mut self.current_doctype));
        self.emit_token(Token::Doctype(doctype.into()));
    }

    #[inline(always)]
    fn set_self_closing(&mut self) {
        match self.current_token.as_mut().unwrap() {
            Token::Start(StartToken {
                ref mut self_closing,
                ..
            }) => {
                *self_closing = true;
            }
            other => invalid_state(
                "invalid state in which to mark a tag self-closing",
                Some(other),
            ),
        }
    }

    #[inline(always)]
    fn set_force_quirks(&mut self) {}

    #[inline]
    fn push_tag_name(&mut self, s: &[u8]) {
        self.current_tag.extend_from_slice(s);
    }

    #[inline(always)]
    fn push_comment(&mut self, _s: &[u8]) {}

    fn push_doctype_name(&mut self, s: &[u8]) {
        self.current_doctype.extend_from_slice(s);
    }

    fn init_doctype(&mut self) {
        self.current_token = Some(Token::Doctype(symbols::Empty.into()));
    }

    #[inline]
    fn init_attribute(&mut self) {
        self.flush_current_attribute();
        self.current_attribute = Some(Default::default());
    }

    #[inline]
    fn push_attribute_name(&mut self, s: &[u8]) {
        self.current_attribute
            .as_mut()
            .unwrap()
            .0
            .extend_from_slice(s);
    }

    #[inline]
    fn push_attribute_value(&mut self, s: &[u8]) {
        self.current_attribute
            .as_mut()
            .unwrap()
            .1
            .extend_from_slice(s);
    }

    #[inline(always)]
    fn set_doctype_public_identifier(&mut self, _value: &[u8]) {}
    #[inline(always)]
    fn set_doctype_system_identifier(&mut self, _value: &[u8]) {}
    #[inline(always)]
    fn push_doctype_public_identifier(&mut self, _value: &[u8]) {}
    #[inline(always)]
    fn push_doctype_system_identifier(&mut self, _value: &[u8]) {}

    fn current_is_appropriate_end_tag_token(&mut self) -> bool {
        match self.current_token {
            Some(Token::End(tag)) => !(self.last_start_tag == "") && tag == self.last_start_tag,
            _ => false,
        }
    }
}

#[inline]
fn smallvec_to_smallstr_trimmed(vec: SmallVec<[u8; 16]>) -> SmallString<[u8; 16]> {
    match String::from_utf8_lossy(vec.as_slice()) {
        Cow::Borrowed(s) => SmallString::from_str(s.trim()),
        Cow::Owned(s) => SmallString::from_str(s.trim()),
    }
}

#[inline]
fn smallvec_to_smallstr(vec: SmallVec<[u8; 16]>) -> SmallString<[u8; 16]> {
    match String::from_utf8_lossy(vec.as_slice()) {
        Cow::Borrowed(s) => SmallString::from_str(s),
        Cow::Owned(s) => SmallString::from_string(s),
    }
}

#[cold]
#[inline(never)]
fn invalid_state(msg: &str, token: Option<&Token>) -> ! {
    panic!("{}: {:#?}", msg, token)
}
