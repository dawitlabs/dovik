use super::value::{literal, tag_of};
use super::Emitter;
use crate::ir::{Function, InstKind};
use crate::token::Span;

impl Emitter<'_> {
    pub(crate) fn collection(&mut self, function: &Function, kind: &InstKind, span: Span) {
        match kind {
            InstKind::ListNew { dest, element, items } => {
                let tag = tag_of(element);

                if items.is_empty() {
                    self.line(&format!("  %r{dest} = call i64 @dovik_list_new(i8 {tag})"));
                    return;
                }

                let count = items.len();
                self.line(&format!(
                    "  %r{dest} = call i64 @dovik_list_new_sized(i8 {tag}, i64 {count})"
                ));

                for item in items {
                    let word = self.as_word(item, element);
                    self.line(&format!(
                        "  call void @dovik_list_push_fast(i64 %r{dest}, i64 {word})"
                    ));
                }
            }
            InstKind::ListGet { dest, list, index, checked } => {
                let (line, column) = self.position(span);
                let word = self.next_temp();

                if *checked {
                    self.line(&format!(
                        "  {word} = call i64 @dovik_list_get(i64 {}, i64 {}, i32 {line}, i32 {column})",
                        literal(list),
                        literal(index)
                    ));
                } else {
                    self.line(&format!(
                        "  {word} = call i64 @dovik_list_get_fast(i64 {}, i64 {})",
                        literal(list),
                        literal(index)
                    ));
                }
                let ty = function.registers[*dest].clone();
                self.from_word(*dest, &word, &ty);
            }
            InstKind::ListSet { list, index, value } => {
                let (line, column) = self.position(span);
                let ty = self.type_of(function, value);
                let word = self.as_word(value, &ty);
                self.line(&format!(
                    "  call void @dovik_list_set(i64 {}, i64 {}, i64 {word}, i32 {line}, i32 {column})",
                    literal(list),
                    literal(index)
                ));
            }
            InstKind::DictNew { dest, key, value, entries } => {
                let ktag = tag_of(key);
                let vtag = tag_of(value);
                self.line(&format!(
                    "  %r{dest} = call i64 @dovik_dict_new(i8 {ktag}, i8 {vtag})"
                ));
                for (entry_key, entry_value) in entries {
                    let kword = self.as_word(entry_key, key);
                    let vword = self.as_word(entry_value, value);
                    self.line(&format!(
                        "  call void @dovik_dict_set(i64 %r{dest}, i8 {ktag}, i64 {kword}, i8 {vtag}, i64 {vword})"
                    ));
                }
            }
            InstKind::DictGet { dest, dict, key } => {
                let (line, column) = self.position(span);
                let key_type = self.type_of(function, key);
                let ktag = tag_of(&key_type);
                let kword = self.as_word(key, &key_type);
                let word = self.next_temp();
                self.line(&format!(
                    "  {word} = call i64 @dovik_dict_get(i64 {}, i8 {ktag}, i64 {kword}, i32 {line}, i32 {column})",
                    literal(dict)
                ));
                let ty = function.registers[*dest].clone();
                self.from_word(*dest, &word, &ty);
            }
            InstKind::DictSet { dict, key, value } => {
                let key_type = self.type_of(function, key);
                let value_type = self.type_of(function, value);
                let ktag = tag_of(&key_type);
                let vtag = tag_of(&value_type);
                let kword = self.as_word(key, &key_type);
                let vword = self.as_word(value, &value_type);
                self.line(&format!(
                    "  call void @dovik_dict_set(i64 {}, i8 {ktag}, i64 {kword}, i8 {vtag}, i64 {vword})",
                    literal(dict)
                ));
            }
            _ => {}
        }
    }
}
