use crate::{
    asset::{Asset, AssetMemoryUsage},
    error::{AssetError, AssetLoadError},
    gpu_upload::GpuResourceHandle,
    id::AssetTypeId,
    loader::{AssetLoader, LoadContext, LoadedAsset, LoaderSettings},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shader {
    pub stages: Vec<ShaderStageSource>,
    pub reflection: Option<ShaderReflection>,
    pub gpu: Option<GpuResourceHandle>,
}

impl Asset for Shader {
    const TYPE_NAME: &'static str = "Shader";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0003);
}

impl AssetMemoryUsage for Shader {
    fn cpu_bytes(&self) -> u64 {
        self.stages
            .iter()
            .map(|stage| match &stage.source {
                ShaderSource::Wgsl(source) | ShaderSource::Glsl(source) => source.len() as u64,
                ShaderSource::Spirv(words) => (words.len() * 4) as u64,
            })
            .sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShaderStageSource {
    pub stage: ShaderStage,
    pub source: ShaderSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShaderSource {
    Wgsl(String),
    Glsl(String),
    Spirv(Vec<u32>),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShaderReflection {
    pub bind_groups: Vec<String>,
    pub vertex_inputs: Vec<String>,
}

pub struct ShaderLoader;

impl ShaderLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ShaderLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoader for ShaderLoader {
    fn name(&self) -> &'static str {
        "ShaderLoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["wgsl", "glsl", "shader"]
    }

    fn asset_type(&self) -> AssetTypeId {
        Shader::TYPE_ID
    }

    fn load(
        &self,
        ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Decode {
            message: format!("shader source must be UTF-8: {error}"),
        })?;
        if source.trim().is_empty() {
            return Err(AssetError::Decode {
                message: "shader source is empty".to_owned(),
            });
        }
        let uncommented_lines = shader_source_lines_without_comments(source)?;
        validate_shader_source_structure(&uncommented_lines)?;
        let stage = shader_stage_from_label(ctx.path().label())?;
        let reflection = reflect_wgsl_shader(&uncommented_lines, stage)?;
        let shader = Shader {
            stages: vec![ShaderStageSource {
                stage,
                source: ShaderSource::Wgsl(source.to_owned()),
            }],
            reflection: shader_reflection_or_none(reflection),
            gpu: None,
        };
        Ok(LoadedAsset::new(shader).shader_upload(
            ctx.id(),
            Shader::TYPE_ID,
            Some(ctx.path().display_string()),
            bytes.to_vec(),
        ))
    }
}

fn shader_reflection_or_none(reflection: ShaderReflection) -> Option<ShaderReflection> {
    if reflection.bind_groups.is_empty() && reflection.vertex_inputs.is_empty() {
        None
    } else {
        Some(reflection)
    }
}

fn shader_stage_from_label(label: Option<&str>) -> Result<ShaderStage, AssetError> {
    match label {
        Some(label) if label.eq_ignore_ascii_case("vertex") => Ok(ShaderStage::Vertex),
        Some(label) if label.eq_ignore_ascii_case("fragment") => Ok(ShaderStage::Fragment),
        Some(label) if label.eq_ignore_ascii_case("compute") => Ok(ShaderStage::Compute),
        Some(label) => Err(AssetError::Decode {
            message: format!("unsupported shader stage label `{label}`"),
        }),
        None => Ok(ShaderStage::Fragment),
    }
}

fn shader_source_lines_without_comments(source: &str) -> Result<Vec<String>, AssetError> {
    let mut lines = Vec::new();
    let mut block_comment_start = None;
    for (line_index, line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let mut cleaned = String::with_capacity(line.len());
        let mut offset = 0;
        while offset < line.len() {
            if block_comment_start.is_some() {
                let rest = &line[offset..];
                if let Some(end) = rest.find("*/") {
                    offset += end + 2;
                    block_comment_start = None;
                } else {
                    offset = line.len();
                }
                continue;
            }

            let rest = &line[offset..];
            let line_comment = rest.find("//");
            let block_comment = rest.find("/*");
            match (line_comment, block_comment) {
                (Some(line_comment), Some(block_comment)) if line_comment < block_comment => {
                    cleaned.push_str(&rest[..line_comment]);
                    offset = line.len();
                }
                (_, Some(block_comment)) => {
                    cleaned.push_str(&rest[..block_comment]);
                    block_comment_start = Some((line_number, offset + block_comment + 1));
                    offset += block_comment + 2;
                }
                (Some(line_comment), None) => {
                    cleaned.push_str(&rest[..line_comment]);
                    offset = line.len();
                }
                (None, None) => {
                    cleaned.push_str(rest);
                    offset = line.len();
                }
            }
        }
        lines.push(cleaned);
    }

    if let Some((line, column)) = block_comment_start {
        return Err(AssetError::Decode {
            message: format!(
                "shader source has unclosed block comment opened on line {line}, column {column}"
            ),
        });
    }

    Ok(lines)
}

fn validate_shader_source_structure(lines: &[String]) -> Result<(), AssetError> {
    let mut stack = Vec::new();
    for (line_index, line) in lines.iter().enumerate() {
        let line_number = line_index + 1;
        for (column_index, character) in line.chars().enumerate() {
            let column_number = column_index + 1;
            match character {
                '(' | '[' | '{' => stack.push((character, line_number, column_number)),
                ')' | ']' | '}' => {
                    let Some((open, open_line, open_column)) = stack.pop() else {
                        return Err(AssetError::Decode {
                            message: format!(
                                "shader source has unmatched `{character}` on line {line_number}, column {column_number}"
                            ),
                        });
                    };
                    if !shader_brackets_match(open, character) {
                        return Err(AssetError::Decode {
                            message: format!(
                                "shader source closes `{open}` from line {open_line}, column {open_column} with `{character}` on line {line_number}, column {column_number}"
                            ),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    if let Some((open, line, column)) = stack.pop() {
        return Err(AssetError::Decode {
            message: format!(
                "shader source has unclosed `{open}` opened on line {line}, column {column}"
            ),
        });
    }

    Ok(())
}

fn shader_brackets_match(open: char, close: char) -> bool {
    matches!((open, close), ('(', ')') | ('[', ']') | ('{', '}'))
}

fn reflect_wgsl_shader(
    lines: &[String],
    stage: ShaderStage,
) -> Result<ShaderReflection, AssetError> {
    let mut reflection = ShaderReflection::default();
    let mut pending_group = None;
    let mut pending_binding = None;
    for (line_index, line) in lines.iter().enumerate() {
        let line_number = line_index + 1;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let group = shader_attribute_u32(line, "group", line_number)?;
        let binding = shader_attribute_u32(line, "binding", line_number)?;
        if let Some(group) = group {
            pending_group = Some((group, line_number));
        }
        if let Some(binding) = binding {
            pending_binding = Some((binding, line_number));
        }

        if group.is_some()
            || binding.is_some()
            || pending_group.is_some()
            || pending_binding.is_some()
        {
            if shader_var_name(line).is_some() {
                let (group, binding) = match (pending_group, pending_binding) {
                    (Some((group, _)), Some((binding, _))) => (group, binding),
                    _ => {
                        return Err(AssetError::Decode {
                            message: format!(
                                "shader resource binding on line {} must include both @group and @binding",
                                pending_shader_binding_line(&pending_group, &pending_binding, line_number)
                            ),
                        });
                    }
                };
                pending_group = None;
                pending_binding = None;
                let label = shader_binding_label(line, group, binding);
                push_unique(&mut reflection.bind_groups, label);
            } else if group.is_none() && binding.is_none() && !line.starts_with('@') {
                return Err(AssetError::Decode {
                    message: format!(
                        "shader resource binding on line {} must include both @group and @binding",
                        pending_shader_binding_line(&pending_group, &pending_binding, line_number)
                    ),
                });
            }
        }

        if stage == ShaderStage::Vertex {
            if let Some(location) = shader_attribute_u32(line, "location", line_number)? {
                if let Some(name) = shader_vertex_input_name(line, "location") {
                    push_unique(
                        &mut reflection.vertex_inputs,
                        format!("location={location},name={name}"),
                    );
                }
            }
        }
    }

    if pending_group.is_some() || pending_binding.is_some() {
        return Err(AssetError::Decode {
            message: format!(
                "shader resource binding on line {} must include both @group and @binding",
                pending_shader_binding_line(&pending_group, &pending_binding, lines.len())
            ),
        });
    }

    Ok(reflection)
}

fn pending_shader_binding_line(
    group: &Option<(u32, usize)>,
    binding: &Option<(u32, usize)>,
    fallback: usize,
) -> usize {
    group
        .as_ref()
        .map(|(_, line)| *line)
        .or_else(|| binding.as_ref().map(|(_, line)| *line))
        .unwrap_or(fallback)
}

fn shader_attribute_u32(
    line: &str,
    attribute: &str,
    line_number: usize,
) -> Result<Option<u32>, AssetError> {
    let pattern = format!("@{attribute}(");
    let Some(start) = line.find(&pattern) else {
        return Ok(None);
    };
    let value_start = start + pattern.len();
    let Some(value_end) = line[value_start..]
        .find(')')
        .map(|offset| value_start + offset)
    else {
        return Err(AssetError::Decode {
            message: format!("shader @{attribute} attribute on line {line_number} is missing `)`"),
        });
    };
    let value = line[value_start..value_end].trim();
    value
        .parse::<u32>()
        .map(Some)
        .map_err(|error| AssetError::Decode {
            message: format!(
                "invalid shader @{attribute} attribute on line {line_number}: {error}"
            ),
        })
}

fn shader_binding_label(line: &str, group: u32, binding: u32) -> String {
    if let Some(name) = shader_var_name(line) {
        format!("group={group},binding={binding},name={name}")
    } else {
        format!("group={group},binding={binding}")
    }
}

fn shader_var_name(line: &str) -> Option<&str> {
    let var_start = find_shader_token(line, "var")?;
    let mut rest = line[var_start + "var".len()..].trim_start();
    if let Some(after_address_space) = rest.strip_prefix('<') {
        let end = after_address_space.find('>')?;
        rest = after_address_space[end + 1..].trim_start();
    }
    shader_identifier_prefix(rest)
}

fn shader_vertex_input_name<'a>(line: &'a str, attribute: &str) -> Option<&'a str> {
    let pattern = format!("@{attribute}(");
    let start = line.find(&pattern)?;
    let after_location = &line[start + pattern.len()..];
    let close = after_location.find(')')?;
    shader_identifier_prefix(after_location[close + 1..].trim_start())
}

fn find_shader_token(line: &str, token: &str) -> Option<usize> {
    line.match_indices(token)
        .find(|(index, _)| {
            let before = line[..*index].chars().next_back();
            let after = line[*index + token.len()..].chars().next();
            !before.is_some_and(is_shader_identifier_char)
                && !after.is_some_and(is_shader_identifier_char)
        })
        .map(|(index, _)| index)
}

fn shader_identifier_prefix(value: &str) -> Option<&str> {
    let end = value
        .char_indices()
        .take_while(|(_, character)| is_shader_identifier_char(*character))
        .last()
        .map(|(index, character)| index + character.len_utf8())?;
    let name = &value[..end];
    if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        None
    } else {
        Some(name)
    }
}

fn is_shader_identifier_char(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}
