use std::io::{self, Write};
use thiserror::Error;

pub mod transcriber;
pub use transcriber::Transcriber;

pub fn transcribe<O: Write>(input: &[u8], output: &mut O) -> SamupResult {
    let mut transcriber = Transcriber::new();
    while transcriber.ix < input.len() {
        transcriber.transcribe(input, output)?;
    }
    transcriber.finish(output)?;
    Ok(())
}

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

pub type SamupResult<T = ()> = Result<T, SamupError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum C {
    Whitespace,
    Newline,
    Underscore,
    Asterisk,
    Octothorpe,
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
            // #
            35 => C::Octothorpe,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FootNoteIx(u8);

impl FootNoteIx {
    fn push_digit(&mut self, c: u8) {
        let c: u8 = char::from(c)
            .to_digit(10)
            .expect("not a digit")
            .try_into()
            .expect("bad digit");
        self.0 *= 10;
        self.0 += c;
    }
    fn ix(&self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HLevel(u8);

impl HLevel {
    fn new() -> Self {
        Self(1)
    }
    fn level(&self) -> u8 {
        self.0
    }
    fn inc_level(&mut self) -> bool {
        if self.0 < 6 {
            self.0 += 1;
            true
        } else {
            false
        }
    }
    fn as_octothorpes(&self) -> &[u8] {
        match self.0 {
            1 => b"#",
            2 => b"##",
            3 => b"###",
            4 => b"####",
            5 => b"#####",
            6 => b"######",
            _ => panic!("unreachable HLevel"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tag {
    H(HLevel),
    I,
    P,
    Strong,
    Link(String),
    // ...[^1]
    FootNoteLink(FootNoteIx),
    // [^1]: ...
    FootNoteRef(FootNoteIx),
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Tag::H(n) => {
                let level = n.level();
                f.write_fmt(format_args!("<h{level}>"))
            }
            Tag::I => f.write_str("<i>"),
            Tag::P => f.write_str("<p>"),
            Tag::Strong => f.write_str("<strong>"),
            Tag::Link(url) => f.write_fmt(format_args!("<link: {url}>")),
            Tag::FootNoteLink(n) => {
                let ix = n.ix();
                f.write_fmt(format_args!("<footnote link {ix}>"))
            }
            Tag::FootNoteRef(n) => {
                let ix = n.ix();
                f.write_fmt(format_args!("<footnote ref {ix}>"))
            }
        }
    }
}

impl Tag {
    fn write_open<O: Write>(&self, output: &mut O) -> Result<(), io::Error> {
        match self {
            Tag::H(n) => {
                let level = n.level();
                output.write_fmt(format_args!("<h{level}>"))
            }
            Tag::I => output.write_all(b"<i>"),
            Tag::P => output.write_all(b"<p>"),
            Tag::Strong => output.write_all(b"<strong>"),
            Tag::Link(url) => write!(output, "<a href=\"{url}\" target=\"_blank\">"),
            Tag::FootNoteLink(note_no) => {
                let note_no = note_no.ix();
                write!(
                    output,
                    "<a id=\"link-{note_no}\" target=\"#ref-{note_no}\"><sup>{note_no}</sup></a>"
                )
            }
            Tag::FootNoteRef(note_no) => {
                let note_no = note_no.ix();
                write!(
                    output,
                    "<p class=\"footnote\" id=\"ref-{note_no}\"><span class=\"footnote\">{note_no}:</span> "
                )
            }
        }
    }
    fn write_close<O: Write>(&self, output: &mut O) -> Result<(), io::Error> {
        match self {
            Tag::H(n) => {
                let level = n.level();
                output.write_fmt(format_args!("</h{level}>"))
            }
            Tag::I => output.write_all(b"</i>"),
            Tag::P => output.write_all(b"</p>"),
            Tag::Strong => output.write_all(b"</strong>"),
            Tag::Link(_) => write!(output, "</a>"),
            Tag::FootNoteLink(_) => Ok(()),
            Tag::FootNoteRef(note_no) => {
                let note_no = note_no.ix();
                write!(output, "<a href=\"#link-{note_no}\">\u{1f519}</a></p>")
            }
        }
    }
    fn new_link(c: u8) -> Self {
        let c: &[u8] = &[c];
        Tag::Link((unsafe { str::from_utf8_unchecked(c) }).into())
    }
    fn write_link_no_title<O: Write>(&self, output: &mut O) -> Result<(), io::Error> {
        if let Tag::Link(url) = self {
            write!(output, "<a href=\"{url}\" target=\"_blank\">{url}</a>")
        } else {
            panic!()
        }
    }
    fn push_link(&mut self, s: &str) {
        if let Tag::Link(u) = self {
            u.push_str(s);
        } else {
            panic!()
        }
    }
    fn link_url(&self) -> &str {
        if let Tag::Link(u) = self { u } else { panic!() }
    }
    fn new_h() -> Self {
        Tag::H(HLevel::new())
    }
    fn inc_h(&mut self) -> bool {
        if let Tag::H(n) = self {
            n.inc_level()
        } else {
            panic!()
        }
    }
}
