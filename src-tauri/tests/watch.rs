use std::fs::{exists, remove_file, OpenOptions};
use std::io::{Read, Seek, Write};

#[test]
fn test_file() {
    let mut f = OpenOptions::new().write(true).read(true).create(true).open("tests/test.txt").unwrap();
    let mut buf = String::new();

    f.set_len(0).unwrap();
    f.read_to_string(&mut buf).unwrap();
    assert!(buf.is_empty());

    f.write("I am a line".as_bytes()).unwrap();
    f.rewind().unwrap();
    f.read_to_string(&mut buf).unwrap();

    assert!(!buf.is_empty());
    assert_eq!(buf, "I am a line");

    remove_file("tests/test.txt").unwrap();
    assert!(!exists("tests/test.txt").unwrap())
}

#[test]
fn test_watch_integration() {}