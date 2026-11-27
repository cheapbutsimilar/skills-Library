//! Skills Library의 순수 로직이에요. OS·Tauri에 의존하지 않아서 단위 테스트로 검증해요.
//!
//! - [`skill`] — SKILL.md 머리말·본문 읽고 쓰기
//! - [`validate`] — 스킬 규칙 검사 (이름·설명 길이, 태그 …)
//! - [`library`] — 라이브러리 폴더 (만들기·저장·이름 바꾸기·가져오기)
//! - [`targets`] — 설치 위치 (Claude Code, Codex, 프로젝트)
//! - [`sync`] — 설치 기록과 동기화 상태
//! - [`ops`] — 설치·업데이트·제거·가져오기
//! - [`cleanup`] — 정리: 버전 통일, 예전 위치 옮기기
//! - [`snapshot`] — 화면에 보낼 전체 상태
//! - [`settings`] — 설정 스키마·검증·마이그레이션
//! - [`zipio`] — Claude 앱용 ZIP 내보내기·가져오기
//! - [`github`] — GitHub 저장소·폴더 URL에서 스킬 가져오기
//! - [`share`] — 공유 코드·링크로 라이브러리 주고받기 (ZIP 묶기·코드 읽기)
//! - [`diff`] — 라이브러리 원본과 설치본 비교
//! - [`i18n`] — 영어·한국어 메시지

#[macro_use]
pub mod i18n;
pub mod atomic;
pub mod diff;
pub mod github;
pub mod cleanup;
pub mod fsutil;
pub mod library;
pub mod ops;
pub mod settings;
pub mod share;
pub mod skill;
pub mod snapshot;
pub mod sync;
pub mod targets;
pub mod validate;
pub mod zipio;
