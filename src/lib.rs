use std::collections::VecDeque;
use std::io::{self, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SamupError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("bad stack: expected {expected} got {got}")]
    BadStack { expected: Tag, got: Tag },
    #[error("bad stack: expected {expected} got None")]
    ShortStack { expected: Tag },
    #[error("syntax error")]
    Syntax,
}

impl SamupError {
    fn stack(expected: Tag, got: Option<Tag>) -> Self {
        if let Some(got) = got {
            Self::BadStack { expected, got }
        } else {
            Self::ShortStack { expected }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum C {
    Whitespace,
    Newline,
    Underscore,
    Asterisk,
    // NOTE: only in FootNoteLink or FootNoteRef
    Caret,
    // NOTE: only in FootNoteRef
    Colon,
    SqBracketL,
    SqBracketR,
    ParenL,
    ParenR,
    // Quote,
    Digit, // for footnotes
    Content,
}

impl C {
    fn as_char(&self) -> 
}

impl From<u8> for C {
    fn from(v: u8) -> C {
        match v {
            // space | \t
            32 | 9 => C::Whitespace,
            // \n | \r
            10 | 13 => C::Newline,
            // _
            95 => C::Underscore,
            // *
            42 => C::Asterisk,
            // ^ (NOTE: only in FootNoteLink or FootNoteRef)
            94 => C::Caret,
            // : (NOTE: only in FootNoteRef)
            58 => C::Colon,
            // [
            91 => C::SqBracketL,
            // ]
            93 => C::SqBracketR,
            // (
            40 => C::ParenL,
            // )
            41 => C::ParenR,
            // "
            // 34 => C::Quote,
            // 0..=9
            48..=57 => C::Digit,
            _ => C::Content,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tag {
    H1,
    H2,
    I,
    P,
    Strong,
    Link(String),
    // ...[^1]
    FootNoteLink(u8),
    // [^1]: ...
    FootNoteRef(u8),
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Tag::H1 => f.write_str("<h1>"),
            Tag::H2 => f.write_str("<h2>"),
            Tag::I => f.write_str("<i>"),
            Tag::P => f.write_str("<p>"),
            Tag::Strong => f.write_str("<strong>"),
            Tag::Link(url) => f.write_fmt(format_args!("<link: {url}>")),
            Tag::FootNoteLink(n) => f.write_fmt(format_args!("<footnote link {n}>")),
            Tag::FootNoteRef(n) => f.write_fmt(format_args!("<footnote ref {n}>")),
        }
    }
}

impl Tag {
    fn write_open<O: Write>(&self, output: &mut O) -> Result<(), io::Error> {
        match self {
            Tag::H1 => output.write_all(b"<h1>"),
            Tag::H2 => output.write_all(b"<h2>"),
            Tag::I => output.write_all(b"<i>"),
            Tag::P => output.write_all(b"<p>"),
            Tag::Strong => output.write_all(b"<strong>"),
            Tag::Link(url) => write!(output, "<a href=\"{url}\" target=\"_blank\">"),
            Tag::FootNoteLink(note_no) => write!(
                output,
                "<a id=\"link-{note_no}\" target=\"#ref-{note_no}\"><sup>{note_no}</sup></a>"
            ),
            Tag::FootNoteRef(note_no) => write!(
                output,
                "<p class=\"footnote\" id=\"ref-{note_no}\"><span class=\"footnote\">{note_no}:</span> "
            ),
        }
    }
    fn write_close<O: Write>(&self, output: &mut O) -> Result<(), io::Error> {
        match self {
            Tag::H1 => output.write_all(b"</h1>"),
            Tag::H2 => output.write_all(b"</h2>"),
            Tag::I => output.write_all(b"</i>"),
            Tag::P => output.write_all(b"</p>"),
            Tag::Strong => output.write_all(b"</strong>"),
            Tag::Link(_) => write!(output, "</a>"),
            Tag::FootNoteLink(_) => Ok(()),
            Tag::FootNoteRef(note_no) => {
                write!(output, "<a href=\"#link-{note_no}\">\u{1f519}</a></p>")
            }
        }
    }
    fn write_link_no_title<O: Write>(&self, output: &mut O) -> Result<(), io::Error> {
        if let Tag::Link(url) = self {
            write!(output, "<a href=\"{url}\" target=\"_blank\">{url}</a>")
        } else {
            panic!()
        }
    }
}

#[derive(Debug)]
pub struct ParserState {
    ix: usize,
    prev_c: C,
    tag_stack: VecDeque<Tag>,
}

impl ParserState {
    fn new() -> Self {
        Self {
            ix: 0,
            prev_c: C::Whitespace,
            tag_stack: VecDeque::new(),
        }
    }
    pub fn parse<O: Write>(&mut self, input: &[u8], output: &mut O) -> Result<(), SamupError> {
        let curr_char = input[self.ix];
        let curr_c: C = curr_char.into();
        let next_c = match curr_c {
            C::Whitespace => self.parse_whitespace(curr_char, output)?,
            C::Newline => self.parse_newline(curr_char, output)?,
            C::Underscore => self.parse_underscore(output)?,
            C::Asterisk => self.parse_asterisk(output)?,
            C::Caret => self.parse_caret(output)?,
            C::Colon => self.parse_colon(output)?,
            C::SqBracketL => self.parse_sq_bracket_l(output)?,
            C::SqBracketR => self.parse_sq_bracket_r(output)?,
            C::ParenL => self.parse_paren_l(output)?,
            C::ParenR => self.parse_paren_r(output)?,
            C::Digit => self.parse_digit(curr_char, output)?,
            C::Content => {
                output.write_all(&[curr_char])?;
                None
            }
        };
        self.prev_c = next_c.unwrap_or(curr_c);
        self.ix += 1;
        Ok(())
    }
    fn parse_whitespace<O: Write>(
        &mut self,
        curr_char: u8,
        output: &mut O,
    ) -> Result<Option<C>, SamupError> {
        match self.prev_c {
            // `  ` | `\n ` | `C `
            C::Whitespace | C::Newline | C::Content => {
                output.write_all(&[curr_char])?;
            }
            // `_ `
            C::Underscore => match self.tag_stack.pop_front() {
                Some(Tag::I) => {
                    Tag::I.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(other) => {
                    output.write_fmt(format_args!("_{curr_char}"))?;
                    self.tag_stack.push_front(other);
                }
                None => {
                    output.write_fmt(format_args!("_{curr_char}"))?;
                }
            },
            // `* `
            C::Asterisk => match self.tag_stack.pop_front() {
                Some(Tag::Strong) => {
                    Tag::Strong.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(other) => {
                    output.write_fmt(format_args!("*{curr_char}"))?;
                    self.tag_stack.push_front(other);
                }
                None => {
                    output.write_fmt(format_args!("*{curr_char}"))?;
                }
            },
            // `[^ `
            C::Caret => {
                output.write_fmt(format_args!("[^{curr_char}"))?;
            }
            // `\d]: `
            C::Colon => match self.tag_stack.pop_front() {
                Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_open(output)?;
                    self.tag_stack.push_front(tag);
                    output.write_all(&[curr_char])?;
                }
                other => return Err(SamupError::stack(Tag::FootNoteRef(0), other)),
            },
            // `[ `
            C::SqBracketL => output.write_fmt(format_args!("[{curr_char}"))?,
            // `] `
            C::SqBracketR => match self.tag_stack.pop_front() {
                Some(Tag::Link(u)) => {
                    output.write_all(u.as_bytes())?;
                    Tag::Link(u).write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(Tag::FootNoteLink(n)) => {
                    output.write_fmt(format_args!("[^{n}]{curr_char}"))?
                }
                Some(other) => {
                    output.write_fmt(format_args!("]{curr_char}"))?;
                    self.tag_stack.push_front(other);
                }
                None => output.write_fmt(format_args!("]{curr_char}"))?,
            },
            C::ParenL => match self.tag_stack.pop_front() {
                Some(Tag::Link(ref url)) => {
                    output.write_fmt(format_args!("[{url}]("))?;
                }
                Some(tag) => {
                    output.write_all(b"(")?;
                    self.tag_stack.push_front(tag);
                }
                None => {
                    output.write_all(b"(")?;
                }
            },
            C::ParenR => match self.tag_stack.pop_front() {
                Some(tag @ Tag::Link(_)) => {
                    tag.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(other) => {
                    output.write_all(&[curr_char])?;
                    self.tag_stack.push_front(other);
                }
                None => output.write_all(&[curr_char])?,
            },
            // `\d `
            C::Digit => match self.tag_stack.pop_front() {
                Some(Tag::FootNoteLink(_)) | Some(Tag::FootNoteRef(_)) => {
                    return Err(SamupError::Syntax);
                }
                Some(t) => {
                    output.write_all(&[curr_char])?;
                    self.tag_stack.push_front(t);
                }
                None => output.write_all(&[curr_char])?,
            },
        };
        Ok(None)
    }
    fn parse_newline<O: Write>(
        &mut self,
        curr_char: u8,
        output: &mut O,
    ) -> Result<Option<C>, SamupError> {
        match self.prev_c {
            C::Whitespace | C::Content => output.write_all(&[curr_char])?,
            C::Newline => match self.tag_stack.pop_front() {
                Some(tag @ Tag::P) => tag.write_close(output)?,
                Some(other) => {
                    output.write_all(&[curr_char])?;
                    self.tag_stack.push_front(other);
                }
                None => output.write_all(&[curr_char])?,
            },
            C::Digit => match self.tag_stack.pop_front() {
                Some(Tag::FootNoteLink(_)) | Some(Tag::FootNoteRef(_)) => {
                    return Err(SamupError::Syntax);
                }
                Some(t) => {
                    output.write_all(&[curr_char])?;
                    self.tag_stack.push_front(t);
                }
                None => output.write_all(&[curr_char])?,
            },
            C::Colon => match self.tag_stack.pop_front() {
                Some(Tag::FootNoteRef(_)) => return Err(SamupError::Syntax),
                Some(t) => {
                    output.write_all(&[curr_char])?;
                    self.tag_stack.push_front(t);
                }
                None => output.write_all(&[curr_char])?,
            },
            C::Underscore => match self.tag_stack.pop_front() {
                Some(Tag::I) => {
                    Tag::I.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(other) => {
                    output.write_fmt(format_args!("_{curr_char}"))?;
                    self.tag_stack.push_front(other)
                }
                None => output.write_fmt(format_args!("_{curr_char}"))?,
            },
            C::Asterisk => match self.tag_stack.pop_front() {
                Some(Tag::I) => {
                    Tag::I.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(other) => {
                    output.write_fmt(format_args!("*{curr_char}"))?;
                    self.tag_stack.push_front(other)
                }
                None => output.write_fmt(format_args!("*{curr_char}"))?,
            },
            C::Caret => {
                output.write_fmt(format_args!("[^{curr_char}"))?;
            }
            C::SqBracketL => output.write_fmt(format_args!("[{curr_char}"))?,
            C::SqBracketR => match self.tag_stack.pop_front() {
                Some(Tag::Link(u)) => {
                    output.write_all(u.as_bytes())?;
                    Tag::Link(u).write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(Tag::FootNoteLink(n)) => {
                    // oops
                    output.write_fmt(format_args!("[^{n}]{curr_char}"))?
                }
                Some(other) => {
                    output.write_fmt(format_args!("]{curr_char}"))?;
                    self.tag_stack.push_front(other);
                }
                None => output.write_fmt(format_args!("]{curr_char}"))?,
            },
            C::ParenL => match self.tag_stack.pop_front() {
                Some(Tag::Link(ref url)) => {
                    output.write_fmt(format_args!("[{url}]("))?;
                }
                Some(tag) => {
                    output.write_all(b"(")?;
                    self.tag_stack.push_front(tag);
                }
                None => {
                    output.write_all(b"(")?;
                }
            },
            C::ParenR => match self.tag_stack.pop_front() {
                Some(tag @ Tag::Link(_)) => {
                    tag.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(other) => {
                    output.write_all(&[curr_char])?;
                    self.tag_stack.push_front(other);
                }
                None => output.write_all(&[curr_char])?,
            },
        }
        Ok(None)
    }
    fn parse_underscore<O: Write>(&mut self, output: &mut O) -> Result<Option<C>, SamupError> {
        match self.prev_c {
            C::Whitespace => {
                Tag::I.write_open(output)?;
                self.tag_stack.push_front(Tag::I);
            }
            C::Newline => match self.tag_stack.pop_front() {
                Some(tag) => {
                    Tag::I.write_open(output)?;
                    self.tag_stack.push_front(tag);
                    self.tag_stack.push_front(Tag::I);
                }
                None => {
                    Tag::P.write_open(output)?;
                    Tag::I.write_open(output)?;
                    self.tag_stack.push_front(Tag::P);
                    self.tag_stack.push_front(Tag::I);
                }
            },
            C::Caret => output.write_fmt(format_args!("[^"))?,
            C::Colon => match self.tag_stack.pop_front() {
                Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_open(output)?;
                    Tag::I.write_open(output)?;
                    self.tag_stack.push_front(tag);
                    self.tag_stack.push_front(Tag::I)
                }
                Some(tag) => {
                    output.write_all(b":")?;
                    Tag::I.write_open(output)?;
                    self.tag_stack.push_front(tag);
                    self.tag_stack.push_front(Tag::I)
                }
                None => {
                    output.write_all(b":")?;
                    Tag::P.write_open(output)?;
                    Tag::I.write_open(output)?;
                    self.tag_stack.push_front(Tag::P);
                    self.tag_stack.push_front(Tag::I);
                }
            },
            C::SqBracketL => {
                if self.tag_stack.front().is_none() {
                    Tag::P.write_open(output)?;
                    self.tag_stack.push_front(Tag::P);
                };
                output.write_all(b"[")?;
                Tag::I.write_open(output)?;
                self.tag_stack.push_front(Tag::I);
            }
            // __ -> _
            C::Underscore | C::Content => (),
            C::Asterisk => {
                Tag::Strong.write_open(output)?;
                self.tag_stack.push_front(Tag::Strong);
            }
            C::SqBracketR => match self.tag_stack.pop_front() {
                Some(tag @ Tag::Link(_)) => {
                    tag.write_link_no_title(output)?;
                }
                Some(tag @ Tag::FootNoteLink(_)) => {
                    tag.write_open(output)?;
                }
                Some(tag) => {
                    output.write_all(b"]")?;
                    self.tag_stack.push_front(tag);
                }
                None => output.write_all(b"]")?,
            },
            C::ParenL => match self.tag_stack.pop_front() {
                Some(Tag::Link(ref url)) => {
                    output.write_fmt(format_args!("[{url}]("))?;
                }
                Some(tag) => {
                    output.write_all(b"(")?;
                    self.tag_stack.push_front(tag);
                }
                None => {
                    output.write_all(b"(")?;
                }
            },
            C::ParenR => match self.tag_stack.pop_front() {
                Some(tag @ Tag::Link(_)) => tag.write_close(output)?,
                Some(tag) => {
                    output.write_all(b")")?;
                    self.tag_stack.push_front(tag);
                }
                None => {
                    output.write_all(b")")?;
                }
            },
            C::Digit => match self.tag_stack.pop_front() {
                Some(Tag::FootNoteRef(n)) => {
                    output.write_fmt(format_args!("[^{n}]"))?;
                }
                Some(tag) => {
                    self.tag_stack.push_front(tag);
                }
                None => (),
            },
        };
        Ok(None)
    }
    fn parse_asterisk<O: Write>(&mut self, output: &mut O) -> Result<Option<C>, SamupError> {
        match self.prev_c {
            C::Whitespace => {
                Tag::Strong.write_open(output)?;
                self.tag_stack.push_front(Tag::Strong);
            }
            C::Newline => match self.tag_stack.pop_front() {
                Some(tag) => {
                    Tag::Strong.write_open(output)?;
                    self.tag_stack.push_front(tag);
                    self.tag_stack.push_front(Tag::Strong);
                }
                None => {
                    Tag::P.write_open(output)?;
                    Tag::Strong.write_open(output)?;
                    self.tag_stack.push_front(Tag::P);
                    self.tag_stack.push_front(Tag::Strong);
                }
            },
            C::Caret => output.write_fmt(format_args!("[^"))?,
            C::Colon => match self.tag_stack.pop_front() {
                Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_open(output)?;
                    Tag::Strong.write_open(output)?;
                    self.tag_stack.push_front(tag);
                    self.tag_stack.push_front(Tag::Strong)
                }
                Some(tag) => {
                    output.write_all(b":")?;
                    Tag::Strong.write_open(output)?;
                    self.tag_stack.push_front(tag);
                    self.tag_stack.push_front(Tag::Strong)
                }
                None => {
                    output.write_all(b":")?;
                    Tag::P.write_open(output)?;
                    Tag::Strong.write_open(output)?;
                    self.tag_stack.push_front(Tag::P);
                    self.tag_stack.push_front(Tag::Strong);
                }
            },
            C::SqBracketL => {
                if self.tag_stack.front().is_none() {
                    Tag::P.write_open(output)?;
                    self.tag_stack.push_front(Tag::P);
                };
                output.write_all(b"[")?;
                Tag::Strong.write_open(output)?;
                self.tag_stack.push_front(Tag::Strong);
            }
            // ** -> *
            C::Asterisk | C::Content => (),
            C::Underscore => {
                Tag::I.write_open(output)?;
                self.tag_stack.push_front(Tag::I);
            }
            C::SqBracketR => match self.tag_stack.pop_front() {
                Some(tag @ Tag::Link(_)) => {
                    tag.write_link_no_title(output)?;
                }
                Some(tag @ Tag::FootNoteLink(_)) => {
                    tag.write_open(output)?;
                }
                Some(tag) => {
                    output.write_all(b"]")?;
                    self.tag_stack.push_front(tag);
                }
                None => output.write_all(b"]")?,
            },
            C::ParenL => match self.tag_stack.pop_front() {
                Some(Tag::Link(ref url)) => {
                    output.write_fmt(format_args!("[{url}]("))?;
                }
                Some(tag) => {
                    output.write_all(b"(")?;
                    self.tag_stack.push_front(tag);
                }
                None => {
                    output.write_all(b"(")?;
                }
            },
            C::ParenR => match self.tag_stack.pop_front() {
                Some(tag @ Tag::Link(_)) => tag.write_close(output)?,
                Some(tag) => {
                    output.write_all(b")")?;
                    self.tag_stack.push_front(tag);
                }
                None => {
                    output.write_all(b")")?;
                }
            },
            C::Digit => match self.tag_stack.pop_front() {
                Some(Tag::FootNoteRef(n)) => {
                    output.write_fmt(format_args!("[^{n}]"))?;
                }
                Some(tag) => {
                    self.tag_stack.push_front(tag);
                }
                None => (),
            },
        };
        Ok(None)
    }
    fn parse_caret<O: Write>(&mut self, output: &mut O) -> Result<Option<C>, SamupError> {
        let mut next_c = None;
        match self.prev_c {
            C::SqBracketL => (),
            _ => {
                output.write_all(b"^")?;
                next_c = Some(C::Content);
            }
        };
        Ok(next_c)
    }
    fn parse_colon<O: Write>(&mut self, output: &mut O) -> Result<Option<C>, SamupError> {
        let mut next_c = None;
        match self.prev_c {
            C::SqBracketR => (),
            _ => {
                output.write_all(b":")?;
                next_c = Some(C::Content);
            }
        };
        Ok(next_c)
    }
    fn parse_sq_bracket_l<O: Write>(&mut self, output: &mut O) -> Result<Option<C>, SamupError> {
        match self.prev_c {
            C::Underscore => {
                Tag::I.write_open(output)?;
                self.tag_stack.push_front(Tag::I);
            }
            C::Asterisk => {
                Tag::Strong.write_open(output)?;
                self.tag_stack.push_front(Tag::I);
            }
            C::SqBracketL => {
                output.write_all(b"[")?;
            }
            C::SqBracketR => match self.tag_stack.pop_front() {
                Some(tag @ Tag::Link(_)) => {
                    tag.write_link_no_title(output)?;
                }
                Some(tag) => {
                    self.tag_stack.push_front(tag);
                }
                None => (),
            },
            C::Caret => {
                output.write_all(b"[^")?;
            }
            C::ParenR => match self.tag_stack.pop_front() {
                Some(tag @ Tag::Link(_)) => {
                    tag.write_close(output)?;
                }
                Some(tag) => {
                    self.tag_stack.push_front(tag);
                }
                None => (),
            },
            C::Digit => match self.tag_stack.pop_front() {
                Some(Tag::FootNoteLink(n)) | Some(Tag::FootNoteRef(n)) => {
                    output.write_fmt(format_args!("[^{n}]"))?;
                }
                Some(tag) => self.tag_stack.push_front(tag),
                None => (),
            },
            C::Colon => match self.tag_stack.pop_front() {
                Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_open(output)?;
                    self.tag_stack.push_front(tag);
                }
                Some(tag) => {
                    self.tag_stack.push_front(tag);
                }
                None => (),
            },
            C::ParenL => match self.tag_stack.pop_front() {
                Some(Tag::Link(ref url)) => {
                    output.write_fmt(format_args!("[{url}]("))?;
                }
                Some(tag) => {
                    output.write_all(b"(")?;
                    self.tag_stack.push_front(tag);
                }
                None => {
                    output.write_all(b"(")?;
                }
            },
            C::Content | C::Newline | C::Whitespace => (),
        };
        Ok(None)
    }
    fn parse_sq_bracket_r<O: Write>(&mut self, output: &mut O) -> Result<Option<C>, SamupError> {
        match self.tag_stack.pop_front() {
            Some(tag @ Tag::Link(ref url)) => {
                match self.prev_c {
                        C::Whitespace => {
                            self.tag_stack.push_front(tag);
                        },
                        C::Newline => {
                            output.write_fmt(format_args!("{url}\n]"));
                        },
                        C::Underscore => {
                            url.push_str("_");
                            self.tag_stack.push_front(tag);
                        },
                        C::Asterisk => {
                            url.push_str("*");
                            self.tag_stack.push_front(tag);
                        },
                        C::Caret => {
                            url.push_str("^");
                            self.tag_stack.push_front(tag);
                        },
                    }
            }
        }
        // match self.prev_c {
        //     C::Underscore => {
        //         Tag::I.write_open(output)?;
        //         self.tag_stack.push_front(Tag::I);
        //     }
        //     C::Asterisk => {
        //         Tag::Strong.write_open(output)?;
        //         self.tag_stack.push_front(Tag::I);
        //     }
        //     C::SqBracketL => {
        //         output.write_all(b"[")?;
        //     }
        // }
        Ok(None)
    }
    fn parse_paren_l<O: Write>(&mut self, output: &mut O) -> Result<Option<C>, SamupError> {
        todo!()
    }
    fn parse_paren_r<O: Write>(&mut self, output: &mut O) -> Result<Option<C>, SamupError> {
        todo!()
    }
    fn parse_digit<O: Write>(
        &mut self,
        curr_char: u8,
        output: &mut O,
    ) -> Result<Option<C>, SamupError> {
        todo!()
    }
}
