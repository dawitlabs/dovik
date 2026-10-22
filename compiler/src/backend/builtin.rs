use super::value::tag_of;
use super::Emitter;
use crate::builtins::Builtin;
use crate::ir::{Function, Value};
use crate::token::Span;
use crate::types::Type;

impl Emitter<'_> {
    pub(crate) fn builtin(
        &mut self,
        function: &Function,
        dest: Option<usize>,
        which: Builtin,
        args: &[(Value, Type)],
        span: Span,
    ) {
        let (line, column) = self.position(span);
        let mut words: Vec<String> = Vec::new();
        let mut tags: Vec<u8> = Vec::new();

        for (value, ty) in args {
            tags.push(tag_of(ty));
            words.push(self.as_word(value, ty));
        }

        match which {
            Builtin::Append => self.line(&format!(
                "  call void @dovik_list_push(i64 {}, i64 {})",
                words[0], words[1]
            )),
            Builtin::Insert => self.line(&format!(
                "  call void @dovik_list_insert(i64 {}, i64 {}, i64 {}, i32 {line}, i32 {column})",
                words[0], words[1], words[2]
            )),
            Builtin::RemoveAt => {
                let word = self.next_temp();
                self.line(&format!(
                    "  {word} = call i64 @dovik_list_remove_at(i64 {}, i64 {}, i32 {line}, i32 {column})",
                    words[0], words[1]
                ));

                if let Some(dest) = dest {
                    let ty = function.registers[dest].clone();
                    self.from_word(dest, &word, &ty);
                }
            }
            Builtin::HasKey => self.boolean_call(
                dest,
                &format!(
                    "@dovik_dict_has_key(i64 {}, i8 {}, i64 {})",
                    words[0], tags[1], words[1]
                ),
            ),
            Builtin::RemoveKey => self.boolean_call(
                dest,
                &format!(
                    "@dovik_dict_remove_key(i64 {}, i8 {}, i64 {})",
                    words[0], tags[1], words[1]
                ),
            ),
            Builtin::ToString => {
                if let Some(dest) = dest {
                    self.line(&format!(
                        "  %r{dest} = call i64 @dovik_to_string(i8 {}, i64 {})",
                        tags[0], words[0]
                    ));
                }
            }
            Builtin::ParseInt => {
                if let Some(dest) = dest {
                    self.line(&format!(
                        "  %r{dest} = call i64 @dovik_parse_int(i64 {}, i32 {line}, i32 {column})",
                        words[0]
                    ));
                }
            }
            Builtin::ParseFloat => {
                if let Some(dest) = dest {
                    self.line(&format!(
                        "  %r{dest} = call double @dovik_parse_float(i64 {}, i32 {line}, i32 {column})",
                        words[0]
                    ));
                }
            }
            Builtin::CanParseInt => {
                self.boolean_call(dest, &format!("@dovik_can_parse_int(i64 {})", words[0]))
            }
            Builtin::CanParseFloat => {
                self.boolean_call(dest, &format!("@dovik_can_parse_float(i64 {})", words[0]))
            }
            Builtin::FileExists => {
                self.boolean_call(dest, &format!("@dovik_file_exists(i64 {})", words[0]))
            }
            Builtin::Contains => self.boolean_call(
                dest,
                &format!("@dovik_str_contains(i64 {}, i64 {})", words[0], words[1]),
            ),
            Builtin::StartsWith => self.boolean_call(
                dest,
                &format!("@dovik_str_starts_with(i64 {}, i64 {})", words[0], words[1]),
            ),
            Builtin::EndsWith => self.boolean_call(
                dest,
                &format!("@dovik_str_ends_with(i64 {}, i64 {})", words[0], words[1]),
            ),
            Builtin::Split => self.plain_call(dest, &format!(
                "i64 @dovik_str_split(i64 {}, i64 {})", words[0], words[1]
            )),
            Builtin::Join => self.plain_call(dest, &format!(
                "i64 @dovik_str_join(i64 {}, i64 {})", words[0], words[1]
            )),
            Builtin::Trim => {
                self.plain_call(dest, &format!("i64 @dovik_str_trim(i64 {})", words[0]))
            }
            Builtin::Upper => {
                self.plain_call(dest, &format!("i64 @dovik_str_upper(i64 {})", words[0]))
            }
            Builtin::Lower => {
                self.plain_call(dest, &format!("i64 @dovik_str_lower(i64 {})", words[0]))
            }
            Builtin::Substring => self.plain_call(dest, &format!(
                "i64 @dovik_str_substring(i64 {}, i64 {}, i64 {}, i32 {line}, i32 {column})",
                words[0], words[1], words[2]
            )),
            Builtin::Replace => self.plain_call(dest, &format!(
                "i64 @dovik_str_replace(i64 {}, i64 {}, i64 {})",
                words[0], words[1], words[2]
            )),
            Builtin::ReadInput => self.plain_call(
                dest,
                &format!("i64 @dovik_read_input(i32 {line}, i32 {column})"),
            ),
            Builtin::Assert => {
                let flag = self.next_temp();
                self.line(&format!("  {flag} = zext i1 {} to i8", super::value::literal(&args[0].0)));
                self.line(&format!(
                    "  call void @dovik_assert(i8 {flag}, i32 {line}, i32 {column})"
                ));
            }
            Builtin::Now => self.plain_call(dest, "double @dovik_now()"),
            Builtin::Clock => self.plain_call(dest, "double @dovik_clock()"),
            Builtin::Sort => {
                self.line(&format!("  call void @dovik_list_sort(i64 {})", words[0]))
            }
            Builtin::ListDir => self.plain_call(dest, &format!(
                "i64 @dovik_list_dir(i64 {}, i32 {line}, i32 {column})", words[0]
            )),
            Builtin::IsDir => {
                self.boolean_call(dest, &format!("@dovik_is_dir(i64 {})", words[0]))
            }
            Builtin::CreateDir => self.line(&format!(
                "  call void @dovik_create_dir(i64 {}, i32 {line}, i32 {column})", words[0]
            )),
            Builtin::RemoveDir => self.line(&format!(
                "  call void @dovik_remove_dir(i64 {}, i32 {line}, i32 {column})", words[0]
            )),
            Builtin::RemoveFile => self.line(&format!(
                "  call void @dovik_remove_file(i64 {}, i32 {line}, i32 {column})", words[0]
            )),
            Builtin::IndexOf => self.plain_call(dest, &format!(
                "i64 @dovik_str_index_of(i64 {}, i64 {}, i32 {line}, i32 {column})",
                words[0], words[1]
            )),
            Builtin::ReadFile => {
                if let Some(dest) = dest {
                    self.line(&format!(
                        "  %r{dest} = call i64 @dovik_read_file(i64 {}, i32 {line}, i32 {column})",
                        words[0]
                    ));
                }
            }
            Builtin::WriteFile => self.line(&format!(
                "  call void @dovik_write_file(i64 {}, i64 {}, i32 {line}, i32 {column})",
                words[0], words[1]
            )),
            Builtin::AppendFile => self.line(&format!(
                "  call void @dovik_append_file(i64 {}, i64 {}, i32 {line}, i32 {column})",
                words[0], words[1]
            )),
            Builtin::ArgumentCount => {
                if let Some(dest) = dest {
                    self.line(&format!("  %r{dest} = call i64 @dovik_argument_count()"));
                }
            }
            Builtin::Argument => {
                if let Some(dest) = dest {
                    self.line(&format!(
                        "  %r{dest} = call i64 @dovik_argument(i64 {}, i32 {line}, i32 {column})",
                        words[0]
                    ));
                }
            }
            _ => {}
        }
    }

    fn plain_call(&mut self, dest: Option<usize>, call: &str) {
        match dest {
            Some(dest) => self.line(&format!("  %r{dest} = call {call}")),
            None => {
                let temp = self.next_temp();
                self.line(&format!("  {temp} = call {call}"));
            }
        }
    }

    fn boolean_call(&mut self, dest: Option<usize>, call: &str) {
        let byte = self.next_temp();
        self.line(&format!("  {byte} = call i8 {call}"));

        if let Some(dest) = dest {
            self.line(&format!("  %r{dest} = icmp ne i8 {byte}, 0"));
        }
    }
}
