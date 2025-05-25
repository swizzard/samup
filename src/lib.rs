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
    // ParenL,
    ParenR,
    // Quote,
    Digit, // for footnotes
    Content,
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
            // :
            58 => C::Colon,
            // [
            91 => C::SqBracketL,
            // ]
            93 => C::SqBracketR,
            // (
            // 40 => C::ParenL,
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
    pub fn parse<O: Write>(
        &mut self,
        input: &[u8],
        output: &mut O,
    ) -> Result<(), SamupError> {
        let curr_char = input[self.ix];
        let curr_c: C = curr_char.into();
        match curr_c {
            C::Whitespace => match self.prev_c {
                // `  ` | `\n ` | `C `
                C::Whitespace | C::Newline | C::Content => {
                    output.write_all(&[curr_char])?;
                }
                // `_ `
                C::Underscore => match self.tag_stack.pop_front() {
                    Some(Tag::I) => {
                        Tag::I.write_close(output)?;
                        output.write_all(&[curr_char])?;
                    },
                    Some(other) => {
                        output.write_fmt(format_args!("_{curr_char}"))?;
                        self.tag_stack.push_front(other)
                    },
                    None => output.write_fmt(format_args!("_{curr_char}"))?
                },
                // `* `
                C::Asterisk => match self.tag_stack.pop_front() {
                    Some(Tag::I) => {
                        Tag::I.write_close(output)?;
                        output.write_all(&[curr_char])?;
                    }
                    Some(other) => {
                        output.write_fmt(format_args!("*{curr_char}"))?;
                        self.tag_stack.push_front(other)
                    },
                    None => output.write_fmt(format_args!("*{curr_char}"))?
                },
                // `[^ `
                C::Caret => output.write_fmt(format_args!("[^{curr_char}"))?,
                // `\d]: `
                C::Colon => match self.tag_stack.pop_front() {
                    Some(tag @ Tag::FootNoteRef(_)) => {
                        tag.write_open(output)?;
                        self.tag_stack.push_front(tag);
                        output.write_all(&[curr_char])?;
                    },
                    other => return Err(SamupError::stack(Tag::FootNoteRef(0), other))
                }
                // `[ `
                C::SqBracketL => {
                     match self.tag_stack.pop_front() {
                         Some(Tag::Link(_)) | Some(Tag::FootNoteLink(_)) | Some(Tag::FootNoteRef(_)) | None => {
                            output.write_fmt(format_args!("[{curr_char}"))?;
                         },
                         Some(other) => {
                            output.write_fmt(format_args!("[{curr_char}"))?;
                            self.tag_stack.push_front(other);
                         }
                     }
                },
                // `] `
                C::SqBracketR => {
                    match self.tag_stack.pop_front() {
                        Some(Tag::Link(u)) => { 
                            output.write_all(u.as_bytes())?;
                            Tag::Link(u).write_close(output)?;
                            output.write_all(&[curr_char])?;
                        },
                        Some(tag @ Tag::FootNoteRef(_)) => {
                            tag.write_close(output)?;
                            output.write_all(&[curr_char])?;
                        },
                        Some(Tag::FootNoteLink(_)) => output.write_all(&[curr_char])?,
                        Some(other) => {
                            output.write_all(&[curr_char])?;
                            self.tag_stack.push_front(other);
                        },
                        None => output.write_all(&[curr_char])?
                    }
                },
                // // `( `
                // C::ParenL => todo!(),
                // `) `
                C::ParenR => {
                    match self.tag_stack.pop_front() {
                        Some(tag @ Tag::Link(_)) => {
                            tag.write_close(output)?;
                        output.write_all(&[curr_char])?;
                        },
                        Some(other) => {
                            output.write_all(&[curr_char])?;
                            self.tag_stack.push_front(other);
                        },
                        None => output.write_all(&[curr_char])?
                    }
                },
                // `\d `
                C::Digit => {
                    match self.tag_stack.pop_front() {
                        Some(Tag::FootNoteLink(_)) | Some(Tag::FootNoteRef(_)) => return Err(SamupError::Syntax),
                        Some(t) => {
                            output.write_all(&[curr_char])?;
                            self.tag_stack.push_front(t);
                            output.write_all(&[curr_char])?;
                        },
                        None => output.write_all(&[curr_char])?
                    }
                }
            },
            C::Newline => match self.prev_c {
                C::Whitespace | C::Caret | C::Content => {
                    output.write_all(&[curr_char])?
                },
                C::Newline => {
                    match self.tag_stack.pop_front() {
                        Some(tag @ Tag::P) => tag.write_close(output)?,
                        Some(other) => {
                            output.write_all(&[curr_char])?;
                            self.tag_stack.push_front(other);
                        },
                        None => output.write_all(&[curr_char])?
                    }
                },
                C::Digit => match self.tag_stack.pop_front() {
                     Some(Tag::FootNoteLink(_)) | Some(Tag::FootNoteRef(_)) => {
                        return Err(SamupError::Syntax)
                     },
                    Some(t) => {
                        output.write_all(&[curr_char])?;
                        self.tag_stack.push_front(t);
                    },
                    None => output.write_all(&[curr_char])?
                },
                C::Colon => match self.tag_stack.pop_front() {
                     Some(Tag::FootNoteRef(_)) => {
                        return Err(SamupError::Syntax)
                     },
                    Some(t) => {
                        output.write_all(&[curr_char])?;
                        self.tag_stack.push_front(t);
                    },
                    None => output.write_all(&[curr_char])?
                },
                C::Underscore => match self.tag_stack.pop_front() {
                    Some(Tag::I) => {
                        Tag::I.write_close(output)?;
                        output.write_all(&[curr_char])?;
                    },
                    Some(other) => {
                        output.write_fmt(format_args!("_{curr_char}"))?;
                        self.tag_stack.push_front(other)
                    },
                    None => output.write_fmt(format_args!("_{curr_char}"))?
                },
                C::Asterisk => match self.tag_stack.pop_front() {
                    Some(Tag::I) => {
                        Tag::I.write_close(output)?;
                        output.write_all(&[curr_char])?;
                    }
                    Some(other) => {
                        output.write_fmt(format_args!("*{curr_char}"))?;
                        self.tag_stack.push_front(other)
                    },
                    None => output.write_fmt(format_args!("*{curr_char}"))?
                },
                C::SqBracketL => todo!(),
                C::SqBracketR => todo!(),
                // C::ParenL => todo!(),
                C::ParenR => todo!(),
            },
            C::Underscore => todo!(),
            C::Asterisk => todo!(),
            C::Caret => todo!(),
            C::Colon => todo!(),
            C::SqBracketL => todo!(),
            C::SqBracketR => todo!(),
            // C::ParenL => todo!(),
            C::ParenR => todo!(),
            C::Digit => todo!(),
            C::Content => {
                output.write_all(&[curr_char])?;
            }
        };
        self.prev_c = curr_c;
        self.ix += 1;
        Ok(())
    }
}
