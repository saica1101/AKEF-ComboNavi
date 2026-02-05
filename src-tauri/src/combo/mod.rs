//! Combo command parser and types
//! 
//! Handles parsing of the custom combo file format used by AKEF ComboNavi.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Input type for a combo command
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputType {
    /// Normal tap input - triggered on key down
    Tap,
    /// Hold input - requires holding key for specified duration
    Hold { duration_ms: u64 },
}

impl Default for InputType {
    fn default() -> Self {
        InputType::Tap
    }
}

/// Key identifier for combo commands
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KeyIdentifier {
    /// Number key (1-4 for operator skills)
    Number(u8),
    /// E key for chain/link attacks
    Chain,
    /// Left click or configured heavy attack key
    HeavyAttack,
    /// Mouse left button
    MouseLeft,
}

impl KeyIdentifier {
    /// Parse key identifier from string
    pub fn from_str(s: &str) -> Option<Self> {
        let s = s.trim();
        
        // Check for number keys
        if let Ok(num) = s.parse::<u8>() {
            if (1..=9).contains(&num) {
                return Some(KeyIdentifier::Number(num));
            }
        }
        
        // Check for special keys
        match s.to_uppercase().as_str() {
            "E" => Some(KeyIdentifier::Chain),
            "L" => Some(KeyIdentifier::HeavyAttack),
            _ => None,
        }
    }
}

/// A single combo command entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComboCommand {
    /// Key to press
    pub key: KeyIdentifier,
    /// Type of input (tap or hold)
    pub input_type: InputType,
    /// Character name (e.g., "リーフォン")
    pub character: String,
    /// Skill type (e.g., "必殺技", "戦技", "連携")
    pub skill_type: String,
    /// Optional memo/note
    pub memo: String,
    /// Whether this is a title/header line
    pub is_title: bool,
}

/// Parsed combo file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComboFile {
    /// Title/preset name (first line with #)
    pub title: String,
    /// List of combo commands
    pub commands: Vec<ComboCommand>,
}

/// Parse error types
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Empty file
    EmptyFile,
    /// Invalid line format
    InvalidFormat { line: usize, content: String },
    /// Invalid key identifier
    InvalidKey { line: usize, key: String },
    /// IO error
    IoError(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::EmptyFile => write!(f, "File is empty"),
            ParseError::InvalidFormat { line, content } => {
                write!(f, "Invalid format at line {}: {}", line, content)
            }
            ParseError::InvalidKey { line, key } => {
                write!(f, "Invalid key '{}' at line {}", key, line)
            }
            ParseError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

/// Default hold duration in milliseconds (300ms)
const DEFAULT_HOLD_DURATION_MS: u64 = 300;

/// EOF marker
const EOF_MARKER: &str = "!!!!!";

/// Parse a single line of the combo file
/// 
/// Format: `KEY,CHARACTER,SKILL_TYPE,MEMO|`
/// - KEY: `[0-9]` for tap, `U[0-9]` for hold, `E` for chain, `L` for heavy attack
/// - `#` prefix indicates title line
fn parse_line(line: &str, line_number: usize) -> Result<Option<ComboCommand>, ParseError> {
    let line = line.trim();
    
    // Skip empty lines
    if line.is_empty() {
        return Ok(None);
    }
    
    // Check for EOF marker
    if line.starts_with(EOF_MARKER) {
        return Ok(None);
    }
    
    // Remove trailing pipe if present
    let line = line.trim_end_matches('|').trim();
    
    // Split by comma
    let parts: Vec<&str> = line.split(',').collect();
    if parts.is_empty() {
        return Err(ParseError::InvalidFormat {
            line: line_number,
            content: line.to_string(),
        });
    }
    
    let key_str = parts[0].trim();
    let character = parts.get(1).unwrap_or(&"").trim().to_string();
    let skill_type = parts.get(2).unwrap_or(&"").trim().to_string();
    let memo = parts.get(3).unwrap_or(&"").trim().to_string();
    
    // Check if this is a title line
    if key_str.starts_with('#') {
        return Ok(Some(ComboCommand {
            key: KeyIdentifier::Number(0),
            input_type: InputType::Tap,
            character,
            skill_type,
            memo,
            is_title: true,
        }));
    }
    
    // Parse key and input type
    let (key, input_type) = if key_str.starts_with('U') || key_str.starts_with('u') {
        // Ultimate/Hold input
        let key_part = &key_str[1..];
        let key = KeyIdentifier::from_str(key_part).ok_or_else(|| ParseError::InvalidKey {
            line: line_number,
            key: key_str.to_string(),
        })?;
        (key, InputType::Hold { duration_ms: DEFAULT_HOLD_DURATION_MS })
    } else {
        // Normal tap input
        let key = KeyIdentifier::from_str(key_str).ok_or_else(|| ParseError::InvalidKey {
            line: line_number,
            key: key_str.to_string(),
        })?;
        (key, InputType::Tap)
    };
    
    Ok(Some(ComboCommand {
        key,
        input_type,
        character,
        skill_type,
        memo,
        is_title: false,
    }))
}

/// Parse combo file content
pub fn parse_combo_content(content: &str) -> Result<ComboFile, ParseError> {
    let mut title = String::new();
    let mut commands = Vec::new();
    
    for (line_number, line) in content.lines().enumerate() {
        if let Some(cmd) = parse_line(line, line_number + 1)? {
            if cmd.is_title && title.is_empty() {
                // Use character field as title for # lines
                title = if cmd.character.is_empty() {
                    "Untitled".to_string()
                } else {
                    cmd.character.clone()
                };
            }
            commands.push(cmd);
        }
    }
    
    if commands.is_empty() {
        return Err(ParseError::EmptyFile);
    }
    
    Ok(ComboFile { title, commands })
}

/// Parse combo file from path
pub fn parse_combo_file<P: AsRef<Path>>(path: P) -> Result<ComboFile, ParseError> {
    let content = std::fs::read_to_string(path).map_err(|e| ParseError::IoError(e.to_string()))?;
    parse_combo_content(&content)
}

/// Serialize combo file to string
pub fn serialize_combo_file(combo: &ComboFile) -> String {
    let mut output = String::new();
    
    for cmd in &combo.commands {
        let key_str = if cmd.is_title {
            "#".to_string()
        } else {
            let key_base = match &cmd.key {
                KeyIdentifier::Number(n) => n.to_string(),
                KeyIdentifier::Chain => "E".to_string(),
                KeyIdentifier::HeavyAttack => "L".to_string(),
                KeyIdentifier::MouseLeft => "L".to_string(),
            };
            
            match &cmd.input_type {
                InputType::Tap => key_base,
                InputType::Hold { .. } => format!("U{}", key_base),
            }
        };
        
        output.push_str(&format!(
            "{},{},{},{}|\n",
            key_str, cmd.character, cmd.skill_type, cmd.memo
        ));
    }
    
    output.push_str(EOF_MARKER);
    output.push('\n');
    
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_tap_command() {
        let content = "2,リーフォン,戦技,|";
        let result = parse_combo_content(content).unwrap();
        assert_eq!(result.commands.len(), 1);
        assert_eq!(result.commands[0].key, KeyIdentifier::Number(2));
        assert!(matches!(result.commands[0].input_type, InputType::Tap));
        assert_eq!(result.commands[0].character, "リーフォン");
    }
    
    #[test]
    fn test_parse_hold_command() {
        let content = "U2,リーフォン,必殺技,|";
        let result = parse_combo_content(content).unwrap();
        assert_eq!(result.commands.len(), 1);
        assert_eq!(result.commands[0].key, KeyIdentifier::Number(2));
        assert!(matches!(result.commands[0].input_type, InputType::Hold { duration_ms: 300 }));
    }
    
    #[test]
    fn test_parse_chain_command() {
        let content = "E,チェン,連携,|";
        let result = parse_combo_content(content).unwrap();
        assert_eq!(result.commands.len(), 1);
        assert_eq!(result.commands[0].key, KeyIdentifier::Chain);
    }
    
    #[test]
    fn test_parse_full_file() {
        let content = r#"#,物理,,|
U2,リーフォン,必殺技,|
2,リーフォン,戦技,|
E,チェン,連携,|
!!!!!"#;
        let result = parse_combo_content(content).unwrap();
        assert_eq!(result.title, "物理");
        assert_eq!(result.commands.len(), 4); // Including title line
    }
}
