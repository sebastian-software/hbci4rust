use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::{HbciError, HbciErrorKind, HbciResult};

pub type Properties = BTreeMap<String, String>;

pub fn to_ins_code(hbci_code: Option<&str>) -> Option<String> {
    let hbci_code = hbci_code?;
    if hbci_code.chars().count() < 3 {
        return Some(hbci_code.to_owned());
    }

    Some(
        hbci_code
            .chars()
            .enumerate()
            .map(|(index, character)| if index == 1 { 'I' } else { character })
            .collect(),
    )
}

pub fn to_parameter_code(hbci_code: Option<&str>) -> Option<String> {
    to_ins_code(hbci_code).map(|code| format!("{code}S"))
}

pub fn to_boolean(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.trim().eq_ignore_ascii_case("true"))
}

pub fn has_text(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

pub fn join_strings(values: Option<&[&str]>, separator: Option<&str>) -> Option<String> {
    let values = values?;
    let mut result = String::new();
    let mut first = true;

    for value in values {
        if !first && let Some(separator) = separator {
            result.push_str(separator);
        }
        first = false;
        result.push_str(value);
    }

    Some(result)
}

pub fn safe_filename(filename: Option<&str>) -> Option<String> {
    let filename = filename?;
    if filename.is_empty() {
        return Some(String::new());
    }

    let absolute = absolute_path_for(filename);
    let original_name = absolute
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let safe_name = sanitize_filename_component(original_name);
    let new_path = match absolute.parent() {
        Some(parent) => parent.join(safe_name),
        None => PathBuf::from(safe_name),
    };
    let new_name = new_path.to_string_lossy().into_owned();

    if new_name == filename {
        Some(filename.to_owned())
    } else {
        Some(new_name)
    }
}

fn absolute_path_for(filename: &str) -> PathBuf {
    let path = Path::new(filename);
    if path.is_absolute() {
        return path.to_path_buf();
    }

    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn sanitize_filename_component(name: &str) -> String {
    name.chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
        })
        .take(25)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterQuery {
    query: Cow<'static, str>,
    params_set: bool,
}

impl ParameterQuery {
    pub const BPD_PINTAN_CAN1STEP: Self =
        Self::new_static("Params*.TAN2StepPar*.ParTAN2Step*.can1step", false);
    pub const BPD_PINTAN_ORDERHASHMODE: Self =
        Self::new_static("Params*.TAN2StepPar{0}.ParTAN2Step*.orderhashmode", true);
    pub const BPD_DECOUPLED_TIME_BEFORE_FIRST_STATUS_REQUEST: Self = Self::new_static(
        "Params*.TAN2StepPar*.ParTAN2Step*.TAN2StepParams*.decoupled_time_before_first_status_request",
        false,
    );
    pub const BPD_DECOUPLED_TIME_BEFORE_NEXT_STATUS_REQUEST: Self = Self::new_static(
        "Params*.TAN2StepPar*.ParTAN2Step*.TAN2StepParams*.decoupled_time_before_next_status_request",
        false,
    );
    pub const BPD_DECOUPLED_MAX_STATUS_REQUESTS: Self = Self::new_static(
        "Params*.TAN2StepPar*.ParTAN2Step*.TAN2StepParams*.decoupled_max_status_requests",
        false,
    );
    pub const BPD_PINTAN_PINLEN_MIN: Self =
        Self::new_static("Params*.PinTanPar*.ParPinTan*.pinlen_min", false);
    pub const BPD_PINTAN_PINLEN_MAX: Self =
        Self::new_static("Params*.PinTanPar*.ParPinTan*.pinlen_max", false);

    const fn new_static(query: &'static str, need_params: bool) -> Self {
        Self {
            query: Cow::Borrowed(query),
            params_set: !need_params,
        }
    }

    pub fn with_parameters(&self, parameters: &[&str]) -> Self {
        let mut query = self.query.to_string();
        for (index, parameter) in parameters.iter().enumerate() {
            query = query.replace(&format!("{{{index}}}"), parameter);
        }

        Self {
            query: Cow::Owned(query),
            params_set: true,
        }
    }

    pub fn get_query(&self) -> HbciResult<&str> {
        if !self.params_set {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("Parameters not set in query: {}", self.query),
            ));
        }

        Ok(self.query.as_ref())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ParameterFinder;

impl ParameterFinder {
    pub fn find(props: &Properties, path: Option<&str>) -> Properties {
        let Some(path) = path.filter(|path| !path.is_empty()) else {
            return props.clone();
        };

        let mut next = Properties::new();
        let Some(key) = path.split('.').next() else {
            return next;
        };
        let matcher = PathSegmentMatcher::new(key);

        for (name, value) in props {
            let Some(first_segment) = name.split('.').next() else {
                continue;
            };
            if !matcher.matches(first_segment) {
                continue;
            }
            let new_name = name
                .find('.')
                .map(|index| name[index + 1..].to_owned())
                .unwrap_or_else(|| name.clone());
            next.insert(new_name, value.clone());
        }

        let Some((_, tail)) = path.split_once('.') else {
            return next;
        };

        Self::find(&next, Some(tail))
    }

    pub fn find_query(props: &Properties, query: &ParameterQuery) -> HbciResult<Properties> {
        Ok(Self::find(props, Some(query.get_query()?)))
    }

    pub fn find_all(props: &Properties, path: Option<&str>) -> Properties {
        let Some(path) = path.filter(|path| !path.is_empty()) else {
            return props.clone();
        };

        let path_segments = path.split('.').collect::<Vec<_>>();
        let mut rest = props.clone();

        for (index, key) in path_segments.iter().enumerate().take(100) {
            let matcher = PathSegmentMatcher::new(key);
            for name in props.keys() {
                let matches = name
                    .split('.')
                    .nth(index)
                    .is_some_and(|segment| matcher.matches(segment));
                if !matches {
                    rest.remove(name);
                }
            }
        }

        rest
    }

    pub fn find_all_query(props: &Properties, query: &ParameterQuery) -> HbciResult<Properties> {
        Ok(Self::find_all(props, Some(query.get_query()?)))
    }

    pub fn get_value(
        props: &Properties,
        path: Option<&str>,
        default_value: Option<&str>,
    ) -> Option<String> {
        Self::find_all(props, path)
            .values()
            .next()
            .cloned()
            .or_else(|| default_value.map(str::to_owned))
    }

    pub fn get_value_query(
        props: &Properties,
        query: &ParameterQuery,
        default_value: Option<&str>,
    ) -> HbciResult<Option<String>> {
        Ok(Self::get_value(
            props,
            Some(query.get_query()?),
            default_value,
        ))
    }
}

#[derive(Debug, Clone, Copy)]
struct PathSegmentMatcher<'a> {
    value: &'a str,
    starts_with: bool,
    ends_with: bool,
}

impl<'a> PathSegmentMatcher<'a> {
    fn new(value: &'a str) -> Self {
        Self {
            starts_with: value.ends_with('*'),
            ends_with: value.starts_with('*'),
            value: value.trim_matches('*'),
        }
    }

    fn matches(self, candidate: &str) -> bool {
        match (self.starts_with, self.ends_with) {
            (true, false) => candidate.starts_with(self.value),
            (false, true) => candidate.ends_with(self.value),
            (true, true) => candidate.contains(self.value),
            (false, false) => candidate == self.value,
        }
    }
}
