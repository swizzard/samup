use crate::{C, SamupError, Tag};
use std::collections::VecDeque;
use std::io::Write;

#[derive(Debug)]
pub struct Transcriber {
    pub ix: usize,
    prev_c: C,
    tag_stack: VecDeque<Tag>,
}

impl Transcriber {
    pub fn new() -> Self {
        Self {
            ix: 0,
            prev_c: C::Whitespace,
            tag_stack: VecDeque::new(),
        }
    }
    pub fn transcribe<O: Write>(&mut self, input: &[u8], output: &mut O) -> Result<(), SamupError> {
        let curr_char = input[self.ix];
        let curr_c: C = curr_char.into();
        let next_c = match curr_c {
            C::Whitespace => self.transcribe_whitespace(curr_char, output)?,
            C::Newline => self.transcribe_newline(curr_char, output)?,
            C::Underscore => self.transcribe_underscore(output)?,
            C::Asterisk => self.transcribe_asterisk(output)?,
            C::Caret => self.transcribe_caret(output)?,
            C::Colon => self.transcribe_colon(output)?,
            C::SqBracketL => self.transcribe_sq_bracket_l(output)?,
            C::SqBracketR => self.transcribe_sq_bracket_r(output)?,
            C::ParenL | C::ParenR => self.transcribe_paren(output)?,
            C::Digit => self.transcribe_digit(curr_char, output)?,
            C::Content => {
                output.write_all(&[curr_char])?;
                None
            }
        };
        self.prev_c = next_c.unwrap_or(curr_c);
        self.ix += 1;
        Ok(())
    }
    pub fn finish<O: Write>(&mut self, output: &mut O) -> Result<(), SamupError> {
        match self.prev_c {
            _ => todo!(),
        };
        while let Some(tag) = self.tag_stack.pop_front() {
            match tag {
                _ => todo!(),
            }
        }
        Ok(())
    }
    fn transcribe_whitespace<O: Write>(
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
                Some(other) => {
                    output.write_fmt(format_args!(":{curr_char}"))?;
                    self.tag_stack.push_front(other);
                }
                None => {
                    output.write_fmt(format_args!(":{curr_char}"))?;
                }
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
                    let n = n.ix();
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
    fn transcribe_newline<O: Write>(
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
                    let n = n.ix();
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
    fn transcribe_underscore<O: Write>(&mut self, output: &mut O) -> Result<Option<C>, SamupError> {
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
                    let n = n.ix();
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
    fn transcribe_asterisk<O: Write>(&mut self, output: &mut O) -> Result<Option<C>, SamupError> {
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
                    let n = n.ix();
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
    fn transcribe_caret<O: Write>(&mut self, output: &mut O) -> Result<Option<C>, SamupError> {
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
    fn transcribe_colon<O: Write>(&mut self, output: &mut O) -> Result<Option<C>, SamupError> {
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
    fn transcribe_sq_bracket_l<O: Write>(
        &mut self,
        output: &mut O,
    ) -> Result<Option<C>, SamupError> {
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
                    output.write_all(b")")?;
                    self.tag_stack.push_front(tag);
                }
                None => (),
            },
            C::Digit => match self.tag_stack.pop_front() {
                Some(Tag::FootNoteLink(n)) | Some(Tag::FootNoteRef(n)) => {
                    let n = n.ix();
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
    fn transcribe_sq_bracket_r<O: Write>(
        &mut self,
        output: &mut O,
    ) -> Result<Option<C>, SamupError> {
        let mut next_c: Option<C> = None;
        match self.tag_stack.pop_front() {
            Some(mut tag @ Tag::Link(_)) => match self.prev_c {
                C::Whitespace => {
                    self.tag_stack.push_front(tag);
                }
                C::Newline => {
                    let url = tag.link_url();
                    output.write_fmt(format_args!("{url}\n]"))?;
                }
                C::Underscore => {
                    tag.push_link("_");
                    self.tag_stack.push_front(tag);
                }
                C::Asterisk => {
                    tag.push_link("*");
                    self.tag_stack.push_front(tag);
                }
                C::Caret => {
                    tag.push_link("^");
                    self.tag_stack.push_front(tag);
                }
                C::Colon => {
                    tag.push_link(":");
                    self.tag_stack.push_front(tag);
                }
                C::SqBracketL => {
                    tag.push_link("[");
                    self.tag_stack.push_front(tag);
                }
                C::SqBracketR => {
                    tag.push_link("]");
                    self.tag_stack.push_front(tag);
                }
                C::ParenL => {
                    tag.push_link("(");
                    self.tag_stack.push_front(tag)
                }
                C::ParenR => {
                    tag.push_link(")");
                    self.tag_stack.push_front(tag)
                }
                C::Digit | C::Content => self.tag_stack.push_front(tag),
            },
            Some(tag @ Tag::FootNoteLink(_)) | Some(tag @ Tag::FootNoteRef(_)) => {
                self.tag_stack.push_front(tag);
            }
            Some(tag) => {
                output.write_all(b"]")?;
                self.tag_stack.push_front(tag);
                next_c = Some(C::Content);
            }
            None => {
                output.write_all(b"]")?;
                next_c = Some(C::Content);
            }
        }
        Ok(next_c)
    }
    fn transcribe_paren<O: Write>(&mut self, output: &mut O) -> Result<Option<C>, SamupError> {
        match self.prev_c {
            C::Whitespace | C::Newline | C::Content => (),
            C::SqBracketL => {
                output.write_all(b"[")?;
            }
            C::ParenL => {
                output.write_all(b"(")?;
            }
            C::ParenR => {
                output.write_all(b")")?;
            }
            C::SqBracketR => match self.tag_stack.pop_front() {
                Some(tag @ Tag::Link(_)) => {
                    tag.write_open(output)?;
                    self.tag_stack.push_front(tag);
                }
                Some(tag @ Tag::FootNoteLink(_)) => {
                    tag.write_open(output)?;
                }
                Some(tag) => {
                    output.write_all(b"]")?;
                    self.tag_stack.push_front(tag);
                }
                None => {
                    output.write_all(b"]")?;
                }
            },
            C::Underscore => {
                Tag::I.write_open(output)?;
                self.tag_stack.push_front(Tag::I);
            }
            C::Asterisk => {
                Tag::Strong.write_open(output)?;
                self.tag_stack.push_front(Tag::Strong);
            }
            C::Caret => output.write_all(b"[^")?,
            C::Colon => match self.tag_stack.pop_front() {
                Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_open(output)?;
                    self.tag_stack.push_front(tag);
                }
                Some(tag) => {
                    output.write_all(b":")?;
                    self.tag_stack.push_front(tag);
                }
                None => {
                    output.write_all(b":")?;
                }
            },
            C::Digit => match self.tag_stack.pop_front() {
                Some(Tag::FootNoteLink(n)) | Some(Tag::FootNoteRef(n)) => {
                    let n = n.ix();
                    output.write_fmt(format_args!("[^{n}"))?;
                }
                // unreachable?
                Some(tag) => self.tag_stack.push_front(tag),
                // unreachable?
                None => (),
            },
        };
        Ok(None)
    }
    fn transcribe_digit<O: Write>(
        &mut self,
        curr_char: u8,
        output: &mut O,
    ) -> Result<Option<C>, SamupError> {
        let mut next_c: Option<C> = None;
        match self.tag_stack.pop_front() {
            Some(tag @ Tag::FootNoteLink(mut n)) | Some(tag @ Tag::FootNoteRef(mut n)) => {
                n.push_digit(curr_char);
                self.tag_stack.push_front(tag);
            }
            Some(mut tag @ Tag::Link(_)) => {
                tag.push_link(str::from_utf8(&[curr_char]).unwrap());
                self.tag_stack.push_front(tag);
                next_c = Some(C::Content);
            }
            Some(tag) => {
                output.write_all(&[curr_char])?;
                self.tag_stack.push_front(tag);
                next_c = Some(C::Content);
            }
            None => {
                output.write_all(&[curr_char])?;
                next_c = Some(C::Content);
            }
        };
        Ok(next_c)
    }
}

impl Default for Transcriber {
    fn default() -> Self {
        Self::new()
    }
}
