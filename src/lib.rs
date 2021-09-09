#![no_std]

// TODO(robert) floating point number types
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum JSONValueType {
    String,
    Number,
    Object,
    Array,
    Bool,
    Null,
}

#[derive(Copy, Clone, Debug)]
pub struct JSONValue<'a> {
    pub contents: &'a str,
    pub value_type: JSONValueType,
}

fn trim_start(value: &str) -> (&str, usize) {
    let value_len = value.len();
    // NOTE(robert): This trims from the "start" which may be different for RTL languages.  What do
    // we do for JSON?
    let value = value.trim_start();
    (value, value_len - value.len())
}

impl<'a> JSONValue<'a> {
    pub fn parse(contents: &'a str) -> Result<(JSONValue, usize), &'static str> {
        let (contents, whitespace_trimmed) = trim_start(contents);
        let (value_type, value_len) = match contents.chars().next() {
            Some('{') => {
                let mut value_len = 1;
                let mut contents = &contents[value_len..];
                while !contents.is_empty() {
                    if contents.trim_start().starts_with('}') {
                        value_len += trim_start(contents).1 + 1;
                        break;
                    }
                    let (item, item_len) = JSONValue::parse(contents)?;
                    if item.value_type != JSONValueType::String {
                        return Err("Cannot parse object key");
                    }
                    let (new_contents, whitespace) = trim_start(&contents[item_len..]);
                    contents = new_contents;
                    value_len += item_len + whitespace;
                    if contents.is_empty() {
                        return Err("End of stream while parsing object");
                    } else if contents.starts_with(':') {
                        value_len += 1;
                        contents = &contents[1..];
                    } else {
                        return Err("Illegal token while parsing object");
                    }

                    let (_, item_len) = JSONValue::parse(contents)?;
                    let (new_contents, whitespace) = trim_start(&contents[item_len..]);
                    contents = new_contents;
                    value_len += item_len + whitespace;
                    if contents.is_empty() {
                        return Err("End of stream while parsing object");
                    } else if contents.starts_with(',') {
                        value_len += 1;
                        contents = &contents[1..];
                    } else if !contents.starts_with('}') {
                        return Err("Illegal token while parsing object");
                    }
                }
                (JSONValueType::Object, value_len)
            }
            Some('[') => {
                let mut value_len = 1;
                let mut contents = &contents[value_len..];
                while !contents.is_empty() {
                    if contents.trim_start().starts_with(']') {
                        value_len += trim_start(contents).1 + 1;
                        break;
                    }
                    let (_, item_len) = JSONValue::parse(contents)?;
                    let (new_contents, whitespace) = trim_start(&contents[item_len..]);
                    contents = new_contents;
                    value_len += item_len + whitespace;
                    if contents.is_empty() {
                        return Err("End of stream while parsing array");
                    } else if contents.starts_with(',') {
                        value_len += 1;
                        contents = &contents[1..];
                    } else if !contents.starts_with(']') {
                        return Err("Illegal token while parsing array");
                    }
                }
                (JSONValueType::Array, value_len)
            }
            Some('"') => {
                let mut value_len = 1;
                let mut is_escaped = false;
                for chr in contents[1..].chars() {
                    value_len += chr.len_utf8();
                    if chr == '"' && !is_escaped {
                        break;
                    } else if chr == '\\' {
                        is_escaped = !is_escaped;
                    } else {
                        is_escaped = false;
                    }
                }
                (JSONValueType::String, value_len)
            }
            Some('0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '-') => {
                let mut value_len = 0;
                for chr in contents.chars() {
                    match chr {
                        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '-' | 'e'
                        | 'E' | '.' => {
                            if chr == '-' && value_len > 0 {
                                return Err("Unexpected '-' while parsing number");
                            }
                            value_len += chr.len_utf8();
                        }
                        _ => {
                            break;
                        }
                    }
                }
                (JSONValueType::Number, value_len)
            }
            Some('t') => {
                if &contents[..4] != "true" {
                    return Err("Unrecognised token");
                }
                (JSONValueType::Bool, 4)
            }
            Some('f') => {
                if &contents[..5] != "false" {
                    return Err("Unrecognised token");
                }
                (JSONValueType::Bool, 5)
            }
            Some('n') => {
                if &contents[..4] != "null" {
                    return Err("Unrecognised token");
                }
                (JSONValueType::Null, 4)
            }
            _ => {
                return Err("Could not interpret start of token");
            }
        };
        Ok((
            JSONValue {
                contents: &contents[..value_len],
                value_type,
            },
            whitespace_trimmed + value_len,
        ))
    }

    pub fn read_integer(&self) -> Result<isize, &'static str> {
        if self.value_type != JSONValueType::Number {
            return Err("Cannot parse value as integer");
        }
        let mut ans = 0;
        let neg = self.contents.starts_with('-');
        for chr in self.contents.chars() {
            if !chr.is_digit(10) {
                return Err("Cannot parse value as integer");
            }
            ans = ans * 10 + chr.to_digit(10).unwrap() as isize;
        }

        Ok(if neg { -ans } else { ans })
    }

    pub fn read_float(&self) -> Result<f32, &'static str> {
        if self.value_type != JSONValueType::Number {
            return Err("Cannot parse value as float");
        }
        let mut ans = 0.0;
        let neg = self.contents.starts_with('-');
        let mut integer = true;
        let mut column = 0.1;
        for chr in self.contents.chars() {
            if chr.is_digit(10) {
                if integer {
                    ans *= 10.0;
                    ans += chr.to_digit(10).unwrap() as f32;
                } else {
                    ans += chr.to_digit(10).unwrap() as f32 * column;
                    column /= 10.;
                }
            }
            if chr == '.' {
                integer = false;
            }
        }

        Ok(if neg { -ans } else { ans })
    }

    // TODO(robert): String can be escaped and all manner of trickery.  We need to deal with that
    // by returning some kind of iterator over characters here.
    pub fn read_string(&self) -> Result<&str, &'static str> {
        if self.value_type != JSONValueType::String {
            return Err("Cannot parse value as string");
        }
        Ok(&self.contents[1..self.contents.len() - 1])
    }

    // TODO(robert): This should be an iterator of `JSONValue`s
    // TODO(robert): Handle out of bounds
    pub fn get_nth_array_item(&self, n: usize) -> Result<JSONValue, &'static str> {
        if self.value_type != JSONValueType::Array {
            return Err("Cannot parse value as an array");
        }
        let mut contents = &self.contents[1..];
        for _ in 0..n {
            let (_, value_len) = JSONValue::parse(contents).unwrap();
            contents = &contents[value_len..].trim_start()[1..];
        }
        Ok(JSONValue::parse(contents)?.0)
    }

    // TODO(robert): This should be an iterator of `JSONValue`s
    pub fn get_key_value(&self, key: &str) -> Result<JSONValue, &'static str> {
        if self.value_type != JSONValueType::Object {
            return Err("Cannot parse value as an object");
        }
        let mut contents = &self.contents[1..];
        while !contents.is_empty() {
            let (this_key, key_len) = JSONValue::parse(contents).unwrap();
            contents = &contents[key_len..].trim_start()[1..];
            if this_key.read_string().unwrap() == key {
                return Ok(JSONValue::parse(contents)?.0);
            } else {
                let (_, value_len) = JSONValue::parse(contents).unwrap();
                contents = &contents[value_len..].trim_start()[1..];
            }
        }
        Err("Key not found")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn integer() {
        let (value, value_len) = JSONValue::parse("42").unwrap();
        assert_eq!(value.value_type, JSONValueType::Number);
        assert_eq!(value_len, 2);
        assert_eq!(value.read_integer(), Ok(42));
        assert!(value.read_string().is_err());
    }

    #[test]
    fn float() {
        let (value, value_len) = JSONValue::parse("3.141592").unwrap();
        assert_eq!(value.value_type, JSONValueType::Number);
        assert_eq!(value_len, "3.141592".len());
        assert!(value.read_integer().is_err());
        assert!(value.read_string().is_err());
        assert!((value.read_float().unwrap() - 3.141592).abs() < 0.0001);
    }

    #[test]
    fn string() {
        let (value, value_len) = JSONValue::parse("\"hello world\"").unwrap();
        assert_eq!(value.value_type, JSONValueType::String);
        assert_eq!(value_len, "\"hello world\"".len());
        assert!(value.read_integer().is_err());
        assert_eq!(value.read_string(), Ok("hello world"));
    }

    #[test]
    fn array() {
        let (value, value_len) = JSONValue::parse("[1,2,3]").unwrap();
        assert_eq!(value.value_type, JSONValueType::Array);
        assert_eq!(value_len, "[1,2,3]".len());
        let (value, value_len) = JSONValue::parse("[]").unwrap();
        assert_eq!(value.value_type, JSONValueType::Array);
        assert_eq!(value_len, "[]".len());
        let (value, value_len) = JSONValue::parse("  [\n  ]").unwrap();
        assert_eq!(value.value_type, JSONValueType::Array);
        assert_eq!(value_len, "  [\n  ]".len());
        let (value, value_len) = JSONValue::parse("[1  ,  2\t,\r3\n]").unwrap();
        assert_eq!(value.value_type, JSONValueType::Array);
        assert_eq!(value_len, "[1  ,  2\t,\r3\n]".len());

        assert!(value.read_integer().is_err());
        assert!(value.read_string().is_err());
        assert_eq!(value.get_nth_array_item(0).unwrap().read_integer(), Ok(1));
        assert_eq!(value.get_nth_array_item(1).unwrap().read_integer(), Ok(2));
        assert_eq!(value.get_nth_array_item(2).unwrap().read_integer(), Ok(3));
    }

    #[test]
    fn object() {
        let input = "{
        \"id\": 0,
        \"name\": \"Ginger Fuller\"
      }";
        let (value, value_len) = JSONValue::parse(input).unwrap();
        assert_eq!(value.value_type, JSONValueType::Object);
        assert_eq!(value_len, input.len());

        assert!(value.read_integer().is_err());
        assert!(value.read_string().is_err());
        assert_eq!(value.get_key_value("id").unwrap().read_integer(), Ok(0));
        assert_eq!(
            value.get_key_value("name").unwrap().read_string(),
            Ok("Ginger Fuller")
        );

        assert!(JSONValue::parse("{\"foo\":[{}]}").is_ok());
        assert!(JSONValue::parse("[{\"foo\":{}}]").is_ok());
    }
    #[test]

    fn this_broke_once() {
        assert!(JSONValue::parse(
            r##"
[{"a":{"email":"d@"},"m":"#20\n\n.\n"}]
    "##
        )
        .is_ok());
    }

    #[test]
    fn integer_whitespace() {
        let (value, value_len) = JSONValue::parse("  42	").unwrap();
        assert_eq!(value.value_type, JSONValueType::Number);
        assert_eq!(value_len, "  42".len());
        let (value, value_len) = JSONValue::parse("\n 42\r").unwrap();
        assert_eq!(value.value_type, JSONValueType::Number);
        assert_eq!(value_len, "\n 42".len());
    }

    #[test]
    fn string_whitespace() {
        let (value, value_len) = JSONValue::parse("  \"foo me a bar\"	").unwrap();
        assert_eq!(value.value_type, JSONValueType::String);
        assert_eq!(value_len, "  \"foo me a bar\"".len());
        let (value, value_len) = JSONValue::parse("\n \"a bar\n I said.\"\r").unwrap();
        assert_eq!(value.value_type, JSONValueType::String);
        assert_eq!(value_len, "\n \"a bar\n I said.\"".len());
    }
}
