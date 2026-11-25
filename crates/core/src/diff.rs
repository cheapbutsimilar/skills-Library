//! 라이브러리 원본(a)과 설치본(b) 비교. 덮어쓰기·가져오기 전에 무엇이 달라지는지 보여줘요.
//!
//! - 파일 단위: 추가(설치본에만), 삭제(라이브러리에만), 수정, 같음
//! - 텍스트 파일은 줄 단위 차이(Myers)를 앞뒤 3줄과 함께 묶음(hunk)으로 돌려줘요.

use crate::fsutil::{self, join_rel};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// 한 파일에서 이보다 크면 줄 비교를 하지 않아요.
const TEXT_MAX: u64 = 512 * 1024;
/// 줄이 이보다 많이 다르면 "너무 많이 달라요"로 줄여요 (시간·메모리 상한).
const MAX_EDITS: usize = 4000;
const CONTEXT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Change {
    /// 설치본에만 있어요
    Added,
    /// 라이브러리에만 있어요
    Removed,
    Modified,
    Same,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    /// ' ' 같음, '-' 라이브러리에만, '+' 설치본에만
    pub op: char,
    pub a: Option<usize>,
    pub b: Option<usize>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    pub a_start: usize,
    pub b_start: usize,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub path: String,
    pub change: Change,
    /// 줄 비교를 했는지 (텍스트가 아니거나 너무 크면 false)
    pub text: bool,
    pub a_bytes: u64,
    pub b_bytes: u64,
    pub added: usize,
    pub removed: usize,
    pub hunks: Vec<Hunk>,
    /// 너무 많이 달라서 줄 비교를 줄였어요
    pub truncated: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirDiff {
    pub files: Vec<FileDiff>,
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
    pub same: usize,
}

fn read_text(p: &Path) -> Option<String> {
    let m = fs::metadata(p).ok()?;
    if m.len() > TEXT_MAX {
        return None;
    }
    let b = fs::read(p).ok()?;
    if b.iter().take(8000).any(|x| *x == 0) {
        return None;
    }
    String::from_utf8(b).ok()
}

fn lines(s: &str) -> Vec<&str> {
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    s.split_inclusive('\n').map(|l| l.trim_end_matches(['\n', '\r'])).collect()
}

/// Myers 차이. 같음/지움/더함 연산 목록을 돌려줘요. 너무 많이 다르면 `None`.
fn myers<'a>(a: &[&'a str], b: &[&'a str]) -> Option<Vec<(char, usize, usize)>> {
    let n = a.len() as isize;
    let m = b.len() as isize;
    let max = (n + m) as usize;
    let limit = max.min(MAX_EDITS);
    let off = limit as isize + 1;
    let mut v = vec![0isize; 2 * limit + 3];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    let mut found = None;
    for d in 0..=limit as isize {
        trace.push(v.clone());
        let mut k = -d;
        while k <= d {
            let i = (k + off) as usize;
            let mut x = if k == -d || (k != d && v[i - 1] < v[i + 1]) { v[i + 1] } else { v[i - 1] + 1 };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[i] = x;
            if x >= n && y >= m {
                found = Some(d);
                break;
            }
            k += 2;
        }
        if found.is_some() {
            break;
        }
    }
    let d_end = found?;
    // 거꾸로 따라가며 연산을 모아요.
    let mut ops = Vec::new();
    let (mut x, mut y) = (n, m);
    for d in (1..=d_end).rev() {
        let v = &trace[d as usize];
        let k = x - y;
        let i = |k: isize| (k + off) as usize;
        let prev_k = if k == -d || (k != d && v[i(k - 1)] < v[i(k + 1)]) { k + 1 } else { k - 1 };
        let prev_x = v[i(prev_k)];
        let prev_y = prev_x - prev_k;
        while x > prev_x && y > prev_y {
            x -= 1;
            y -= 1;
            ops.push((' ', x as usize, y as usize));
        }
        if x == prev_x {
            y -= 1;
            ops.push(('+', x as usize, y as usize));
        } else {
            x -= 1;
            ops.push(('-', x as usize, y as usize));
        }
        x = prev_x;
        y = prev_y;
    }
    while x > 0 && y > 0 {
        x -= 1;
        y -= 1;
        ops.push((' ', x as usize, y as usize));
    }
    ops.reverse();
    Some(ops)
}

/// 두 텍스트의 줄 차이를 묶음으로 만들어요. (묶음, 더한 줄, 지운 줄, 줄였는지)
pub fn text_hunks(a: &str, b: &str) -> (Vec<Hunk>, usize, usize, bool) {
    let la = lines(a);
    let lb = lines(b);
    let Some(ops) = myers(&la, &lb) else {
        // 너무 많이 달라요: 통째로 바뀐 것으로 보여줘요 (앞 200줄씩).
        let mut lines_out: Vec<Line> = la.iter().take(200).enumerate().map(|(i, t)| Line { op: '-', a: Some(i + 1), b: None, text: t.to_string() }).collect();
        lines_out.extend(lb.iter().take(200).enumerate().map(|(i, t)| Line { op: '+', a: None, b: Some(i + 1), text: t.to_string() }));
        return (vec![Hunk { a_start: 1, b_start: 1, lines: lines_out }], lb.len(), la.len(), true);
    };
    let added = ops.iter().filter(|o| o.0 == '+').count();
    let removed = ops.iter().filter(|o| o.0 == '-').count();
    let changed: Vec<usize> = ops.iter().enumerate().filter(|(_, o)| o.0 != ' ').map(|(i, _)| i).collect();
    let mut hunks = Vec::new();
    let mut idx = 0;
    while idx < changed.len() {
        let start = changed[idx].saturating_sub(CONTEXT);
        let mut end = changed[idx];
        while idx + 1 < changed.len() && changed[idx + 1] <= end + 2 * CONTEXT + 1 {
            idx += 1;
            end = changed[idx];
        }
        let end = (end + CONTEXT).min(ops.len() - 1);
        let lines_out: Vec<Line> = ops[start..=end]
            .iter()
            .map(|&(op, ai, bi)| Line {
                op,
                a: (op != '+').then_some(ai + 1),
                b: (op != '-').then_some(bi + 1),
                text: if op == '+' { lb[bi].to_string() } else { la[ai].to_string() },
            })
            .collect();
        let (a0, b0) = (ops[start].1 + 1, ops[start].2 + 1);
        hunks.push(Hunk { a_start: a0, b_start: b0, lines: lines_out });
        idx += 1;
    }
    (hunks, added, removed, false)
}

/// 라이브러리 폴더(a)와 설치본 폴더(b)를 비교해요. 다른 파일이 먼저, 같은 파일은 뒤에 와요.
pub fn compare(a_dir: &Path, b_dir: &Path) -> DirDiff {
    let fa: BTreeSet<String> = fsutil::list_files(a_dir).unwrap_or_default().into_iter().collect();
    let fb: BTreeSet<String> = fsutil::list_files(b_dir).unwrap_or_default().into_iter().collect();
    let mut out = DirDiff::default();
    for path in fa.union(&fb) {
        let pa = join_rel(a_dir, path);
        let pb = join_rel(b_dir, path);
        let a_bytes = if fa.contains(path) { fs::metadata(&pa).map(|m| m.len()).unwrap_or(0) } else { 0 };
        let b_bytes = if fb.contains(path) { fs::metadata(&pb).map(|m| m.len()).unwrap_or(0) } else { 0 };
        let change = match (fa.contains(path), fb.contains(path)) {
            (true, false) => Change::Removed,
            (false, true) => Change::Added,
            _ => {
                if fs::read(&pa).ok() == fs::read(&pb).ok() {
                    Change::Same
                } else {
                    Change::Modified
                }
            }
        };
        let ta = if change == Change::Added { Some(String::new()) } else { read_text(&pa) };
        let tb = if change == Change::Removed { Some(String::new()) } else { read_text(&pb) };
        let mut f = FileDiff { path: path.clone(), change, text: false, a_bytes, b_bytes, added: 0, removed: 0, hunks: vec![], truncated: false };
        if change != Change::Same {
            if let (Some(ta), Some(tb)) = (ta, tb) {
                let (hunks, added, removed, truncated) = text_hunks(&ta, &tb);
                f = FileDiff { text: true, hunks, added, removed, truncated, ..f };
            }
        } else {
            f.text = true;
        }
        match change {
            Change::Added => out.added += 1,
            Change::Removed => out.removed += 1,
            Change::Modified => out.modified += 1,
            Change::Same => out.same += 1,
        }
        out.files.push(f);
    }
    out.files.sort_by_key(|f| (f.change == Change::Same, f.path != crate::skill::SKILL_FILE, f.path.clone()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsutil::testdir::TestDir;

    fn render(h: &[Hunk]) -> String {
        h.iter().flat_map(|h| h.lines.iter().map(|l| format!("{}{}", l.op, l.text))).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn line_diff_basics() {
        let (h, add, rem, tr) = text_hunks("a\nb\nc\n", "a\nB\nc\nd\n");
        assert_eq!((add, rem, tr), (2, 1, false));
        assert_eq!(render(&h), " a\n-b\n+B\n c\n+d");
        let l = &h[0].lines;
        assert_eq!((l[1].a, l[1].b), (Some(2), None));
        assert_eq!((l[2].a, l[2].b), (None, Some(2)));
        let (h, add, rem, _) = text_hunks("same\n", "same\n");
        assert!(h.is_empty() && add == 0 && rem == 0);
        let (h, ..) = text_hunks("", "new\n");
        assert_eq!(render(&h), "+new");
    }

    #[test]
    fn hunks_keep_three_lines_of_context() {
        let a: String = (1..=30).map(|i| format!("{i}\n")).collect();
        let edit = |m: &[(u32, &str)]| -> String {
            (1..=30u32).map(|i| m.iter().find(|x| x.0 == i).map(|x| format!("{}\n", x.1)).unwrap_or(format!("{i}\n"))).collect()
        };
        let b = edit(&[(5, "five"), (25, "twenty-five")]);
        let (h, ..) = text_hunks(&a, &b);
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].lines.first().unwrap().text, "2");
        assert_eq!(h[0].lines.last().unwrap().text, "8");
        assert_eq!((h[1].a_start, h[1].b_start), (22, 22));
        let (h, ..) = text_hunks(&a, &edit(&[(5, "x"), (10, "y")]));
        assert_eq!(h.len(), 1, "가까운 변경은 한 묶음");
    }

    #[test]
    fn crlf_is_same_line() {
        let (h, ..) = text_hunks("a\r\nb\r\n", "a\nb\n");
        assert!(h.is_empty());
    }

    #[test]
    fn compares_folders() {
        let t = TestDir::new("diff");
        t.write("lib/SKILL.md", "---\nname: s\n---\nold\n");
        t.write("lib/gone.md", "x");
        t.write("lib/same.md", "same");
        t.write("lib/bin.dat", "\0\x01");
        t.write("inst/SKILL.md", "---\nname: s\n---\nnew\n");
        t.write("inst/same.md", "same");
        t.write("inst/notes.md", "team rule\n");
        t.write("inst/bin.dat", "\0\x02");
        let d = compare(&t.path().join("lib"), &t.path().join("inst"));
        assert_eq!((d.added, d.removed, d.modified, d.same), (1, 1, 2, 1));
        let paths: Vec<_> = d.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["SKILL.md", "bin.dat", "gone.md", "notes.md", "same.md"]);
        let skill = &d.files[0];
        assert_eq!((skill.change, skill.added, skill.removed), (Change::Modified, 1, 1));
        assert!(!d.files[1].text, "바이너리는 줄 비교 안 함");
        assert_eq!(d.files[3].change, Change::Added);
        assert_eq!(render(&d.files[3].hunks), "+team rule");
    }
}
