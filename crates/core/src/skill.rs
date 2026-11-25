//! SKILL.md 읽기·쓰기. 머리말(YAML frontmatter)과 본문을 나눠요.
//!
//! Claude, Claude Code, Codex는 모두 같은 형식을 써요:
//! ```text
//! ---
//! name: my-skill
//! description: 언제 이 스킬을 쓰는지
//! ---
//! 본문 (마크다운)
//! ```

use serde_yaml::{Mapping, Value};

pub const SKILL_FILE: &str = "SKILL.md";

#[derive(Debug, Clone, Default)]
pub struct Parsed {
    pub has_frontmatter: bool,
    pub fm: Option<Mapping>,
    pub fm_error: Option<String>,
    pub name: String,
    pub description: String,
    pub body: String,
}

/// (머리말 텍스트, 본문)으로 나눠요. 머리말이 없으면 `None`.
pub fn split(raw: &str) -> (Option<&str>, &str) {
    let s = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    let first_nl = match s.find('\n') {
        Some(i) => i,
        None => return (None, s),
    };
    if s[..first_nl].trim_end() != "---" {
        return (None, s);
    }
    let rest = &s[first_nl + 1..];
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        let t = line.trim_end();
        if t == "---" || t == "..." {
            let fm = &rest[..offset];
            let body = &rest[offset + line.len()..];
            return (Some(fm), body);
        }
        offset += line.len();
    }
    (None, s)
}

fn scalar(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => serde_yaml::to_string(other).unwrap_or_default().trim().to_string(),
    }
}

pub fn parse(raw: &str) -> Parsed {
    let (fm_text, body) = split(raw);
    let mut p = Parsed { body: body.to_string(), ..Default::default() };
    let Some(fm_text) = fm_text else {
        return p;
    };
    p.has_frontmatter = true;
    if fm_text.trim().is_empty() {
        p.fm = Some(Mapping::new());
        return p;
    }
    match serde_yaml::from_str::<Value>(fm_text) {
        Ok(Value::Mapping(m)) => {
            if let Some(v) = m.get("name") {
                p.name = scalar(v).trim().to_string();
            }
            if let Some(v) = m.get("description") {
                p.description = scalar(v).trim().to_string();
            }
            p.fm = Some(m);
        }
        Ok(Value::Null) => p.fm = Some(Mapping::new()),
        Ok(_) => p.fm_error = Some(tr!("머리말이 'key: value' 형식이 아니에요", "The frontmatter is not in 'key: value' form")),
        Err(e) => p.fm_error = Some(short_yaml_error(&e.to_string())),
    }
    p
}

fn short_yaml_error(e: &str) -> String {
    let e: String = e.chars().take(160).collect();
    e.trim().to_string()
}

/// 이름·설명·본문을 바꾼 SKILL.md를 만들어요. 다른 머리말 키(license, allowed-tools …)는 그대로 둬요.
pub fn with_fields(raw: &str, name: &str, description: &str, body: Option<&str>) -> Result<String, String> {
    let p = parse(raw);
    if let Some(e) = p.fm_error {
        return Err(tr!("머리말을 읽을 수 없어서 양식으로 저장할 수 없어요 — 원본 편집에서 고쳐 주세요 ({e})", "The frontmatter cannot be read, so the form cannot save — fix it in the raw editor ({e})"));
    }
    let mut map = p.fm.unwrap_or_default();
    let name_key = Value::String("name".into());
    let desc_key = Value::String("description".into());
    if map.contains_key(&name_key) || map.contains_key(&desc_key) {
        map.insert(name_key, Value::String(name.trim().into()));
        map.insert(desc_key, Value::String(description.trim().into()));
    } else {
        // 이름·설명이 맨 위에 오게 새로 만들어요.
        let mut m = Mapping::new();
        m.insert(name_key, Value::String(name.trim().into()));
        m.insert(desc_key, Value::String(description.trim().into()));
        for (k, v) in map {
            m.insert(k, v);
        }
        map = m;
    }
    let body = body.map(|b| b.to_string()).unwrap_or(p.body);
    Ok(compose_map(&map, &body))
}

fn compose_map(map: &Mapping, body: &str) -> String {
    let yaml = serde_yaml::to_string(map).unwrap_or_default();
    let body = body.strip_prefix('\n').unwrap_or(body);
    format!("---\n{yaml}---\n\n{body}")
}

pub fn compose(name: &str, description: &str, body: &str) -> String {
    let mut m = Mapping::new();
    m.insert("name".into(), Value::String(name.into()));
    m.insert("description".into(), Value::String(description.into()));
    compose_map(&m, body)
}

pub fn template_body(name: &str) -> String {
    format!(
        "# {name}\n\n\
         ## 언제 쓰나요\n\n\
         - 이 스킬이 필요한 상황을 적어 주세요.\n\n\
         ## 방법\n\n\
         1. 첫 번째 단계\n\
         2. 두 번째 단계\n\n\
         ## 참고\n\n\
         - 길어지는 내용은 `reference.md` 같은 별도 파일로 나누고 여기서 링크하세요.\n"
    )
}

/// 문자열을 스킬 이름 규칙(영어 소문자·숫자·하이픈, 64자)에 맞게 바꿔요.
pub fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in s.trim().chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    let mut out: String = out.trim_matches('-').chars().take(64).collect();
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "skill".into()
    } else {
        out
    }
}

pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "---\nname: pdf-tools\ndescription: \"PDF를 다뤄요: 합치기, 나누기\"\nlicense: MIT\nallowed-tools: Read, Grep\n---\n\n# PDF\n본문\n";

    #[test]
    fn parses_frontmatter_and_body() {
        let p = parse(SAMPLE);
        assert!(p.has_frontmatter);
        assert_eq!(p.name, "pdf-tools");
        assert_eq!(p.description, "PDF를 다뤄요: 합치기, 나누기");
        assert!(p.body.contains("# PDF"));
    }

    #[test]
    fn crlf_and_bom_are_ok() {
        let raw = "\u{feff}---\r\nname: a\r\ndescription: b\r\n---\r\nbody";
        let p = parse(raw);
        assert_eq!(p.name, "a");
        assert_eq!(p.description, "b");
        assert_eq!(p.body, "body");
    }

    #[test]
    fn no_frontmatter() {
        let p = parse("# 그냥 마크다운");
        assert!(!p.has_frontmatter);
        assert!(p.name.is_empty());
    }

    #[test]
    fn bad_yaml_reports_error() {
        let p = parse("---\nname: [unclosed\n---\n");
        assert!(p.fm_error.is_some());
    }

    #[test]
    fn with_fields_keeps_other_keys_and_order() {
        let out = with_fields(SAMPLE, "pdf-kit", "새 설명: 콜론 포함", Some("# 새 본문\n")).unwrap();
        let p = parse(&out);
        assert_eq!(p.name, "pdf-kit");
        assert_eq!(p.description, "새 설명: 콜론 포함");
        assert!(out.contains("license: MIT"));
        assert!(out.find("name:").unwrap() < out.find("license:").unwrap());
        assert_eq!(p.body.trim(), "# 새 본문");
    }

    #[test]
    fn with_fields_adds_frontmatter_when_missing() {
        let out = with_fields("# 본문만", "x", "y", None).unwrap();
        let p = parse(&out);
        assert_eq!((p.name.as_str(), p.description.as_str()), ("x", "y"));
        assert_eq!(p.body.trim(), "# 본문만");
    }

    #[test]
    fn slugify_and_validate() {
        assert_eq!(slugify("My PDF Tools!"), "my-pdf-tools");
        assert_eq!(slugify("한글만"), "skill");
        assert_eq!(slugify("--a__b--"), "a-b");
        assert!(is_valid_name("a-b-1"));
        assert!(!is_valid_name("A"));
        assert!(!is_valid_name("a--b"));
        assert!(!is_valid_name("-a"));
        assert!(!is_valid_name(&"a".repeat(65)));
    }
}
