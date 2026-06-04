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
