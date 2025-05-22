use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{is_a, tag, take_till1},
    character::complete::{alphanumeric1, newline},
    multi::{many_till, many0, many1},
    sequence::preceded,
};

#[derive(Debug, PartialEq)]
pub enum El {
    Header(String),
    Paragraph(String),
}

const fn is_special_char(c: char) -> bool {
    c == '*' || c == '_' || c == '['
}

const fn ends_block(c: char) -> bool {
    c == '\n'
}

fn parse_header(input: &str) -> IResult<&str, El> {
    preceded(many0(newline), preceded(tag("#"), take_till1(ends_block)))
        .parse(input)
        .map(|(remaining, parsed)| (remaining, El::Header(parsed.trim_start().into())))
}

fn parse_paragraph(input: &str) -> IResult<&str, El> {
    let (remaining, (ss, _)) = preceded(
        many1(newline),
        many_till(alt((alphanumeric1, is_a(".,?"))), tag("\n\n")),
    )
    .parse(input)?;
    let mut s = String::with_capacity(ss.len());
    for v in ss {
        s.push_str(v);
    }
    Ok((remaining, El::Paragraph(s)))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_header() {
        let s = "# header";
        let (rem, el) = parse_header(s).unwrap();
        assert_eq!(rem, "", "simple");
        assert_eq!(el, El::Header(String::from("header")), "simple");
        let s = "\n#header";
        let (rem, el) = parse_header(s).expect("leading newline");
        assert_eq!(rem, "", "leading newline");
        assert_eq!(el, El::Header(String::from("header")), "leading newline");
        let s = "# header\ncontent";
        let (rem, el) = parse_header(s).expect("trailing content");
        assert_eq!(rem, "\ncontent", "trailing content");
        assert_eq!(el, El::Header(String::from("header")), "trailing content");
    }
    #[test]
    fn test_parse_paragraph() {
        let s = "\ncontent\n\n";
        let (rem, el) = parse_paragraph(s).expect("simple");
        assert_eq!(rem, "", "simple");
        assert_eq!(el, El::Paragraph(String::from("content")), "simple")
    }
}
