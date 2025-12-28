
use regex::Regex;

pub fn extract_first_url(text: &str, _def: &str) -> String {
    // 匹配 http:// 或 https:// 开头的 URL
    // 这里使用一个相对宽松但实用的正则，能匹配大多数常见 URL
    let re = Regex::new(r"(?i)https?://[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)").unwrap();

    // find 返回第一个匹配的子串的 (&str, 位置)
    if let Some(matched) = re.find(text) {
        matched.as_str().to_string()
    } else {
        _def.to_string()
    }
}