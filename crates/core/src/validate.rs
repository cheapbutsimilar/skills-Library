//! 스킬 검사. 오류(error)는 에이전트가 스킬을 못 읽는 것, 경고(warn)는 읽히지만 고치면 좋은 것이에요.

use crate::skill::{self, Parsed};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Error,
    Warn,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub level: Level,
    pub code: &'static str,
    pub msg: String,
}

fn err(code: &'static str, msg: impl Into<String>) -> Issue {
    Issue { level: Level::Error, code, msg: msg.into() }
}
fn warn(code: &'static str, msg: impl Into<String>) -> Issue {
    Issue { level: Level::Warn, code, msg: msg.into() }
}

pub const NAME_MAX: usize = 64;
pub const DESC_MAX: usize = 1024;
pub const BODY_LINES_MAX: usize = 500;

fn has_xml_tag(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'<' {
            let j = i + 1;
            let j = if j < b.len() && b[j] == b'/' { j + 1 } else { j };
            if j < b.len() && b[j].is_ascii_alphabetic() {
                if let Some(end) = s[j..].find('>') {
                    let inner = &s[j..j + end];
                    if inner.chars().all(|c| c.is_ascii_alphanumeric() || " -_=\"'/.:".contains(c)) {
                        return true;
                    }
                }
            }
        }
        i += 1;
    }
    false
}

/// `dir_name`은 스킬 폴더 이름, `has_skill_md`는 SKILL.md가 있는지예요.
pub fn check(p: &Parsed, dir_name: &str, has_skill_md: bool) -> Vec<Issue> {
    let mut out = Vec::new();
    if !has_skill_md {
        out.push(err("no-skill-md", format!("{} 파일이 없어요", skill::SKILL_FILE)));
        return out;
    }
    if !p.has_frontmatter {
        out.push(err("no-frontmatter", "맨 위에 ---로 감싼 머리말(name, description)이 필요해요"));
        return out;
    }
    if let Some(e) = &p.fm_error {
        out.push(err("yaml", format!("머리말 YAML을 읽을 수 없어요 — {e}")));
        return out;
    }

    let name = p.name.as_str();
    let n = name.chars().count();
    if name.is_empty() {
        out.push(err("name-empty", "name이 비어 있어요"));
    } else if n > NAME_MAX {
        out.push(err("name-long", format!("name은 {NAME_MAX}자까지예요 (지금 {n}자)")));
    } else if !skill::is_valid_name(name) {
        out.push(err("name-chars", "name은 영어 소문자·숫자·하이픈(-)만 쓸 수 있어요 (하이픈은 처음·끝·연속 불가)"));
    }
    if !name.is_empty() && (name.contains("claude") || name.contains("anthropic")) {
        out.push(warn("name-reserved", "name에 'claude'·'anthropic'이 들어가면 Claude 앱에 올릴 수 없어요"));
    }
    if !name.is_empty() && skill::is_valid_name(name) && name != dir_name {
        out.push(warn(
            "name-dir",
            format!("폴더 이름({dir_name})과 name이 달라요 — 저장하면 폴더 이름을 name에 맞춰요"),
        ));
    }

    let d = p.description.as_str();
    let dn = d.chars().count();
    if d.is_empty() {
        out.push(err("desc-empty", "description이 비어 있어요 — 언제 쓰는 스킬인지 적어 주세요"));
    } else if dn > DESC_MAX {
        out.push(err("desc-long", format!("description은 {DESC_MAX}자까지예요 (지금 {dn}자)")));
    }
    if has_xml_tag(d) {
        out.push(err("desc-xml", "description에 XML 태그(<…>)는 쓸 수 없어요"));
    }
    if !d.is_empty() && dn < 20 {
        out.push(warn("desc-short", "description이 짧아요 — 무엇을 하고 언제 쓰는지 적으면 에이전트가 더 잘 골라요"));
    }

    let lines = p.body.lines().count();
    if p.body.trim().is_empty() {
        out.push(warn("body-empty", "본문이 비어 있어요"));
    } else if lines > BODY_LINES_MAX {
        out.push(warn(
            "body-long",
            format!("본문이 {lines}줄이에요 — {BODY_LINES_MAX}줄을 넘으면 별도 파일로 나누는 게 좋아요"),
        ));
    }
    out
}

pub fn has_errors(issues: &[Issue]) -> bool {
    issues.iter().any(|i| i.level == Level::Error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::parse;

    fn codes(raw: &str, dir: &str) -> Vec<&'static str> {
        check(&parse(raw), dir, true).into_iter().map(|i| i.code).collect()
    }

    #[test]
    fn good_skill_has_no_issues() {
        let raw = "---\nname: good\ndescription: 좋은 스킬이에요. 이럴 때 쓰면 돼요 — 충분히 길게.\n---\n# 본문\n";
        assert!(codes(raw, "good").is_empty());
    }

    #[test]
    fn detects_common_problems() {
        assert_eq!(codes("# x", "a"), vec!["no-frontmatter"]);
        let c = codes("---\nname: Bad_Name\ndescription: \n---\n", "a");
        assert!(c.contains(&"name-chars"));
        assert!(c.contains(&"desc-empty"));
        assert!(c.contains(&"body-empty"));
        let c = codes("---\nname: my-claude-helper\ndescription: <b>태그</b> 들어간 설명이에요 아주 길게 씁니다\n---\nx", "my-claude-helper");
        assert!(c.contains(&"name-reserved"));
        assert!(c.contains(&"desc-xml"));
        let c = codes("---\nname: abc\ndescription: 충분히 긴 설명을 여기에 씁니다 정말로요\n---\nx", "other");
        assert_eq!(c, vec!["name-dir"]);
    }

    #[test]
    fn comparison_signs_are_not_tags() {
        assert!(!has_xml_tag("1 < 2 and 3 > 2"));
        assert!(has_xml_tag("use <example> here"));
    }

    #[test]
    fn missing_skill_md() {
        assert_eq!(check(&parse(""), "x", false)[0].code, "no-skill-md");
    }
}
