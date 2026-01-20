use iced::keyboard::{self, Key, Modifiers};

/// Maps an Iced keyboard event to a VT sequence of bytes.
/// Returns None if the key should be ignored.
pub fn map_key_to_input(key: Key, modifiers: Modifiers) -> Option<Vec<u8>> {
    match key {
        Key::Character(c) => {
            let s = c.as_str();

            // Handle Control + Character (e.g. Ctrl+C = 0x03)
            // ONLY if Ctrl is pressed (not Shift or other modifiers)
            if modifiers.control() && !modifiers.shift() && !modifiers.alt() {
                let bytes = s.as_bytes();
                if bytes.len() == 1 {
                    let b = bytes[0];
                    if b >= b'a' && b <= b'z' {
                        return Some(vec![b - b'a' + 1]);
                    } else if b >= b'A' && b <= b'Z' {
                        return Some(vec![b - b'A' + 1]);
                    } else if b == b'[' {
                        return Some(vec![0x1b]); // ESC
                    } else if b == b'\\' {
                        return Some(vec![0x1c]);
                    } else if b == b']' {
                        return Some(vec![0x1d]);
                    } else if b == b'^' {
                        return Some(vec![0x1e]);
                    } else if b == b'_' {
                        return Some(vec![0x1f]);
                    }
                }
                // If Ctrl is pressed but we don't recognize the combination, ignore it
                return None;
            }

            // Manual Shift mapping for characters that Iced doesn't handle correctly on macOS
            // When Shift is pressed but the character isn't already shifted
            if modifiers.shift() && s.len() == 1 {
                let shifted = match s.chars().next().unwrap() {
                    // Number row
                    '1' => '!',
                    '2' => '@',
                    '3' => '#',
                    '4' => '$',
                    '5' => '%',
                    '6' => '^',
                    '7' => '&',
                    '8' => '*',
                    '9' => '(',
                    '0' => ')',
                    // Punctuation
                    '-' => '_',
                    '=' => '+',
                    '[' => '{',
                    ']' => '}',
                    '\\' => '|',
                    ';' => ':',
                    '\'' => '"',
                    ',' => '<',
                    '.' => '>',
                    '/' => '?',
                    '`' => '~',
                    // If already uppercase/shifted, keep it
                    c => c,
                };
                return Some(shifted.to_string().as_bytes().to_vec());
            }

            // Standard character (including Shift+character like ':', '!', etc.)
            return Some(s.as_bytes().to_vec());
        }

        Key::Named(named) => match named {
            keyboard::key::Named::Enter => Some(vec![0x0d]), // CR
            keyboard::key::Named::Backspace => Some(vec![0x7f]), // DEL (usually used for backspace in modern terms)
            keyboard::key::Named::Tab => Some(vec![0x09]),
            keyboard::key::Named::Space => Some(vec![0x20]),
            keyboard::key::Named::Escape => Some(vec![0x1b]),

            keyboard::key::Named::ArrowUp => Some(vec![0x1b, b'O', b'A']),
            keyboard::key::Named::ArrowDown => Some(vec![0x1b, b'O', b'B']),
            keyboard::key::Named::ArrowRight => Some(vec![0x1b, b'O', b'C']),
            keyboard::key::Named::ArrowLeft => Some(vec![0x1b, b'O', b'D']),

            keyboard::key::Named::Home => Some(vec![0x1b, b'[', b'H']),
            keyboard::key::Named::End => Some(vec![0x1b, b'[', b'F']),
            keyboard::key::Named::PageUp => Some(vec![0x1b, b'[', b'5', b'~']),
            keyboard::key::Named::PageDown => Some(vec![0x1b, b'[', b'6', b'~']),
            keyboard::key::Named::Insert => Some(vec![0x1b, b'[', b'2', b'~']),
            keyboard::key::Named::Delete => Some(vec![0x1b, b'[', b'3', b'~']),

            keyboard::key::Named::F1 => Some(vec![0x1b, b'O', b'P']),
            keyboard::key::Named::F2 => Some(vec![0x1b, b'O', b'Q']),
            keyboard::key::Named::F3 => Some(vec![0x1b, b'O', b'R']),
            keyboard::key::Named::F4 => Some(vec![0x1b, b'O', b'S']),
            keyboard::key::Named::F5 => Some(vec![0x1b, b'[', b'1', b'5', b'~']),
            keyboard::key::Named::F6 => Some(vec![0x1b, b'[', b'1', b'7', b'~']),
            keyboard::key::Named::F7 => Some(vec![0x1b, b'[', b'1', b'8', b'~']),
            keyboard::key::Named::F8 => Some(vec![0x1b, b'[', b'1', b'9', b'~']),
            keyboard::key::Named::F9 => Some(vec![0x1b, b'[', b'2', b'0', b'~']),
            keyboard::key::Named::F10 => Some(vec![0x1b, b'[', b'2', b'1', b'~']),
            keyboard::key::Named::F11 => Some(vec![0x1b, b'[', b'2', b'3', b'~']),
            keyboard::key::Named::F12 => Some(vec![0x1b, b'[', b'2', b'4', b'~']),

            _ => None,
        },
        _ => None,
    }
}
