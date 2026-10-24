//! Front-end HTML / CSS / JS asset loader

pub fn get_html() -> &'static str {
    include_str!("index.html")
}
