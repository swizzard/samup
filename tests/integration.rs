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
fn test_paragraph() -> SamupResult {
    let mut output = Vec::new();
    let input = b"abc\n\ndef";
    let expected_out = b"<p>abc</p>\n<p>def</p>";
    transcribe(input, &mut output)?;
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
// fn test_h() -> SamupResult {
//     let mut output = Vec::new();
//     let input = b"# highlight 1\ncontent";
//     let expected_output = b"<h1>highlight 1</h1><p>content</p>";
//     transcribe(input, &mut output)?;
//     let s = unsafe { str::from_utf8_unchecked(&output) };
//     println!("test_h actually {s}");
//     let o: &[u8] = output.as_ref();
//     assert_eq!(&expected_output, &o);
//     Ok(())
// }

// #[test]
// fn test_link
