use samup::{SamupResult, transcribe};

#[test]
fn test_whitespace_content() -> SamupResult {
    let mut output = Vec::new();
    let input = b"a \nb";
    let expected_out = b"<p>a \nb</p>";
    transcribe(input, &mut output)?;
    let o: &[u8] = output.as_ref();
    assert_eq!(&expected_out, &o);
    Ok(())
}

#[test]
fn test_block() -> SamupResult {
    let mut output = Vec::new();
    let input = b"abc\n\ndef";
    let expected_out = b"<p>cba</p>\n<p>fed</p>";
    transcribe(input, &mut output)?;
    let s = unsafe { str::from_utf8_unchecked(&output) };
    println!("{s}");
    let o: &[u8] = output.as_ref();
    assert_eq!(&expected_out, &o);
    Ok(())
}

#[test]
fn test_inline() -> SamupResult {
    let mut output = Vec::new();
    let input = b"_italic_ *strong*";
    let expected_out = b"<i>italic</i> <strong>strong</strong>";
    transcribe(input, &mut output)?;
    let o: &[u8] = output.as_ref();
    assert_eq!(&expected_out, &o);
    output.clear();
    let input = b"_italic";
    let expected_out = b"<i>italic</i>";
    transcribe(input, &mut output)?;
    let o: &[u8] = output.as_ref();
    assert_eq!(&expected_out, &o, "unterminated");
    Ok(())
}

// #[test]
// fn test_link
