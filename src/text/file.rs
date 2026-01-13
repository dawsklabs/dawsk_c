use std::collections::HashMap;
use std::{cell::RefCell, fs};

thread_local! {
    static FILES: RefCell<HashMap<&'static str, Vec<u8>>> =
        RefCell::new(HashMap::default());
    static CURRENT_FILE: RefCell<&'static str> =
        RefCell::new("unknown");
}

pub fn set(file: &'static str) {
    let bytes = fs::read(file).unwrap_or({
        println!("File not found: {}", file);
        vec![]
    });

    FILES.with(|map| {
        map.borrow_mut().insert(file, bytes);
    });

    CURRENT_FILE.with(|f| *f.borrow_mut() = file);
}

pub fn get() -> &'static str {
    CURRENT_FILE.with(|f| *f.borrow())
}

pub fn content() -> Vec<u8> {
    if let Some(c) = FILES.with(|map| {
        let map = map.borrow();
        map.get(get()).cloned()
    }) {
        return c;
    } else {
        return vec![]; // Datei nicht gefunden
    }
}
