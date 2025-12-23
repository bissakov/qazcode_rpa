rust_i18n::i18n!("crates/rpa-studio/locales", fallback = "en");

fn main() {
    println!("Default locale: {:?}", rust_i18n::locale());
    println!("Available locales: {:?}", rust_i18n::available_locales!());
    println!("EN menu.file: {}", rust_i18n::t!("menu.file"));

    rust_i18n::set_locale("ru");
    println!("\nRU menu.file: {}", rust_i18n::t!("menu.file"));
}
