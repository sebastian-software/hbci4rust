use std::collections::BTreeMap;

use super::{
    DefinitionKind, ProtocolSyntax, SyntaxChild, SyntaxChildKind, SyntaxDefinition, SyntaxValidSet,
    SyntaxValue,
};
use crate::error::{HbciError, HbciErrorKind, HbciResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbciMessage {
    root: SyntaxElement,
}

impl HbciMessage {
    pub fn from_syntax(syntax: &ProtocolSyntax, name: &str) -> HbciResult<Self> {
        let definition = syntax.definition(name).ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::Protocol,
                format!("message definition {name} is not defined"),
            )
        })?;
        if definition.kind != DefinitionKind::Msg {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("definition {name} is not a message definition"),
            ));
        }

        let mut builder = MessageBuilder::new(syntax);
        Ok(Self {
            root: builder.build_definition(definition, name, name, None, Occurrence::root())?,
        })
    }

    pub fn name(&self) -> &str {
        self.root.name()
    }

    pub fn path(&self) -> &str {
        self.root.path()
    }

    pub fn root(&self) -> &SyntaxElement {
        &self.root
    }

    pub fn element(&self, path: &str) -> Option<&SyntaxElement> {
        self.root.element(path)
    }

    pub fn value(&self, path: &str) -> Option<&str> {
        self.element(path).and_then(SyntaxElement::value)
    }

    pub fn set_value(&mut self, path: &str, value: impl Into<String>) -> HbciResult<()> {
        self.root.set_value(path, value.into())
    }

    pub fn values(&self) -> BTreeMap<String, String> {
        let mut values = BTreeMap::new();
        self.root.collect_values(&mut values);
        values
    }

    pub fn data(&self) -> BTreeMap<String, String> {
        let prefix = format!("{}.", self.name());
        self.values()
            .into_iter()
            .filter_map(|(path, value)| {
                path.strip_prefix(&prefix)
                    .map(|path| (path.to_owned(), value))
            })
            .collect()
    }

    pub fn to_fints_string(&self) -> HbciResult<String> {
        self.root
            .render(None, true)
            .map(|rendered| rendered.unwrap_or_default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxElement {
    kind: SyntaxElementKind,
    type_name: String,
    name: String,
    path: String,
    occurrence_index: usize,
    min_num: usize,
    max_num: usize,
    needs_request_tag: bool,
    requested: bool,
    value: Option<String>,
    valid_values: Vec<String>,
    children: Vec<SyntaxElement>,
}

impl SyntaxElement {
    fn new(
        kind: SyntaxElementKind,
        type_name: String,
        name: String,
        parent_path: Option<&str>,
        occurrence: Occurrence,
    ) -> Self {
        let path = path_with_counter(parent_path, &name, occurrence.index);
        Self {
            kind,
            type_name,
            name,
            path,
            occurrence_index: occurrence.index,
            min_num: occurrence.min_num,
            max_num: occurrence.max_num,
            needs_request_tag: false,
            requested: false,
            value: None,
            valid_values: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn kind(&self) -> SyntaxElementKind {
        self.kind
    }

    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn occurrence_index(&self) -> usize {
        self.occurrence_index
    }

    pub fn min_num(&self) -> usize {
        self.min_num
    }

    pub fn max_num(&self) -> usize {
        self.max_num
    }

    pub fn needs_request_tag(&self) -> bool {
        self.needs_request_tag
    }

    pub fn is_requested(&self) -> bool {
        self.requested
    }

    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    pub fn valid_values(&self) -> &[String] {
        &self.valid_values
    }

    pub fn children(&self) -> &[SyntaxElement] {
        &self.children
    }

    pub fn to_fints_string(&self) -> HbciResult<String> {
        self.render(None, true)
            .map(|rendered| rendered.unwrap_or_default())
    }

    pub fn element(&self, path: &str) -> Option<&SyntaxElement> {
        if self.path == path {
            return Some(self);
        }

        self.children.iter().find_map(|child| child.element(path))
    }

    fn element_mut(&mut self, path: &str) -> Option<&mut SyntaxElement> {
        if self.path == path {
            return Some(self);
        }

        self.children
            .iter_mut()
            .find_map(|child| child.element_mut(path))
    }

    fn set_value(&mut self, path: &str, value: String) -> HbciResult<()> {
        let element = self.element_mut(path).ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::Protocol,
                format!("message element path {path} is not defined"),
            )
        })?;

        match element.kind {
            SyntaxElementKind::De => {
                element.value = Some(value);
                Ok(())
            }
            _ if value == "requested" => {
                element.requested = true;
                Ok(())
            }
            _ => Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("message element path {path} does not refer to a data element"),
            )),
        }
    }

    fn add_valid_values(&mut self, path: &str, values: &[String]) -> HbciResult<()> {
        let element = self.element_mut(path).ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::Protocol,
                format!("message valid-values path {path} is not defined"),
            )
        })?;

        if element.kind != SyntaxElementKind::De {
            return Err(HbciError::new(
                HbciErrorKind::Protocol,
                format!("message valid-values path {path} does not refer to a data element"),
            ));
        }

        element.valid_values.extend(values.iter().cloned());
        Ok(())
    }

    fn collect_values(&self, values: &mut BTreeMap<String, String>) {
        if let (SyntaxElementKind::De, Some(value)) = (self.kind, &self.value) {
            values.insert(self.path.clone(), value.clone());
        }

        for child in &self.children {
            child.collect_values(values);
        }
    }

    fn has_any_value(&self) -> bool {
        self.value.is_some() || self.requested || self.children.iter().any(Self::has_any_value)
    }

    fn render(
        &self,
        parent_kind: Option<SyntaxElementKind>,
        required: bool,
    ) -> HbciResult<Option<String>> {
        if self.needs_request_tag && !self.requested {
            return Ok(None);
        }

        match self.kind {
            SyntaxElementKind::De => self.render_de(required),
            SyntaxElementKind::Msg => self.render_msg(),
            SyntaxElementKind::Seg => {
                self.render_delimited_children(parent_kind, '+', true, required)
            }
            SyntaxElementKind::Deg => {
                let trim_trailing = parent_kind != Some(SyntaxElementKind::Deg);
                self.render_delimited_children(parent_kind, ':', trim_trailing, required)
            }
            SyntaxElementKind::Sf => self.render_concatenated_children(required),
        }
    }

    fn render_de(&self, required: bool) -> HbciResult<Option<String>> {
        match &self.value {
            Some(value) => Ok(Some(render_data_element(&self.type_name, value)?)),
            None if required => Err(HbciError::new(
                HbciErrorKind::Protocol,
                format!("message element {} has no value", self.path),
            )),
            None => Ok(None),
        }
    }

    fn render_msg(&self) -> HbciResult<Option<String>> {
        let mut rendered = String::new();
        for child in &self.children {
            match child.render_optional_aware(Some(self.kind))? {
                Some(child_rendered) => rendered.push_str(&child_rendered),
                None if child.min_num > 0 => {
                    return Err(HbciError::new(
                        HbciErrorKind::Protocol,
                        format!("required message element {} is not renderable", child.path),
                    ));
                }
                None => {}
            }
        }
        Ok(Some(rendered))
    }

    fn render_concatenated_children(&self, required: bool) -> HbciResult<Option<String>> {
        let mut rendered = String::new();
        let mut has_rendered_child = false;

        for child in &self.children {
            match child.render_optional_aware(Some(self.kind))? {
                Some(child_rendered) => {
                    has_rendered_child = true;
                    rendered.push_str(&child_rendered);
                }
                None if child.min_num > 0 => {
                    return Err(HbciError::new(
                        HbciErrorKind::Protocol,
                        format!("required message element {} is not renderable", child.path),
                    ));
                }
                None => {}
            }
        }

        if has_rendered_child || required {
            Ok(Some(rendered))
        } else {
            Ok(None)
        }
    }

    fn render_delimited_children(
        &self,
        parent_kind: Option<SyntaxElementKind>,
        delimiter: char,
        trim_trailing_empty: bool,
        required: bool,
    ) -> HbciResult<Option<String>> {
        let mut rendered_children = Vec::with_capacity(self.children.len());
        let mut has_rendered_child = false;

        for child in &self.children {
            match child.render_optional_aware(Some(self.kind))? {
                Some(rendered) => {
                    has_rendered_child |= !rendered.is_empty();
                    rendered_children.push(rendered);
                }
                None if child.min_num > 0 => {
                    return Err(HbciError::new(
                        HbciErrorKind::Protocol,
                        format!("required message element {} is not renderable", child.path),
                    ));
                }
                None => rendered_children.push(String::new()),
            }
        }

        if trim_trailing_empty {
            while rendered_children.last().is_some_and(String::is_empty) {
                rendered_children.pop();
            }
        }

        if !has_rendered_child && !required {
            return Ok(None);
        }

        let mut rendered = rendered_children.join(&delimiter.to_string());
        if self.kind == SyntaxElementKind::Seg {
            rendered.push('\'');
        }

        if rendered.is_empty() && parent_kind.is_some() && !required {
            Ok(None)
        } else {
            Ok(Some(rendered))
        }
    }

    fn render_optional_aware(
        &self,
        parent_kind: Option<SyntaxElementKind>,
    ) -> HbciResult<Option<String>> {
        let required = self.min_num > 0;
        match self.render(parent_kind, required) {
            Ok(rendered) => Ok(rendered),
            Err(_) if !required && !self.has_any_value() => Ok(None),
            Err(err) => Err(err),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxElementKind {
    Msg,
    Seg,
    Deg,
    De,
    Sf,
}

impl TryFrom<DefinitionKind> for SyntaxElementKind {
    type Error = HbciError;

    fn try_from(value: DefinitionKind) -> Result<Self, Self::Error> {
        match value {
            DefinitionKind::Msg => Ok(Self::Msg),
            DefinitionKind::Seg => Ok(Self::Seg),
            DefinitionKind::Deg => Ok(Self::Deg),
            DefinitionKind::Sf => Ok(Self::Sf),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Occurrence {
    index: usize,
    min_num: usize,
    max_num: usize,
}

impl Occurrence {
    fn root() -> Self {
        Self {
            index: 0,
            min_num: 1,
            max_num: 1,
        }
    }

    fn from_child(child: &SyntaxChild, index: usize) -> HbciResult<Self> {
        Ok(Self {
            index,
            min_num: parse_occurrence_attr(&child.min_num, 1, "minnum")?,
            max_num: parse_occurrence_attr(&child.max_num, 1, "maxnum")?,
        })
    }
}

struct MessageBuilder<'a> {
    syntax: &'a ProtocolSyntax,
    stack: Vec<String>,
}

impl<'a> MessageBuilder<'a> {
    fn new(syntax: &'a ProtocolSyntax) -> Self {
        Self {
            syntax,
            stack: Vec::new(),
        }
    }

    fn build_definition(
        &mut self,
        definition: &SyntaxDefinition,
        type_name: &str,
        name: &str,
        parent_path: Option<&str>,
        occurrence: Occurrence,
    ) -> HbciResult<SyntaxElement> {
        if self.stack.iter().any(|entry| entry == type_name) {
            return Err(HbciError::new(
                HbciErrorKind::Protocol,
                format!("message syntax definition {type_name} is recursive"),
            ));
        }

        self.stack.push(type_name.to_owned());
        let element =
            self.build_definition_inner(definition, type_name, name, parent_path, occurrence);
        self.stack.pop();
        element
    }

    fn build_definition_inner(
        &mut self,
        definition: &SyntaxDefinition,
        type_name: &str,
        name: &str,
        parent_path: Option<&str>,
        occurrence: Occurrence,
    ) -> HbciResult<SyntaxElement> {
        let mut element = SyntaxElement::new(
            SyntaxElementKind::try_from(definition.kind)?,
            type_name.to_owned(),
            name.to_owned(),
            parent_path,
            occurrence,
        );
        element.needs_request_tag = definition.needs_request_tag;

        for child in &definition.children {
            let child_count = parse_occurrence_attr(&child.min_num, 1, "minnum")?.max(1);
            for child_index in 0..child_count {
                let child_element = self.build_child(child, element.path(), child_index)?;
                element.children.push(child_element);
            }
        }

        self.apply_values(&mut element, &definition.values)?;
        self.apply_valids(&mut element, &definition.valids)?;
        Ok(element)
    }

    fn build_child(
        &mut self,
        child: &SyntaxChild,
        parent_path: &str,
        occurrence_index: usize,
    ) -> HbciResult<SyntaxElement> {
        let child_name = child.name.as_deref().unwrap_or(&child.type_name);
        let occurrence = Occurrence::from_child(child, occurrence_index)?;

        match child.kind {
            SyntaxChildKind::De => Ok(SyntaxElement::new(
                SyntaxElementKind::De,
                child.type_name.clone(),
                child_name.to_owned(),
                Some(parent_path),
                occurrence,
            )),
            SyntaxChildKind::Deg => self.build_referenced_definition(
                child,
                child_name,
                parent_path,
                occurrence,
                DefinitionKind::Deg,
            ),
            SyntaxChildKind::Seg => self.build_referenced_definition(
                child,
                child_name,
                parent_path,
                occurrence,
                DefinitionKind::Seg,
            ),
            SyntaxChildKind::Sf => self.build_referenced_definition(
                child,
                child_name,
                parent_path,
                occurrence,
                DefinitionKind::Sf,
            ),
            SyntaxChildKind::EntityRef => Err(HbciError::new(
                HbciErrorKind::Protocol,
                format!(
                    "message syntax still contains unexpanded entity reference {}",
                    child.type_name
                ),
            )),
        }
    }

    fn build_referenced_definition(
        &mut self,
        child: &SyntaxChild,
        name: &str,
        parent_path: &str,
        occurrence: Occurrence,
        expected_kind: DefinitionKind,
    ) -> HbciResult<SyntaxElement> {
        let definition = self.syntax.definition(&child.type_name).ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::Protocol,
                format!("syntax definition {} is not defined", child.type_name),
            )
        })?;
        if definition.kind != expected_kind {
            return Err(HbciError::new(
                HbciErrorKind::Protocol,
                format!(
                    "syntax definition {} has kind {:?}, expected {:?}",
                    child.type_name, definition.kind, expected_kind
                ),
            ));
        }

        self.build_definition(
            definition,
            &child.type_name,
            name,
            Some(parent_path),
            occurrence,
        )
    }

    fn apply_values(&self, element: &mut SyntaxElement, values: &[SyntaxValue]) -> HbciResult<()> {
        for value in values {
            let path = format!("{}.{}", element.path(), value.path);
            element.set_value(&path, value.value.clone())?;
        }
        Ok(())
    }

    fn apply_valids(
        &self,
        element: &mut SyntaxElement,
        valids: &[SyntaxValidSet],
    ) -> HbciResult<()> {
        for valid in valids {
            let path = format!("{}.{}", element.path(), valid.path);
            element.add_valid_values(&path, &valid.values)?;
        }
        Ok(())
    }
}

fn parse_occurrence_attr(
    value: &Option<String>,
    default: usize,
    attribute_name: &str,
) -> HbciResult<usize> {
    let Some(value) = value else {
        return Ok(default);
    };

    value.parse().map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::Protocol,
            format!("invalid protocol {attribute_name} value: {value}"),
            err,
        )
    })
}

fn path_with_counter(parent_path: Option<&str>, name: &str, index: usize) -> String {
    match parent_path {
        Some(parent_path) if !parent_path.is_empty() => {
            format!("{parent_path}.{}", with_counter(name, index))
        }
        _ => with_counter(name, index),
    }
}

fn with_counter(name: &str, index: usize) -> String {
    if index == 0 {
        name.to_owned()
    } else {
        format!("{name}_{}", index + 1)
    }
}

fn render_data_element(type_name: &str, value: &str) -> HbciResult<String> {
    if type_name == "Bin" {
        return render_binary_data_element(value);
    }

    Ok(quote_data_element(value))
}

fn quote_data_element(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '+' | ':' | '\'' | '?' | '@' => quoted.push('?'),
            _ => {}
        }
        quoted.push(character);
    }
    quoted
}

fn render_binary_data_element(value: &str) -> HbciResult<String> {
    let Some(payload) = value.strip_prefix('B') else {
        return Err(HbciError::new(
            HbciErrorKind::Unsupported,
            "numeric binary data element rendering is not ported yet",
        ));
    };

    Ok(format!("@{}@{}", payload.len(), payload))
}
