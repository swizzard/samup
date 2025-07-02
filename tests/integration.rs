use samup::{SamupResult, transcribe};

// let s = unsafe { str::from_utf8_unchecked(&output) };
// println!("test_ actually {s}");

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
    let expected_out = b"\n<p><i>italic</i> <strong>strong</strong></p>";
    transcribe(input, &mut output)?;
    let o: &[u8] = output.as_ref();
    assert_eq!(&expected_out, &o, "terminated");
    output.clear();
    let input = b"_italic";
    let expected_out = b"\n<p><i>italic</i></p>";
    transcribe(input, &mut output)?;
    let o: &[u8] = output.as_ref();
    assert_eq!(&expected_out, &o, "unterminated");
    Ok(())
}

#[test]
fn test_h() -> SamupResult {
    let mut output = Vec::new();
    let input = b"# h";
    let expected_output = b"\n<h1>h</h1>";
    transcribe(input, &mut output)?;
    let o: &[u8] = output.as_ref();
    assert_eq!(&expected_output, &o, "h1");
    output.clear();
    let input = b"####### h6";
    let expected_output = b"\n<h6># h6</h6>";
    transcribe(input, &mut output)?;
    let o: &[u8] = output.as_ref();
    assert_eq!(&expected_output, &o, "h6#");
    output.clear();
    let input = b"# h1\n## h2";
    let expected_output = b"\n<h1>h1</h1>\n<h2>h2</h2>";
    transcribe(input, &mut output)?;
    let o: &[u8] = output.as_ref();
    assert_eq!(&expected_output, &o, "h multiple");
    Ok(())
}

#[test]
fn test_link_no_label() -> SamupResult {
    let mut output = Vec::new();
    let input = b"[https://swizzard.pizza]";
    let expected_output =
        b"\n<p><a href=\"https://swizzard.pizza\" target=\"_blank\">https://swizzard.pizza</a></p>";
    transcribe(input, &mut output)?;
    let s = unsafe { str::from_utf8_unchecked(&output) };
    println!("test_link_no_label actually {s}");
    let o: &[u8] = output.as_ref();
    assert_eq!(&expected_output, &o, "link no label");
    Ok(())
}

#[test]
fn test_link_label() -> SamupResult {
    let mut output = Vec::new();
    let input = b"[https://swizzard.pizza](my website)";
    let expected_output =
        b"\n<p><a href=\"https://swizzard.pizza\" target=\"_blank\">my website</a></p>";
    transcribe(input, &mut output)?;
    let o: &[u8] = output.as_ref();
    assert_eq!(&expected_output, &o, "link label");
    Ok(())
}

#[test]
fn test_foot_note_link() -> SamupResult {
    let mut output = Vec::new();
    let input = b"note[^1]";
    let expected_output = b"<p>note<a id=\"link-1\" target=\"#ref-1\"><sup>1</sup></a></p>";
    transcribe(input, &mut output)?;
    // let s = unsafe { str::from_utf8_unchecked(&output) };
    // println!("test_foot_note_link actually {s}");
    let o: &[u8] = output.as_ref();
    assert_eq!(&expected_output, &o, "foot note link");
    output.clear();
    let input = b"note[^12]";
    let expected_output = b"<p>note<a id=\"link-12\" target=\"#ref-12\"><sup>12</sup></a></p>";
    transcribe(input, &mut output)?;
    let s = unsafe { str::from_utf8_unchecked(&output) };
    println!("test_foot_note_link actually {s}");
    let o: &[u8] = output.as_ref();
    assert_eq!(&expected_output, &o, "foot note link");
    Ok(())
}

// #[test]
// fn test_foot_note_ref() -> SamupResult {}
