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
            _ => {}
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
