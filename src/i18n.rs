//! Application language selection and lightweight runtime translation helpers.

use std::str::FromStr;
use std::sync::atomic::{AtomicU8, Ordering};

const LANGUAGE_EN_US: u8 = 0;
const LANGUAGE_ZH_CN: u8 = 1;

static CURRENT_LANGUAGE: AtomicU8 = AtomicU8::new(LANGUAGE_EN_US);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    EnUs,
    ZhCn,
}

impl Language {
    pub fn code(self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::ZhCn => "zh-CN",
        }
    }

    pub fn selector(self) -> &'static str {
        match self {
            Self::EnUs => "en",
            Self::ZhCn => "zh",
        }
    }

    pub fn parse_raw(value: &str) -> Option<Self> {
        let normalized = normalize_language_tag(value);
        if normalized.is_empty() {
            return None;
        }

        if normalized == "zh" || normalized.starts_with("zh-") {
            return Some(Self::ZhCn);
        }

        if normalized == "en" || normalized.starts_with("en-") {
            return Some(Self::EnUs);
        }

        None
    }

    pub fn is_zh(self) -> bool {
        matches!(self, Self::ZhCn)
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::EnUs
    }
}

impl FromStr for Language {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse_raw(value).ok_or_else(|| "supported values: zh, en".to_string())
    }
}

pub fn normalize_language_tag(value: &str) -> String {
    value
        .trim()
        .split('.')
        .next()
        .unwrap_or_default()
        .replace('_', "-")
        .to_ascii_lowercase()
}

pub fn set_current_language(lang: Language) {
    let code = match lang {
        Language::EnUs => LANGUAGE_EN_US,
        Language::ZhCn => LANGUAGE_ZH_CN,
    };
    CURRENT_LANGUAGE.store(code, Ordering::SeqCst);
}

pub fn current_language() -> Language {
    match CURRENT_LANGUAGE.load(Ordering::SeqCst) {
        LANGUAGE_ZH_CN => Language::ZhCn,
        _ => Language::EnUs,
    }
}

pub fn is_zh() -> bool {
    current_language().is_zh()
}

pub fn pick<'a>(zh: &'a str, en: &'a str) -> &'a str {
    if is_zh() {
        zh
    } else {
        en
    }
}
