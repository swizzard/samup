use crate::{C, InnerLink, LinkState, SamupResult, Tag};
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
            prev_c: C::Newline,
            tag_stack: VecDeque::new(),
        }
    }
    pub fn transcribe<O: Write>(&mut self, input: &[u8], output: &mut O) -> SamupResult {
        let curr_char = input[self.ix];
        let curr_c: C = curr_char.into();
        let next_c = match curr_c {
            C::Whitespace => self.transcribe_whitespace(curr_char, output)?,
            C::Newline => self.transcribe_newline(curr_char, output)?,
            C::Underscore => self.transcribe_underscore(output)?,
            C::Asterisk => self.transcribe_asterisk(output)?,
            C::Octothorpe => self.transcribe_octothorpe(output)?,
            C::Caret => self.transcribe_caret(output)?,
            C::Colon => self.transcribe_colon(output)?,
            C::SqBracketL => self.transcribe_sq_bracket_l(output)?,
            C::SqBracketR => self.transcribe_sq_bracket_r(output)?,
            C::ParenL | C::ParenR => self.transcribe_paren(output)?,
            C::Digit => self.transcribe_digit(curr_char, output)?,
            C::Content => self.transcribe_content(curr_char, output)?,
        };
        self.prev_c = next_c.unwrap_or(curr_c);
        self.ix += 1;
        Ok(())
    }
    pub fn finish<O: Write>(&mut self, output: &mut O) -> SamupResult {
        match self.prev_c {
            C::Whitespace | C::Newline | C::Content => (),
            C::Underscore => {
                if let Some(tag @ Tag::I) = self.pop_tag() {
                    tag.write_close(output)?
                } else {
                    output.write_all(b"_")?;
                }
            }
            C::Asterisk => {
                if let Some(tag @ Tag::Strong) = self.pop_tag() {
                    tag.write_close(output)?;
                } else {
                    output.write_all(b"*")?;
                }
            }
            C::Octothorpe => {
                if let Some(Tag::H(mut n)) = self.pop_tag() {
                    let inced = n.inc_level();
                    output.write_all(n.as_octothorpes())?;
                    if !inced {
                        output.write_all(b"#")?;
                    }
                } else {
                    output.write_all(b"#")?;
                }
            }
            C::Caret => {
                output.write_all(b"[^")?;
            }
            C::Colon => {
                if let Some(Tag::FootNoteRef(n)) = self.pop_tag() {
                    let n = n.ix();
                    output.write_fmt(format_args!("[^{n}]:"))?;
                } else {
                    output.write_all(b":")?;
                }
            }
            C::SqBracketL => {
                output.write_all(b"[")?;
            }
            C::SqBracketR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => {
                    if self.stack_empty() {
                        Tag::P.write_open(output)?;
                        self.push_tag(Tag::P);
                    }
                    tag.write_link_no_title(output)?;
                }
                Some(tag @ Tag::FootNoteLink(_)) | Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_close(output)?;
                }
                _ => output.write_all(b"]")?,
            },
            C::ParenL => {
                if let Some(tag @ Tag::Link(_)) = self.pop_tag() {
                    tag.write_link_no_title(output)?;
                };
                output.write_all(b"(")?;
            }
            C::ParenR => {
                if let Some(tag @ Tag::Link(_)) = self.pop_tag() {
                    tag.write_close(output)?;
                } else {
                    output.write_all(b")")?;
                }
            }
            C::Digit => {
                if let Some(Tag::FootNoteLink(n)) | Some(Tag::FootNoteRef(n)) = self.pop_tag() {
                    let n = n.ix();
                    output.write_fmt(format_args!("[^{n}"))?;
                }
            }
        };
        while let Some(tag) = self.pop_tag() {
            match tag {
                Tag::H(_) | Tag::I | Tag::P | Tag::Strong | Tag::FootNoteRef(_) => {
                    tag.write_close(output)?;
                }
                Tag::Link(u) => {
                    output.write_fmt(format_args!("[{u}"))?;
                }
                Tag::FootNoteLink(n) => {
                    let n = n.ix();
                    output.write_fmt(format_args!("[^{n}]"))?;
                }
            }
        }
        Ok(())
    }
    fn transcribe_whitespace<O: Write>(
        &mut self,
        curr_char: u8,
        output: &mut O,
    ) -> SamupResult<Option<C>> {
        match self.prev_c {
            C::Whitespace | C::Content => {
                output.write_all(&[curr_char])?;
            }
            C::Newline => {
                output.write_fmt(format_args!("\n{curr_char}"))?;
            }
            C::Underscore => match self.pop_tag() {
                Some(Tag::I) => {
                    Tag::I.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(tag) => {
                    output.write_fmt(format_args!("_{curr_char}"))?;
                    self.push_tag(tag);
                }
                None => {
                    output.write_fmt(format_args!("_{curr_char}"))?;
                }
            },
            C::Asterisk => match self.pop_tag() {
                Some(Tag::Strong) => {
                    Tag::Strong.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(tag) => {
                    output.write_fmt(format_args!("*{curr_char}"))?;
                    self.push_tag(tag);
                }
                None => {
                    output.write_fmt(format_args!("*{curr_char}"))?;
                }
            },
            C::Octothorpe => match self.pop_tag() {
                Some(tag @ Tag::H(_)) => {
                    tag.write_open(output)?;
                    self.push_tag(tag)
                }
                Some(tag) => {
                    output.write_fmt(format_args!("#{curr_char}"))?;
                    self.push_tag(tag)
                }
                None => output.write_fmt(format_args!("#{curr_char}"))?,
            },
            C::Caret => {
                output.write_fmt(format_args!("[^{curr_char}"))?;
            }
            C::Colon => match self.pop_tag() {
                Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_open(output)?;
                    self.push_tag(tag);
                    output.write_all(&[curr_char])?;
                }
                Some(other) => {
                    output.write_fmt(format_args!(":{curr_char}"))?;
                    self.push_tag(other);
                }
                None => {
                    output.write_fmt(format_args!(":{curr_char}"))?;
                }
            },
            C::SqBracketL => {
                if self.stack_empty() {
                    Tag::P.write_open(output)?;
                    output.write_fmt(format_args!("[{curr_char}"))?;
                    self.push_tag(Tag::P);
                } else {
                    output.write_fmt(format_args!("[{curr_char}"))?
                }
            }
            C::SqBracketR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => {
                    if self.stack_empty() {
                        Tag::P.write_open(output)?;
                        self.push_tag(Tag::P);
                    }
                    tag.write_link_no_title(output)?;
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
                    self.push_tag(other);
                }
                None => output.write_fmt(format_args!("]{curr_char}"))?,
            },
            C::ParenL => match self.pop_tag() {
                Some(Tag::Link(ref url)) => {
                    output.write_fmt(format_args!("[{url}]("))?;
                }
                Some(tag) => {
                    output.write_all(b"(")?;
                    self.push_tag(tag);
                }
                None => {
                    output.write_all(b"(")?;
                }
            },
            C::ParenR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => {
                    tag.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(other) => {
                    output.write_all(&[curr_char])?;
                    self.push_tag(other);
                }
                None => output.write_all(&[curr_char])?,
            },
            C::Digit => match self.pop_tag() {
                Some(Tag::FootNoteLink(n)) | Some(Tag::FootNoteRef(n)) => {
                    let n = n.ix();
                    output.write_fmt(format_args!("[^{n}{curr_char}"))?;
                }
                Some(t) => {
                    output.write_all(&[curr_char])?;
                    self.push_tag(t);
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
    ) -> SamupResult<Option<C>> {
        match self.prev_c {
            C::Whitespace | C::Content => (), // output.write_all(&[curr_char])?,
            C::Newline => match self.pop_tag() {
                Some(tag @ Tag::P) => {
                    tag.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(tag) => {
                    output.write_all(&[curr_char])?;
                    self.push_tag(tag);
                }
                None => {
                    Tag::P.write_open(output)?;
                    self.push_tag(Tag::P);
                }
            },
            C::Digit => match self.pop_tag() {
                Some(Tag::FootNoteLink(n)) | Some(Tag::FootNoteRef(n)) => {
                    let n = n.ix();
                    output.write_fmt(format_args!("[^{n}{curr_char}"))?;
                }
                Some(tag) => {
                    output.write_all(&[curr_char])?;
                    self.push_tag(tag);
                }
                None => output.write_all(&[curr_char])?,
            },
            C::Colon => match self.pop_tag() {
                Some(Tag::FootNoteRef(n)) => {
                    let n = n.ix();
                    output.write_fmt(format_args!("[^{n}]:{curr_char}"))?;
                }
                Some(tag) => {
                    output.write_all(&[curr_char])?;
                    self.push_tag(tag);
                }
                None => output.write_all(&[curr_char])?,
            },
            C::Underscore => match self.pop_tag() {
                Some(Tag::I) => {
                    Tag::I.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(tag) => {
                    output.write_fmt(format_args!("_{curr_char}"))?;
                    self.push_tag(tag)
                }
                None => output.write_fmt(format_args!("_{curr_char}"))?,
            },
            C::Asterisk => match self.pop_tag() {
                Some(Tag::I) => {
                    Tag::I.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(tag) => {
                    output.write_fmt(format_args!("*{curr_char}"))?;
                    self.push_tag(tag)
                }
                None => output.write_fmt(format_args!("*{curr_char}"))?,
            },
            C::Octothorpe => match self.pop_tag() {
                Some(Tag::H(n)) => {
                    output.write_all(n.as_octothorpes())?;
                    output.write_all(&[curr_char])?;
                }
                Some(tag) => {
                    output.write_fmt(format_args!("#{curr_char}"))?;
                    self.push_tag(tag)
                }
                None => output.write_fmt(format_args!("#{curr_char}"))?,
            },
            C::Caret => {
                output.write_fmt(format_args!("[^{curr_char}"))?;
            }
            C::SqBracketL => {
                if self.stack_empty() {
                    Tag::P.write_open(output)?;
                    output.write_fmt(format_args!("[{curr_char}"))?;
                    self.push_tag(Tag::P);
                } else {
                    output.write_fmt(format_args!("[{curr_char}"))?;
                }
            }
            C::SqBracketR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => {
                    if self.stack_empty() {
                        Tag::P.write_open(output)?;
                        self.push_tag(Tag::P);
                    }
                    tag.write_link_no_title(output)?;
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
                Some(tag) => {
                    output.write_fmt(format_args!("]{curr_char}"))?;
                    self.push_tag(tag);
                }
                None => output.write_fmt(format_args!("]{curr_char}"))?,
            },
            C::ParenL => match self.pop_tag() {
                Some(Tag::Link(ref url)) => {
                    output.write_fmt(format_args!("[{url}]("))?;
                }
                Some(tag) => {
                    output.write_all(b"(")?;
                    self.push_tag(tag);
                }
                None => {
                    output.write_all(b"(")?;
                }
            },
            C::ParenR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => {
                    tag.write_close(output)?;
                    output.write_all(&[curr_char])?;
                }
                Some(other) => {
                    output.write_all(&[curr_char])?;
                    self.push_tag(other);
                }
                None => output.write_all(&[curr_char])?,
            },
        }
        Ok(None)
    }
    fn transcribe_underscore<O: Write>(&mut self, output: &mut O) -> SamupResult<Option<C>> {
        match self.prev_c {
            C::Whitespace => {
                Tag::I.write_open(output)?;
                self.push_tag(Tag::I);
            }
            C::Newline => match self.pop_tag() {
                Some(tag) => {
                    output.write_all(b"\n")?;
                    Tag::I.write_open(output)?;
                    self.push_tag(tag);
                    self.push_tag(Tag::I);
                }
                None => {
                    output.write_all(b"\n")?;
                    Tag::P.write_open(output)?;
                    Tag::I.write_open(output)?;
                    self.push_tag(Tag::P);
                    self.push_tag(Tag::I);
                }
            },
            C::Octothorpe => match self.pop_tag() {
                Some(tag @ Tag::H(_)) => {
                    tag.write_open(output)?;
                    self.push_tag(tag);
                    Tag::I.write_open(output)?;
                    self.push_tag(Tag::I);
                }
                Some(tag) => {
                    output.write_all(b"#")?;
                    Tag::I.write_open(output)?;
                    self.push_tag(tag);
                    self.push_tag(Tag::I);
                }
                None => {
                    output.write_all(b"#")?;
                    Tag::I.write_open(output)?;
                    self.push_tag(Tag::I);
                }
            },
            C::Caret => output.write_fmt(format_args!("[^"))?,
            C::Colon => match self.pop_tag() {
                Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_open(output)?;
                    Tag::I.write_open(output)?;
                    self.push_tag(tag);
                    self.push_tag(Tag::I)
                }
                Some(tag) => {
                    output.write_all(b":")?;
                    Tag::I.write_open(output)?;
                    self.push_tag(tag);
                    self.push_tag(Tag::I)
                }
                None => {
                    output.write_all(b":")?;
                    Tag::P.write_open(output)?;
                    Tag::I.write_open(output)?;
                    self.push_tag(Tag::P);
                    self.push_tag(Tag::I);
                }
            },
            C::SqBracketL => {
                if self.stack_empty() {
                    Tag::P.write_open(output)?;
                    self.push_tag(Tag::P);
                };
                output.write_all(b"[")?;
                Tag::I.write_open(output)?;
                self.push_tag(Tag::I);
            }
            // __ -> _
            C::Underscore | C::Content => (),
            C::Asterisk => {
                Tag::Strong.write_open(output)?;
                self.push_tag(Tag::Strong);
            }
            C::SqBracketR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => {
                    if self.stack_empty() {
                        Tag::P.write_open(output)?;
                        self.push_tag(Tag::P);
                    };
                    tag.write_link_no_title(output)?;
                }
                Some(tag @ Tag::FootNoteLink(_)) => {
                    tag.write_open(output)?;
                }
                Some(tag) => {
                    output.write_all(b"]")?;
                    self.push_tag(tag);
                }
                None => output.write_all(b"]")?,
            },
            C::ParenL => match self.pop_tag() {
                Some(Tag::Link(ref url)) => {
                    output.write_fmt(format_args!("[{url}]("))?;
                }
                Some(tag) => {
                    output.write_all(b"(")?;
                    self.push_tag(tag);
                }
                None => {
                    output.write_all(b"(")?;
                }
            },
            C::ParenR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => tag.write_close(output)?,
                Some(tag) => {
                    output.write_all(b")")?;
                    self.push_tag(tag);
                }
                None => {
                    output.write_all(b")")?;
                }
            },
            C::Digit => match self.pop_tag() {
                Some(Tag::FootNoteRef(n)) => {
                    let n = n.ix();
                    output.write_fmt(format_args!("[^{n}]"))?;
                }
                Some(tag) => {
                    self.push_tag(tag);
                }
                None => (),
            },
        };
        Ok(None)
    }
    fn transcribe_asterisk<O: Write>(&mut self, output: &mut O) -> SamupResult<Option<C>> {
        match self.prev_c {
            C::Whitespace => {
                Tag::Strong.write_open(output)?;
                self.push_tag(Tag::Strong);
            }
            C::Newline => match self.pop_tag() {
                Some(tag) => {
                    output.write_all(b"\n")?;
                    Tag::Strong.write_open(output)?;
                    self.push_tag(tag);
                    self.push_tag(Tag::Strong);
                }
                None => {
                    output.write_all(b"\n")?;
                    Tag::P.write_open(output)?;
                    Tag::Strong.write_open(output)?;
                    self.push_tag(Tag::P);
                    self.push_tag(Tag::Strong);
                }
            },
            C::Octothorpe => match self.pop_tag() {
                Some(tag @ Tag::H(_)) => {
                    tag.write_open(output)?;
                    self.push_tag(tag);
                    Tag::Strong.write_open(output)?;
                    self.push_tag(Tag::Strong);
                }
                Some(tag) => {
                    output.write_all(b"#")?;
                    Tag::I.write_open(output)?;
                    self.push_tag(tag);
                    self.push_tag(Tag::Strong);
                }
                None => {
                    output.write_all(b"#")?;
                    Tag::I.write_open(output)?;
                    self.push_tag(Tag::Strong);
                }
            },
            C::Caret => output.write_fmt(format_args!("[^"))?,
            C::Colon => match self.pop_tag() {
                Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_open(output)?;
                    Tag::Strong.write_open(output)?;
                    self.push_tag(tag);
                    self.push_tag(Tag::Strong)
                }
                Some(tag) => {
                    output.write_all(b":")?;
                    Tag::Strong.write_open(output)?;
                    self.push_tag(tag);
                    self.push_tag(Tag::Strong)
                }
                None => {
                    output.write_all(b":")?;
                    Tag::P.write_open(output)?;
                    Tag::Strong.write_open(output)?;
                    self.push_tag(Tag::P);
                    self.push_tag(Tag::Strong);
                }
            },
            C::SqBracketL => {
                if self.stack_empty() {
                    Tag::P.write_open(output)?;
                    self.push_tag(Tag::P);
                };
                output.write_all(b"[")?;
                Tag::Strong.write_open(output)?;
                self.push_tag(Tag::Strong);
            }
            // ** -> *
            C::Asterisk | C::Content => (),
            C::Underscore => {
                Tag::I.write_open(output)?;
                self.push_tag(Tag::I);
            }
            C::SqBracketR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => {
                    if self.stack_empty() {
                        Tag::P.write_open(output)?;
                        self.push_tag(Tag::P);
                    };
                    tag.write_link_no_title(output)?;
                }
                Some(tag @ Tag::FootNoteLink(_)) => {
                    tag.write_open(output)?;
                }
                Some(tag) => {
                    output.write_all(b"]")?;
                    self.push_tag(tag);
                }
                None => output.write_all(b"]")?,
            },
            C::ParenL => match self.pop_tag() {
                Some(Tag::Link(ref url)) => {
                    output.write_fmt(format_args!("[{url}]("))?;
                }
                Some(tag) => {
                    output.write_all(b"(")?;
                    self.push_tag(tag);
                }
                None => {
                    output.write_all(b"(")?;
                }
            },
            C::ParenR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => tag.write_close(output)?,
                Some(tag) => {
                    output.write_all(b")")?;
                    self.push_tag(tag);
                }
                None => {
                    output.write_all(b")")?;
                }
            },
            C::Digit => match self.pop_tag() {
                Some(Tag::FootNoteRef(n)) => {
                    let n = n.ix();
                    output.write_fmt(format_args!("[^{n}]"))?;
                }
                Some(tag) => {
                    self.push_tag(tag);
                }
                None => (),
            },
        };
        Ok(None)
    }
    fn transcribe_octothorpe<O: Write>(&mut self, output: &mut O) -> SamupResult<Option<C>> {
        match self.prev_c {
            C::Content | C::Whitespace => (),
            C::Newline => {
                match self.pop_tag() {
                    Some(tag @ Tag::H(_)) => {
                        tag.write_close(output)?;
                        output.write_all(b"\n")?;
                    }
                    Some(tag) => {
                        output.write_all(b"\n")?;
                        self.push_tag(tag)
                    }
                    None => output.write_all(b"\n")?,
                }
                self.push_tag(Tag::new_h());
                return Ok(Some(C::Octothorpe));
            }
            C::Octothorpe => match self.pop_tag() {
                Some(mut tag @ Tag::H(_)) => {
                    if !tag.inc_h() {
                        tag.write_open(output)?;
                        self.push_tag(tag);
                    } else {
                        self.push_tag(tag);
                        return Ok(Some(C::Octothorpe));
                    }
                }
                Some(tag) => {
                    self.push_tag(tag);
                }
                None => (),
            },
            C::Asterisk => {
                Tag::Strong.write_open(output)?;
                output.write_all(b"#")?;
                self.push_tag(Tag::Strong);
            }
            C::Underscore => {
                Tag::I.write_open(output)?;
                output.write_all(b"#")?;
                self.push_tag(Tag::I);
            }
            C::SqBracketL => {
                match self.pop_tag() {
                    None => {
                        Tag::P.write_open(output)?;
                        self.push_tag(Tag::P);
                    }
                    Some(Tag::Link(_)) | Some(Tag::FootNoteRef(_)) | Some(Tag::FootNoteLink(_)) => {
                    }
                    Some(tag) => self.push_tag(tag),
                }
                output.write_all(b"[")?;
            }
            C::SqBracketR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => {
                    if self.stack_empty() {
                        Tag::P.write_open(output)?;
                        self.push_tag(Tag::P);
                    };
                    tag.write_link_no_title(output)?
                }
                Some(tag @ Tag::FootNoteLink(_)) => tag.write_open(output)?,
                Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_open(output)?;
                    self.push_tag(tag);
                }
                Some(tag) => {
                    output.write_all(b"]")?;
                    self.push_tag(tag)
                }
                None => output.write_all(b"]")?,
            },
            C::ParenL => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => self.push_tag(tag),
                Some(tag) => {
                    output.write_all(b"(")?;
                    self.push_tag(tag);
                }
                None => output.write_all(b"(")?,
            },
            C::ParenR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => tag.write_close(output)?,
                Some(tag) => {
                    output.write_all(b")")?;
                    self.push_tag(tag);
                }
                None => output.write_all(b")")?,
            },
            C::Caret => {
                match self.pop_tag() {
                    None | Some(Tag::FootNoteLink(_)) | Some(Tag::FootNoteRef(_)) => (),
                    Some(tag) => self.push_tag(tag),
                }
                output.write_all(b"^")?;
            }
            C::Colon => match self.pop_tag() {
                Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_open(output)?;
                    self.push_tag(tag);
                }
                Some(tag) => {
                    output.write_all(b":")?;
                    self.push_tag(tag);
                }
                None => output.write_all(b":")?,
            },
            C::Digit => match self.pop_tag() {
                Some(Tag::FootNoteLink(n)) | Some(Tag::FootNoteRef(n)) => {
                    let n = n.ix();
                    output.write_fmt(format_args!("[^{n}"))?;
                }
                Some(tag) => self.push_tag(tag),
                None => (),
            },
        }
        output.write_all(b"#")?;
        Ok(Some(C::Content))
    }
    fn transcribe_caret<O: Write>(&mut self, output: &mut O) -> SamupResult<Option<C>> {
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
    fn transcribe_colon<O: Write>(&mut self, output: &mut O) -> SamupResult<Option<C>> {
        match self.prev_c {
            C::SqBracketR => match self.pop_tag() {
                Some(Tag::FootNoteLink(n)) => {
                    self.push_tag(Tag::FootNoteRef(n));
                    return Ok(None);
                }
                Some(tag) => {
                    output.write_all(b"]:")?;
                    self.push_tag(tag);
                }
                None => output.write_all(b"]:")?,
            },
            _ => match self.pop_tag() {
                Some(mut tag @ Tag::Link(_)) => {
                    tag.push_link(":");
                    self.push_tag(tag);
                }
                Some(tag) => {
                    output.write_all(b":")?;
                    self.push_tag(tag);
                }
                None => output.write_all(b":")?,
            },
        };
        Ok(Some(C::Content))
    }
    fn transcribe_sq_bracket_l<O: Write>(&mut self, output: &mut O) -> SamupResult<Option<C>> {
        match self.prev_c {
            C::Underscore => {
                Tag::I.write_open(output)?;
                self.push_tag(Tag::I);
            }
            C::Asterisk => {
                Tag::Strong.write_open(output)?;
                self.push_tag(Tag::I);
            }
            C::Octothorpe => match self.pop_tag() {
                Some(tag @ Tag::H(_)) => {
                    tag.write_open(output)?;
                    self.push_tag(tag);
                }
                Some(tag) => {
                    output.write_all(b"#")?;
                    self.push_tag(tag);
                }
                None => output.write_all(b"#")?,
            },
            C::SqBracketL => {
                if self.stack_empty() {
                    Tag::P.write_open(output)?;
                    self.push_tag(Tag::P);
                }
                output.write_all(b"[")?;
            }
            C::SqBracketR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => {
                    if self.stack_empty() {
                        Tag::P.write_open(output)?;
                        self.push_tag(Tag::P);
                    };
                    tag.write_link_no_title(output)?;
                }
                Some(tag) => {
                    self.push_tag(tag);
                }
                None => {
                    if self.stack_empty() {
                        Tag::P.write_open(output)?;
                        self.push_tag(Tag::P);
                    }
                }
            },
            C::Caret => {
                output.write_all(b"[^")?;
            }
            C::ParenR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => {
                    tag.write_close(output)?;
                }
                Some(tag) => {
                    output.write_all(b")")?;
                    self.push_tag(tag);
                }
                None => (),
            },
            C::Digit => match self.pop_tag() {
                Some(Tag::FootNoteLink(n)) | Some(Tag::FootNoteRef(n)) => {
                    let n = n.ix();
                    output.write_fmt(format_args!("[^{n}]"))?;
                }
                Some(tag) => self.push_tag(tag),
                None => (),
            },
            C::Colon => match self.pop_tag() {
                Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_open(output)?;
                    self.push_tag(tag);
                }
                Some(tag) => {
                    self.push_tag(tag);
                }
                None => (),
            },
            C::ParenL => match self.pop_tag() {
                Some(Tag::Link(ref url)) => {
                    output.write_fmt(format_args!("[{url}]("))?;
                }
                Some(tag) => {
                    output.write_all(b"(")?;
                    self.push_tag(tag);
                }
                None => {
                    output.write_all(b"(")?;
                }
            },
            C::Newline => {
                match self.pop_tag() {
                    Some(tag @ Tag::H(_)) => {
                        tag.write_close(output)?;
                        output.write_all(b"\n")?;
                    }
                    Some(tag) => {
                        output.write_all(b"\n")?;
                        self.push_tag(tag);
                    }
                    None => (),
                };
            }
            C::Content | C::Whitespace => (),
        };
        Ok(None)
    }
    fn transcribe_sq_bracket_r<O: Write>(&mut self, output: &mut O) -> SamupResult<Option<C>> {
        let mut next_c: Option<C> = None;
        match self.pop_tag() {
            Some(
                mut tag @ Tag::Link(InnerLink {
                    state: LinkState::Link,
                    ..
                }),
            ) => match self.prev_c {
                C::Whitespace => {
                    self.push_tag(tag);
                }
                C::Newline => {
                    let url = tag.link_url();
                    output.write_fmt(format_args!("{url}\n]"))?;
                    return Ok(next_c);
                }
                C::Underscore => {
                    tag.push_link("_");
                    self.push_tag(tag);
                }
                C::Asterisk => {
                    tag.push_link("*");
                    self.push_tag(tag);
                }
                C::Caret => {
                    tag.push_link("^");
                    self.push_tag(tag);
                }
                C::Colon => {
                    tag.push_link(":");
                    self.push_tag(tag);
                }
                C::SqBracketL => {
                    tag.push_link("[");
                    self.push_tag(tag);
                }
                C::SqBracketR => {
                    tag.push_link("]");
                    self.push_tag(tag);
                }
                C::ParenL => {
                    tag.push_link("(");
                    self.push_tag(tag)
                }
                C::ParenR => {
                    tag.push_link(")");
                    self.push_tag(tag)
                }
                C::Octothorpe => {
                    tag.push_link("#");
                    self.push_tag(tag)
                }
                C::Digit | C::Content => self.push_tag(tag),
            },
            Some(
                tag @ Tag::Link(InnerLink {
                    state: LinkState::Label,
                    ..
                }),
            ) => match self.prev_c {
                C::Newline => {
                    let url = tag.link_url();
                    output.write_fmt(format_args!("{url}\n]"))?;
                }
                C::Underscore => {
                    output.write_all(b"_")?;
                    self.push_tag(tag);
                }
                C::Asterisk => {
                    output.write_all(b"*")?;
                    self.push_tag(tag);
                }
                C::Caret => {
                    output.write_all(b"^")?;
                    self.push_tag(tag);
                }
                C::Colon => {
                    output.write_all(b":")?;
                    self.push_tag(tag);
                }
                C::SqBracketL => {
                    output.write_all(b"[")?;
                    self.push_tag(tag);
                }
                C::SqBracketR => {
                    output.write_all(b"]")?;
                    self.push_tag(tag);
                }
                C::ParenL => {
                    output.write_all(b"(")?;
                    self.push_tag(tag)
                }
                C::ParenR => {
                    output.write_all(b")")?;
                    self.push_tag(tag)
                }
                C::Octothorpe => {
                    output.write_all(b"#")?;
                    self.push_tag(tag)
                }
                C::Digit | C::Content | C::Whitespace => self.push_tag(tag),
            },

            Some(tag @ Tag::FootNoteLink(_)) | Some(tag @ Tag::FootNoteRef(_)) => {
                self.push_tag(tag);
            }
            Some(tag) => {
                output.write_all(b"]")?;
                self.push_tag(tag);
                next_c = Some(C::Content);
            }
            None => {
                Tag::P.write_open(output)?;
                self.push_tag(Tag::P);
                output.write_all(b"]")?;
                next_c = Some(C::Content);
            }
        }
        Ok(next_c)
    }
    fn transcribe_paren<O: Write>(&mut self, output: &mut O) -> SamupResult<Option<C>> {
        match self.prev_c {
            C::Whitespace | C::Content => (),
            C::Newline => (),
            C::SqBracketL => {
                if self.stack_empty() {
                    Tag::P.write_open(output)?;
                    self.push_tag(Tag::P);
                }
                output.write_all(b"[")?;
            }
            C::ParenL => {
                output.write_all(b"(")?;
            }
            C::ParenR => {
                output.write_all(b")")?;
            }
            C::SqBracketR => match self.pop_tag() {
                Some(
                    mut tag @ Tag::Link(InnerLink {
                        state: LinkState::Link,
                        ..
                    }),
                ) => {
                    tag.write_open(output)?;
                    tag.end_url();
                    self.push_tag(tag);
                }
                Some(tag @ Tag::FootNoteLink(_)) => {
                    tag.write_open(output)?;
                }
                Some(tag) => {
                    output.write_all(b"]")?;
                    self.push_tag(tag);
                }
                None => {
                    Tag::P.write_open(output)?;
                    self.push_tag(Tag::P);
                    output.write_all(b"]")?;
                }
            },
            C::Underscore => {
                Tag::I.write_open(output)?;
                self.push_tag(Tag::I);
            }
            C::Asterisk => {
                Tag::Strong.write_open(output)?;
                self.push_tag(Tag::Strong);
            }
            C::Octothorpe => match self.pop_tag() {
                Some(tag @ Tag::H(_)) => {
                    tag.write_open(output)?;
                    self.push_tag(tag);
                }
                Some(tag) => {
                    output.write_all(b"#")?;
                    self.push_tag(tag);
                }
                None => output.write_all(b"#")?,
            },
            C::Caret => output.write_all(b"[^")?,
            C::Colon => match self.pop_tag() {
                Some(tag @ Tag::FootNoteRef(_)) => {
                    tag.write_open(output)?;
                    self.push_tag(tag);
                }
                Some(tag) => {
                    output.write_all(b":")?;
                    self.push_tag(tag);
                }
                None => {
                    output.write_all(b":")?;
                }
            },
            C::Digit => match self.pop_tag() {
                Some(Tag::FootNoteLink(n)) | Some(Tag::FootNoteRef(n)) => {
                    let n = n.ix();
                    output.write_fmt(format_args!("[^{n}"))?;
                }
                // unreachable?
                Some(tag) => self.push_tag(tag),
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
    ) -> SamupResult<Option<C>> {
        if self.prev_c == C::Caret {
            match self.pop_tag() {
                // no nested footnotes
                Some(Tag::FootNoteLink(n)) | Some(Tag::FootNoteRef(n)) => {
                    let ix = n.ix();
                    let c = crate::char_to_digit(curr_char);
                    output.write_fmt(format_args!("[^{ix}{c}"))?
                }
                Some(tag) => {
                    self.push_tag(tag);
                    self.push_tag(Tag::new_fn_link(curr_char));
                    return Ok(None);
                }
                None => {
                    self.push_tag(Tag::new_fn_link(curr_char));
                    return Ok(None);
                }
            }
        } else {
            match self.pop_tag() {
                Some(mut tag @ Tag::FootNoteLink(_)) | Some(mut tag @ Tag::FootNoteRef(_)) => {
                    tag.push_fn_digit(curr_char);
                    self.push_tag(tag);
                    return Ok(None);
                }
                Some(mut tag @ Tag::Link(_)) => {
                    tag.push_link(str::from_utf8(&[curr_char]).unwrap());
                    self.push_tag(tag);
                }
                Some(tag) => {
                    output.write_all(&[curr_char])?;
                    self.push_tag(tag);
                }
                None => {
                    output.write_all(&[curr_char])?;
                }
            };
        }
        Ok(Some(C::Content))
    }
    fn transcribe_content<O: Write>(
        &mut self,
        curr_char: u8,
        output: &mut O,
    ) -> SamupResult<Option<C>> {
        match self.prev_c {
            C::Whitespace | C::Content => match self.pop_tag() {
                None => {
                    Tag::P.write_open(output)?;
                    self.push_tag(Tag::P);
                }
                Some(
                    mut tag @ Tag::Link(InnerLink {
                        state: LinkState::Link,
                        ..
                    }),
                ) => {
                    tag.push_link(str::from_utf8(&[curr_char]).unwrap());
                    self.push_tag(tag);
                    return Ok(None);
                }
                Some(tag) => {
                    self.push_tag(tag);
                }
            },
            C::Newline => match self.pop_tag() {
                Some(tag @ Tag::H(_)) => {
                    tag.write_close(output)?;
                    output.write_all(b"\n")?;
                }
                Some(tag) => {
                    output.write_all(b"\n")?;
                    self.push_tag(tag)
                }
                None => {
                    Tag::P.write_open(output)?;
                    self.push_tag(Tag::P);
                }
            },
            C::Underscore => match self.pop_tag() {
                None => {
                    Tag::P.write_open(output)?;
                    self.push_tag(Tag::P);
                }
                Some(tag) => {
                    self.push_tag(tag);
                }
            },
            C::Asterisk => match self.pop_tag() {
                None => {
                    Tag::P.write_open(output)?;
                    self.push_tag(Tag::P);
                }
                Some(tag) => {
                    self.push_tag(tag);
                }
            },
            C::Octothorpe => match self.pop_tag() {
                Some(tag @ Tag::H(_)) => {
                    tag.write_open(output)?;
                    self.push_tag(tag);
                }
                Some(tag) => {
                    output.write_all(b"#")?;
                    self.push_tag(tag);
                }
                None => output.write_all(b"#")?,
            },
            C::Caret => output.write_all(b"^")?,
            C::Colon => output.write_all(b":")?,
            C::SqBracketL => {
                match self.pop_tag() {
                    Some(Tag::Link(s)) => {
                        output.write_fmt(format_args!("[{s}"))?;
                    }
                    Some(tag) => self.push_tag(tag),
                    None => {}
                };
                self.push_tag(Tag::new_link(curr_char));
                return Ok(None);
            }
            C::SqBracketR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) | Some(tag @ Tag::FootNoteLink(_)) => {
                    tag.write_open(output)?;
                    self.push_tag(tag);
                }
                Some(Tag::FootNoteRef(n)) => {
                    let ix = n.ix();
                    output.write_fmt(format_args!("[^{ix}]"))?;
                }
                Some(tag) => self.push_tag(tag),
                None => {
                    if self.stack_empty() {
                        Tag::P.write_open(output)?;
                        self.push_tag(Tag::P);
                    }
                }
            },
            C::ParenL => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => {
                    self.push_tag(tag);
                }
                Some(tag) => {
                    output.write_all(b"(")?;
                    self.push_tag(tag);
                }
                None => output.write_all(b"(")?,
            },
            C::ParenR => match self.pop_tag() {
                Some(tag @ Tag::Link(_)) => {
                    tag.write_close(output)?;
                }
                Some(tag) => {
                    output.write_all(b")")?;
                    self.push_tag(tag);
                }
                None => output.write_all(b")")?,
            },
            C::Digit => match self.pop_tag() {
                Some(Tag::FootNoteLink(n)) | Some(Tag::FootNoteRef(n)) => {
                    let ix = n.ix();
                    output.write_fmt(format_args!("[^{ix}"))?;
                }
                // shouldn't happen
                Some(tag) => self.push_tag(tag),
                None => (),
            },
        }
        output.write_all(&[curr_char])?;
        Ok(None)
    }
    fn push_tag(&mut self, tag: Tag) {
        self.tag_stack.push_front(tag);
    }
    fn pop_tag(&mut self) -> Option<Tag> {
        self.tag_stack.pop_front()
    }
    fn stack_empty(&self) -> bool {
        self.tag_stack.front().is_none()
    }
}

impl Default for Transcriber {
    fn default() -> Self {
        Self::new()
    }
}
