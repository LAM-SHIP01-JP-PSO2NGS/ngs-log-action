#[tokio::test]
async fn experimental1() {
 use num_format::{Locale, ToFormattedString};
 println!("{}", 1234567890.to_formatted_string(&Locale::ja));
}
